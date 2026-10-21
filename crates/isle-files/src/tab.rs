//! A tab: where it is, its history, the folder's entries as they arrive, and the selection,
//! held by path so it survives every change (v1 1b50dce).

use std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use freya::prelude::*;
use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::StreamExt;
use isle_fs::{
    Entry, Folder, ListError, Listed, Order,
    mounts::{self, Class},
    watch::{Change, Watch, Watcher},
};

use crate::{icons, places::Place};

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Loading,
    Ready,
    Failed(ListError),
    /// The folder shown was moved or deleted.
    Gone,
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub id: u64,
    pub place: Place,
    pub back: Vec<Place>,
    pub forward: Vec<Place>,
    pub folder: Folder,
    pub status: Status,
    /// Counts every change to `folder`, so a view redraws only when it moved.
    pub rev: u64,
    /// The selected items in the order they were chosen.
    pub selection: Vec<PathBuf>,
    /// Where the keyboard is: the item arrows and Shift move from.
    pub cursor: Option<PathBuf>,
    /// An item to select and scroll to once it arrives.
    pub reveal: Option<PathBuf>,
    /// The folder is on a mount that answers slowly.
    pub slow: bool,
}

impl Tab {
    pub fn new(id: u64, place: Place) -> Self {
        Self {
            id,
            place,
            back: Vec::new(),
            forward: Vec::new(),
            folder: Folder::new(Order::default()),
            status: Status::Loading,
            rev: 0,
            selection: Vec::new(),
            cursor: None,
            reveal: None,
            slow: false,
        }
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        self.selection.iter().any(|p| p == path)
    }

    pub fn select_only(&mut self, path: PathBuf) {
        self.selection = vec![path.clone()];
        self.cursor = Some(path);
    }

    /// Adds or removes one item, as ⌘-click does.
    pub fn toggle(&mut self, path: PathBuf) {
        match self.selection.iter().position(|p| *p == path) {
            Some(i) => {
                self.selection.remove(i);
            }
            None => self.selection.push(path.clone()),
        }
        self.cursor = Some(path);
    }

    /// Selects everything from the cursor to `path`, as ⇧-click does.
    pub fn extend_to(&mut self, path: &Path) {
        let entries = self.folder.entries();
        let to = self.folder.position(path);
        let from = self.cursor.as_deref().and_then(|c| self.folder.position(c)).or(to);
        if let (Some(from), Some(to)) = (from, to) {
            let (a, b) = if from <= to { (from, to) } else { (to, from) };
            self.selection = entries[a..=b].iter().map(|e| e.path.clone()).collect();
        }
    }

    /// Deselects everything, the cursor too (`apps.md` Files 17).
    pub fn deselect(&mut self) {
        self.selection.clear();
        self.cursor = None;
    }

    /// Selection held by path drops what is no longer listed.
    fn prune(&mut self) {
        let folder = &self.folder;
        self.selection.retain(|p| folder.position(p).is_some());
        if self.cursor.as_deref().is_some_and(|c| folder.position(c).is_none()) {
            self.cursor = self.selection.last().cloned();
        }
    }

    /// The item to reveal, selected once it has arrived.
    fn take_reveal(&mut self) {
        if let Some(path) = self.reveal.clone()
            && self.folder.position(&path).is_some()
        {
            self.reveal = None;
            self.select_only(path);
        }
    }
}

/// What the thread reading a tab's place sends it.
enum Message {
    Listed(Listed),
    Changed(Update),
}

/// A watched change, read again off the window's thread.
enum Update {
    Entries { upserts: Vec<Entry>, removed: Vec<PathBuf> },
    Reread,
    Gone,
}

/// What keeps a tab's place live: the listing's generation and the folder's watch.
#[derive(Clone, Default)]
pub struct Live(Rc<RefCell<LiveInner>>);

#[derive(Default)]
struct LiveInner {
    generation: Arc<AtomicU64>,
    watch: Option<Watch>,
    /// The channel to the task applying what is read; closed, the task ends.
    sender: Option<UnboundedSender<Message>>,
}

/// Finds and keeps what a batch's rows will draw with: their kinds' words and icons.
fn warm(entries: &[Entry]) {
    let mut seen = HashSet::new();
    for kind in entries.iter().filter_map(|e| e.mime.as_deref()) {
        if seen.insert(kind) {
            isle_apps::mime::types().comment(kind);
            icons::warm(kind, icons::SMALL);
        }
    }
}

/// Lists a gathering's paths.
fn gather(paths: Vec<PathBuf>, send: impl Fn(Listed) -> bool) {
    let entries: Vec<Entry> = paths.iter().filter_map(|p| Entry::at(p).ok()).collect();
    warm(&entries);
    if send(Listed::Batch(entries)) {
        send(Listed::Done);
    }
}

/// The local volumes, for the computer's view.
fn volumes() -> Vec<PathBuf> {
    let table = mounts::read();
    let mut seen = HashSet::new();
    table
        .iter()
        .filter(|m| m.class == Class::Local && !m.point.starts_with("/boot") && !m.point.starts_with("/var"))
        // A btrfs device is mounted once per subvolume; its first mount stands for it.
        .filter(|m| seen.insert(m.source.clone()))
        .map(|m| m.point.clone())
        .collect()
}

/// Reads touched names again, off the window's thread.
fn reread(dir: &Path, change: Change) -> Update {
    match change {
        Change::Gone => Update::Gone,
        Change::Overflow => Update::Reread,
        Change::Touched(names) => {
            let mut upserts = Vec::new();
            let mut removed = Vec::new();
            for name in names {
                let path = dir.join(name);
                match Entry::at(&path) {
                    Ok(entry) => upserts.push(entry),
                    Err(_) => removed.push(path),
                }
            }
            warm(&upserts);
            Update::Entries { upserts, removed }
        }
    }
}

/// Moves a tab to `place`, keeping history unless `history` is false.
pub fn go(mut tab: State<Tab>, live: &Live, place: Place, reveal: Option<PathBuf>) {
    {
        let mut t = tab.write();
        if t.place != place {
            let from = std::mem::replace(&mut t.place, place);
            t.back.push(from);
            t.forward.clear();
        }
        t.reveal = reveal;
    }
    load(tab, live, false);
}

pub fn back(mut tab: State<Tab>, live: &Live) {
    {
        let mut t = tab.write();
        let Some(to) = t.back.pop() else { return };
        let from = std::mem::replace(&mut t.place, to);
        t.forward.push(from);
        t.reveal = None;
    }
    load(tab, live, false);
}

pub fn forward(mut tab: State<Tab>, live: &Live) {
    {
        let mut t = tab.write();
        let Some(to) = t.forward.pop() else { return };
        let from = std::mem::replace(&mut t.place, to);
        t.back.push(from);
        t.reveal = None;
    }
    load(tab, live, false);
}

/// Reads the tab's place: from empty, or `again` over what is shown, keeping its rows,
/// selection and scroll while it reads.
pub fn load(mut tab: State<Tab>, live: &Live, again: bool) {
    let (tx, mut rx) = unbounded::<Message>();
    let generation = {
        let mut inner = live.0.borrow_mut();
        inner.watch = None;
        if let Some(old) = inner.sender.replace(tx.clone()) {
            old.close_channel();
        }
        inner.generation.fetch_add(1, Ordering::SeqCst) + 1
    };
    let place = {
        let mut t = tab.write();
        if !again {
            let (order, hidden) = (t.folder.order(), t.folder.shows_hidden());
            t.folder = Folder::new(order);
            t.folder.set_show_hidden(hidden);
            t.selection.clear();
            t.cursor = None;
        }
        t.status = Status::Loading;
        t.slow = false;
        t.rev += 1;
        t.place.clone()
    };
    let current = live.0.borrow().generation.clone();
    let send = {
        let current = current.clone();
        move |listed: Listed| {
            current.load(Ordering::SeqCst) == generation && tx.unbounded_send(Message::Listed(listed)).is_ok()
        }
    };
    let dir = place.folder().map(Path::to_owned);
    std::thread::Builder::new()
        .name("isle-files-list".into())
        .spawn(move || match place {
            Place::Folder(dir) => isle_fs::list(&dir, 512, |listed| {
                if let Listed::Batch(batch) = &listed {
                    warm(batch);
                }
                send(listed)
            }),
            Place::Recents => gather(crate::places::recents(200), send),
            Place::Computer => gather(volumes(), send),
            Place::Trash => {
                send(Listed::Done);
            }
        })
        .ok();

    let live = live.clone();
    spawn(async move {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        while let Some(message) = rx.next().await {
            if current.load(Ordering::SeqCst) != generation {
                return;
            }
            let mut t = tab.write();
            match message {
                Message::Listed(Listed::Batch(batch)) => {
                    if again {
                        seen.extend(batch.iter().map(|e| e.path.clone()));
                    }
                    t.folder.merge(batch);
                    t.take_reveal();
                    t.rev += 1;
                }
                Message::Listed(Listed::Done) => {
                    if again {
                        t.folder.keep_only(&seen);
                        t.prune();
                    }
                    t.status = Status::Ready;
                    t.rev += 1;
                    drop(t);
                    if let Some(dir) = &dir {
                        watch(&live, dir, generation);
                    }
                }
                Message::Listed(Listed::Failed(e)) => {
                    t.status = if e == ListError::Gone { Status::Gone } else { Status::Failed(e) };
                    t.rev += 1;
                }
                Message::Changed(Update::Entries { upserts, removed }) => {
                    for path in removed {
                        t.folder.remove(&path);
                    }
                    for entry in upserts {
                        t.folder.upsert(entry);
                    }
                    t.prune();
                    t.take_reveal();
                    t.rev += 1;
                }
                Message::Changed(Update::Reread) => {
                    drop(t);
                    load(tab, &live, true);
                    return;
                }
                Message::Changed(Update::Gone) => {
                    t.status = Status::Gone;
                    t.rev += 1;
                }
            }
        }
    });
}

/// Watches a folder on a local disk; a remote folder is read again when shown instead.
fn watch(live: &Live, dir: &Path, generation: u64) {
    let Some(tx) = live.0.borrow().sender.clone() else { return };
    let table = mounts::read();
    if mounts::of(&table, dir).is_some_and(|m| m.class != Class::Local) {
        return;
    }
    let Some(watcher) = Watcher::get() else { return };
    let current = live.0.borrow().generation.clone();
    let owned = dir.to_owned();
    let watch = watcher.watch(dir, move |change| {
        if current.load(Ordering::SeqCst) != generation {
            return;
        }
        let (dir, tx) = (owned.clone(), tx.clone());
        std::thread::spawn(move || {
            let _ = tx.unbounded_send(Message::Changed(reread(&dir, change)));
        });
    });
    live.0.borrow_mut().watch = watch;
}
