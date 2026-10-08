//! The D-Bus interfaces of `io.github.alexparlett.Isle`, as proxies and the types they carry.

use serde::{Deserialize, Serialize};
use zbus::proxy;
use zvariant::Type;

pub const BUS_NAME: &str = "io.github.alexparlett.Isle";
pub const VERSION_PATH: &str = "/io/github/alexparlett/Isle/Version";
pub const SESSION_PATH: &str = "/io/github/alexparlett/Isle/Session";
pub const WINDOWS_PATH: &str = "/io/github/alexparlett/Isle/Windows";
pub const DISPLAYS_PATH: &str = "/io/github/alexparlett/Isle/Displays";
pub const TEST_PATH: &str = "/io/github/alexparlett/Isle/Test";
pub const SETTINGS_PATH: &str = "/io/github/alexparlett/Isle/Settings";
pub const SHELL_PATH: &str = "/io/github/alexparlett/Isle/Shell";
pub const DESKTOPS_PATH: &str = "/io/github/alexparlett/Isle/Desktops";
pub const KEYBOARD_PATH: &str = "/io/github/alexparlett/Isle/Keyboard";
pub const WALLPAPER_PATH: &str = "/io/github/alexparlett/Isle/Wallpaper";
pub const ISLAND_PATH: &str = "/io/github/alexparlett/Isle/Island";
pub const MENUS_PATH: &str = "/io/github/alexparlett/Isle/Menus";
pub const SERVICES_PATH: &str = "/io/github/alexparlett/Isle/Services";
pub const NOTIFICATIONS_PATH: &str = "/io/github/alexparlett/Isle/Notifications";
pub const FOCUS_PATH: &str = "/io/github/alexparlett/Isle/Focus";

/// One key of `settings.kdl`, as the Settings interface introspects it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct SettingKey {
    pub key: String,
    pub kind: String,
    pub default: String,
    pub constraint: String,
    pub doc: String,
}

/// What one screen's wallpaper shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct WallpaperInfo {
    pub output: String,
    /// The `source` setting in force for the screen, for the appearance.
    pub source: String,
    /// The picture shown, or empty for the drawn backdrop or a colour.
    pub file: String,
    /// Why the source is not shown, or empty.
    pub problem: String,
}

/// Whether `settings.kdl` is in force, or the rejected hand edit that is not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct SettingsStatus {
    pub ok: bool,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct WindowInfo {
    pub id: u64,
    pub app_id: String,
    /// The app the window belongs to, which groups it for hide, quit and the switcher.
    pub app: String,
    pub title: String,
    /// The output the window is on, or empty while it is on none.
    pub output: String,
    pub focused: bool,
    /// The window's box in the global space, in logical points.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// `floating`, `tiled:<zone>`, `filled` or `fullscreen`.
    pub mode: String,
    pub minimised: bool,
    /// Its app is hidden.
    pub hidden: bool,
    /// The window it is a dialog of, or 0.
    pub parent: u64,
    /// It asked for focus without a fresh activation token.
    pub attention: bool,
    /// Its attention is urgent.
    pub urgent: bool,
    pub not_responding: bool,
    /// Its desktop's place in its output's row, from 1.
    pub desktop: u32,
    /// It is a game: its keys go raw and only the escape set acts over it.
    pub game: bool,
}

/// One desktop of an output's row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct DesktopInfo {
    pub output: String,
    /// Its place in the row, from 1.
    pub index: u32,
    pub current: bool,
    /// `desktop`, or `full-screen` for a window's own.
    pub kind: String,
    /// The full-screen window's id, or 0.
    pub window: u64,
    pub windows: u32,
}

/// An app with windows, or running in the background, as the switcher sees it; most
/// recently used first, background apps last.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct AppInfo {
    /// Its desktop id, or its windows' app id when no entry claims it.
    pub app: String,
    pub name: String,
    pub hidden: bool,
    /// Its toplevels, dialogs aside, on every desktop, minimised and hidden ones included.
    pub windows: u32,
    pub attention: bool,
    pub urgent: bool,
    pub not_responding: bool,
    /// Every window it has is minimised.
    pub minimised: bool,
    /// It has no window and a process still running.
    pub background: bool,
    /// LauncherEntry's count, or -1 for none.
    pub badge: i64,
    /// LauncherEntry's progress from 0 to 1, or -1 for none.
    pub progress: f64,
    /// Its icon's file, or empty.
    pub icon: String,
}

/// A keyboard as Isle reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct KeyboardInfo {
    /// The backend's id for it, which changes when it is plugged again.
    pub id: String,
    /// What its settings are kept under.
    pub name: String,
    /// `mac` or `pc`.
    pub profile: String,
    /// Whether the profile was guessed from the name rather than set.
    pub guessed: bool,
    /// Left Alt and left Super traded.
    pub swap: bool,
    /// Its layouts as XKB names them: `gb(mac)`.
    pub layouts: String,
    pub model: String,
    /// v1's xremap, whose virtual keyboard rewrites keys before Isle sees them.
    pub remapper: bool,
    /// Whether the seat has its keymap now.
    pub active: bool,
}

/// One action of the shortcut set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct ShortcutInfo {
    pub id: String,
    pub label: String,
    pub chords: Vec<String>,
    /// The packet that builds the action; empty once built.
    pub packet: String,
}

/// The seat's keyboard state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct KeyboardStatus {
    /// The active layout, by XKB's name for it.
    pub layout: String,
    pub caps_lock: bool,
    pub num_lock: bool,
    /// The app whose window holds the keys now; empty when none does.
    pub inhibited_by: String,
    /// The keys were taken back from the window with focus, until it is given them again.
    pub taken_back: bool,
}

/// A chord the recording grab took, or the keyboard it detected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct RecordInfo {
    /// The chord in Mac words; empty when cleared or cancelled.
    pub chord: String,
    /// The shortcut that already holds the chord; empty when none does.
    pub holder: String,
    pub cancelled: bool,
    /// The keyboard a detect heard from, and whether it has the key left of Z.
    pub keyboard: String,
    pub iso: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct DisplayInfo {
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub width: i32,
    pub height: i32,
    /// Millihertz.
    pub refresh: i32,
    pub scale: f64,
    pub x: i32,
    pub y: i32,
}

/// One instance of a shell surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct SurfaceInfo {
    pub name: String,
    pub output: String,
    pub open: bool,
    /// The surface's box relative to its output, in logical points.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Panics in the last minute.
    pub panics: u32,
    /// Off after too many panics, until restarted.
    pub disabled: bool,
    /// Updates run because the surface's waker fired.
    pub wakes: u64,
    /// Times its tree was drawn into its texture.
    pub draws: u64,
    /// Times its glass was blurred, over all its shapes.
    pub glass_runs: u64,
    /// The furthest its glass tint has moved toward tinted to hold the floor, 0 to 1.
    pub glass_floor: f64,
    /// Draw times while measuring, in microseconds: p50, p95, the longest.
    pub draw_p50: u64,
    pub draw_p95: u64,
    pub draw_max: u64,
    /// Glass passes that blurred while measuring, in microseconds: p50, p95, the longest.
    pub glass_p50: u64,
    pub glass_p95: u64,
    pub glass_max: u64,
}

pub mod version {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Version1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Version"
    )]
    pub trait Version {
        #[zbus(property)]
        fn version(&self) -> zbus::Result<String>;
        #[zbus(property)]
        fn commit(&self) -> zbus::Result<String>;
        /// Seconds since the Unix epoch.
        #[zbus(property)]
        fn built(&self) -> zbus::Result<u64>;
        #[zbus(property)]
        fn features(&self) -> zbus::Result<Vec<String>>;
        /// The compositor's process id, which changes when the supervisor restarts it.
        #[zbus(property)]
        fn pid(&self) -> zbus::Result<u32>;
    }
}

pub mod settings {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Settings1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Settings"
    )]
    pub trait Settings {
        fn get(&self, key: &str) -> zbus::Result<zvariant::OwnedValue>;
        /// Validates, writes and applies; returns the keys that changed. Refused while a
        /// rejected hand edit stands.
        fn set(&self, key: &str, value: &zvariant::Value<'_>) -> zbus::Result<Vec<String>>;
        fn schema(&self) -> zbus::Result<Vec<SettingKey>>;
        fn status(&self) -> zbus::Result<SettingsStatus>;
        /// Reads settings.kdl and puts it in force, after a safe start.
        fn use_mine(&self) -> zbus::Result<Vec<String>>;
        #[zbus(signal)]
        fn changed(&self, keys: Vec<String>) -> zbus::Result<()>;
        #[zbus(signal)]
        fn status_changed(&self) -> zbus::Result<()>;
    }
}

pub mod session {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Session1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Session"
    )]
    pub trait Session {
        /// Ends the session as a clean exit.
        fn quit(&self) -> zbus::Result<()>;
        /// Locks the session.
        fn lock(&self) -> zbus::Result<()>;
        #[zbus(property)]
        fn locked(&self) -> zbus::Result<bool>;
        #[zbus(signal)]
        fn lock_changed(&self) -> zbus::Result<()>;
    }
}

pub mod windows {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Windows1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Windows"
    )]
    pub trait Windows {
        fn list(&self) -> zbus::Result<Vec<WindowInfo>>;
        /// False when no window has that id.
        fn focus(&self, id: u64) -> zbus::Result<bool>;
        /// Asks the window to close; false when no window has that id.
        fn close(&self, id: u64) -> zbus::Result<bool>;
        fn minimise(&self, id: u64) -> zbus::Result<bool>;
        fn unminimise(&self, id: u64) -> zbus::Result<bool>;
        /// Apps with windows, most recently used first.
        fn apps(&self) -> zbus::Result<Vec<AppInfo>>;
        /// Shows an app's windows and focuses its most recent; false when it has none.
        fn activate_app(&self, app: &str) -> zbus::Result<bool>;
        /// Closes every window of an app, then signals its processes; false when it has none.
        fn quit_app(&self, app: &str) -> zbus::Result<bool>;
        fn stop_app(&self, app: &str) -> zbus::Result<bool>;
        /// Starts an app by desktop id, or one of its entry's actions; false when it has
        /// nothing to run.
        fn launch(&self, app: &str, action: &str, uris: &[&str]) -> zbus::Result<bool>;
        /// Opens a link or file in the app that handles it.
        fn open(&self, target: &str) -> zbus::Result<bool>;
        /// Hides or shows every window of an app; false when it has none.
        fn set_app_hidden(&self, app: &str, hidden: bool) -> zbus::Result<bool>;
        /// Tiles a window to a zone by name (`left-half`, `fill`, ...); false when the window
        /// or the zone is unknown, or the zone has no room.
        fn tile(&self, id: u64, zone: &str) -> zbus::Result<bool>;
        /// Returns a tiled or filled window to its box before; false when it was neither.
        fn restore(&self, id: u64) -> zbus::Result<bool>;
        /// Moves and sizes a window, in the global space's logical points.
        fn set_box(&self, id: u64, x: i32, y: i32, width: i32, height: i32) -> zbus::Result<bool>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}

pub mod shell {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Shell1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Shell"
    )]
    pub trait Shell {
        fn list(&self) -> zbus::Result<Vec<SurfaceInfo>>;
        /// False when no surface has that name.
        fn open(&self, name: &str) -> zbus::Result<bool>;
        fn close(&self, name: &str) -> zbus::Result<bool>;
        /// Bytes in Skia's resource cache and its limit; zero before any surface has drawn.
        fn cache(&self) -> zbus::Result<(u64, u64)>;
        /// Whether the user's tokens.kdl is in force, or where it went wrong.
        fn tokens_status(&self) -> zbus::Result<SettingsStatus>;
        /// Turns a surface that panicked too often back on, closed.
        fn restart(&self, name: &str) -> zbus::Result<bool>;
        /// The surface's accessibility tree as JSON: each node's id, role, label, value,
        /// children and focus.
        fn tree(&self, name: &str) -> zbus::Result<String>;
    }
}

pub mod desktops {
    use super::*;

    /// An empty output name means the output with focus.
    #[proxy(
        interface = "io.github.alexparlett.Isle.Desktops1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Desktops"
    )]
    pub trait Desktops {
        fn list(&self) -> zbus::Result<Vec<DesktopInfo>>;
        /// Shows a desktop, by its place from 1; false when there is none.
        fn switch(&self, output: &str, index: u32) -> zbus::Result<bool>;
        /// Adds a desktop at the end of the row; its place, or 0 at the limit.
        fn add(&self, output: &str) -> zbus::Result<u32>;
        /// Removes a desktop, its windows joining the one before; false for the last one or
        /// a full-screen one.
        fn remove(&self, output: &str, index: u32) -> zbus::Result<bool>;
        /// Moves a window to a desktop of its output, by place, and shows it there.
        fn move_window(&self, id: u64, index: u32) -> zbus::Result<bool>;
        /// Puts a window into full screen on a desktop of its own, or back.
        fn set_full_screen(&self, id: u64, on: bool) -> zbus::Result<bool>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}

pub mod keyboard {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Keyboard1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Keyboard"
    )]
    pub trait Keyboard {
        fn list(&self) -> zbus::Result<Vec<KeyboardInfo>>;
        fn status(&self) -> zbus::Result<KeyboardStatus>;
        /// Sets a keyboard's profile by its name: `mac`, `pc` or `auto`; writes the settings.
        fn set_profile(&self, name: &str, profile: &str) -> zbus::Result<()>;
        /// Every action of the shortcut set with its chords.
        fn shortcuts(&self) -> zbus::Result<Vec<ShortcutInfo>>;
        /// Whether the user's translation.kdl is in force, and where it is wrong if not.
        fn tables_status(&self) -> zbus::Result<SettingsStatus>;
        /// The Mac chord that sends an app `chord`, for a menu item it declares by it; empty
        /// when none does.
        fn mac_chord(&self, app: &str, chord: &str) -> zbus::Result<String>;
        /// Suspends desktop chords and translation and returns the next chord; Escape
        /// cancels, Delete clears, focus moving ends it. Gives up after a minute.
        fn record(&self) -> zbus::Result<RecordInfo>;
        /// Returns the keyboard the next key comes from.
        fn detect(&self) -> zbus::Result<RecordInfo>;
        /// Types text into a window, focusing it, untranslated; false when it is gone.
        fn type_text(&self, window: u64, text: &str) -> zbus::Result<bool>;
        /// Sends a window a Mac chord as its own (Cmd+V is a terminal's Ctrl+Shift+V).
        fn send_chord(&self, window: u64, chord: &str) -> zbus::Result<bool>;
        /// Takes the keys back from the window holding them, or gives them back.
        fn take_keys_back(&self) -> zbus::Result<()>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}

pub mod displays {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Displays1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Displays"
    )]
    pub trait Displays {
        fn list(&self) -> zbus::Result<Vec<DisplayInfo>>;
        /// Releases every output and probes the connectors again, as one step.
        fn redetect(&self) -> zbus::Result<()>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}

pub mod test {
    use super::*;

    /// Served only by builds with the `test-hooks` feature, never by a package.
    #[proxy(
        interface = "io.github.alexparlett.Isle.Test1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Test"
    )]
    pub trait Test {
        /// Posts to the island as a service would, from JSON.
        fn island(&self, json: &str) -> zbus::Result<()>;
        /// Acts on Control Centre as its controls would.
        fn panel(&self, json: &str) -> zbus::Result<()>;
        /// The banner stack as JSON.
        fn banners(&self) -> zbus::Result<String>;
        /// Ends the lock with no password.
        fn unlock(&self) -> zbus::Result<()>;
        /// The lock as JSON.
        fn lock_status(&self) -> zbus::Result<String>;
        /// What an output's last frame was made of, front to back.
        fn frame_kinds(&self, output: &str) -> zbus::Result<Vec<String>>;
        /// Acts on the banners as their controls would.
        fn banner(&self, json: &str) -> zbus::Result<()>;
        /// Gives an output a backlight named `test`, set through logind.
        fn fake_screen(&self, connector: &str, current: u32, max: u32) -> zbus::Result<()>;

        /// Drops the tray's watcher as a crash would.
        fn tray_kill(&self) -> zbus::Result<()>;
        /// Holds the main loop for `millis`, from its next dispatch.
        fn stall(&self, millis: u32) -> zbus::Result<()>;
        /// A key by its evdev code, from a test keyboard with the Mac profile.
        fn key(&self, code: u32, pressed: bool) -> zbus::Result<()>;
        /// Adds a keyboard by name, as if plugged in; its id for `key_on`.
        fn add_keyboard(&self, name: &str) -> zbus::Result<String>;
        fn remove_keyboard(&self, id: &str) -> zbus::Result<()>;
        /// A key from a keyboard `add_keyboard` made.
        fn key_on(&self, id: &str, code: u32, pressed: bool) -> zbus::Result<()>;
        /// Held chords told of their release so far.
        fn held_releases(&self) -> zbus::Result<u32>;
        /// The switcher while held, as JSON, or `null`.
        fn switcher(&self) -> zbus::Result<String>;
        /// Each update of the switcher's previews in microseconds, while measuring.
        fn switcher_updates(&self) -> zbus::Result<Vec<u64>>;
        /// Times the switcher's panel has been drawn.
        fn switcher_draws(&self) -> zbus::Result<u32>;
        /// Moves the pointer to a point in the global space.
        fn pointer_to(&self, x: f64, y: f64) -> zbus::Result<()>;
        /// Moves the pointer by a relative amount, as a mouse does, held to the outputs.
        fn pointer_by(&self, dx: f64, dy: f64) -> zbus::Result<()>;
        /// A button by its evdev code: 272 is the left button.
        fn button(&self, button: u32, pressed: bool) -> zbus::Result<()>;
        /// Scrolls as a mouse wheel does, in steps of 120 per notch; positive is down.
        fn axis(&self, v120_x: i32, v120_y: i32) -> zbus::Result<()>;
        /// Starts or stops the full-output probe's tile fades.
        fn animate_probe_full(&self, on: bool) -> zbus::Result<()>;
        /// Times every surface draw, waiting for the GPU after each.
        fn measure(&self, on: bool) -> zbus::Result<()>;
        /// Turns shell transitions off or on.
        fn transitions(&self, on: bool) -> zbus::Result<()>;
        /// Turns window motion off or on, apart from the shell's transitions.
        fn window_motion(&self, on: bool) -> zbus::Result<()>;
        /// The X display once Xwayland's window manager runs, else -1.
        fn x_display(&self) -> zbus::Result<i32>;
        /// Bytes the allocator has handed out and not had back, from glibc's `mallinfo2`.
        fn malloc_in_use(&self) -> zbus::Result<u64>;
        /// The counts inside a surface's tree, for finding what grows.
        fn surface_debug(&self, name: &str) -> zbus::Result<String>;
        /// Makes the probe panic when it next renders, handles a key, or draws: `render`,
        /// `event` or `draw`.
        fn panic_probe(&self, fault: &str) -> zbus::Result<()>;
        /// The probe surface's model: the field's text, the presses counted, whether it
        /// animates.
        fn probe(&self) -> zbus::Result<(String, u32, bool)>;
        /// Adds a virtual display; headless only.
        #[allow(clippy::too_many_arguments)]
        fn add_output(
            &self,
            name: &str,
            make: &str,
            model: &str,
            serial: &str,
            width: i32,
            height: i32,
        ) -> zbus::Result<()>;
        fn remove_output(&self, name: &str) -> zbus::Result<bool>;
        /// Frames presented on an output since it was added.
        fn frames(&self, output: &str) -> zbus::Result<u64>;
        /// Draws an output and writes it to `path` as a PNG.
        fn screenshot(&self, output: &str, path: &str) -> zbus::Result<()>;
        /// Panics in the worker's D-Bus service.
        fn panic_service(&self) -> zbus::Result<()>;
        /// Panics on the main loop.
        fn panic_core(&self) -> zbus::Result<()>;
    }
}

pub mod wallpaper {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Wallpaper1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Wallpaper"
    )]
    pub trait Wallpaper {
        /// Every screen's wallpaper; a source that cannot be shown names its problem.
        fn status(&self) -> zbus::Result<Vec<WallpaperInfo>>;
    }
}

pub mod island {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Island1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Island"
    )]
    pub trait Island {
        /// Each output's island as JSON: what it shows and its shape's size.
        fn status(&self) -> zbus::Result<String>;
    }
}

pub mod menus {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Menus1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Menus"
    )]
    pub trait Menus {
        /// The front app's menu bar as JSON: titles, items, hints and marks.
        fn bar(&self) -> zbus::Result<String>;
        /// Chooses the item at a path of labels, a title first; false when there is none.
        fn activate(&self, path: &[&str]) -> zbus::Result<bool>;
        /// Reveals the menu bar with a title's menu open.
        fn reveal(&self, title: &str) -> zbus::Result<bool>;
        /// A window's own menus as JSON, and where they came from.
        fn show(&self, id: u64) -> zbus::Result<String>;
        /// The front app's menu items whose labels hold `query`, with their paths, as JSON.
        fn search(&self, query: &str) -> zbus::Result<String>;
    }
}

pub mod services {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Services1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Services"
    )]
    pub trait Services {
        /// Each system service as JSON: whether its daemon answers, and its state.
        fn status(&self) -> zbus::Result<String>;
    }
}

pub mod notifications {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Notifications1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Notifications"
    )]
    pub trait Notifications {
        /// Every notification Isle holds, live and recorded, as JSON.
        fn list(&self) -> zbus::Result<String>;
        /// Closes one, as its close control does; false when there is none.
        fn close(&self, id: u32) -> zbus::Result<bool>;
        /// Runs one's action, `default` being its click.
        fn invoke(&self, id: u32, key: &str) -> zbus::Result<bool>;
        fn reply(&self, id: u32, text: &str) -> zbus::Result<bool>;
        fn clear(&self) -> zbus::Result<()>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}

pub mod focus {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Focus1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Focus"
    )]
    pub trait Focus {
        /// The Focus in force, why, and the modes, as JSON.
        fn status(&self) -> zbus::Result<String>;
        /// Turns a mode on by hand for `minutes`, or until turned off with 0.
        fn set(&self, mode: &str, minutes: u32) -> zbus::Result<()>;
        /// Turns the Focus in force off.
        fn off(&self) -> zbus::Result<()>;
        #[zbus(signal)]
        fn changed(&self) -> zbus::Result<()>;
    }
}
