//! The scan on threads of its own (`p19b-cleaner.md` choices 8, 9 and 14): the walk published at
//! most once a `timing-scan-layout-ms`, the rules run over each publication, duplicates compared
//! at idle I/O priority once the walk ends, and the system's share read beside it. On a btrfs
//! disk the walk is then made again with each file's bytes on disk (B139).

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use isle_scan::{Context, Rules, Scan, duplicates, system};

use crate::model::{self, Snapshot};

pub enum Ask {
    Scan(Vec<PathBuf>),
    Stop,
    /// Read the system's share again, after a clear.
    System,
}

pub enum Event {
    Snapshot(Arc<Snapshot>),
    /// The scan's generation is being measured on disk.
    Measuring(u64),
    /// Measuring that generation gave nothing: the figures stand as the walk read them.
    Measured(u64),
    /// The scan's generation, and its duplicate groups.
    Duplicates(u64, Arc<Vec<duplicates::Group>>),
    System(Vec<system::Share>),
}

/// Starts the scanner; dropping the sender ends it and cancels its scan.
pub fn start(every: Duration) -> (mpsc::Sender<Ask>, UnboundedReceiver<Event>) {
    let (ask, asked) = mpsc::channel();
    let (tx, rx) = unbounded();
    std::thread::Builder::new()
        .name("cleaner-scan".into())
        .spawn(move || run(asked, tx, every))
        .expect("the scanner's thread");
    (ask, rx)
}

fn read_system(tx: &UnboundedSender<Event>) {
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.unbounded_send(Event::System(system::read()));
    });
}

/// Whether any of `roots`, or a mount under one, is btrfs, whose `st_blocks` count bytes before
/// compression.
fn on_btrfs(roots: &[PathBuf]) -> bool {
    let mounts = isle_fs::mounts::read();
    roots.iter().any(|r| {
        isle_fs::mounts::of(&mounts, r).is_some_and(|m| m.fstype == "btrfs")
            || mounts
                .iter()
                .any(|m| m.fstype == "btrfs" && m.point.starts_with(r))
    })
}

/// Reads every file's bytes on disk through the helper, then walks `roots` again with them and
/// publishes that as the scan's last word, unless another scan has begun.
fn measure(
    g: u64,
    roots: Vec<PathBuf>,
    latest: Arc<AtomicU64>,
    rules: Arc<Rules>,
    ctx: Arc<Context>,
    tx: UnboundedSender<Event>,
) {
    let _ = tx.unbounded_send(Event::Measuring(g));
    let out = system::measure_command().output();
    let sizes = match out {
        Ok(o) if o.status.success() => isle_scan::ondisk::OnDisk::read(&o.stdout[..]).ok(),
        Ok(o) => {
            tracing::warn!(status = ?o.status, error = %String::from_utf8_lossy(&o.stderr).trim(), "no bytes on disk");
            None
        }
        Err(e) => {
            tracing::warn!("no bytes on disk: {e}");
            None
        }
    };
    if latest.load(Ordering::Acquire) != g {
        return;
    }
    let Some(sizes) = sizes.filter(|s| !s.is_empty()) else {
        let _ = tx.unbounded_send(Event::Measured(g));
        return;
    };
    let scan = Arc::new(Scan::start_sized(roots.clone(), Arc::new(sizes)));
    while !scan.progress().done || scan.running() {
        if latest.load(Ordering::Acquire) != g {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut snap = model::snapshot(g, &scan, &roots, &rules, &ctx);
    snap.measured = true;
    if latest.load(Ordering::Acquire) == g {
        let _ = tx.unbounded_send(Event::Snapshot(Arc::new(snap)));
    }
}

fn run(asked: mpsc::Receiver<Ask>, tx: UnboundedSender<Event>, every: Duration) {
    let rules = Arc::new(Rules::load());
    let ctx = Arc::new(Context::here(isle_games::Roots::find().steam_libraries()));
    let latest = Arc::new(AtomicU64::new(0));
    read_system(&tx);
    let mut current: Option<(u64, Arc<Scan>, Vec<PathBuf>)> = None;
    let mut generation = 0u64;
    let mut comparing: Option<Arc<AtomicBool>> = None;
    loop {
        // A scan is published until its finished snapshot has gone out, at once when it has ended.
        let wait = current.as_ref().map(|(_, s, _)| {
            let p = s.progress();
            if p.done || p.partial {
                Duration::ZERO
            } else {
                every
            }
        });
        let ask = match wait {
            Some(wait) => asked.recv_timeout(wait),
            None => asked
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match ask {
            Ok(Ask::Scan(roots)) => {
                if let Some(stop) = comparing.take() {
                    stop.store(true, Ordering::Relaxed);
                }
                generation += 1;
                latest.store(generation, Ordering::Release);
                current = Some((generation, Arc::new(Scan::start(roots.clone())), roots));
            }
            Ok(Ask::Stop) => {
                if let Some((_, scan, _)) = &current {
                    scan.stop();
                }
            }
            Ok(Ask::System) => {
                read_system(&tx);
                continue;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        let Some((g, scan, roots)) = &current else {
            continue;
        };
        // The workers end soon after the walk does, or after Stop; the last publication waits
        // for them.
        let ended = {
            let p = scan.progress();
            p.done || p.partial
        };
        if ended {
            while scan.running() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let snap = model::snapshot(*g, scan, roots, &rules, &ctx);
        let finished = snap.finished() && !scan.running();
        let candidates =
            finished.then(|| scan.read(|tree| duplicates::candidates(tree, &snap.finds)));
        if tx.unbounded_send(Event::Snapshot(Arc::new(snap))).is_err() {
            return;
        }
        if let Some(candidates) = candidates {
            let stop = Arc::new(AtomicBool::new(false));
            comparing = Some(stop.clone());
            let (dups_tx, dups_g) = (tx.clone(), *g);
            std::thread::Builder::new()
                .name("cleaner-dups".into())
                .spawn(move || {
                    duplicates::idle_io();
                    let groups = duplicates::compare(candidates, &stop);
                    if !stop.load(Ordering::Relaxed) {
                        let _ = dups_tx.unbounded_send(Event::Duplicates(dups_g, Arc::new(groups)));
                    }
                })
                .expect("the duplicates' thread");
            if on_btrfs(roots) && !scan.progress().partial {
                let (roots, latest, rules, ctx, tx, g) = (
                    roots.clone(),
                    latest.clone(),
                    rules.clone(),
                    ctx.clone(),
                    tx.clone(),
                    *g,
                );
                std::thread::Builder::new()
                    .name("cleaner-measure".into())
                    .spawn(move || measure(g, roots, latest, rules, ctx, tx))
                    .expect("the measuring thread");
            }
            current = None;
        }
    }
}
