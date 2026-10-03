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

pub mod displays {
    use super::*;

    #[proxy(
        interface = "io.github.alexparlett.Isle.Displays1",
        default_service = "io.github.alexparlett.Isle",
        default_path = "/io/github/alexparlett/Isle/Displays"
    )]
    pub trait Displays {
        fn list(&self) -> zbus::Result<Vec<DisplayInfo>>;
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
        /// Panics in the worker's D-Bus service.
        fn panic_service(&self) -> zbus::Result<()>;
        /// Panics on the main loop.
        fn panic_core(&self) -> zbus::Result<()>;
    }
}
