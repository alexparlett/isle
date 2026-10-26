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

use crate::{icons, places::Place, view::Shown};

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
    /// How the place is shown.
    pub shown: Shown,
    /// The folder shown, as its parent lists it, for the preview of nothing selected.
    pub here: Option<Entry>,
    /// The place is on a remote mount: no thumbnails, no counts.
    pub remote: bool,
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
            shown: Shown::default(),
            here: None,
            remote: false,
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
    pub fn prune(&mut self) {
        let folder = &self.folder;
        self.selection.retain(|p| folder.position(p).is_some());
        if self.cursor.as_deref().is_some_and(|c| folder.position(c).is_none()) {
            self.cursor = self.selection.last().cloned();
        }
    }

    /// Selects `path` now, or once it has arrived.
    pub fn reveal(&mut self, path: PathBuf) {
        self.reveal = Some(path);
        self.take_reveal();
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
    Here(Entry, bool),
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

impl Live {
    pub fn new(shown_for: ShownFor) -> Self {
        Self(Rc::new(RefCell::new(LiveInner {
            shown_for: Some(shown_for),
            ..Default::default()
        })))
    }
}

/// A place's remembered view.
pub type ShownFor = Rc<dyn Fn(&Place) -> Shown>;

#[derive(Default)]
struct LiveInner {
    shown_for: Option<ShownFor>,
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
            icons::warm(kind, icons::LARGE);
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

/// As `gather`, each entry shown by its name in `names`.
fn gather_named(paths: Vec<PathBuf>, names: Vec<String>, send: impl Fn(Listed) -> bool) {
    let entries: Vec<Entry> = paths
        .iter()
        .zip(names)
        .filter_map(|(p, name)| Entry::at(p).ok().map(|e| Entry { name, ..e }))
        .collect();
    warm(&entries);
    if send(Listed::Batch(entries)) {
        send(Listed::Done);
    }
}

/// The local volumes, for the computer's view, each with its name: its filesystem's
/// label, its folder's name, or System for the root.
fn volumes() -> Vec<(PathBuf, String)> {
    let table = mounts::read();
    let labels = labels();
    let mut seen = HashSet::new();
    table
        .iter()
        .filter(|m| m.class == Class::Local && !m.point.starts_with("/boot") && !m.point.starts_with("/var"))
        // A btrfs device is mounted once per subvolume; its first mount stands for it.
        .filter(|m| seen.insert(m.source.clone()))
        .map(|m| {
            let device = std::fs::canonicalize(&m.source).unwrap_or_else(|_| PathBuf::from(&m.source));
            let name = labels
                .iter()
                .find(|(d, _)| *d == device)
                .map(|(_, l)| l.clone())
                .or_else(|| m.point.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "System".into());
            (m.point.clone(), name)
        })
        .collect()
}

/// Each labelled device and its label, from udev's links.
fn labels() -> Vec<(PathBuf, String)> {
    let Ok(dir) = std::fs::read_dir("/dev/disk/by-label") else { return Vec::new() };
    dir.flatten()
        .filter_map(|link| {
            let device = std::fs::canonicalize(link.path()).ok()?;
            // udev writes a space in a label as `\x20`.
            let label = link.file_name().to_string_lossy().replace("\\x20", " ");
            Some((device, label))
        })
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

/// Moves a tab to `place` in the place of the one shown, as a search changed in its scope bar.
pub fn replace(mut tab: State<Tab>, live: &Live, place: Place) {
    {
        let mut t = tab.write();
        t.place = place;
        t.reveal = None;
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
            if let Some(shown_for) = live.0.borrow().shown_for.clone() {
                t.shown = shown_for(&t.place);
            }
            let (order, hidden) = (t.shown.order(), t.folder.shows_hidden());
            t.folder = Folder::new(order);
            t.folder.set_show_hidden(hidden);
            t.selection.clear();
            t.cursor = None;
        }
        t.status = Status::Loading;
        t.slow = false;
        if !again {
            t.here = None;
        }
        t.rev += 1;
        t.place.clone()
    };
    let current = live.0.borrow().generation.clone();
    let here = tx.clone();
    let send = {
        let current = current.clone();
        move |listed: Listed| {
            current.load(Ordering::SeqCst) == generation && tx.unbounded_send(Message::Listed(listed)).is_ok()
        }
    };
    let dir = place.folder().map(Path::to_owned);
    let hidden = tab.peek().folder.shows_hidden();
    std::thread::Builder::new()
        .name("isle-files-list".into())
        .spawn(move || match place {
            Place::Folder(dir) => {
                if let Ok(entry) = Entry::at(&dir) {
                    let table = mounts::read();
                    let remote = mounts::of(&table, &dir).is_some_and(|m| m.class == Class::Remote);
                    let _ = here.unbounded_send(Message::Here(entry, remote));
                }
                isle_fs::list(&dir, 512, |listed| {
                if let Listed::Batch(batch) = &listed {
                    warm(batch);
                }
                send(listed)
            })
            }
            Place::Recents => gather(crate::places::recents(200), send),
            Place::Computer => {
                let (points, names): (Vec<PathBuf>, Vec<String>) = volumes().into_iter().unzip();
                gather_named(points, names, send)
            }
            Place::Search(search) => crate::search::run(&search, hidden, |listed| {
                if let Listed::Batch(batch) = &listed {
                    warm(batch);
                }
                send(listed)
            }),
            Place::Trash => {
                let items: Vec<Entry> = isle_fs::trash::all_trashes()
                    .iter()
                    .flat_map(isle_fs::trash::list)
                    .filter_map(|t| {
                        let mut entry = Entry::at(&t.file).ok()?;
                        entry.trashed = Some(std::sync::Arc::new(t));
                        Some(entry)
                    })
                    .collect();
                warm(&items);
                if send(Listed::Batch(items)) {
                    send(Listed::Done);
                }
            }
        })
        .ok();

    let live = live.clone();
    spawn(async move {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let (started, mut first) = (std::time::Instant::now(), true);
        while let Some(message) = rx.next().await {
            if current.load(Ordering::SeqCst) != generation {
                return;
            }
            let mut t = tab.write();
            match message {
                Message::Here(entry, remote) => {
                    t.here = Some(entry);
                    t.remote = remote;
                }
                Message::Listed(Listed::Batch(batch)) => {
                    if again {
                        seen.extend(batch.iter().map(|e| e.path.clone()));
                    }
                    let count = batch.len();
                    t.folder.merge(batch);
                    t.take_reveal();
                    t.rev += 1;
                    if std::mem::take(&mut first) && !again {
                        tracing::info!(entries = count, ms = started.elapsed().as_millis() as u64, "first batch");
                    }
                }
                Message::Listed(Listed::Done) => {
                    if again {
                        t.folder.keep_only(&seen);
                        t.prune();
                    }
                    t.status = Status::Ready;
                    t.rev += 1;
                    if matches!(t.place, Place::Search(_)) {
                        tracing::info!(found = t.folder.len(), "searched");
                    } else if !again {
                        tracing::info!(entries = t.folder.len(), ms = started.elapsed().as_millis() as u64, "listed");
                    }
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
    if mounts::of(&table, dir).is_some_and(|m| m.class == Class::Remote) {
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
