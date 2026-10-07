//! The system's daemons as the desktop knows them: each service's last snapshot, taken in on
//! the loop, where the island's glyphs, Control Centre and the OSDs read them
//! (`p8-services.md` choice 2).

pub mod login;

use std::{collections::BTreeMap, error::Error, time::Duration};

use isle_bus::Snapshot;
use serde::Serialize;
use tokio::sync::watch;

use crate::{state::State, worker::ToLoop};

/// The least time between two snapshots of one service reaching the loop, about a frame.
const FRAME: Duration = Duration::from_millis(8);

#[derive(Default)]
pub struct Services {
    pub login: Snapshot<login::Login>,
    /// Snapshots taken in, by service.
    pub applied: BTreeMap<&'static str, u64>,
}

impl Services {
    /// Every service's availability and state, with how many snapshots it has delivered.
    pub fn status(&self) -> String {
        fn entry<T: Serialize>(s: &Snapshot<T>, applied: u64) -> serde_json::Value {
            serde_json::json!({ "available": s.available, "state": s.state, "applied": applied })
        }
        let applied = |name| self.applied.get(name).copied().unwrap_or(0);
        serde_json::json!({ "login": entry(&self.login, applied("login")) }).to_string()
    }
}

/// Carries a service's snapshots to the loop: the latest of a burst, once a frame at most.
pub async fn forward<T>(
    name: &'static str,
    mut snapshots: watch::Receiver<Snapshot<T>>,
    to_loop: ToLoop,
    apply: fn(&mut State, Snapshot<T>),
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    T: Clone + Send + Sync + 'static,
{
    loop {
        snapshots.changed().await?;
        let snapshot = snapshots.borrow_and_update().clone();
        to_loop.post(move |state| {
            *state.isle.services.applied.entry(name).or_default() += 1;
            apply(state, snapshot);
        });
        tokio::time::sleep(FRAME).await;
    }
}
