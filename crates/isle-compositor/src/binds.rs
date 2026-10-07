//! The shortcut set: each action's chords from `shortcuts` in `settings.kdl`, matched against
//! the physical keys, and the actions they run.

use std::collections::HashMap;

use isle_keys::{
    chord::{Chord, Mods},
    shortcuts::Shortcut,
};
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
    /// Cmd+Tab's switcher, from the end with Shift.
    Switcher(bool),
    FullScreen,
    ZoomMenu,
    /// Along the row of desktops by a step.
    Desktop(i32),
    /// To a desktop by its place, from 0.
    JumpDesktop(usize),
    CarryWindow(i32),
    /// The kernel's VT keys: Ctrl+Option+F1 to F12.
    SwitchVt(i32),
    NextLayout,
    Redetect,
    Media(Media),
    TakeKeysBack,
    ControlCentre,
    MenuBar,
    HelpSearch,
    WindowMenu,
    /// An action a later packet builds; its chord is the desktop's already.
    Reserved(&'static str),
    /// The chord the recording grab took.
    Recorded(Chord),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Media {
    VolumeUp,
    VolumeDown,
    Mute,
    MicMute,
    BrightnessUp,
    BrightnessDown,
    PlayPause,
    Next,
    Previous,
}

impl Media {
    /// v1's commands, until P8's services take the keys.
    fn command(self) -> &'static str {
        match self {
            Media::VolumeUp => "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+",
            Media::VolumeDown => "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-",
            Media::Mute => "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",
            Media::MicMute => "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle",
            Media::BrightnessUp => "brightnessctl -q -c backlight set 5%+",
            Media::BrightnessDown => "brightnessctl -q -c backlight set 5%-",
            Media::PlayPause => "playerctl play-pause",
            Media::Next => "playerctl next",
            Media::Previous => "playerctl previous",
        }
    }
}

/// The action a shortcut id runs.
fn action_for(id: &'static str) -> Action {
    match id {
        "close-window" => Action::CloseWindow,
        "minimise" => Action::Minimise,
        "minimise-app" => Action::MinimiseApp,
        "hide" => Action::Hide,
        "hide-others" => Action::HideOthers,
        "quit-app" => Action::QuitApp,
        "full-screen" => Action::FullScreen,
        "tile-left" => Action::Tile(Zone::LeftHalf),
        "tile-right" => Action::Tile(Zone::RightHalf),
        "tile-centre" => Action::Tile(Zone::Centre),
        "fill" => Action::Fill,
        "restore" => Action::Restore,
        "tile-top-left" => Action::Tile(Zone::TopLeft),
        "tile-top-right" => Action::Tile(Zone::TopRight),
        "tile-bottom-left" => Action::Tile(Zone::BottomLeft),
        "tile-bottom-right" => Action::Tile(Zone::BottomRight),
        "tile-beside" => Action::Tile(Zone::Beside),
        "next-display" => Action::Display(1),
        "previous-display" => Action::Display(-1),
        "zoom-menu" => Action::ZoomMenu,
        "switcher" => Action::Switcher(false),
        "switcher-back" => Action::Switcher(true),
        "next-window" => Action::CycleWindows(true),
        "previous-window" => Action::CycleWindows(false),
        "desktop-left" => Action::Desktop(-1),
        "desktop-right" => Action::Desktop(1),
        "carry-left" => Action::CarryWindow(-1),
        "carry-right" => Action::CarryWindow(1),
        "terminal" => Action::Terminal,
        "log-out-now" => Action::Quit,
        "redetect-displays" => Action::Redetect,
        "next-layout" => Action::NextLayout,
        "take-keys-back" => Action::TakeKeysBack,
        "control-centre" => Action::ControlCentre,
        "menu-bar" => Action::MenuBar,
        "help-search" => Action::HelpSearch,
        "window-menu" => Action::WindowMenu,
        "volume-up" => Action::Media(Media::VolumeUp),
        "volume-down" => Action::Media(Media::VolumeDown),
        "mute" => Action::Media(Media::Mute),
        "mic-mute" => Action::Media(Media::MicMute),
        "brightness-up" => Action::Media(Media::BrightnessUp),
        "brightness-down" => Action::Media(Media::BrightnessDown),
        "play-pause" => Action::Media(Media::PlayPause),
        "next-track" => Action::Media(Media::Next),
        "previous-track" => Action::Media(Media::Previous),
        other => Action::Reserved(other),
    }
}

/// What a chord bound: the action, its shortcut, and the modifiers it was pressed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bound {
    pub action: Action,
    pub shortcut: Option<&'static Shortcut>,
    pub mods: Mods,
}

/// The shortcut set as the keyboard reads it, built when the settings change.
#[derive(Default)]
pub struct Bindings {
    chords: HashMap<Chord, &'static Shortcut>,
    /// The modifiers that with 1 to 9 jump to a desktop.
    jump: Mods,
    /// The modifiers that make a press a window drag.
    pub drag: Mods,
    layouts: usize,
}

impl Bindings {
    /// The settings' chords; ones the settings check refused never get here.
    pub fn new(settings: &isle_settings::schema::Settings, layouts: usize) -> Self {
        let shortcuts = &settings.shortcuts;
        let mut chords = HashMap::new();
        for (id, texts) in shortcuts.chords() {
            let Some(shortcut) = isle_keys::shortcuts::find(id) else {
                warn!(id, "a shortcut with no action");
                continue;
            };
            for text in texts {
                if let Some(chord) = Chord::parse(text) {
                    chords.insert(chord, shortcut);
                }
            }
        }
        Bindings {
            chords,
            jump: Mods::parse(&shortcuts.desktop_jump).unwrap_or_default(),
            drag: Mods::parse(&shortcuts.window_drag).unwrap_or_default(),
            layouts,
        }
    }

    /// The shortcut that holds a chord, for the recorder.
    pub fn holder(&self, chord: Chord) -> Option<&'static str> {
        if let Some(shortcut) = self.chords.get(&chord) {
            return Some(shortcut.id);
        }
        let digit = (keysyms::KEY_1..=keysyms::KEY_9).contains(&chord.sym.raw());
        (!self.jump.is_empty() && chord.mods == self.jump && digit).then_some("desktop-jump")
    }

    /// The action bound to a physical chord, if any.
    pub fn lookup(&self, chord: Chord) -> Option<Bound> {
        let raw = chord.sym.raw();
        let bound = |action, shortcut| Bound {
            action,
            shortcut,
            mods: chord.mods,
        };
        if chord.mods.ctrl && chord.mods.option && (keysyms::KEY_F1..=keysyms::KEY_F12).contains(&raw) {
            return Some(bound(Action::SwitchVt((raw - keysyms::KEY_F1 + 1) as i32), None));
        }
        if let Some(shortcut) = self.chords.get(&chord) {
            // With one layout Ctrl+Space stays the app's (`competition.md`).
            if shortcut.id == "next-layout" && self.layouts < 2 {
                return None;
            }
            return Some(bound(action_for(shortcut.id), Some(shortcut)));
        }
        if !self.jump.is_empty() && chord.mods == self.jump {
            let digit = (keysyms::KEY_1..=keysyms::KEY_9).contains(&raw);
            if digit {
                return Some(bound(Action::JumpDesktop((raw - keysyms::KEY_1) as usize), None));
            }
        }
        None
    }
}

impl State {
    pub fn run_action(&mut self, action: Action) {
        info!(?action, "bind");
        match action {
            Action::Terminal => spawn(&self.isle.settings.shortcuts.terminal_command),
            Action::CloseWindow => match self.delegate(false) {
                Some(strokes) => self.send_strokes(&strokes),
                None => {
                    if let Some(id) = self.focused_window() {
                        self.close_window(id);
                    }
                }
            },
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
            Action::QuitApp => match self.delegate(true) {
                Some(strokes) => self.send_strokes(&strokes),
                None => {
                    if let Some(app) = self.isle.front_app() {
                        self.quit_app(&app);
                    }
                }
            },
            Action::CycleWindows(forward) => self.cycle_app_windows(forward),
            Action::Switcher(backwards) => self.open_switcher(backwards),
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
            Action::NextLayout => self.next_layout(),
            Action::Redetect => self.redetect(),
            Action::Media(media) => spawn(media.command()),
            Action::TakeKeysBack => self.toggle_keys_back(),
            Action::ControlCentre => self.control_centre_chord(),
            Action::MenuBar => self.menu_bar_chord(),
            Action::HelpSearch => self.help_chord(),
            Action::WindowMenu => {
                if let Some(id) = self.focused_window() {
                    self.open_window_menu(id, None, true);
                }
            }
            Action::Recorded(chord) => self.finish_recording(chord),
            Action::Reserved(id) => {
                let packet = isle_keys::shortcuts::find(id).and_then(|s| s.packet);
                info!(id, packet, "a shortcut whose action is not built yet");
            }
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
