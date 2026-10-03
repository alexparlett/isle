//! The domains P2 keeps. Later packets add theirs (window places, launch counts, ...).

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
