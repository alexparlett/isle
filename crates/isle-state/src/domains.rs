//! The state domains. Later packets add theirs (launch counts, session restore, ...).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Domain;

/// The supervisor's record of recent compositor deaths, for its backoff and safe start.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Crashes {
    /// Unix seconds of each death soon after its start, newest last.
    pub quick_deaths: Vec<u64>,
}

impl Domain for Crashes {
    const NAME: &'static str = "crashes";
    const VERSION: u32 = 1;
}

/// Whether v1's preferences have been imported (B27); P14 does the import.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Import {
    /// Unix seconds of the import, if it has run.
    pub v1_imported_at: Option<u64>,
}

impl Domain for Import {
    const NAME: &'static str = "import";
    const VERSION: u32 = 1;
}

/// Each app's main window's box, by app and then by output identity, relative to the
/// origin of that output's work area.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Places {
    pub apps: BTreeMap<String, BTreeMap<String, Place>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Unix seconds when it was recorded, so the latest of an app's places can stand in for
    /// an output that is absent.
    pub at: u64,
}

/// How many desktops each output's row has, by the output's identity.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Desktops {
    pub counts: BTreeMap<String, u32>,
}

impl Domain for Desktops {
    const NAME: &'static str = "desktops";
    const VERSION: u32 = 1;
}

impl Domain for Places {
    const NAME: &'static str = "places";
    const VERSION: u32 = 1;
}

/// Each screen's wallpaper as last shown, by the output's identity, so the next session's
/// first frame is the picture's mean colour and a folder carries on where it was.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wallpapers {
    pub screens: BTreeMap<String, WallpaperScreen>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WallpaperScreen {
    /// The `source` setting this was shown for.
    pub source: String,
    /// The picture shown: the source itself, or the folder's current one.
    pub file: Option<String>,
    /// The picture's mean colour, sRGB.
    pub mean: Option<[u8; 3]>,
    /// Unix seconds when a folder last moved on.
    pub changed_at: u64,
}

impl Domain for Wallpapers {
    const NAME: &'static str = "wallpapers";
    const VERSION: u32 = 1;
}
