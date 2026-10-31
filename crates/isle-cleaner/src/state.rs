//! What Cleaner remembers, in its `isle-state` domain (`p19b-cleaner.md` choice 22): the view,
//! Space's map or list, Untouched's period, recent folders and the tables' columns. No scan is
//! kept.

use std::{collections::HashMap, path::PathBuf};

use isle_state::{Domain, StateDir};
use serde::{Deserialize, Serialize};

use crate::model::View;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
    #[default]
    SixMonths,
    OneYear,
    TwoYears,
}

impl Period {
    pub const ALL: [Period; 3] = [Period::SixMonths, Period::OneYear, Period::TwoYears];

    pub fn title(self) -> &'static str {
        match self {
            Period::SixMonths => "6 months",
            Period::OneYear => "1 year",
            Period::TwoYears => "2 years",
        }
    }

    pub fn secs(self) -> i64 {
        const DAY: i64 = 24 * 3600;
        match self {
            Period::SixMonths => 182 * DAY,
            Period::OneYear => 365 * DAY,
            Period::TwoYears => 730 * DAY,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Remembered {
    pub view: View,
    /// Space shows its list rather than its map.
    pub list: bool,
    pub period: Period,
    /// Folders scanned by Choose folder…, the latest first.
    pub recent: Vec<PathBuf>,
    /// Column widths by table and column.
    pub widths: HashMap<String, f32>,
}

pub const RECENT: usize = 5;

impl Remembered {
    pub fn scanned_folder(&mut self, path: PathBuf) {
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(RECENT);
    }
}

impl Domain for Remembered {
    const NAME: &'static str = "cleaner";
    const VERSION: u32 = 1;
}

pub fn load() -> Remembered {
    let (data, loaded) = StateDir::new(StateDir::default_dir()).load::<Remembered>();
    if !matches!(
        loaded,
        isle_state::Loaded::Read | isle_state::Loaded::Missing
    ) {
        tracing::warn!(?loaded, "Cleaner's state");
    }
    data
}

pub fn save(data: &Remembered) {
    if let Err(err) = StateDir::new(StateDir::default_dir()).save(data) {
        tracing::warn!("Cleaner's state could not be saved: {err}");
    }
}
