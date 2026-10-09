//! The state domains. Later packets add theirs (session restore, ...).

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

/// Spotlight's Run history, newest first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Commands {
    pub run: Vec<String>,
}

impl Domain for Commands {
    const NAME: &'static str = "commands";
    const VERSION: u32 = 1;
}

/// How often each of Spotlight's results has been chosen, by the result's key
/// (`app:<desktop id>`, `action:<id>`, ...), for ranking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub uses: BTreeMap<String, Use>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Use {
    /// The count as it stood at `last`; it decays from there.
    pub weight: f64,
    /// Unix seconds of the latest choice.
    pub last: u64,
}

impl Domain for Usage {
    const NAME: &'static str = "usage";
    const VERSION: u32 = 1;
}

/// The capture toolbar's last mode, where it was dragged to and its last portion, by the
/// screen's identity, relative to that screen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Capture {
    pub mode: String,
    pub toolbar: Option<Spot>,
    pub portion: Option<Portion>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    pub output: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Portion {
    pub output: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Domain for Capture {
    const NAME: &'static str = "capture";
    const VERSION: u32 = 1;
}
