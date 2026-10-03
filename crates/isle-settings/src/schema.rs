//! What `settings.kdl` holds: P2's keys. Later packets add theirs here.

use std::collections::BTreeMap;

use crate::{Choice, Section};

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Settings {
    /// The DRM render node to draw with; the boot GPU when unset.
    pub render_device: Option<String>,
    #[setting(section)]
    pub keyboard: Keyboard,
    /// Displays, by make, model and serial.
    #[setting(keyed)]
    pub output: BTreeMap<String, Output>,
    /// Pointing devices, by name.
    #[setting(keyed)]
    pub pointer: BTreeMap<String, Pointer>,
    #[setting(section)]
    pub binds: Binds,
    #[setting(section)]
    pub debug: Debug,
    #[setting(app = "io.github.alexparlett.Isle.Example")]
    pub example_app: ExampleApp,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Keyboard {
    /// XKB layouts, the first the default.
    #[setting(default = vec!["us".to_owned()])]
    pub layouts: Vec<String>,
    /// XKB variants, one per layout; empty for the plain one.
    pub variants: Vec<String>,
    /// XKB options, such as `compose:ralt`.
    pub options: Vec<String>,
    /// Keys per second while a key is held.
    #[setting(default = 25, range = 1..=100)]
    pub repeat_rate: i32,
    /// Milliseconds before a held key repeats.
    #[setting(default = 600, range = 100..=2000)]
    pub repeat_delay: i32,
    #[setting(default = true)]
    pub numlock: bool,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Output {
    #[setting(default = true)]
    pub enabled: bool,
    /// `WIDTHxHEIGHT` or `WIDTHxHEIGHT@HZ`; the preferred mode when unset.
    pub mode: Option<String>,
    #[setting(default = 1.0, range = 0.5..=4.0)]
    pub scale: f64,
    /// Where the output sits in the layout, in logical points: `x`.
    pub x: Option<i32>,
    pub y: Option<i32>,
    #[setting(default = Transform::Normal)]
    pub transform: Transform,
    #[setting(default = Vrr::Fullscreen)]
    pub vrr: Vrr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum Transform {
    Normal,
    #[setting(name = "90")]
    Rotate90,
    #[setting(name = "180")]
    Rotate180,
    #[setting(name = "270")]
    Rotate270,
    Flipped,
    #[setting(name = "flipped-90")]
    Flipped90,
    #[setting(name = "flipped-180")]
    Flipped180,
    #[setting(name = "flipped-270")]
    Flipped270,
}

/// When variable refresh is on (B30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum Vrr {
    Off,
    /// While a full-screen app is in front.
    Fullscreen,
    Always,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Pointer {
    #[setting(default = AccelProfile::Flat)]
    pub accel_profile: AccelProfile,
    /// From -1, slowest, to 1, fastest.
    #[setting(default = 0.0, range = -1.0..=1.0)]
    pub speed: f64,
    pub natural_scroll: Option<bool>,
    #[setting(default = false)]
    pub left_handed: bool,
    pub tap: Option<bool>,
    pub scroll_method: Option<ScrollMethod>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum AccelProfile {
    Flat,
    Adaptive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum ScrollMethod {
    TwoFinger,
    Edge,
    OnButtonDown,
    NoScroll,
}

/// Until P5's shortcut set: a chord per action, written as `Super+Return`.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Binds {
    #[setting(default = "Super+Return".to_owned())]
    pub terminal: String,
    #[setting(default = "Super+W".to_owned())]
    pub close_window: String,
    #[setting(default = "Super+Shift+Escape".to_owned())]
    pub quit: String,
    /// The terminal the bind starts.
    #[setting(default = "kitty".to_owned())]
    pub terminal_command: String,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Debug {
    /// Whether a main loop stuck for ten seconds ends the process (P2 choice 7).
    #[setting(default = true)]
    pub watchdog_abort: bool,
}

/// An app's preferences, to show how one is kept (B27); the first real app replaces it.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct ExampleApp {
    #[setting(default = false)]
    pub show_hidden: bool,
    #[setting(default = 280, range = 120..=600)]
    pub sidebar_width: i32,
}
