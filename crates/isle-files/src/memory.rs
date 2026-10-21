//! What Files remembers between launches, in its `isle-state` domain (`conventions.md`,
//! What an app remembers): the windows and their tabs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::places::Place;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memory {
    /// Each open window's tabs, by the window's number.
    pub windows: BTreeMap<u64, WindowMemory>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowMemory {
    pub tabs: Vec<Place>,
    pub current: usize,
}

impl isle_state::Domain for Memory {
    const NAME: &'static str = "files";
    const VERSION: u32 = 1;
}

fn dir() -> isle_state::StateDir {
    isle_state::StateDir::new(isle_state::StateDir::default_dir())
}

pub fn load() -> Memory {
    dir().load::<Memory>().0
}

pub fn save(memory: &Memory) {
    if let Err(err) = dir().save(memory) {
        tracing::warn!("Files' windows are not kept: {err}");
    }
}
