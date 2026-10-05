//! Keyboards: each by its name with its profile and keymap, the seat's keymap following the
//! keyboard that types, and the PC swap of Alt and Super.

use std::collections::HashMap;

use isle_keys::{
    keymap::{Iso, KeymapSpec, Layouts, MacVariants},
    profile::Profile,
};
use isle_settings::schema::{KeyboardModel, KeyboardProfile};
use smithay::{
    backend::input::{InputTime, KeyState},
    input::keyboard::Keycode,
};
use tracing::{info, warn};

use crate::{state::State, worker::Event};

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
}

#[derive(Default)]
pub struct Keyboards {
    pub devices: Vec<Device>,
    mac: Option<MacVariants>,
    /// Compiled keymaps as text, by spec; `None` for a spec XKB refused.
    keymaps: HashMap<KeymapSpec, Option<String>>,
    /// The spec the seat's keymap was made from.
    pub active: Option<KeymapSpec>,
    /// Whether the seat has had a keymap of Isle's, so Num Lock starts from the settings once.
    started: bool,
    /// The keyboard that typed last.
    last: Option<String>,
    /// Keys held, by device and evdev code, with the code the seat was sent.
    held: HashMap<(String, u32), u32>,
}

impl Keyboards {
    fn mac(&mut self) -> &MacVariants {
        self.mac.get_or_insert_with(MacVariants::load)
    }

    pub fn get(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    fn keymap_text(&mut self, spec: &KeymapSpec) -> Option<String> {
        self.keymaps
            .entry(spec.clone())
            .or_insert_with(|| {
                let text = spec.compile_text();
                if text.is_none() {
                    warn!(keymap = spec.describe(), "XKB does not know this keymap");
                }
                text
            })
            .clone()
    }
}

impl State {
    fn layouts(&self) -> Layouts {
        let k = &self.isle.settings.keyboard;
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
        self.isle.keyboards.keymap_text(&spec);
        self.isle.keyboards.devices.push(Device {
            id: id.to_owned(),
            name: name.to_owned(),
            hint,
            profile,
            guessed,
            swap,
            spec,
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
        let Some(text) = self.isle.keyboards.keymap_text(spec) else {
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
    pub fn on_device_key(
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
        let (swap, spec) = {
            let device = self.isle.keyboards.get(id).unwrap();
            (device.swap, device.spec.clone())
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
        self.on_key(Keycode::new(sent + 8), key_state, time);
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
            .filter(|d| d.id != ISLE_DEVICE)
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

/// Left Alt and left Super traded.
fn swapped(code: u32) -> u32 {
    match code {
        KEY_LEFTALT => KEY_LEFTMETA,
        KEY_LEFTMETA => KEY_LEFTALT,
        other => other,
    }
}
