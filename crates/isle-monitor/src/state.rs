//! What Monitor remembers, in its `isle-state` domain (`p19a-monitor.md` choice 26): the
//! resource, the filter, each resource's columns, widths and sort, the Disk and Network
//! graphs' choice and Update Frequency.

use std::collections::HashMap;

use isle_state::{Domain, StateDir};
use serde::{Deserialize, Serialize};

use crate::model::{Filter, Resource};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Remembered {
    pub resource: Resource,
    pub filter: Filter,
    /// A resource's columns after the name where the user changed them.
    pub columns: HashMap<Resource, Vec<String>>,
    /// Column widths by column id, shared by every resource that shows it.
    pub widths: HashMap<String, f32>,
    /// A resource's sort where the user changed it: its column and whether descending.
    pub sorts: HashMap<Resource, (String, bool)>,
    /// Disk's and Network's graphs draw data rather than operations or packets.
    pub disk_data: bool,
    pub network_data: bool,
    /// Seconds between samples: 1, 2 or 5.
    pub every: u64,
}

impl Default for Remembered {
    fn default() -> Self {
        Self {
            resource: Resource::Cpu,
            filter: Filter::All,
            columns: HashMap::new(),
            widths: HashMap::new(),
            sorts: HashMap::new(),
            disk_data: false,
            network_data: false,
            every: 2,
        }
    }
}

impl Domain for Remembered {
    const NAME: &'static str = "monitor";
    const VERSION: u32 = 1;
}

pub fn load() -> Remembered {
    let (data, loaded) = StateDir::new(StateDir::default_dir()).load::<Remembered>();
    if !matches!(loaded, isle_state::Loaded::Read | isle_state::Loaded::Missing) {
        tracing::warn!(?loaded, "Monitor's state");
    }
    data
}

pub fn save(data: &Remembered) {
    if let Err(err) = StateDir::new(StateDir::default_dir()).save(data) {
        tracing::warn!("Monitor's state could not be saved: {err}");
    }
}
