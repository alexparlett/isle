//! The file-name index on the worker: built from `spotlight`'s folders, kept live with one
//! inotify watch per folder, built again when a watch overflows or the folders change
//! (`p10-spotlight.md` choice 12).

use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
    ffi::OsStr,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use isle_search::files::{FileIndex, Reach, Shared};
use isle_fs::watch::{Change, Watch, Watcher};
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use crate::worker::ToLoop;

/// How long changes settle before their folders are read again.
const SETTLE: Duration = Duration::from_millis(300);

static BUILDS: AtomicU64 = AtomicU64::new(0);
static SYNCS: AtomicU64 = AtomicU64::new(0);

/// Times the index was built, and folders read again since, from the compositor's start.
pub fn counts() -> (u64, u64) {
    (BUILDS.load(Ordering::Relaxed), SYNCS.load(Ordering::Relaxed))
}

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
        BUILDS.fetch_add(1, Ordering::Relaxed);
        let folders: Vec<PathBuf> = index.folders().cloned().collect();
        let shared: Shared = Arc::new(RwLock::new(index));
        let posted = shared.clone();
        to_loop.post(move |state| state.isle.files = Some(posted));

        let Ok(watcher) = Watcher::for_names() else {
            warn!("the file index is not kept live: inotify refused");
            reach.changed().await?;
            continue;
        };
        let (tx, mut rx) = mpsc::unbounded_channel::<(PathBuf, Change)>();
        let mut watches: HashMap<PathBuf, Watch> = HashMap::with_capacity(folders.len());
        let refused = folders
            .into_iter()
            .filter(|folder| !subscribe(&watcher, &tx, &mut watches, folder.clone()))
            .count();
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
                    if !collect(event, &mut dirty) {
                        break 'live;
                    }
                    tokio::time::sleep(SETTLE).await;
                    while let Ok(event) = rx.try_recv() {
                        if !collect(event, &mut dirty) {
                            break 'live;
                        }
                    }
                    if dirty.is_empty() {
                        continue;
                    }
                    SYNCS.fetch_add(dirty.len() as u64, Ordering::Relaxed);
                    let index = shared.clone();
                    let synced = tokio::task::spawn_blocking(move || {
                        let plans: Vec<_> = {
                            let index = index.read().unwrap();
                            dirty.iter().map(|d| index.plan(d)).collect()
                        };
                        let mut index = index.write().unwrap();
                        plans.into_iter().map(|p| index.apply(p)).collect::<Vec<_>>()
                    })
                    .await?;
                    for synced in synced {
                        for folder in synced.removed {
                            watches.remove(&folder);
                        }
                        for folder in synced.added {
                            subscribe(&watcher, &tx, &mut watches, folder);
                        }
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

/// Watches `folder`, its changes sent on `tx`; false when it cannot be watched.
fn subscribe(
    watcher: &std::sync::Arc<Watcher>,
    tx: &mpsc::UnboundedSender<(PathBuf, Change)>,
    watches: &mut HashMap<PathBuf, Watch>,
    folder: PathBuf,
) -> bool {
    let tx = tx.clone();
    let dir = folder.clone();
    match watcher.watch(&folder, move |change| {
        let _ = tx.send((dir.clone(), change));
    }) {
        Some(watch) => {
            watches.insert(folder, watch);
            true
        }
        None => false,
    }
}

/// Adds the folder to read again for a change in it; false when the index must be built
/// afresh: a watch overflowed, or ignore rules changed.
fn collect((folder, change): (PathBuf, Change), dirty: &mut BTreeSet<PathBuf>) -> bool {
    match change {
        Change::Overflow => false,
        Change::Gone => {
            dirty.insert(folder);
            true
        }
        Change::Touched(names) => {
            if names.iter().any(|n| n == OsStr::new(".gitignore") || n == OsStr::new(".ignore")) {
                return false;
            }
            // A hidden name never joins the index.
            if names.iter().any(|n| !n.as_encoded_bytes().starts_with(b".")) {
                dirty.insert(folder);
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(what: &str, test: impl Fn() -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while !test() {
            assert!(Instant::now() < until, "{what} within five seconds");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn reads_are_not_changes_and_each_change_is_read_once() {
        let root = std::env::temp_dir().join(format!("isle-file-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let root = root.as_path();
        for dir in ["a/b", "target/debug"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        std::fs::write(root.join("a/notes.txt"), "notes").unwrap();

        let (sender, _channel) = smithay::reexports::calloop::channel::channel();
        let (reach_tx, reach_rx) = watch::channel(Some(Reach {
            roots: vec![root.to_owned()],
            exclude: Vec::new(),
        }));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.spawn(watch(ToLoop::new(sender), reach_rx));
        wait_for("the index built", || counts().0 == 1);

        for _ in 0..5 {
            std::fs::read(root.join("a/notes.txt")).unwrap();
            std::fs::read(root.join(".gitignore")).unwrap();
            std::fs::read_dir(root.join("a")).unwrap().for_each(drop);
            std::fs::write(root.join("a/notes.txt"), "more notes").unwrap();
        }
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(counts().1, 0, "reading and writing are not changes");

        std::fs::write(root.join("a/b/new.txt"), "").unwrap();
        wait_for("the create read", || counts().1 == 1);
        std::fs::rename(root.join("a/b/new.txt"), root.join("a/b/renamed.txt")).unwrap();
        wait_for("the rename read", || counts().1 == 2);
        std::fs::remove_file(root.join("a/b/renamed.txt")).unwrap();
        wait_for("the delete read", || counts().1 == 3);
        std::fs::write(root.join("a/.hidden"), "").unwrap();
        std::fs::write(root.join("target/debug/out"), "").unwrap();
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(counts().1, 3, "the reads settle, and hidden and ignored names are not read");
        drop(reach_tx);
        let _ = std::fs::remove_dir_all(root);
    }
}
