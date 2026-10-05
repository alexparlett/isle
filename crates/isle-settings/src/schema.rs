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
    pub appearance: Appearance,
    #[setting(section)]
    pub windows: Windows,
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

/// The user's choices within the design tokens (`data/tokens/schema.md`).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Appearance {
    #[setting(default = AppearanceMode::Dark)]
    pub mode: AppearanceMode,
    /// One of the accents `tokens.kdl` defines; its default when unknown.
    #[setting(default = "blue".to_owned())]
    pub accent: String,
    /// The glass from clear, 0, to tinted, 1.
    #[setting(default = 0.5, range = 0.0..=1.0)]
    pub glass: f64,
    /// Every text style multiplies by this, up to the tokens' `type.scale-max`.
    #[setting(default = 1.0, range = 1.0..=2.0)]
    pub text_scale: f64,
    #[setting(default = false)]
    pub increase_contrast: bool,
    #[setting(default = false)]
    pub reduce_motion: bool,
    #[setting(default = false)]
    pub reduce_transparency: bool,
}

/// Light or dark; `auto` follows the sun once Settings can tell where it is (P13), dark until.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum AppearanceMode {
    Dark,
    Light,
    Auto,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Windows {
    /// Tiled and filled windows keep a margin around and between them.
    #[setting(default = true)]
    pub tile_margins: bool,
    /// Apps, by desktop id, that closing their last window hides rather than closes.
    pub keep_running: Vec<String>,
    /// What double-clicking a title bar does.
    #[setting(default = TitlebarAction::Fill)]
    pub titlebar_action: TitlebarAction,
    /// Isle draws title bars on windows that let it.
    #[setting(default = true)]
    pub title_bars: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum TitlebarAction {
    Fill,
    Minimise,
    None,
}

/// Until P5's shortcut set: a chord per action, written as `Super+Return`; empty turns one
/// off. Super is Cmd, Alt is Option.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Binds {
    #[setting(default = "Super+Return".to_owned())]
    pub terminal: String,
    #[setting(default = "Super+W".to_owned())]
    pub close_window: String,
    #[setting(default = "Super+Shift+Escape".to_owned())]
    pub quit: String,
    /// Left half; again, two-thirds, then a third.
    #[setting(default = "Ctrl+Alt+Left".to_owned())]
    pub tile_left: String,
    #[setting(default = "Ctrl+Alt+Right".to_owned())]
    pub tile_right: String,
    /// The centred 60%; again, the centre third.
    #[setting(default = "Ctrl+Alt+C".to_owned())]
    pub tile_centre: String,
    #[setting(default = "Ctrl+Alt+F".to_owned())]
    pub fill: String,
    /// Back to the box before a tile or fill.
    #[setting(default = "Ctrl+Alt+R".to_owned())]
    pub restore: String,
    #[setting(default = "Ctrl+Alt+U".to_owned())]
    pub tile_top_left: String,
    #[setting(default = "Ctrl+Alt+I".to_owned())]
    pub tile_top_right: String,
    #[setting(default = "Ctrl+Alt+J".to_owned())]
    pub tile_bottom_left: String,
    #[setting(default = "Ctrl+Alt+K".to_owned())]
    pub tile_bottom_right: String,
    /// The widest gap beside the other windows.
    #[setting(default = "Ctrl+Alt+B".to_owned())]
    pub tile_beside: String,
    #[setting(default = "Ctrl+Alt+Super+Right".to_owned())]
    pub next_display: String,
    #[setting(default = "Ctrl+Alt+Super+Left".to_owned())]
    pub previous_display: String,
    #[setting(default = "Super+H".to_owned())]
    pub hide: String,
    #[setting(default = "Super+Alt+H".to_owned())]
    pub hide_others: String,
    #[setting(default = "Super+M".to_owned())]
    pub minimise: String,
    #[setting(default = "Super+Alt+M".to_owned())]
    pub minimise_app: String,
    #[setting(default = "Super+Q".to_owned())]
    pub quit_app: String,
    /// The front app's next window.
    #[setting(default = "Super+grave".to_owned())]
    pub next_window: String,
    #[setting(default = "Super+Shift+grave".to_owned())]
    pub previous_window: String,
    #[setting(default = "Ctrl+Super+F".to_owned())]
    pub full_screen: String,
    /// The zoom menu for the focused window.
    #[setting(default = "Ctrl+Alt+Z".to_owned())]
    pub zoom_menu: String,
    #[setting(default = "Ctrl+Left".to_owned())]
    pub desktop_left: String,
    #[setting(default = "Ctrl+Right".to_owned())]
    pub desktop_right: String,
    /// The modifiers that with 1 to 9 jump to a desktop.
    #[setting(default = "Ctrl".to_owned())]
    pub desktop_jump: String,
    /// The focused window one desktop along, the view following.
    #[setting(default = "Ctrl+Shift+Super+Left".to_owned())]
    pub carry_left: String,
    #[setting(default = "Ctrl+Shift+Super+Right".to_owned())]
    pub carry_right: String,
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
