//! The bind table until P5's shortcut set: a few chords from `binds` in `settings.kdl`.

use smithay::input::keyboard::{Keysym, ModifiersState, keysyms, xkb};
use tracing::{info, warn};

use crate::{state::State, wm::geometry::Zone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Terminal,
    CloseWindow,
    Quit,
    /// A zone for the focused window; a repeat moves along the zone's cycle.
    Tile(Zone),
    Fill,
    Restore,
    /// The next display (1) or the previous (-1).
    Display(i32),
    /// The kernel's VT keys, Ctrl+Alt+F1 to F12, which XKB turns into these keysyms.
    SwitchVt(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Chord {
    logo: bool,
    ctrl: bool,
    alt: bool,
    shift: bool,
    key: Keysym,
}

/// Reads `Super+Shift+Escape`; `None` when it names no key.
fn parse(text: &str) -> Option<Chord> {
    let mut chord = Chord {
        logo: false,
        ctrl: false,
        alt: false,
        shift: false,
        key: Keysym::from(keysyms::KEY_NoSymbol),
    };
    let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let key = parts.pop()?;
    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "super" | "cmd" | "logo" => chord.logo = true,
            "ctrl" | "control" => chord.ctrl = true,
            "alt" | "option" => chord.alt = true,
            "shift" => chord.shift = true,
            _ => return None,
        }
    }
    chord.key = xkb::keysym_from_name(key, xkb::KEYSYM_CASE_INSENSITIVE);
    (chord.key.raw() != keysyms::KEY_NoSymbol).then_some(chord)
}

impl State {
    /// The VT a key switches to, from its symbol with modifiers applied.
    pub fn vt_for(sym: Keysym) -> Option<Action> {
        let raw = sym.raw();
        (keysyms::KEY_XF86Switch_VT_1..=keysyms::KEY_XF86Switch_VT_12)
            .contains(&raw)
            .then(|| Action::SwitchVt((raw - keysyms::KEY_XF86Switch_VT_1 + 1) as i32))
    }

    /// The action bound to `sym` with `modifiers`, if any. `sym` is the key's unshifted
    /// symbol, which is what a case-insensitive name resolves to.
    pub fn bound_action(&self, modifiers: &ModifiersState, sym: Keysym) -> Option<Action> {
        let binds = &self.isle.settings.binds;
        let pressed = Chord {
            logo: modifiers.logo,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            key: sym,
        };
        [
            (&binds.terminal, Action::Terminal),
            (&binds.close_window, Action::CloseWindow),
            (&binds.quit, Action::Quit),
            (&binds.tile_left, Action::Tile(Zone::LeftHalf)),
            (&binds.tile_right, Action::Tile(Zone::RightHalf)),
            (&binds.tile_centre, Action::Tile(Zone::Centre)),
            (&binds.fill, Action::Fill),
            (&binds.restore, Action::Restore),
            (&binds.tile_top_left, Action::Tile(Zone::TopLeft)),
            (&binds.tile_top_right, Action::Tile(Zone::TopRight)),
            (&binds.tile_bottom_left, Action::Tile(Zone::BottomLeft)),
            (&binds.tile_bottom_right, Action::Tile(Zone::BottomRight)),
            (&binds.tile_beside, Action::Tile(Zone::Beside)),
            (&binds.next_display, Action::Display(1)),
            (&binds.previous_display, Action::Display(-1)),
        ]
        .into_iter()
        .find(|(text, _)| parse(text) == Some(pressed))
        .map(|(_, action)| action)
    }

    pub fn run_action(&mut self, action: Action) {
        info!(?action, "bind");
        match action {
            Action::Terminal => spawn(&self.isle.settings.binds.terminal_command),
            Action::CloseWindow => {
                let focus = self
                    .isle
                    .seat
                    .get_keyboard()
                    .and_then(|k| k.current_focus());
                if let Some(window) = focus
                    .as_ref()
                    .and_then(|f| f.surface())
                    .and_then(|s| self.isle.window_for_surface(s))
                {
                    window.toplevel().unwrap().send_close();
                }
            }
            Action::Quit => self.isle.loop_signal.stop(),
            Action::Tile(zone) => self.tile_by_chord(zone),
            Action::Fill => {
                if let Some(id) = self.focused_window() {
                    self.toggle_fill(id);
                }
            }
            Action::Restore => {
                if let Some(id) = self.focused_window() {
                    self.restore_box(id);
                }
            }
            Action::Display(step) => self.move_to_display(step),
            Action::SwitchVt(vt) => self.switch_vt(vt),
        }
    }
}

/// Starts `command` through the shell; a thread waits on it so it leaves no zombie.
pub fn spawn(command: &str) {
    let command = command.to_owned();
    let result = std::thread::Builder::new()
        .name("spawn".into())
        .spawn(move || {
            match std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(&command)
                .spawn()
            {
                Ok(mut child) => {
                    let _ = child.wait();
                }
                Err(err) => warn!(command, "could not start: {err}"),
            }
        });
    if let Err(err) = result {
        warn!("could not start a spawn thread: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_parse_with_any_case_and_order_of_modifiers() {
        let c = parse("Super+Shift+Escape").unwrap();
        assert!(c.logo && c.shift && !c.ctrl && !c.alt);
        assert_eq!(c.key.raw(), keysyms::KEY_Escape);
        assert_eq!(parse("shift+super+escape"), Some(c));
        assert_eq!(
            parse("Super+Return").unwrap().key.raw(),
            keysyms::KEY_Return
        );
        assert!(parse("Hyper+Q").is_none());
        assert!(parse("Super+Nonsense").is_none());
    }
}
