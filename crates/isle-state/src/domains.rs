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

impl Domain for Places {
    const NAME: &'static str = "places";
    const VERSION: u32 = 1;
}
