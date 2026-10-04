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

/// One key of `settings.kdl`, as the Settings interface introspects it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct SettingKey {
    pub key: String,
    pub kind: String,
    pub default: String,
    pub constraint: String,
    pub doc: String,
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
    pub title: String,
    /// The output the window is on, or empty while it is on none.
    pub output: String,
    pub focused: bool,
    /// The window's box in the global space, in logical points.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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
        /// Holds the main loop for `millis`, from its next dispatch.
        fn stall(&self, millis: u32) -> zbus::Result<()>;
        /// A key by its evdev code, as a keyboard sends it.
        fn key(&self, code: u32, pressed: bool) -> zbus::Result<()>;
        /// Moves the pointer to a point in the global space.
        fn pointer_to(&self, x: f64, y: f64) -> zbus::Result<()>;
        /// A button by its evdev code: 272 is the left button.
        fn button(&self, button: u32, pressed: bool) -> zbus::Result<()>;
        /// Scrolls as a mouse wheel does, in steps of 120 per notch; positive is down.
        fn axis(&self, v120_x: i32, v120_y: i32) -> zbus::Result<()>;
        /// Turns shell transitions off or on.
        fn transitions(&self, on: bool) -> zbus::Result<()>;
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
