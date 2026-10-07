//! Cmd+Tab: the hold, the keys read while it lasts, and the commit (`p7-switcher.md`
//! choices 1 to 6). The switcher never takes the seat's focus.

use std::time::{Duration, Instant};

use isle_shell::switcher::model::{Effect, Entry, Key, Switcher, Window};
use smithay::{
    input::keyboard::{Keysym, keysyms},
    output::Output,
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
};
use tracing::{debug, info};

use crate::state::State;

/// The switcher while its chord is held.
pub struct SwitcherState {
    pub model: Switcher,
    /// The output it opened on: the focused window's, else the pointer's.
    pub output: Option<Output>,
    timer: Option<RegistrationToken>,
}

impl SwitcherState {
    pub fn drawn(&self) -> bool {
        self.model.drawn()
    }
}

impl crate::state::Isle {
    /// The switcher's entries as the apps are now.
    pub fn switcher_entries(&self) -> Vec<Entry> {
        self.app_infos()
            .into_iter()
            .map(|info| {
                let windows: Vec<Window> = self
                    .app_windows(&info.app)
                    .into_iter()
                    .filter_map(|id| self.wm.get(id))
                    .map(|r| Window {
                        id: r.id,
                        title: r.names().1,
                        minimised: r.minimised,
                    })
                    .collect();
                let recent = self
                    .wm
                    .recent
                    .iter()
                    .copied()
                    .find(|id| windows.iter().any(|w| w.id == *id))
                    .or(windows.last().map(|w| w.id));
                Entry {
                    cover: self
                        .games
                        .for_app(&info.app)
                        .is_some_and(|g| g.cover.is_some()),
                    name: info.name,
                    icon: (!info.icon.is_empty()).then(|| info.icon.into()),
                    windows,
                    recent,
                    hidden: info.hidden,
                    minimised: info.minimised,
                    background: info.background,
                    not_responding: info.not_responding,
                    attention: info.attention,
                    urgent: info.urgent,
                    badge: (info.badge >= 0).then_some(info.badge),
                    progress: (info.progress >= 0.0).then_some(info.progress),
                    app: info.app,
                }
            })
            .collect()
    }
}

/// What a key is to the switcher by its symbol in the seat's keymap: `None` for a modifier,
/// which goes on to the engine so the release is seen.
fn switcher_key(
    role: isle_keys::keyinfo::Role,
    sym: Option<Keysym>,
    shift: bool,
) -> Option<Option<Key>> {
    use isle_keys::keyinfo::Role;
    if role != Role::Key {
        return None;
    }
    let key = match sym.map(|s| s.raw()) {
        Some(keysyms::KEY_Tab | keysyms::KEY_ISO_Left_Tab) if shift => Key::Previous,
        Some(keysyms::KEY_Tab | keysyms::KEY_ISO_Left_Tab) => Key::Next,
        Some(keysyms::KEY_Left) => Key::Left,
        Some(keysyms::KEY_Right) => Key::Right,
        Some(keysyms::KEY_Up) => Key::Up,
        Some(keysyms::KEY_Down) => Key::Down,
        Some(keysyms::KEY_Return | keysyms::KEY_KP_Enter) => Key::Return,
        Some(keysyms::KEY_Escape) => Key::Escape,
        Some(keysyms::KEY_q) => Key::Quit,
        Some(keysyms::KEY_h) => Key::Hide,
        Some(keysyms::KEY_w) => Key::Close,
        Some(keysyms::KEY_grave) => Key::Windows { forward: !shift },
        _ => return Some(None),
    };
    Some(Some(key))
}

impl State {
    /// Cmd+Tab or Shift+Cmd+Tab pressed: the switcher opens on the apps as they are, and
    /// nothing moves until the chord is let go.
    pub fn open_switcher(&mut self, backwards: bool) {
        if self.isle.switcher.is_some() {
            return;
        }
        self.close_menu_bar_group();
        let output = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output());
        let front = self.isle.front_app();
        let delay =
            Duration::from_millis(self.isle.shell.theme().tokens.timing_switcher_delay_ms as u64);
        let now = Instant::now();
        let model = Switcher::open(
            self.isle.switcher_entries(),
            front.as_deref(),
            backwards,
            delay,
            now,
        );
        debug!(
            selected = model.selected().map(|e| e.app.as_str()),
            "switcher held"
        );
        self.isle.switcher = Some(SwitcherState {
            model,
            output,
            timer: None,
        });
        self.arm_switcher();
    }

    fn arm_switcher(&mut self) {
        let Some(switcher) = self.isle.switcher.as_mut() else {
            return;
        };
        if let Some(token) = switcher.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let Some(at) = switcher.model.next_change() else {
            return;
        };
        let wait = at.saturating_duration_since(Instant::now());
        let token =
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(wait), |_, _, state| {
                    if let Some(switcher) = state.isle.switcher.as_mut() {
                        switcher.timer = None;
                        if switcher.model.tick(Instant::now()) {
                            state.switcher_changed();
                        }
                    }
                    TimeoutAction::Drop
                });
        if let Ok(token) = token {
            self.isle.switcher.as_mut().unwrap().timer = Some(token);
        }
    }

    /// A key pressed while the switcher is held; true when the switcher took it.
    pub fn switcher_key(&mut self, role: isle_keys::keyinfo::Role, sym: Option<Keysym>) -> bool {
        if self.isle.switcher.is_none() {
            return false;
        }
        let shift = self.held_mods().shift;
        let Some(key) = switcher_key(role, sym, shift) else {
            return false;
        };
        if let Some(key) = key {
            let effects = self
                .isle
                .switcher
                .as_mut()
                .unwrap()
                .model
                .key(key, Instant::now());
            self.switcher_effects(effects);
        }
        true
    }

    /// The chord was let go: the selection comes forward.
    pub fn switcher_released(&mut self) {
        let Some(switcher) = self.isle.switcher.as_mut() else {
            return;
        };
        let effects = switcher.model.commit();
        self.switcher_effects(effects);
    }

    /// Ends the hold without a commit: a switch of console, the lock, the keyboard gone.
    pub fn cancel_switcher(&mut self) {
        if let Some(switcher) = self.isle.switcher.as_mut() {
            switcher.model.cancel();
            self.close_switcher();
        }
    }

    fn switcher_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            info!(?effect, "switcher");
            match effect {
                Effect::Activate(app) => {
                    self.close_switcher();
                    self.activate_app(&app);
                }
                Effect::ActivateWindow(id) => {
                    self.close_switcher();
                    self.activate(id);
                }
                Effect::Relaunch(app) => {
                    self.close_switcher();
                    self.launch(crate::launch::Launch::App {
                        app,
                        action: None,
                        uris: Vec::new(),
                    });
                }
                Effect::Quit(app) => {
                    self.quit_app(&app);
                }
                Effect::Stop(app) => {
                    self.stop_app(&app);
                }
                Effect::Hide(app, hidden) => {
                    self.set_app_hidden(&app, hidden);
                }
                Effect::Close(id) => {
                    self.close_window(id);
                }
            }
        }
        if self.isle.switcher.as_ref().is_some_and(|s| s.model.done()) {
            self.close_switcher();
        } else {
            self.refresh_switcher();
        }
    }

    fn close_switcher(&mut self) {
        let Some(mut switcher) = self.isle.switcher.take() else {
            return;
        };
        if let Some(token) = switcher.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        debug!("switcher closed");
        self.switcher_closed(switcher);
    }

    /// The apps changed while the chord is held.
    pub fn refresh_switcher(&mut self) {
        if self.isle.switcher.is_none() {
            return;
        }
        let entries = self.isle.switcher_entries();
        self.isle.switcher.as_mut().unwrap().model.refresh(entries);
        self.switcher_changed();
    }

    /// Run once per loop iteration: a change of windows while held reaches the switcher.
    pub fn switcher_after_dispatch(&mut self) {
        if self.isle.switcher_dirty.take() {
            self.refresh_switcher();
        }
    }

    /// Something the switcher shows changed.
    fn switcher_changed(&mut self) {
        #[cfg(feature = "test-hooks")]
        if let Some(switcher) = &self.isle.switcher
            && switcher.drawn()
        {
            self.isle.switcher_drawn_count += u32::from(!self.isle.switcher_was_drawn);
            self.isle.switcher_was_drawn = true;
        }
    }

    fn switcher_closed(&mut self, _switcher: SwitcherState) {
        #[cfg(feature = "test-hooks")]
        {
            self.isle.switcher_was_drawn = false;
        }
    }

    /// Opening a centre surface, the switcher included, closes every menu-bar surface: the
    /// bar and its menus, Control Centre, the window menu, About and the zoom menu.
    pub fn close_menu_bar_group(&mut self) {
        if self.isle.bar.output.is_some() {
            self.hide_bar();
        }
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        for name in ["window-menu", "about"] {
            if self.isle.shell.is_open(name) {
                self.close_surface(name);
            }
        }
        self.close_zoom_menu();
    }

    /// The switcher's state for tests and `isle`: the entries, the selection, the window row.
    pub fn switcher_json(&self) -> String {
        let Some(switcher) = &self.isle.switcher else {
            return "null".into();
        };
        let entries: Vec<serde_json::Value> = switcher
            .model
            .entries()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "app": e.app,
                    "name": e.name,
                    "windows": e.windows.iter().map(|w| w.id).collect::<Vec<_>>(),
                    "background": e.background,
                    "hidden": e.hidden,
                    "quitting": switcher.model.quitting(&e.app),
                })
            })
            .collect();
        serde_json::json!({
            "entries": entries,
            "selected": switcher.model.selected().map(|e| e.app.clone()),
            "row": switcher.model.row(),
            "drawn": switcher.drawn(),
            "output": switcher.output.as_ref().map(|o| o.name()),
        })
        .to_string()
    }
}
