//! What `settings.kdl` holds: P2's keys. Later packets add theirs here.

use std::collections::BTreeMap;

use crate::{Choice, Section};

#[derive(Clone, Debug, PartialEq, Section)]
#[setting(retired = "binds", check = crate::shortcuts::check)]
pub struct Settings {
    /// The DRM render node to draw with; the boot GPU when unset.
    pub render_device: Option<String>,
    #[setting(section)]
    pub keyboard: Keyboard,
    /// Keyboards, by name.
    #[setting(keyed)]
    pub keyboard_device: BTreeMap<String, KeyboardDevice>,
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
    pub island: Island,
    #[setting(section)]
    pub island_glyphs: IslandGlyphs,
    #[setting(section)]
    pub shell: Shell,
    #[setting(section)]
    pub control_centre: ControlCentre,
    #[setting(section)]
    pub hot_corners: HotCorners,
    #[setting(section)]
    pub wallpaper: Wallpaper,
    /// A screen's own wallpaper, by make, model and serial, in place of `wallpaper`.
    #[setting(keyed)]
    pub wallpaper_output: BTreeMap<String, Wallpaper>,
    #[setting(section)]
    pub focus: Focus,
    /// A Focus mode's rules, by its name; Do Not Disturb and Game Mode are always there.
    #[setting(keyed)]
    pub focus_mode: BTreeMap<String, FocusMode>,
    #[setting(section)]
    pub notifications: Notifications,
    #[setting(section)]
    pub lock_screen: ScreenLock,
    /// An app's notification rules, by its desktop id.
    #[setting(keyed)]
    pub notification_app: BTreeMap<String, NotificationApp>,
    #[setting(section)]
    pub shortcuts: Shortcuts,
    #[setting(section)]
    pub spotlight: Spotlight,
    #[setting(section)]
    pub capture: Capture,
    #[setting(section)]
    pub night_light: NightLight,
    #[setting(section)]
    pub debug: Debug,
    #[setting(app = "io.github.alexparlett.Isle.Example")]
    pub example_app: ExampleApp,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Keyboard {
    /// XKB layouts, the first the default; empty takes the system's, as `localectl` sets it.
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
    /// A modifier tapped applies to the next key, tapped twice stays until tapped again;
    /// Shift pressed five times turns this on or off.
    #[setting(default = false)]
    pub sticky_keys: bool,
    /// Milliseconds a key must be held to count; 0 turns Slow Keys off.
    #[setting(default = 0, range = 0..=2000)]
    pub slow_keys: i32,
    /// Cmd shortcuts are sent as each app's own (B21).
    #[setting(default = true)]
    pub translate: bool,
    /// Apps that get every key untranslated, by desktop id, app id or X11 class; a trailing
    /// `*` matches the rest. Remote desktops and virtual machines by default.
    #[setting(default = [
        "org.remmina.Remmina", "remmina", "virt-manager", "org.virt-manager.virt-manager",
        "org.gnome.Boxes", "VirtualBox Machine", "VirtualBox Manager", "anydesk", "AnyDesk",
        "xfreerdp", "wlfreerdp", "sdl-freerdp", "org.kde.krdc", "remote-viewer",
        "virt-viewer", "qemu*", "looking-glass-client", "com.moonlight_stream.Moonlight",
        "moonlight", "spicy",
    ].map(String::from).to_vec())]
    pub untranslated_apps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct KeyboardDevice {
    /// Mac or PC; `auto` guesses from the keyboard's name.
    #[setting(default = KeyboardProfile::Auto)]
    pub profile: KeyboardProfile,
    /// On a PC keyboard, left Alt and left Super trade places, so Cmd sits beside the
    /// space bar (B30).
    #[setting(default = true)]
    pub swap_alt_super: bool,
    /// Whether the keyboard has the key left of Z; `auto` guesses from the first layout.
    #[setting(default = KeyboardModel::Auto)]
    pub model: KeyboardModel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum KeyboardProfile {
    Auto,
    Mac,
    Pc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum KeyboardModel {
    Auto,
    Iso,
    Ansi,
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
    /// A full-screen game that asks for it is flipped without waiting for vblank.
    #[setting(default = false)]
    pub tearing: bool,
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

/// Night light: every screen warmed on a schedule (`compositor.md` Outputs 8).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct NightLight {
    #[setting(default = NightSchedule::SunsetToSunrise)]
    pub schedule: NightSchedule,
    /// When a custom schedule turns it on, `HH:MM` in local time.
    #[setting(default = "22:00".to_owned())]
    pub from: String,
    /// When a custom schedule turns it off.
    #[setting(default = "07:00".to_owned())]
    pub to: String,
    /// The colour temperature it warms to, in kelvin: lower is warmer.
    #[setting(default = 3400, range = 2700..=6000)]
    pub warmth: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum NightSchedule {
    /// Only when turned on by hand.
    Off,
    /// From sunset to sunrise where the time zone is.
    SunsetToSunrise,
    /// From `from` to `to`.
    Custom,
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
    /// The glass from clear, 0, to tinted, 1; the floor darkens glass under text only as far
    /// as the text needs (B58).
    #[setting(default = 0.0, range = 0.0..=1.0)]
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

/// Light or dark; `auto` is dark from sunset to sunrise at the time zone's place (`isle-sun`).
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
    /// A window dragged to a screen's edge or corner tiles there.
    #[setting(default = true)]
    pub edge_tiling: bool,
    /// Apps, by desktop id, that closing their last window hides rather than closes.
    pub keep_running: Vec<String>,
    /// What double-clicking a title bar does.
    #[setting(default = TitlebarAction::Fill)]
    pub titlebar_action: TitlebarAction,
    /// Isle draws title bars on windows that let it.
    #[setting(default = true)]
    pub title_bars: bool,
    /// Apps that are games though nothing says so, by desktop id, app id or X11 class.
    pub games: Vec<String>,
    /// Apps that are not games though they look like one.
    pub not_games: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum TitlebarAction {
    Fill,
    Minimise,
    None,
}

/// The desktop's chords: each action's chords in Mac names (`Shift+Cmd+K`) or symbols
/// (`⇧⌘K`); an empty list turns an action off. A chord bound twice is refused.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Shortcuts {
    /// Close the front window; a keep-running app's last window hides it.
    #[setting(default = vec!["Cmd+W".to_owned()])]
    pub close_window: Vec<String>,
    /// Minimise the front window.
    #[setting(default = vec!["Cmd+M".to_owned()])]
    pub minimise: Vec<String>,
    /// Minimise every window of the front app.
    #[setting(default = vec!["Option+Cmd+M".to_owned()])]
    pub minimise_app: Vec<String>,
    /// Hide the front app.
    #[setting(default = vec!["Cmd+H".to_owned()])]
    pub hide: Vec<String>,
    /// Hide every app but the front one.
    #[setting(default = vec!["Option+Cmd+H".to_owned()])]
    pub hide_others: Vec<String>,
    /// Quit the front app, keep-running apps included.
    #[setting(default = vec!["Cmd+Q".to_owned()])]
    pub quit_app: Vec<String>,
    /// Enter or leave full screen.
    #[setting(default = vec!["Ctrl+Cmd+F".to_owned()])]
    pub full_screen: Vec<String>,
    /// Left half; again, two-thirds, then a third.
    #[setting(default = vec!["Ctrl+Option+Left".to_owned()])]
    pub tile_left: Vec<String>,
    /// Right half; again, two-thirds, then a third.
    #[setting(default = vec!["Ctrl+Option+Right".to_owned()])]
    pub tile_right: Vec<String>,
    /// The centred 60%; again, the centre third.
    #[setting(default = vec!["Ctrl+Option+C".to_owned()])]
    pub tile_centre: Vec<String>,
    /// Fill the screen.
    #[setting(default = vec!["Ctrl+Option+F".to_owned()])]
    pub fill: Vec<String>,
    /// Return to the size before a tile or fill.
    #[setting(default = vec!["Ctrl+Option+R".to_owned()])]
    pub restore: Vec<String>,
    /// Top-left quarter.
    #[setting(default = vec!["Ctrl+Option+U".to_owned()])]
    pub tile_top_left: Vec<String>,
    /// Top-right quarter.
    #[setting(default = vec!["Ctrl+Option+I".to_owned()])]
    pub tile_top_right: Vec<String>,
    /// Bottom-left quarter.
    #[setting(default = vec!["Ctrl+Option+J".to_owned()])]
    pub tile_bottom_left: Vec<String>,
    /// Bottom-right quarter.
    #[setting(default = vec!["Ctrl+Option+K".to_owned()])]
    pub tile_bottom_right: Vec<String>,
    /// Fill the widest space beside the other windows.
    #[setting(default = vec!["Ctrl+Option+B".to_owned()])]
    pub tile_beside: Vec<String>,
    /// Move the window to the next display.
    #[setting(default = vec!["Ctrl+Option+Cmd+Right".to_owned()])]
    pub next_display: Vec<String>,
    /// Move the window to the previous display.
    #[setting(default = vec!["Ctrl+Option+Cmd+Left".to_owned()])]
    pub previous_display: Vec<String>,
    /// Open the zoom menu for the front window.
    #[setting(default = vec!["Ctrl+Option+Z".to_owned()])]
    pub zoom_menu: Vec<String>,
    /// Open the window menu.
    #[setting(default = vec!["Ctrl+Option+Space".to_owned()])]
    pub window_menu: Vec<String>,
    /// The front app's next window.
    #[setting(default = vec!["Cmd+`".to_owned()])]
    pub next_window: Vec<String>,
    /// The front app's previous window.
    #[setting(default = vec!["Shift+Cmd+`".to_owned()])]
    pub previous_window: Vec<String>,
    /// Keep the front window on top.
    pub pin: Vec<String>,
    /// Focus the window to the left.
    pub focus_left: Vec<String>,
    /// Focus the window to the right.
    pub focus_right: Vec<String>,
    /// Focus the window above.
    pub focus_up: Vec<String>,
    /// Focus the window below.
    pub focus_down: Vec<String>,
    /// The desktop to the left.
    #[setting(default = vec!["Ctrl+Left".to_owned()])]
    pub desktop_left: Vec<String>,
    /// The desktop to the right.
    #[setting(default = vec!["Ctrl+Right".to_owned()])]
    pub desktop_right: Vec<String>,
    /// Carry the front window one desktop left.
    #[setting(default = vec!["Ctrl+Shift+Cmd+Left".to_owned()])]
    pub carry_left: Vec<String>,
    /// Carry the front window one desktop right.
    #[setting(default = vec!["Ctrl+Shift+Cmd+Right".to_owned()])]
    pub carry_right: Vec<String>,
    /// Mission Control.
    #[setting(default = vec!["Ctrl+Up".to_owned(), "XF86LaunchA".to_owned()])]
    pub mission_control: Vec<String>,
    /// The front app's windows, spread out.
    #[setting(default = vec!["Ctrl+Down".to_owned()])]
    pub app_expose: Vec<String>,
    /// Show the desktop.
    #[setting(default = vec!["Cmd+F3".to_owned()])]
    pub show_desktop: Vec<String>,
    /// The app switcher; it commits when Cmd is released.
    #[setting(default = vec!["Cmd+Tab".to_owned()])]
    pub switcher: Vec<String>,
    /// The app switcher, from the end.
    #[setting(default = vec!["Shift+Cmd+Tab".to_owned()])]
    pub switcher_back: Vec<String>,
    /// Spotlight.
    #[setting(default = vec!["Cmd+Space".to_owned()])]
    pub spotlight: Vec<String>,
    /// Spotlight's Apps view.
    #[setting(default = vec!["XF86LaunchB".to_owned()])]
    pub apps: Vec<String>,
    /// Spotlight in Emoji.
    #[setting(default = vec!["Ctrl+Cmd+Space".to_owned()])]
    pub emoji: Vec<String>,
    /// Force Quit.
    #[setting(default = vec!["Option+Cmd+Escape".to_owned()])]
    pub force_quit: Vec<String>,
    /// Open a terminal.
    pub terminal: Vec<String>,
    /// Open Settings.
    pub settings: Vec<String>,
    /// Open Monitor.
    pub monitor: Vec<String>,
    /// Open a Files window.
    pub files: Vec<String>,
    /// The menu bar, with focus on its first menu.
    #[setting(default = vec!["Ctrl+F2".to_owned()])]
    pub menu_bar: Vec<String>,
    /// The menu bar, searching the front app's menus.
    #[setting(default = vec!["Shift+Cmd+/".to_owned()])]
    pub help_search: Vec<String>,
    /// Control Centre.
    #[setting(default = vec!["Ctrl+F8".to_owned()])]
    pub control_centre: Vec<String>,
    /// Notification Centre.
    #[setting(default = vec!["Ctrl+Option+Cmd+N".to_owned()])]
    pub notification_centre: Vec<String>,
    /// The widgets dashboard.
    #[setting(default = vec!["Ctrl+Option+Cmd+W".to_owned()])]
    pub dashboard: Vec<String>,
    /// Show or hide the dropdown terminal.
    #[setting(default = vec!["Ctrl+Option+`".to_owned()])]
    pub dropdown_terminal: Vec<String>,
    /// Show or hide the on-screen keyboard.
    #[setting(default = vec!["Option+Cmd+F5".to_owned()])]
    pub on_screen_keyboard: Vec<String>,
    /// Screenshot the screen.
    #[setting(default = vec!["Shift+Cmd+3".to_owned(), "Shift+Print".to_owned()])]
    pub screenshot_screen: Vec<String>,
    /// Screenshot a portion or a window.
    #[setting(default = vec!["Shift+Cmd+4".to_owned(), "Print".to_owned()])]
    pub screenshot_pick: Vec<String>,
    /// The capture toolbar.
    #[setting(default = vec!["Shift+Cmd+5".to_owned()])]
    pub capture_toolbar: Vec<String>,
    /// Copy a screenshot of the screen.
    #[setting(default = vec!["Ctrl+Shift+Cmd+3".to_owned(), "Ctrl+Shift+Print".to_owned()])]
    pub screenshot_screen_copy: Vec<String>,
    /// Copy a screenshot of a portion or a window.
    #[setting(default = vec!["Ctrl+Shift+Cmd+4".to_owned(), "Ctrl+Print".to_owned()])]
    pub screenshot_pick_copy: Vec<String>,
    /// The capture toolbar, copying.
    #[setting(default = vec!["Ctrl+Shift+Cmd+5".to_owned()])]
    pub capture_toolbar_copy: Vec<String>,
    /// Pick a colour from the screen.
    #[setting(default = vec!["Shift+Cmd+6".to_owned()])]
    pub colour_pick: Vec<String>,
    /// Stop a recording.
    #[setting(default = vec!["Ctrl+Cmd+Escape".to_owned()])]
    pub stop_recording: Vec<String>,
    /// Lock the screen.
    #[setting(default = vec!["Ctrl+Cmd+Q".to_owned()])]
    pub lock: Vec<String>,
    /// Log out, after asking.
    #[setting(default = vec!["Shift+Cmd+Q".to_owned()])]
    pub log_out: Vec<String>,
    /// Log out now.
    #[setting(default = vec!["Option+Shift+Cmd+Q".to_owned()])]
    pub log_out_now: Vec<String>,
    /// Restart, sleep or shut down.
    #[setting(default = vec!["Ctrl+Cmd+BackSpace".to_owned()])]
    pub session_chooser: Vec<String>,
    /// Turn Do Not Disturb on or off.
    #[setting(default = vec!["Ctrl+Option+Cmd+D".to_owned()])]
    pub do_not_disturb: Vec<String>,
    /// Turn Game Mode on or off.
    #[setting(default = vec!["Ctrl+Option+Cmd+G".to_owned()])]
    pub game_mode: Vec<String>,
    /// Detect the displays again.
    #[setting(default = vec!["Ctrl+Option+Cmd+R".to_owned()])]
    pub redetect_displays: Vec<String>,
    /// The next keyboard layout; only while two or more are set.
    #[setting(default = vec!["Ctrl+Space".to_owned()])]
    pub next_layout: Vec<String>,
    /// Take the keys back from a window holding them, or give them back.
    #[setting(default = vec!["Cmd+Escape".to_owned()])]
    pub take_keys_back: Vec<String>,
    /// Turn Zoom on or off; macOS uses Option+Cmd+8.
    pub zoom: Vec<String>,
    /// Zoom in; macOS uses Option+Cmd+=.
    pub zoom_in: Vec<String>,
    /// Zoom out; macOS uses Option+Cmd+-.
    pub zoom_out: Vec<String>,
    /// Volume up.
    #[setting(default = vec!["XF86AudioRaiseVolume".to_owned()])]
    pub volume_up: Vec<String>,
    /// Volume down.
    #[setting(default = vec!["XF86AudioLowerVolume".to_owned()])]
    pub volume_down: Vec<String>,
    /// Mute or unmute the output.
    #[setting(default = vec!["XF86AudioMute".to_owned()])]
    pub mute: Vec<String>,
    /// Mute or unmute the microphone.
    #[setting(default = vec!["XF86AudioMicMute".to_owned()])]
    pub mic_mute: Vec<String>,
    /// Brightness up.
    #[setting(default = vec!["XF86MonBrightnessUp".to_owned()])]
    pub brightness_up: Vec<String>,
    /// Brightness down.
    #[setting(default = vec!["XF86MonBrightnessDown".to_owned()])]
    pub brightness_down: Vec<String>,
    /// Play or pause.
    #[setting(default = vec!["XF86AudioPlay".to_owned(), "XF86AudioPause".to_owned()])]
    pub play_pause: Vec<String>,
    /// The next track.
    #[setting(default = vec!["XF86AudioNext".to_owned()])]
    pub next_track: Vec<String>,
    /// The previous track.
    #[setting(default = vec!["XF86AudioPrev".to_owned()])]
    pub previous_track: Vec<String>,
    /// The modifiers that with 1 to 9 jump to a desktop.
    #[setting(default = "Ctrl".to_owned())]
    pub desktop_jump: String,
    /// The modifiers that with a press anywhere on a window move it, or with the right
    /// button resize it.
    #[setting(default = "Ctrl+Cmd".to_owned())]
    pub window_drag: String,
    /// What the terminal shortcut starts.
    #[setting(default = "kitty".to_owned())]
    pub terminal_command: String,
}

impl Shortcuts {
    /// Every action's chords, by the action's id.
    pub fn chords(&self) -> Vec<(&'static str, &[String])> {
        vec![
            ("close-window", &self.close_window),
            ("minimise", &self.minimise),
            ("minimise-app", &self.minimise_app),
            ("hide", &self.hide),
            ("hide-others", &self.hide_others),
            ("quit-app", &self.quit_app),
            ("full-screen", &self.full_screen),
            ("tile-left", &self.tile_left),
            ("tile-right", &self.tile_right),
            ("tile-centre", &self.tile_centre),
            ("fill", &self.fill),
            ("restore", &self.restore),
            ("tile-top-left", &self.tile_top_left),
            ("tile-top-right", &self.tile_top_right),
            ("tile-bottom-left", &self.tile_bottom_left),
            ("tile-bottom-right", &self.tile_bottom_right),
            ("tile-beside", &self.tile_beside),
            ("next-display", &self.next_display),
            ("previous-display", &self.previous_display),
            ("zoom-menu", &self.zoom_menu),
            ("window-menu", &self.window_menu),
            ("next-window", &self.next_window),
            ("previous-window", &self.previous_window),
            ("pin", &self.pin),
            ("focus-left", &self.focus_left),
            ("focus-right", &self.focus_right),
            ("focus-up", &self.focus_up),
            ("focus-down", &self.focus_down),
            ("desktop-left", &self.desktop_left),
            ("desktop-right", &self.desktop_right),
            ("carry-left", &self.carry_left),
            ("carry-right", &self.carry_right),
            ("mission-control", &self.mission_control),
            ("app-expose", &self.app_expose),
            ("show-desktop", &self.show_desktop),
            ("switcher", &self.switcher),
            ("switcher-back", &self.switcher_back),
            ("spotlight", &self.spotlight),
            ("apps", &self.apps),
            ("emoji", &self.emoji),
            ("force-quit", &self.force_quit),
            ("terminal", &self.terminal),
            ("settings", &self.settings),
            ("monitor", &self.monitor),
            ("files", &self.files),
            ("menu-bar", &self.menu_bar),
            ("help-search", &self.help_search),
            ("control-centre", &self.control_centre),
            ("notification-centre", &self.notification_centre),
            ("dashboard", &self.dashboard),
            ("dropdown-terminal", &self.dropdown_terminal),
            ("on-screen-keyboard", &self.on_screen_keyboard),
            ("screenshot-screen", &self.screenshot_screen),
            ("screenshot-pick", &self.screenshot_pick),
            ("capture-toolbar", &self.capture_toolbar),
            ("screenshot-screen-copy", &self.screenshot_screen_copy),
            ("screenshot-pick-copy", &self.screenshot_pick_copy),
            ("capture-toolbar-copy", &self.capture_toolbar_copy),
            ("colour-pick", &self.colour_pick),
            ("stop-recording", &self.stop_recording),
            ("lock", &self.lock),
            ("log-out", &self.log_out),
            ("log-out-now", &self.log_out_now),
            ("session-chooser", &self.session_chooser),
            ("do-not-disturb", &self.do_not_disturb),
            ("game-mode", &self.game_mode),
            ("redetect-displays", &self.redetect_displays),
            ("next-layout", &self.next_layout),
            ("take-keys-back", &self.take_keys_back),
            ("zoom", &self.zoom),
            ("zoom-in", &self.zoom_in),
            ("zoom-out", &self.zoom_out),
            ("volume-up", &self.volume_up),
            ("volume-down", &self.volume_down),
            ("mute", &self.mute),
            ("mic-mute", &self.mic_mute),
            ("brightness-up", &self.brightness_up),
            ("brightness-down", &self.brightness_down),
            ("play-pause", &self.play_pause),
            ("next-track", &self.next_track),
            ("previous-track", &self.previous_track),
        ]
    }
}

/// The island's clock (`p1/structure/island.md`).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Island {
    /// 24-hour time; `auto` follows the locale's time format.
    #[setting(default = ClockHours::Auto)]
    pub clock_24_hour: ClockHours,
    #[setting(default = false)]
    pub clock_weekday: bool,
    #[setting(default = false)]
    pub clock_date: bool,
    #[setting(default = false)]
    pub clock_seconds: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum ClockHours {
    Auto,
    On,
    Off,
}

/// When each status glyph shows in the island; the normal state shows nothing.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct IslandGlyphs {
    #[setting(default = GlyphTier::WhenActive)]
    pub wifi: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub bluetooth: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub sound: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub vpn: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub focus: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub updates: GlyphTier,
    #[setting(default = GlyphTier::WhenActive)]
    pub battery: GlyphTier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum GlyphTier {
    Always,
    WhenActive,
    Never,
}

/// Control Centre's grid.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct ControlCentre {
    /// The modules in order, each `id:size` with size `s`, `m`, `l` or `w`; a module that
    /// has nothing to control here closes up.
    #[setting(default = [
        "wifi:m", "now-playing:l", "bluetooth:m", "focus:m", "vpn:m", "sound:w", "display:w",
        "night-light:s", "energy-mode:s", "updates:m", "batteries:m",
    ].map(String::from).to_vec())]
    pub modules: Vec<String>,
    /// Tray items in order by their id; items not named follow in the order they came.
    pub tray: Vec<String>,
    /// Tray items kept out of the tray, by their id.
    pub tray_hidden: Vec<String>,
}

/// What each screen corner does when the pointer rests in it (`mission-control.md`, Hot
/// corners).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct HotCorners {
    #[setting(default = Corner::None)]
    pub top_left: Corner,
    #[setting(default = Corner::None)]
    pub top_right: Corner,
    #[setting(default = Corner::None)]
    pub bottom_left: Corner,
    #[setting(default = Corner::Apps)]
    pub bottom_right: Corner,
    /// The modifiers a corner needs held to fire, as `Cmd` or `Option+Shift`; none by default.
    pub top_left_modifiers: String,
    pub top_right_modifiers: String,
    pub bottom_left_modifiers: String,
    pub bottom_right_modifiers: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum Corner {
    None,
    NotificationCentre,
    Apps,
    LockScreen,
    DisplaySleep,
}

/// The shell's screens./// The shell's screens.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Shell {
    /// The screen events and Spotlight show on, by make, model and serial; unset follows
    /// focus.
    pub main_screen: Option<String>,
}

/// Spotlight's own settings (`p1/structure/spotlight.md`).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Spotlight {
    /// The Apps view's favourites in order, by desktop id or game (`steam:570`).
    pub favourites: Vec<String>,
    /// Folders file search looks in besides home.
    pub files_include: Vec<String>,
    /// Folders file search leaves out.
    pub files_exclude: Vec<String>,
    /// Apps whose copies clipboard history never keeps, by desktop id or app id.
    #[setting(default = vec!["org.keepassxc.KeePassXC".to_owned(), "com.bitwarden.desktop".to_owned()])]
    pub clipboard_exclude: Vec<String>,
    /// The web search's address, with `{query}` where the query goes.
    #[setting(default = "https://duckduckgo.com/?q={query}".to_owned())]
    pub web_search: String,
}

/// Screenshots and recordings: the capture toolbar's Options (`p1/structure/capture.md`).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Capture {
    /// `default` for Pictures/Screenshots and Videos/Recordings, `documents`, `clipboard`,
    /// or a folder's path.
    #[setting(default = "default".to_owned())]
    pub save_to: String,
    /// Seconds before a shot or recording starts: 0, 5 or 10.
    pub timer: i32,
    /// The input device a recording takes sound from, by name; none when unset.
    pub microphone: Option<String>,
    /// Whether a recording takes what the computer plays.
    pub system_audio: bool,
    #[setting(default = true)]
    pub floating_thumbnail: bool,
    /// Whether the toolbar's portion opens where it was last.
    pub remember_selection: bool,
    /// Whether shots and recordings show the pointer.
    pub show_pointer: bool,
    /// Whether a recording marks each click.
    pub show_clicks: bool,
    /// How a picked colour is copied.
    #[setting(default = ColourFormat::Hex)]
    pub colour_format: ColourFormat,
    /// The sound a shot makes.
    #[setting(default = true)]
    pub sound: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum ColourFormat {
    /// `#1e1f23`
    Hex,
    /// `rgb(30, 31, 35)`
    Rgb,
    /// `hsl(228, 8%, 13%)`
    Hsl,
}

/// The picture behind the windows (`p1/structure/wallpaper.md`).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct Wallpaper {
    /// `frost` for Isle's drawn backdrop, a colour as `#rrggbb`, or the path of a picture
    /// or of a folder of pictures.
    #[setting(default = "frost".to_owned())]
    pub source: String,
    /// What shows in the dark appearance instead, when set.
    pub dark_source: Option<String>,
    #[setting(default = Fit::Fill)]
    pub fit: Fit,
    /// `#rrggbb` around a fitted or centred picture; the picture's mean colour when unset.
    pub colour: Option<String>,
    /// When a folder moves on to its next picture.
    #[setting(default = Change::Daily)]
    pub change: Change,
    /// A folder's pictures in random order rather than by name.
    #[setting(default = false)]
    pub shuffle: bool,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Focus {
    /// Game Mode turns on by itself while a detected game is in front.
    #[setting(default = true)]
    pub game_mode_auto: bool,
    /// Games Game Mode stays off for, by app id.
    pub games_off: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct FocusMode {
    /// Its symbol, a Phosphor glyph's name.
    pub symbol: Option<String>,
    /// Apps whose banners and sounds still come through.
    pub allow_apps: Vec<String>,
    /// Time-sensitive (critical) notifications come through.
    #[setting(default = true)]
    pub allow_critical: bool,
    /// When it turns itself on: `HH:MM-HH:MM` and `every-day`, `weekdays`, `weekends` or
    /// days such as `mon,wed`.
    pub schedules: Vec<String>,
    /// Apps whose being in front turns it on.
    pub while_apps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Notifications {
    /// Hold banners while a window is full screen, unless another screen is free of one.
    #[setting(default = true)]
    pub hold_full_screen: bool,
    /// Hold banners while a screen is shared or recorded.
    #[setting(default = true)]
    pub hold_sharing: bool,
    /// Play a sound for notifications.
    #[setting(default = true)]
    pub sounds: bool,
    /// The alert sound, by its name in the sound theme.
    #[setting(default = "message-new-instant".to_owned())]
    pub alert_sound: String,
    /// What the lock screen shows of notifications.
    #[setting(default = LockScreen::Count)]
    pub lock_screen: LockScreen,
}

/// Idle's steps and the lock before sleep (Settings › Lock Screen).
#[derive(Clone, Debug, PartialEq, Section)]
pub struct ScreenLock {
    /// Minutes with no input before the screens dim; 0 never.
    #[setting(default = 5, range = 0..=240)]
    pub dim_after: i32,
    /// Minutes with no input before the session locks; 0 never.
    #[setting(default = 10, range = 0..=240)]
    pub lock_after: i32,
    /// Minutes with no input before the screens turn off; 0 never.
    #[setting(default = 15, range = 0..=240)]
    pub display_off_after: i32,
    /// Lock before every sleep.
    #[setting(default = true)]
    pub lock_on_sleep: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum LockScreen {
    /// Nothing.
    None,
    /// Each app with its count.
    Count,
    /// Each notification's app and summary.
    Previews,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct NotificationApp {
    /// Off mutes the app: no banner, no sound, still kept in the centre.
    #[setting(default = true)]
    pub allow: bool,
    #[setting(default = BannerStyle::Temporary)]
    pub banner_style: BannerStyle,
    #[setting(default = true)]
    pub sound: bool,
    /// The switcher's and the Apps grid's badge for the app.
    #[setting(default = true)]
    pub badges: bool,
    /// Its banners break through a full-screen window.
    #[setting(default = false)]
    pub over_full_screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum BannerStyle {
    /// Leaves by itself.
    Temporary,
    /// Stays until acted on or closed.
    Persistent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum Fit {
    /// Scaled to cover the screen, cropped about the centre.
    Fill,
    /// Scaled to fit whole, on the colour.
    Fit,
    /// One picture pixel to one screen pixel, on the colour.
    Centre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Choice)]
pub enum Change {
    Never,
    Hourly,
    Daily,
    Login,
}

#[derive(Clone, Debug, PartialEq, Section)]
pub struct Debug {
    /// Whether a main loop stuck for ten seconds ends the process (P2 choice 7).
    #[setting(default = true)]
    pub watchdog_abort: bool,
    /// GTK apps are told the desktop shows their menubar, and hide their own.
    #[setting(default = true)]
    pub gtk_global_menu: bool,
}

/// An app's preferences, to show how one is kept (B27); the first real app replaces it.
#[derive(Clone, Debug, PartialEq, Section)]
pub struct ExampleApp {
    #[setting(default = false)]
    pub show_hidden: bool,
    #[setting(default = 280, range = 120..=600)]
    pub sidebar_width: i32,
}
