//! The bind table until P5's shortcut set: a few chords from `binds` in `settings.kdl`.

use isle_keys::chord::Chord;
use smithay::input::keyboard::keysyms;
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
    Hide,
    HideOthers,
    Minimise,
    MinimiseApp,
    QuitApp,
    /// The front app's next window, or its previous.
    CycleWindows(bool),
    FullScreen,
    ZoomMenu,
    /// Along the row of desktops by a step.
    Desktop(i32),
    /// To a desktop by its place, from 0.
    JumpDesktop(usize),
    CarryWindow(i32),
    /// The kernel's VT keys, Ctrl+Alt+F1 to F12, which XKB turns into these keysyms.
    SwitchVt(i32),
}

/// The action bound to a physical chord, if any. The kernel's VT keys are Ctrl+Option+F1 to
/// F12.
pub fn bound_action(settings: &isle_settings::schema::Settings, chord: Chord) -> Option<Action> {
    let raw = chord.sym.raw();
    if chord.mods.ctrl && chord.mods.option && (keysyms::KEY_F1..=keysyms::KEY_F12).contains(&raw) {
        return Some(Action::SwitchVt((raw - keysyms::KEY_F1 + 1) as i32));
    }
    let binds = &settings.binds;
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
        (&binds.hide, Action::Hide),
        (&binds.hide_others, Action::HideOthers),
        (&binds.minimise, Action::Minimise),
        (&binds.minimise_app, Action::MinimiseApp),
        (&binds.quit_app, Action::QuitApp),
        (&binds.next_window, Action::CycleWindows(true)),
        (&binds.previous_window, Action::CycleWindows(false)),
        (&binds.full_screen, Action::FullScreen),
        (&binds.zoom_menu, Action::ZoomMenu),
        (&binds.desktop_left, Action::Desktop(-1)),
        (&binds.desktop_right, Action::Desktop(1)),
        (&binds.carry_left, Action::CarryWindow(-1)),
        (&binds.carry_right, Action::CarryWindow(1)),
    ]
    .into_iter()
    .find(|(text, _)| Chord::parse(text) == Some(chord))
    .map(|(_, action)| action)
    .or_else(|| {
        (1..=9).find_map(|n| {
            let bound = Chord::parse(&format!("{}+{n}", binds.desktop_jump))?;
            (bound == chord).then_some(Action::JumpDesktop(n - 1))
        })
    })
}

impl State {
    pub fn run_action(&mut self, action: Action) {
        info!(?action, "bind");
        match action {
            Action::Terminal => spawn(&self.isle.settings.binds.terminal_command),
            Action::CloseWindow => {
                if let Some(id) = self.focused_window() {
                    self.close_window(id);
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
            Action::Hide => {
                if let Some(app) = self.isle.front_app() {
                    self.set_app_hidden(&app, true);
                }
            }
            Action::HideOthers => self.hide_others(),
            Action::Minimise => self.minimise_focused(),
            Action::MinimiseApp => self.minimise_app(),
            Action::QuitApp => {
                if let Some(app) = self.isle.front_app() {
                    self.quit_app(&app);
                }
            }
            Action::CycleWindows(forward) => self.cycle_app_windows(forward),
            Action::FullScreen => {
                if let Some(id) = self.focused_window() {
                    self.toggle_full_screen(id);
                }
            }
            Action::ZoomMenu => self.zoom_menu_by_key(),
            Action::Desktop(step) => self.step_desktop(step),
            Action::JumpDesktop(index) => self.jump_desktop(index),
            Action::CarryWindow(step) => self.carry_window(step),
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
