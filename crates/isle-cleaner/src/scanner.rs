//! The scan on threads of its own (`p19b-cleaner.md` choices 8, 9 and 14): the walk published at
//! most once a `timing-scan-layout-ms`, the rules run over each publication, duplicates compared
//! at idle I/O priority once the walk ends, and the system's share read beside it.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
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

fn run(asked: mpsc::Receiver<Ask>, tx: UnboundedSender<Event>, every: Duration) {
    let rules = Rules::load();
    let ctx = Context::here(isle_games::Roots::find().steam_libraries());
    read_system(&tx);
    let mut current: Option<(u64, Arc<Scan>, Vec<PathBuf>)> = None;
    let mut generation = 0u64;
    let mut comparing: Option<Arc<AtomicBool>> = None;
    loop {
        let scanning = current.as_ref().is_some_and(|(_, s, _)| {
            let p = s.progress();
            !p.done && !p.partial
        });
        let ask = if scanning {
            asked.recv_timeout(every)
        } else {
            asked
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };
        match ask {
            Ok(Ask::Scan(roots)) => {
                if let Some(stop) = comparing.take() {
                    stop.store(true, Ordering::Relaxed);
                }
                generation += 1;
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
        // The workers end soon after Stop; the last publication waits for them.
        if scan.progress().partial {
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
            let (tx, g) = (tx.clone(), *g);
            std::thread::Builder::new()
                .name("cleaner-dups".into())
                .spawn(move || {
                    duplicates::idle_io();
                    let groups = duplicates::compare(candidates, &stop);
                    if !stop.load(Ordering::Relaxed) {
                        let _ = tx.unbounded_send(Event::Duplicates(g, Arc::new(groups)));
                    }
                })
                .expect("the duplicates' thread");
            current = None;
        }
    }
}
