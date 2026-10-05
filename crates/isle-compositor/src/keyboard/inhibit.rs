//! Keyboard-shortcuts-inhibit and Xwayland's keyboard grabs: a window holding either gets every
//! key raw while it has focus, except Cmd+Escape, which takes the keys back and gives them
//! back.

use smithay::{
    input::SeatHandler,
    reexports::wayland_server::{Resource, protocol::wl_surface::WlSurface},
    wayland::{
        keyboard_shortcuts_inhibit::{
            KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
            KeyboardShortcutsInhibitor, KeyboardShortcutsInhibitorSeat,
        },
        xwayland_keyboard_grab::{XWaylandKeyboardGrab, XWaylandKeyboardGrabHandler},
    },
};
use tracing::info;

use crate::{state::State, worker::Event};

/// The grabs Xwayland holds, by the surface it asked for.
#[derive(Default)]
pub struct X11Grabs(Vec<(WlSurface, XWaylandKeyboardGrab<State>)>);

impl KeyboardShortcutsInhibitHandler for State {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.isle.inhibit_state
    }

    /// Every inhibitor is granted (P1, `window-chrome.md`); the island and the window say so.
    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        info!("keyboard shortcuts inhibited");
        inhibitor.activate();
        self.isle.notify(Event::KeyboardsChanged);
    }

    fn inhibitor_destroyed(&mut self, _inhibitor: KeyboardShortcutsInhibitor) {
        self.isle.notify(Event::KeyboardsChanged);
    }
}

impl XWaylandKeyboardGrabHandler for State {
    /// Kept as an inhibitor rather than a keyboard grab, so focus still moves by Isle's rules.
    fn grab(
        &mut self,
        surface: WlSurface,
        _seat: smithay::input::Seat<Self>,
        grab: XWaylandKeyboardGrab<Self>,
    ) {
        info!("an X11 window grabbed the keyboard");
        self.isle
            .x11_grabs
            .0
            .retain(|(s, g)| s.is_alive() && g.grab().is_alive());
        self.isle.x11_grabs.0.push((surface, grab));
        self.isle.notify(Event::KeyboardsChanged);
    }

    fn keyboard_focus_for_xsurface(
        &self,
        surface: &WlSurface,
    ) -> Option<<Self as SeatHandler>::KeyboardFocus> {
        Some(surface.clone().into())
    }
}

impl State {
    fn focused_surface(&self) -> Option<WlSurface> {
        let focus = self.isle.seat.get_keyboard()?.current_focus()?;
        focus.surface().cloned()
    }

    fn x11_grab_on(&self, surface: &WlSurface) -> bool {
        self.isle
            .x11_grabs
            .0
            .iter()
            .any(|(s, g)| s == surface && g.grab().is_alive())
    }

    /// Whether the window with focus holds the keys now.
    pub fn inhibited(&self) -> bool {
        let Some(surface) = self.focused_surface() else {
            return false;
        };
        let wayland = self
            .isle
            .seat
            .keyboard_shortcuts_inhibitor_for_surface(&surface)
            .is_some_and(|i| i.is_active());
        wayland
            || (self.x11_grab_on(&surface) && self.isle.keys_taken_back.as_ref() != Some(&surface))
    }

    /// The app holding the keys, for the island and the bus; empty when none does.
    pub fn inhibiting_app(&self) -> String {
        if !self.inhibited() {
            return String::new();
        }
        self.focused_window()
            .and_then(|id| self.isle.wm.get(id))
            .map(|r| r.app.clone())
            .unwrap_or_else(|| "a window".into())
    }

    /// Whether the window with focus holds an inhibitor, active or taken back.
    fn holds_keys(&self) -> bool {
        self.focused_surface().is_some_and(|s| {
            self.isle
                .seat
                .keyboard_shortcuts_inhibitor_for_surface(&s)
                .is_some()
                || self.x11_grab_on(&s)
        })
    }

    /// Cmd+Escape and "Take keys back": once to take them, again to give them back.
    pub fn toggle_keys_back(&mut self) {
        if !self.holds_keys() {
            return;
        }
        let surface = self.focused_surface().unwrap();
        let inhibitor = self
            .isle
            .seat
            .keyboard_shortcuts_inhibitor_for_surface(&surface);
        let taking = self.isle.keys_taken_back.as_ref() != Some(&surface);
        if taking {
            info!("keys taken back from the window holding them");
            if let Some(i) = &inhibitor {
                i.inactivate();
            }
            self.isle.keys_taken_back = Some(surface);
        } else {
            info!("keys given back to the window");
            if let Some(i) = &inhibitor {
                i.activate();
            }
            self.isle.keys_taken_back = None;
        }
        self.isle.notify(Event::KeyboardsChanged);
    }

    /// Focus moved: a window the keys were taken from gets them back when it next has focus.
    pub fn keys_back_on_focus(&mut self) {
        if let Some(surface) = self.isle.keys_taken_back.take() {
            if let Some(i) = self
                .isle
                .seat
                .keyboard_shortcuts_inhibitor_for_surface(&surface)
            {
                i.activate();
            }
            self.isle.notify(Event::KeyboardsChanged);
        }
    }
}
