//! The file-name index on the worker: built from `spotlight`'s folders, kept live with one
//! inotify watch per folder, built again when a watch overflows or the folders change
//! (`p10-spotlight.md` choice 12).

use std::{
    collections::BTreeSet,
    error::Error,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use isle_search::files::{FileIndex, Reach, Shared};
use notify::{RecursiveMode, Watcher};
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, warn};

use crate::worker::ToLoop;

/// How long changes settle before their folders are read again.
const SETTLE: Duration = Duration::from_millis(300);

/// The index's reach from the settings: home and the folders added, less those excluded.
pub fn reach(settings: &isle_settings::schema::Spotlight) -> Reach {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let expand = |p: &String| match (p.strip_prefix("~/"), &home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(p),
    };
    Reach {
        roots: home
            .iter()
            .cloned()
            .chain(settings.files_include.iter().map(expand))
            .collect(),
        exclude: settings.files_exclude.iter().map(expand).collect(),
    }
}

pub async fn watch(
    to_loop: ToLoop,
    mut reach: watch::Receiver<Option<Reach>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        let wanted = reach.wait_for(Option::is_some).await?.clone().unwrap();
        reach.mark_unchanged();
        let started = Instant::now();
        let index = tokio::task::spawn_blocking({
            let wanted = wanted.clone();
            move || FileIndex::build(wanted)
        })
        .await?;
        info!(entries = index.len(), folders = index.folders().count(), took = ?started.elapsed(), "file index built");
        let folders: Vec<PathBuf> = index.folders().cloned().collect();
        let shared: Shared = Arc::new(RwLock::new(index));
        let posted = shared.clone();
        to_loop.post(move |state| state.isle.files = Some(posted));

        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = tx.send(event);
            })?;
        let mut refused = 0usize;
        for folder in &folders {
            if watcher.watch(folder, RecursiveMode::NonRecursive).is_err() {
                refused += 1;
            }
        }
        if refused > 0 {
            warn!(refused, "folders the file index could not watch");
        }

        'live: loop {
            tokio::select! {
                changed = reach.changed() => {
                    changed?;
                    break 'live;
                }
                event = rx.recv() => {
                    let Some(event) = event else { break 'live };
                    let mut dirty = BTreeSet::new();
                    let mut collect = |event: notify::Result<notify::Event>| -> bool {
                        match event {
                            Ok(event) if event.need_rescan() => false,
                            Ok(event) => {
                                for path in event.paths {
                                    if let Some(parent) = path.parent() {
                                        dirty.insert(parent.to_owned());
                                    }
                                }
                                true
                            }
                            Err(err) => {
                                debug!("the file watch failed: {err}");
                                false
                            }
                        }
                    };
                    if !collect(event) {
                        break 'live;
                    }
                    tokio::time::sleep(SETTLE).await;
                    while let Ok(event) = rx.try_recv() {
                        if !collect(event) {
                            break 'live;
                        }
                    }
                    let index = shared.clone();
                    let added = tokio::task::spawn_blocking(move || {
                        let mut index = index.write().unwrap();
                        dirty.iter().flat_map(|d| index.sync(d)).collect::<Vec<_>>()
                    })
                    .await?;
                    for folder in added {
                        let _ = watcher.watch(&folder, RecursiveMode::NonRecursive);
                    }
                }
            }
        }
        // The folders changed, or a watch overflowed: build afresh.
        reach.mark_changed();
        if reach.borrow().as_ref() == Some(&wanted) {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
