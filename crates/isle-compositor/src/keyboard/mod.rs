//! Keyboards: each by its name with its profile and keymap, the seat's keymap following the
//! keyboard that types, and the PC swap of Alt and Super.

pub mod games;
pub mod inhibit;
pub mod inject;

use std::collections::HashMap;

use isle_keys::{
    engine::{Ctx, Engine, Out, Stroke},
    keyinfo::KeyInfo,
    keymap::{Iso, KeymapSpec, Layouts, MacVariants},
    profile::Profile,
    tables::{TableError, Tables},
};
use isle_settings::schema::{KeyboardModel, KeyboardProfile};
use smithay::{
    backend::input::{InputTime, KeyState},
    input::keyboard::{FilterResult, Keycode},
    utils::{SERIAL_COUNTER, Serial},
};
use tracing::{info, warn};

use crate::{
    binds::{Action, Bound},
    focus::Focus,
    state::State,
    worker::Event,
};

const KEY_LEFTALT: u32 = 56;
const KEY_LEFTMETA: u32 = 125;

/// Keys Isle sends itself, from the shell's surfaces.
pub const ISLE_DEVICE: &str = "isle";

pub struct Device {
    pub id: String,
    pub name: String,
    /// A profile the backend knows better than the name: a nested session's keys arrive as
    /// the host already made them.
    pub hint: Option<Profile>,
    pub profile: Profile,
    pub guessed: bool,
    pub swap: bool,
    pub spec: KeymapSpec,
    /// Has letter keys: power buttons and the like report as keyboards too.
    pub typing: bool,
}

#[derive(Default)]
pub struct Keyboards {
    pub devices: Vec<Device>,
    mac: Option<MacVariants>,
    /// Compiled keymaps as text with what they say of each key, by spec; `None` for a spec
    /// XKB refused.
    keymaps: HashMap<KeymapSpec, Option<(String, KeyInfo)>>,
    /// The spec the seat's keymap was made from.
    pub active: Option<KeymapSpec>,
    /// Whether the seat has had a keymap of Isle's, so Num Lock starts from the settings once.
    started: bool,
    /// The keyboard that typed last.
    last: Option<String>,
    /// Keys held, by device and evdev code, with the code after the swap.
    held: HashMap<(String, u32), u32>,
    pub engine: Engine,
    /// The system's layout, read once at start.
    pub system: Layouts,
    /// The shipped translation tables with the user's over them.
    tables: Option<Tables>,
    /// Why the user's tables are not in force, when they are not.
    pub tables_error: Option<TableError>,
}

impl Keyboards {
    fn mac(&mut self) -> &MacVariants {
        self.mac.get_or_insert_with(MacVariants::load)
    }

    pub fn get(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    fn compiled(&mut self, spec: &KeymapSpec) -> Option<&(String, KeyInfo)> {
        self.keymaps
            .entry(spec.clone())
            .or_insert_with(|| {
                let context = isle_keys::xkb::Context::new(isle_keys::xkb::CONTEXT_NO_FLAGS);
                let Some(keymap) = spec.compile(&context) else {
                    warn!(keymap = spec.describe(), "XKB does not know this keymap");
                    return None;
                };
                let text = keymap.get_as_string(isle_keys::xkb::KEYMAP_FORMAT_TEXT_V1);
                Some((text, KeyInfo::new(&keymap)))
            })
            .as_ref()
    }

    pub fn set_tables(&mut self, loaded: Result<Tables, TableError>) {
        match loaded {
            Ok(tables) => {
                self.tables = Some(tables);
                self.tables_error = None;
            }
            Err(err) => {
                warn!("translation.kdl rejected, the last good tables stay: {err}");
                self.tables_error = Some(err);
            }
        }
    }

    /// The profile of the keyboard that typed last; Mac before any has.
    pub fn last_profile(&self) -> Profile {
        self.last
            .as_ref()
            .and_then(|id| self.get(id))
            .map_or(Profile::Mac, |d| d.profile)
    }

    pub fn tables(&mut self) -> &Tables {
        self.tables.get_or_insert_with(Tables::shipped)
    }

    /// Runs `f` with the engine and a context over the seat's keymap and the rules of the
    /// app known by `names`.
    pub fn with_engine<R>(
        &mut self,
        translate: bool,
        names: &[&str],
        f: impl FnOnce(&mut Engine, &Ctx) -> R,
    ) -> R {
        static EMPTY: std::sync::OnceLock<KeyInfo> = std::sync::OnceLock::new();
        let option_level3 = self
            .last
            .as_ref()
            .and_then(|id| self.devices.iter().find(|d| &d.id == id))
            .is_some_and(|d| d.profile == Profile::Mac);
        let tables = self.tables.get_or_insert_with(Tables::shipped);
        let keys = self
            .active
            .as_ref()
            .and_then(|spec| self.keymaps.get(spec))
            .and_then(|c| c.as_ref())
            .map(|(_, info)| info)
            .unwrap_or_else(|| EMPTY.get_or_init(KeyInfo::default));
        let app = tables.for_app(names);
        let ctx = Ctx {
            keys,
            translate,
            option_level3,
            rules: &app.rules,
        };
        f(&mut self.engine, &ctx)
    }
}

impl State {
    /// The settings' layouts, or the system's when they name none.
    pub fn layouts(&self) -> Layouts {
        let k = &self.isle.settings.keyboard;
        if k.layouts.is_empty() {
            return self.isle.keyboards.system.clone();
        }
        Layouts {
            layouts: k.layouts.clone(),
            variants: k.variants.clone(),
            options: k.options.clone(),
        }
    }

    /// The profile, guess, swap and keymap a keyboard of this name takes from the settings.
    fn resolve_keyboard(
        &mut self,
        name: &str,
        hint: Option<Profile>,
    ) -> (Profile, bool, bool, KeymapSpec) {
        let config = self
            .isle
            .settings
            .keyboard_device
            .get(name)
            .cloned()
            .unwrap_or_default();
        let (profile, guessed) = match config.profile {
            KeyboardProfile::Mac => (Profile::Mac, false),
            KeyboardProfile::Pc => (Profile::Pc, false),
            KeyboardProfile::Auto => (hint.unwrap_or_else(|| Profile::guess(name)), true),
        };
        let iso = match config.model {
            KeyboardModel::Auto => Iso::Guess,
            KeyboardModel::Iso => Iso::Iso,
            KeyboardModel::Ansi => Iso::Ansi,
        };
        let layouts = self.layouts();
        let mac = self.isle.keyboards.mac().clone();
        let spec = KeymapSpec::resolve(&layouts, profile, iso, &mac);
        let swap = profile == Profile::Pc && config.swap_alt_super;
        (profile, guessed, swap, spec)
    }

    /// A keyboard has appeared; known ones are left as they are.
    pub fn keyboard_added(&mut self, id: &str, name: &str, hint: Option<Profile>) {
        if self.isle.keyboards.get(id).is_some() {
            return;
        }
        let (profile, guessed, swap, spec) = self.resolve_keyboard(name, hint);
        info!(
            id,
            name,
            profile = profile.name(),
            guessed,
            swap,
            keymap = spec.describe(),
            "keyboard"
        );
        // Compiled now, so the first key does not wait for XKB.
        self.isle.keyboards.compiled(&spec);
        self.isle.keyboards.devices.push(Device {
            id: id.to_owned(),
            name: name.to_owned(),
            hint,
            profile,
            guessed,
            swap,
            spec,
            typing: true,
        });
        self.isle.notify(Event::KeyboardsChanged);
    }

    pub fn keyboard_removed(&mut self, id: &str) {
        let before = self.isle.keyboards.devices.len();
        self.isle.keyboards.devices.retain(|d| d.id != id);
        if self.isle.keyboards.devices.len() != before {
            self.isle.notify(Event::KeyboardsChanged);
        }
    }

    /// Puts the keyboard settings in force: each keyboard's profile and keymap again, the
    /// seat's keymap, the repeat.
    pub fn apply_keyboard_settings(&mut self) {
        self.isle.keyboards.keymaps.clear();
        let devices: Vec<(String, Option<Profile>)> = self
            .isle
            .keyboards
            .devices
            .iter()
            .map(|d| (d.name.clone(), d.hint))
            .collect();
        for (i, (name, hint)) in devices.into_iter().enumerate() {
            let (profile, guessed, swap, spec) = self.resolve_keyboard(&name, hint);
            let device = &mut self.isle.keyboards.devices[i];
            device.profile = profile;
            device.guessed = guessed;
            device.swap = swap;
            device.spec = spec;
        }
        // The keyboard that typed last keeps the seat, else a PC keyboard's keymap.
        let last = self.isle.keyboards.last.clone();
        let spec = last
            .and_then(|id| self.isle.keyboards.get(&id).map(|d| d.spec.clone()))
            .unwrap_or_else(|| {
                let mac = self.isle.keyboards.mac().clone();
                KeymapSpec::resolve(&self.layouts(), Profile::Pc, Iso::Guess, &mac)
            });
        self.isle.keyboards.active = None;
        self.set_seat_keymap(&spec);
        self.isle
            .keyboards
            .engine
            .set_sticky(self.isle.settings.keyboard.sticky_keys);
        let keyboard = &self.isle.settings.keyboard;
        self.isle
            .seat
            .get_keyboard()
            .unwrap()
            .change_repeat_info(keyboard.repeat_rate, keyboard.repeat_delay);
        self.isle.notify(Event::KeyboardsChanged);
    }

    /// Gives the seat the keymap of `spec`, keeping Caps Lock and Num Lock as they were; the
    /// first keymap takes Num Lock from the settings.
    fn set_seat_keymap(&mut self, spec: &KeymapSpec) -> bool {
        if self.isle.keyboards.active.as_ref() == Some(spec) {
            return true;
        }
        let Some(text) = self
            .isle
            .keyboards
            .compiled(spec)
            .map(|(text, _)| text.clone())
        else {
            return false;
        };
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let before = keyboard.modifier_state();
        let first = self.isle.keyboards.active.is_none() && !self.isle.keyboards.started;
        if let Err(err) = keyboard.set_keymap_from_string(self, text) {
            warn!(?err, keymap = spec.describe(), "the seat refused a keymap");
            return false;
        }
        let (caps, num) = if first {
            (false, self.isle.settings.keyboard.numlock)
        } else {
            (before.caps_lock, before.num_lock)
        };
        self.isle.keyboards.started = true;
        let mut mods = keyboard.modifier_state();
        if mods.caps_lock != caps || mods.num_lock != num {
            mods.caps_lock = caps;
            mods.num_lock = num;
            keyboard.set_modifier_state(mods);
            keyboard.advertise_modifier_state(self);
        }
        self.isle.keyboards.active = Some(spec.clone());
        true
    }

    /// A key from a keyboard by its backend id: the swap, the seat's keymap following the
    /// keyboard, then the key.
    /// A key from a keyboard, through Slow Keys first: with a delay set, a key counts only
    /// once held that long.
    pub fn on_device_key(
        &mut self,
        id: &str,
        name: &str,
        code: u32,
        key_state: KeyState,
        time: InputTime,
    ) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        let delay = self.isle.settings.keyboard.slow_keys;
        let key = (id.to_owned(), code);
        match key_state {
            KeyState::Pressed if delay > 0 => {
                let (id_owned, name_owned) = (id.to_owned(), name.to_owned());
                let timer = Timer::from_duration(std::time::Duration::from_millis(delay as u64));
                let token = self
                    .isle
                    .loop_handle
                    .insert_source(timer, move |_, _, state| {
                        let key = (id_owned.clone(), code);
                        state.isle.slow_pending.remove(&key);
                        state.isle.slow_accepted.insert(key);
                        state.key_from_device(
                            &id_owned,
                            &name_owned,
                            code,
                            KeyState::Pressed,
                            InputTime::now(),
                        );
                        TimeoutAction::Drop
                    });
                if let Ok(token) = token {
                    self.isle.slow_pending.insert(key, token);
                }
                return;
            }
            KeyState::Released => {
                if let Some(token) = self.isle.slow_pending.remove(&key) {
                    self.isle.loop_handle.remove(token);
                    return;
                }
                self.isle.slow_accepted.remove(&key);
            }
            KeyState::Pressed => {}
        }
        self.key_from_device(id, name, code, key_state, time);
    }

    fn key_from_device(
        &mut self,
        id: &str,
        name: &str,
        code: u32,
        key_state: KeyState,
        time: InputTime,
    ) {
        if self.isle.keyboards.get(id).is_none() {
            let hint =
                matches!(self.backend, crate::backend::Backend::Winit(_)).then_some(Profile::Mac);
            self.keyboard_added(id, name, hint);
        }
        let key = (id.to_owned(), code);
        match key_state {
            KeyState::Pressed if self.isle.bar.keyboard_move.is_some() && self.keyboard_move_key(code) => {
                self.isle.swallowed.insert(key);
                return;
            }
            KeyState::Pressed if code == 1 && self.top_edge_escape() => {
                self.isle.swallowed.insert(key);
                return;
            }
            KeyState::Pressed if self.record_keyboard(name, code) => {
                self.isle.swallowed.insert(key);
                return;
            }
            KeyState::Released if self.isle.swallowed.remove(&key) => return,
            _ => {}
        }
        let game = self.focus_is_game();
        let (swap, spec) = {
            let device = self.isle.keyboards.get(id).unwrap();
            // A game gets the keys as labelled (B30).
            (device.swap && !game, device.spec.clone())
        };
        let sent = match key_state {
            KeyState::Pressed => {
                let sent = if swap { swapped(code) } else { code };
                self.isle.keyboards.held.insert((id.to_owned(), code), sent);
                let keyboard = self.isle.seat.get_keyboard().unwrap();
                // A keymap changes only with nothing held, so no key is released under one
                // it was not pressed in.
                if self.isle.keyboards.active.as_ref() != Some(&spec)
                    && keyboard.pressed_keys().is_empty()
                {
                    self.set_seat_keymap(&spec);
                }
                self.isle.keyboards.last = Some(id.to_owned());
                sent
            }
            KeyState::Released => self
                .isle
                .keyboards
                .held
                .remove(&(id.to_owned(), code))
                .unwrap_or(code),
        };
        let translate = self.translating();
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let bindings = &self.isle.bindings;
        let recording = matches!(self.isle.recording, Some(inject::Recording::Chord(_)));
        let inhibited = self.inhibited();
        let bind = |chord| {
            if recording {
                return Some(Bound {
                    action: Action::Recorded(chord),
                    shortcut: None,
                    mods: chord.mods,
                });
            }
            let bound = bindings.lookup(chord)?;
            let Some(shortcut) = bound.shortcut else {
                // The VT keys and the desktop numbers: the kernel's keys always act.
                return (!inhibited && !game || matches!(bound.action, Action::SwitchVt(_)))
                    .then_some(bound);
            };
            let escape = shortcut.id == "take-keys-back";
            match (inhibited, game) {
                (false, false) => Some(bound),
                // Over a game only the escape set acts, inhibitor or not (B30).
                (_, true) => (shortcut.escape || (escape && inhibited)).then_some(bound),
                (true, false) => escape.then_some(bound),
            }
        };
        let outs =
            self.isle
                .keyboards
                .with_engine(translate, &names, |engine, ctx| match key_state {
                    KeyState::Pressed => engine.press(sent, ctx, bind),
                    KeyState::Released => engine.release(sent, ctx),
                });
        match key_state {
            KeyState::Pressed => {
                for out in &outs {
                    if let Out::Action(bound) = out
                        && let Some(shortcut) = bound.shortcut
                    {
                        if shortcut.repeats {
                            self.start_repeat(sent, bound.action);
                        }
                        if shortcut.holds {
                            self.isle.held_chord = Some((shortcut.id, bound.mods));
                            self.isle.keyboards.engine.keep_latched();
                        }
                    }
                }
            }
            KeyState::Released => {
                if self
                    .isle
                    .repeat
                    .as_ref()
                    .is_some_and(|(code, _)| *code == sent)
                {
                    self.stop_repeat();
                }
            }
        }
        self.emit(outs, time);
        if key_state == KeyState::Released {
            self.check_held_chord();
            if self.isle.keyboards.engine.take_sticky_toggle() {
                self.toggle_sticky_keys();
            }
        }
        self.update_bar_option();
    }

    /// Shift pressed five times: Sticky Keys on or off, written to the settings.
    fn toggle_sticky_keys(&mut self) {
        let on = !self.isle.settings.keyboard.sticky_keys;
        info!(on, "Sticky Keys from Shift pressed five times");
        let (Some(worker), Some(store), Some(to_loop)) = (
            self.isle.worker.clone(),
            self.isle.settings_store.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        let events = self.isle.events.clone();
        worker.spawn(async move {
            let mut store = store.lock().await;
            match store.set("keyboard.sticky-keys", isle_settings::Value::Bool(on)) {
                Ok(keys) => {
                    crate::settings::publish(&to_loop, &events, store.current().clone(), keys)
                }
                Err(err) => warn!(
                    "could not turn Sticky Keys {}: {err}",
                    if on { "on" } else { "off" }
                ),
            }
        });
    }

    /// A held chord whose modifiers are let go is told so: the switcher commits then.
    fn check_held_chord(&mut self) {
        let Some((id, mods)) = self.isle.held_chord else {
            return;
        };
        if self.held_mods().contains(mods) {
            return;
        }
        self.isle.held_chord = None;
        info!(id, "a held chord was released");
        #[cfg(feature = "test-hooks")]
        {
            self.isle.held_releases += 1;
        }
    }

    /// Runs `action` again while `code` is held, after the keyboard's delay and at its rate.
    fn start_repeat(&mut self, code: u32, action: Action) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        self.stop_repeat();
        let keyboard = &self.isle.settings.keyboard;
        let delay = std::time::Duration::from_millis(keyboard.repeat_delay as u64);
        let every = std::time::Duration::from_secs_f64(1.0 / keyboard.repeat_rate.max(1) as f64);
        let token =
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(delay), move |_, _, state| {
                    state.run_action(action);
                    TimeoutAction::ToDuration(every)
                });
        match token {
            Ok(token) => self.isle.repeat = Some((code, token)),
            Err(err) => warn!("could not repeat a shortcut: {err}"),
        }
    }

    fn stop_repeat(&mut self) {
        if let Some((_, token)) = self.isle.repeat.take() {
            self.isle.loop_handle.remove(token);
        }
    }

    /// The seat's next layout, from the layout chord.
    pub fn next_layout(&mut self) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        keyboard.with_xkb_state(self, |mut context| context.cycle_next_layout());
        self.isle.notify(Event::KeyboardsChanged);
        self.island_layout();
    }

    /// The names the focused window's app is known by: its desktop id and its app id or X11
    /// class.
    pub fn focus_names(&self) -> Vec<String> {
        let Some(focus) = self
            .isle
            .seat
            .get_keyboard()
            .and_then(|k| k.current_focus())
        else {
            return Vec::new();
        };
        let Some(record) = focus.surface().and_then(|s| self.isle.wm.for_surface(s)) else {
            return Vec::new();
        };
        let (app_id, _) = record.names();
        vec![record.app.clone(), app_id]
    }

    /// Whether keys are translated for what has focus now: not in a game, not for a window
    /// holding the keys, not for an untranslated app.
    pub fn translating(&self) -> bool {
        let keyboard = &self.isle.settings.keyboard;
        if !keyboard.translate || self.focus_is_game() || self.inhibited() {
            return false;
        }
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        !keyboard
            .untranslated_apps
            .iter()
            .any(|p| isle_keys::tables::matches(p, &names))
    }

    /// Sends the focused app strokes once, as its own chord for an action.
    pub fn send_strokes(&mut self, strokes: &[Stroke]) {
        let translate = self.translating();
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let outs = self
            .isle
            .keyboards
            .with_engine(translate, &names, |engine, ctx| engine.send(strokes, ctx));
        self.emit(outs, InputTime::now());
    }

    /// The focused main window's app's own chord for the desktop's close or quit; dialogs
    /// are closed by the desktop.
    pub fn delegate(&mut self, quit: bool) -> Option<Vec<Stroke>> {
        let id = self.focused_window()?;
        if self.isle.wm.get(id)?.parent.is_some() {
            return None;
        }
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let app = self.isle.keyboards.tables().for_app(&names);
        let strokes = if quit { app.quit } else { app.close };
        strokes.map(<[Stroke]>::to_vec)
    }

    pub fn apply_tables(&mut self, loaded: Result<Tables, TableError>) {
        self.isle.keyboards.set_tables(loaded);
        info!("translation tables reloaded");
        self.isle.notify(Event::KeyboardsChanged);
    }

    /// Sends the engine's keys to the seat, then runs its actions.
    pub fn emit(&mut self, outs: Vec<Out<Bound>>, time: InputTime) {
        let mut actions = Vec::new();
        for out in outs {
            match out {
                Out::Press(code) => self.send_key(code, KeyState::Pressed, time),
                Out::Release(code) => self.send_key(code, KeyState::Released, time),
                Out::Action(bound) => actions.push(bound.action),
            }
        }
        for action in actions {
            self.run_action(action);
        }
    }

    /// One key to the focus, as Isle decided it.
    fn send_key(&mut self, code: u32, key_state: KeyState, time: InputTime) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        keyboard.input::<(), _>(
            self,
            Keycode::new(code + 8),
            key_state,
            SERIAL_COUNTER.next_serial(),
            time,
            |_, _, _| FilterResult::Forward,
        );
    }

    /// Moves keyboard focus. The old focus is sent a release for every key it holds first,
    /// and the new one the modifiers its context implies (B21).
    pub fn focus_keyboard(&mut self, target: Option<Focus>, serial: Serial) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if keyboard.current_focus() == target {
            return;
        }
        let released: Vec<Out<Bound>> = self.isle.keyboards.engine.unwind();
        self.emit(released, InputTime::now());
        keyboard.set_focus(self, target, serial);
        self.keys_back_on_focus();
        if matches!(self.isle.recording, Some(inject::Recording::Chord(_))) {
            self.cancel_recording();
        }
        let translate = self.translating();
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let pressed = self
            .isle
            .keyboards
            .with_engine(translate, &names, |engine, ctx| engine.resync(ctx));
        self.emit(pressed, InputTime::now());
    }

    /// A pointer button pressed with none down before, or the last one released.
    pub fn keyboard_buttons(&mut self, down: bool) {
        let translate = self.translating();
        let names = self.focus_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let outs = self
            .isle
            .keyboards
            .with_engine(translate, &names, |engine, ctx| engine.buttons(down, ctx));
        self.emit(outs, InputTime::now());
    }

    /// The modifiers physically held, Cmd and Option included.
    pub fn held_mods(&self) -> isle_keys::chord::Mods {
        self.isle.keyboards.engine.mods()
    }

    /// The seat's active layout by XKB's name, and Caps Lock and Num Lock.
    pub fn keyboard_status(&mut self) -> (String, bool, bool) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let layout = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            xkb.layout_name(xkb.active_layout()).to_owned()
        });
        let mods = keyboard.modifier_state();
        (layout, mods.caps_lock, mods.num_lock)
    }

    pub fn keyboard_infos(&self) -> Vec<isle_ipc::KeyboardInfo> {
        let active = self.isle.keyboards.active.as_ref();
        self.isle
            .keyboards
            .devices
            .iter()
            .filter(|d| d.id != ISLE_DEVICE && d.typing)
            .map(|d| isle_ipc::KeyboardInfo {
                id: d.id.clone(),
                name: d.name.clone(),
                profile: d.profile.name().to_owned(),
                guessed: d.guessed,
                swap: d.swap,
                layouts: d.spec.describe(),
                model: d.spec.model.clone(),
                remapper: d.name == "xremap",
                active: active == Some(&d.spec),
            })
            .collect()
    }
}

pub fn tables_path() -> std::path::PathBuf {
    crate::settings::config_dir().join("translation.kdl")
}

/// The shipped tables with the user's file over them, if it has one that reads.
pub fn load_tables(path: &std::path::Path) -> Result<Tables, TableError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Tables::shipped().over(Tables::parse(&text)?)),
        Err(_) => Ok(Tables::shipped()),
    }
}

/// Watches the user's tables and reloads them.
pub async fn watch_tables(
    to_loop: crate::worker::ToLoop,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = tables_path();
    crate::tokens::watch_file(path.clone(), move || {
        let loaded = load_tables(&path);
        to_loop.post(move |state| state.apply_tables(loaded));
    })
    .await
}

/// Left Alt and left Super traded.
fn swapped(code: u32) -> u32 {
    match code {
        KEY_LEFTALT => KEY_LEFTMETA,
        KEY_LEFTMETA => KEY_LEFTALT,
        other => other,
    }
}
