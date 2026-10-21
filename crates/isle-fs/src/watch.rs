//! One inotify instance for the process: a watch per folder shown, shared by every view of
//! it, and changes handed over as the names touched, settled for a moment (`p15-files.md`
//! choice 10). The reader stats each name again, so an unpaired rename or a create and
//! delete in one moment comes out right.

use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};

use rustix::{
    event::{PollFd, PollFlags, poll},
    fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags},
    io::Errno,
};
use tracing::warn;

/// What happened in a watched folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// These names were created, removed, renamed, written or had their attributes changed.
    Touched(BTreeSet<OsString>),
    /// The folder itself was removed or moved away.
    Gone,
    /// The kernel dropped events; read the folder again.
    Overflow,
}

type Callback = Arc<dyn Fn(Change) + Send + Sync>;

struct Watched {
    dir: PathBuf,
    subscribers: Vec<(u64, Callback)>,
}

#[derive(Default)]
struct Table {
    by_wd: HashMap<i32, Watched>,
    by_dir: HashMap<PathBuf, i32>,
    next: u64,
}

/// The process's watcher.
pub struct Watcher {
    fd: rustix::fd::OwnedFd,
    table: Mutex<Table>,
}

/// A subscription; dropped, it lets go, and the last one lets go of the folder.
pub struct Watch {
    watcher: Weak<Watcher>,
    wd: i32,
    id: u64,
}

impl Drop for Watch {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.upgrade() {
            watcher.unsubscribe(self.wd, self.id);
        }
    }
}

const SETTLE: Duration = Duration::from_millis(100);
/// A folder written to without pause is still reported this often.
const AT_MOST: Duration = Duration::from_millis(500);

const EVENTS: WatchFlags = WatchFlags::CREATE
    .union(WatchFlags::DELETE)
    .union(WatchFlags::MOVED_FROM)
    .union(WatchFlags::MOVED_TO)
    .union(WatchFlags::MODIFY)
    .union(WatchFlags::CLOSE_WRITE)
    .union(WatchFlags::ATTRIB)
    .union(WatchFlags::DELETE_SELF)
    .union(WatchFlags::MOVE_SELF)
    .union(WatchFlags::ONLYDIR)
    .union(WatchFlags::DONT_FOLLOW);

impl Watcher {
    /// The process's watcher, started on first use.
    pub fn get() -> Option<Arc<Watcher>> {
        static WATCHER: OnceLock<Option<Arc<Watcher>>> = OnceLock::new();
        WATCHER
            .get_or_init(|| match Watcher::start() {
                Ok(w) => Some(w),
                Err(e) => {
                    warn!("no folder is watched, inotify refused: {e}");
                    None
                }
            })
            .clone()
    }

    fn start() -> rustix::io::Result<Arc<Watcher>> {
        let fd = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)?;
        let watcher = Arc::new(Watcher {
            fd,
            table: Mutex::default(),
        });
        let weak = Arc::downgrade(&watcher);
        std::thread::Builder::new()
            .name("isle-fs-watch".into())
            .spawn(move || read(weak))
            .map_err(|_| Errno::AGAIN)?;
        Ok(watcher)
    }

    /// Calls `on` with each change in `dir` until the returned watch is dropped; `None` when
    /// the folder cannot be watched (gone, or the user's watches are spent).
    pub fn watch(self: &Arc<Self>, dir: &Path, on: impl Fn(Change) + Send + Sync + 'static) -> Option<Watch> {
        let mut table = self.table.lock().unwrap();
        let wd = match table.by_dir.get(dir) {
            Some(&wd) => wd,
            None => match inotify::add_watch(&self.fd, dir, EVENTS) {
                Ok(wd) => {
                    table.by_dir.insert(dir.to_owned(), wd);
                    table.by_wd.insert(
                        wd,
                        Watched {
                            dir: dir.to_owned(),
                            subscribers: Vec::new(),
                        },
                    );
                    wd
                }
                Err(e) => {
                    if e == Errno::NOSPC {
                        warn!("the user's inotify watches are spent; {} is not watched", dir.display());
                    }
                    return None;
                }
            },
        };
        table.next += 1;
        let id = table.next;
        table.by_wd.get_mut(&wd)?.subscribers.push((id, Arc::new(on)));
        Some(Watch {
            watcher: Arc::downgrade(self),
            wd,
            id,
        })
    }

    fn unsubscribe(&self, wd: i32, id: u64) {
        let mut table = self.table.lock().unwrap();
        let Some(watched) = table.by_wd.get_mut(&wd) else { return };
        watched.subscribers.retain(|(i, _)| *i != id);
        if watched.subscribers.is_empty() {
            let dir = watched.dir.clone();
            table.by_wd.remove(&wd);
            table.by_dir.remove(&dir);
            let _ = inotify::remove_watch(&self.fd, wd);
        }
    }

    /// Hands each folder's settled changes to its subscribers.
    fn deliver(&self, pending: &mut HashMap<i32, Change>) {
        let calls: Vec<(Vec<Callback>, Change)> = {
            let table = self.table.lock().unwrap();
            pending
                .drain()
                .filter_map(|(wd, change)| {
                    let watched = table.by_wd.get(&wd)?;
                    Some((watched.subscribers.iter().map(|(_, c)| c.clone()).collect(), change))
                })
                .collect()
        };
        for (callbacks, change) in calls {
            for callback in callbacks {
                callback(change.clone());
            }
        }
    }

    fn overflowed(&self, pending: &mut HashMap<i32, Change>) {
        let table = self.table.lock().unwrap();
        for &wd in table.by_wd.keys() {
            pending.insert(wd, Change::Overflow);
        }
    }
}

/// Folds one event into what a folder has pending.
fn fold(pending: &mut HashMap<i32, Change>, wd: i32, flags: ReadFlags, name: Option<&[u8]>) {
    let slot = pending.entry(wd).or_insert_with(|| Change::Touched(BTreeSet::new()));
    if flags.intersects(ReadFlags::DELETE_SELF | ReadFlags::MOVE_SELF) {
        *slot = Change::Gone;
        return;
    }
    if let (Change::Touched(names), Some(name)) = (slot, name) {
        names.insert(std::ffi::OsStr::from_bytes(name).to_owned());
    }
}

fn read(watcher: Weak<Watcher>) {
    let mut buf = [std::mem::MaybeUninit::<u8>::uninit(); 16 * 1024];
    let mut pending: HashMap<i32, Change> = HashMap::new();
    let mut since: Option<Instant> = None;
    loop {
        let Some(w) = watcher.upgrade() else { return };
        let wait = match since {
            Some(first) if first.elapsed() >= AT_MOST => Duration::ZERO,
            Some(_) => SETTLE,
            None => Duration::from_secs(1),
        };
        let ready = {
            let mut fds = [PollFd::new(&w.fd, PollFlags::IN)];
            let timeout = rustix::event::Timespec::try_from(wait).ok();
            poll(&mut fds, timeout.as_ref()).unwrap_or(0) > 0
        };
        if ready {
            let mut reader = inotify::Reader::new(&w.fd, &mut buf);
            loop {
                match reader.next() {
                    Ok(event) => {
                        let flags = event.events();
                        if flags.contains(ReadFlags::QUEUE_OVERFLOW) {
                            w.overflowed(&mut pending);
                        } else if !flags.contains(ReadFlags::IGNORED) {
                            fold(&mut pending, event.wd(), flags, event.file_name().map(|n| n.to_bytes()));
                        }
                        since.get_or_insert_with(Instant::now);
                    }
                    Err(Errno::AGAIN) => break,
                    Err(Errno::INTR) => continue,
                    Err(e) => {
                        warn!("the watcher stopped: {e}");
                        return;
                    }
                }
            }
            if since.is_some_and(|first| first.elapsed() < AT_MOST) {
                continue;
            }
        }
        if !pending.is_empty() {
            w.deliver(&mut pending);
        }
        since = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn next(rx: &mpsc::Receiver<Change>) -> Change {
        rx.recv_timeout(Duration::from_secs(3)).expect("a change within three seconds")
    }

    #[test]
    fn changes_arrive_settled_by_name_and_the_last_watch_lets_go() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = Watcher::get().unwrap();
        let (tx, rx) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let tx = Mutex::new(tx);
        let tx2 = Mutex::new(tx2);
        let first = watcher.watch(dir.path(), move |c| tx.lock().unwrap().send(c).unwrap()).unwrap();
        let second = watcher.watch(dir.path(), move |c| tx2.lock().unwrap().send(c).unwrap()).unwrap();

        std::fs::write(dir.path().join("a"), "one").unwrap();
        std::fs::write(dir.path().join("a"), "two").unwrap();
        std::fs::rename(dir.path().join("a"), dir.path().join("b")).unwrap();
        let expected = Change::Touched(["a".into(), "b".into()].into());
        assert_eq!(next(&rx), expected);
        assert_eq!(next(&rx2), expected);

        drop(first);
        std::fs::write(dir.path().join("c"), "").unwrap();
        assert_eq!(next(&rx2), Change::Touched(["c".into()].into()));
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "a dropped watch hears nothing");

        drop(second);
        assert!(watcher.table.lock().unwrap().by_dir.is_empty());
    }

    #[test]
    fn a_folder_removed_is_gone() {
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("gone");
        std::fs::create_dir(&dir).unwrap();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let _watch = Watcher::get().unwrap().watch(&dir, move |c| tx.lock().unwrap().send(c).unwrap()).unwrap();
        std::fs::remove_dir(&dir).unwrap();
        assert_eq!(next(&rx), Change::Gone);
    }
}
