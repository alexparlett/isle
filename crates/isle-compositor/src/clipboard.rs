//! The clipboard as Isle holds it: selections Isle offers itself, reading what a client offers
//! off the loop, and a pick put where the user was (`p10-spotlight.md` choices 16 and 17).

use std::{
    io::{Read, Write},
    os::fd::OwnedFd,
    sync::Arc,
    time::{Duration, Instant},
};

use smithay::wayland::selection::{
    SelectionTarget,
    data_device::{
        clear_data_device_selection, current_data_device_selection_userdata,
        request_data_device_client_selection, set_data_device_selection,
    },
};
use tracing::warn;

use crate::state::State;

/// The most a read of a client's offer takes in, and how long it may take.
pub const READ_CAP: usize = 8 << 20;
pub const READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Text's types, as clients ask for them.
pub const TEXT_TYPES: [&str; 5] = [
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "TEXT",
    "STRING",
];

/// Something Isle offers as the clipboard: each type with its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub types: Vec<(String, Arc<Vec<u8>>)>,
}

impl Pick {
    pub fn text(text: impl Into<String>) -> Self {
        let bytes = Arc::new(text.into().into_bytes());
        Self {
            types: TEXT_TYPES
                .iter()
                .map(|t| (t.to_string(), bytes.clone()))
                .collect(),
        }
    }

    fn bytes(&self, mime: &str) -> Option<Arc<Vec<u8>>> {
        self.types
            .iter()
            .find(|(t, _)| t == mime)
            .map(|(_, b)| b.clone())
    }
}

/// Who a selection the compositor set belongs to.
#[derive(Clone, Debug)]
pub enum Owner {
    /// An X11 client's, which Isle's window manager reads for a Wayland client.
    Xwayland,
    Isle(Arc<Pick>),
}

/// Writes `bytes` into `fd` off the loop.
pub fn write_off_loop(runtime: &tokio::runtime::Handle, fd: OwnedFd, bytes: Arc<Vec<u8>>) {
    runtime.spawn_blocking(move || {
        let mut file = std::fs::File::from(fd);
        let _ = file.write_all(&bytes);
    });
}

/// Reads a pipe until it closes, at most `READ_CAP` bytes and `READ_TIMEOUT`; `None` past
/// either.
pub fn read_pipe(fd: OwnedFd) -> Option<Vec<u8>> {
    use std::os::fd::AsRawFd;
    let file = std::fs::File::from(fd);
    let raw = file.as_raw_fd();
    let started = Instant::now();
    let mut out = Vec::new();
    let mut file = file;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let left = READ_TIMEOUT.checked_sub(started.elapsed())?;
        let mut poll = libc::pollfd {
            fd: raw,
            events: libc::POLLIN,
            revents: 0,
        };
        // Safety: one valid descriptor, polled for at most the time left.
        let ready = unsafe { libc::poll(&mut poll, 1, left.as_millis() as i32) };
        if ready <= 0 {
            return None;
        }
        match file.read(&mut buf) {
            Ok(0) => return Some(out),
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                if out.len() > READ_CAP {
                    return None;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

impl State {
    /// Makes `pick` the clipboard, offered by Isle.
    pub fn set_clipboard(&mut self, pick: Pick) {
        self.offer(Arc::new(pick));
    }

    fn offer(&mut self, pick: Arc<Pick>) {
        let dh = self.isle.display_handle.clone();
        let types: Vec<String> = pick.types.iter().map(|(t, _)| t.clone()).collect();
        set_data_device_selection(&dh, &self.isle.seat, types.clone(), Owner::Isle(pick));
        self.tell_xwayland(SelectionTarget::Clipboard, Some(types));
    }

    /// Tells Isle's X11 window manager that the Wayland side set a selection, so X11 clients
    /// read it through Xwayland.
    pub fn tell_xwayland(&mut self, ty: SelectionTarget, types: Option<Vec<String>>) {
        if let Some(xwm) = self.isle.xwm.as_mut()
            && let Err(err) = xwm.new_selection(ty, types)
        {
            warn!("could not hand a selection to Xwayland: {err}");
        }
    }

    /// Copies `text` and says so in the island.
    pub fn copy_text(&mut self, text: String) {
        let pick = Pick::text(text);
        self.keep_copy(&pick, String::new());
        self.set_clipboard(pick);
        self.island_copied();
    }

    pub fn island_copied(&mut self) {
        use isle_shell::island::model::{Event, EventKind, Form};
        self.isle.island.model.post(
            Event {
                kind: EventKind::Copied,
                form: Form::Line {
                    symbol: "copy".into(),
                    fill: false,
                    text: "Copied".into(),
                },
                output: None,
                app: None,
                urgent: false,
            },
            Instant::now(),
        );
        self.isle.refresh_island();
    }

    /// The clipboard's text when Isle offers it.
    pub fn isle_clipboard_text(&self) -> Option<String> {
        let owner = current_data_device_selection_userdata(&self.isle.seat).map(|o| o.clone())?;
        match owner {
            Owner::Isle(pick) => TEXT_TYPES
                .iter()
                .find_map(|t| pick.bytes(t))
                .map(|b| String::from_utf8_lossy(&b).into_owned()),
            Owner::Xwayland => None,
        }
    }

    /// The clipboard's text, read off the loop when a client offers it, handed to `then` on
    /// the loop.
    pub fn read_clipboard_text(
        &mut self,
        then: impl FnOnce(&mut State, Option<String>) + Send + 'static,
    ) {
        if let Some(owner) =
            current_data_device_selection_userdata(&self.isle.seat).map(|o| o.clone())
            && let Owner::Isle(pick) = owner
        {
            let text = TEXT_TYPES
                .iter()
                .find_map(|t| pick.bytes(t))
                .map(|b| String::from_utf8_lossy(&b).into_owned());
            return then(self, text);
        }
        let (Some(worker), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return then(self, None);
        };
        let Ok((read, write)) = rustix_pipe() else {
            return then(self, None);
        };
        if let Err(err) =
            request_data_device_client_selection(&self.isle.seat, TEXT_TYPES[0].to_owned(), write)
        {
            warn!("could not read the clipboard: {err}");
            return then(self, None);
        }
        worker.spawn_blocking(move || {
            let text = read_pipe(read).map(|b| String::from_utf8_lossy(&b).into_owned());
            to_loop.post(move |state| then(state, text));
        });
    }

    /// A client asked for a selection the compositor set.
    pub fn send_selection_to(
        &mut self,
        ty: SelectionTarget,
        mime: String,
        fd: OwnedFd,
        owner: &Owner,
    ) {
        match owner {
            Owner::Xwayland => {
                if let Some(xwm) = self.isle.xwm.as_mut()
                    && let Err(err) = xwm.send_selection(ty, mime, fd)
                {
                    warn!("could not read an X11 selection: {err}");
                }
            }
            Owner::Isle(pick) => {
                if let (Some(bytes), Some(worker)) = (pick.bytes(&mime), self.isle.worker.clone()) {
                    write_off_loop(&worker, fd, bytes);
                }
                self.pick_read(pick);
            }
        }
    }

    /// Puts `pick` into the window `target`: it becomes the clipboard and the app is sent
    /// its paste chord. With `restore` the clipboard as it was comes back once the app has
    /// read the pick, or after a second; `copy_only`, or no window, leaves the pick on the
    /// clipboard and says so.
    pub fn insert_pick(&mut self, target: Option<u64>, pick: Pick, copy_only: bool, restore: bool) {
        let Some(target) = target.filter(|_| !copy_only) else {
            self.set_clipboard(pick);
            self.island_copied();
            return;
        };
        if !restore {
            self.set_clipboard(pick);
            self.paste_into(target);
            return;
        }
        self.read_selection(move |state, previous| {
            let pick = Arc::new(pick);
            state.offer(pick.clone());
            state.isle.clipboard.restore = Some(Restore {
                pick,
                previous,
                timer: None,
            });
            state.arm_restore(RESTORE_UNREAD);
            state.paste_into(target);
        });
    }

    fn paste_into(&mut self, target: u64) {
        let chord = isle_keys::chord::Chord::parse("Cmd+V").expect("a chord");
        self.send_chord_to(target, chord);
    }

    /// Puts the clipboard back `after` from now, unless something else took it first.
    fn arm_restore(&mut self, after: Duration) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        let Some(restore) = self.isle.clipboard.restore.as_mut() else {
            return;
        };
        if let Some(token) = restore.timer.take() {
            self.isle.loop_handle.remove(token);
        }
        let token =
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(after), |_, _, state| {
                    if let Some(restore) = state.isle.clipboard.restore.take() {
                        let ours = current_data_device_selection_userdata(&state.isle.seat)
                            .is_some_and(
                                |o| matches!(&*o, Owner::Isle(p) if Arc::ptr_eq(p, &restore.pick)),
                            );
                        if ours {
                            match restore.previous {
                                Some(previous) => state.set_clipboard(previous),
                                None => {
                                    let dh = state.isle.display_handle.clone();
                                    clear_data_device_selection(&dh, &state.isle.seat);
                                    state.tell_xwayland(SelectionTarget::Clipboard, None);
                                }
                            }
                        }
                    }
                    TimeoutAction::Drop
                });
        if let (Ok(token), Some(restore)) = (token, self.isle.clipboard.restore.as_mut()) {
            restore.timer = Some(token);
        }
    }

    /// The clipboard's text, image and file types as a pick of Isle's own, read off the loop
    /// when a client offers them; `None` when nothing is copied.
    pub fn read_selection(&mut self, then: impl FnOnce(&mut State, Option<Pick>) + Send + 'static) {
        let owner = current_data_device_selection_userdata(&self.isle.seat).map(|o| o.clone());
        if let Some(Owner::Isle(pick)) = &owner {
            return then(self, Some((**pick).clone()));
        }
        let from_x11 = matches!(owner, Some(Owner::Xwayland));
        let wanted = kept_types(&self.isle.clipboard.offered);
        let (Some(worker), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return then(self, None);
        };
        if wanted.is_empty() {
            return then(self, None);
        }
        let mut reads = Vec::new();
        for mime in wanted {
            let Ok((read, write)) = rustix_pipe() else {
                continue;
            };
            let asked = if from_x11 {
                self.isle.xwm.as_mut().is_some_and(|xwm| {
                    xwm.send_selection(SelectionTarget::Clipboard, mime.clone(), write)
                        .is_ok()
                })
            } else {
                request_data_device_client_selection(&self.isle.seat, mime.clone(), write).is_ok()
            };
            if asked {
                reads.push((mime, read));
            }
        }
        worker.spawn_blocking(move || {
            let types: Vec<(String, Arc<Vec<u8>>)> = reads
                .into_iter()
                .filter_map(|(mime, fd)| read_pipe(fd).map(|b| (mime, Arc::new(b))))
                .filter(|(_, b)| !b.is_empty())
                .collect();
            let pick = (!types.is_empty()).then_some(Pick { types });
            to_loop.post(move |state| then(state, pick));
        });
    }

    /// A client set the clipboard: its types are kept for reading it later.
    pub fn clipboard_offered(&mut self, types: Vec<String>) {
        self.isle.clipboard.offered = types;
    }

    /// A client is reading a pick Isle offers: a pick waiting to be put back goes back soon
    /// after its last read.
    pub fn pick_read(&mut self, pick: &Arc<Pick>) {
        let ours = self
            .isle
            .clipboard
            .restore
            .as_ref()
            .is_some_and(|r| Arc::ptr_eq(&r.pick, pick));
        if ours {
            self.arm_restore(RESTORE_AFTER_READ);
        }
    }

    /// A clipboard item picked from history.
    pub fn insert_clip(&mut self, target: Option<u64>, id: u64, copy_only: bool) {
        if let Some(pick) = self.isle.clipboard.pick(id) {
            if let Some(at) = self.isle.clipboard.items.iter().position(|i| i.id == id) {
                let mut item = self.isle.clipboard.items.remove(at);
                item.at = unix_now();
                self.isle.clipboard.items.insert(0, item);
                self.save_clipboard(Vec::new(), Vec::new());
            }
            self.insert_pick(target, pick, copy_only, false);
        }
    }
}

/// How long a pick waits to be read before the clipboard is put back, and how long after its
/// last read.
const RESTORE_UNREAD: Duration = Duration::from_secs(1);
const RESTORE_AFTER_READ: Duration = Duration::from_millis(250);

/// The types worth keeping of an offer: its text as UTF-8, an image, a file list.
pub fn kept_types(offered: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let text = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"]
        .iter()
        .find(|t| offered.iter().any(|o| o == *t));
    if let Some(t) = text {
        out.push(t.to_string());
    }
    if let Some(image) = ["image/png", "image/jpeg", "image/webp"]
        .iter()
        .find(|t| offered.iter().any(|o| o == *t))
    {
        out.push(image.to_string());
    }
    if offered.iter().any(|o| o == "text/uri-list") {
        out.push("text/uri-list".into());
    }
    out
}

struct Restore {
    pick: Arc<Pick>,
    previous: Option<Pick>,
    timer: Option<smithay::reexports::calloop::RegistrationToken>,
}

/// A pipe whose ends close on exec.
pub fn rustix_pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
    use std::os::fd::FromRawFd;
    let mut fds = [0; 2];
    // Safety: `fds` has room for both ends.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // Safety: both descriptors are new and owned here.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// How many unpinned copies history keeps, and for how long.
const KEEP_ITEMS: usize = 200;
const KEEP_FOR: u64 = 30 * 86_400;
/// The most of an item's text kept beside it for searching.
const SEARCH_TEXT: usize = 64 * 1024;

/// One kept copy: its types' bytes are files in history's folder, named by id and place.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub id: u64,
    /// Unix seconds.
    pub at: u64,
    pub app: String,
    pub pinned: bool,
    pub types: Vec<String>,
    /// The text, cut for searching; empty for an image or files.
    pub text: String,
    pub size: Option<(u32, u32)>,
}

impl Item {
    fn file(&self, dir: &std::path::Path, n: usize) -> std::path::PathBuf {
        dir.join(format!("{}-{n}", self.id))
    }

    /// The image's file, for thumbnails.
    pub fn image(&self, dir: &std::path::Path) -> Option<std::path::PathBuf> {
        let n = self.types.iter().position(|t| t.starts_with("image/"))?;
        Some(self.file(dir, n))
    }
}

/// Clipboard history in `~/.local/state/isle/clipboard/`, newest first, read at start and
/// written from the worker.
#[derive(Default)]
pub struct ClipboardState {
    dir: std::path::PathBuf,
    pub items: Vec<Item>,
    next: u64,
    /// The types the client that set the clipboard offers.
    offered: Vec<String>,
    /// A pick pasted for the user, with the clipboard to put back after.
    restore: Option<Restore>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl ClipboardState {
    pub fn load(dir: std::path::PathBuf) -> Self {
        let items: Vec<Item> = std::fs::read(dir.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let next = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        Self {
            dir,
            items,
            next,
            ..Default::default()
        }
    }

    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    /// The kept copy `id` as a pick, read from its files.
    pub fn pick(&self, id: u64) -> Option<Pick> {
        let item = self.items.iter().find(|i| i.id == id)?;
        let types: Vec<(String, Arc<Vec<u8>>)> = item
            .types
            .iter()
            .enumerate()
            .filter_map(|(n, t)| {
                std::fs::read(item.file(&self.dir, n))
                    .ok()
                    .map(|b| (t.clone(), Arc::new(b)))
            })
            .collect();
        let mut pick = Pick { types };
        // Text goes out under every name clients ask for it by.
        if let Some(text) = pick
            .types
            .iter()
            .find(|(t, _)| t.starts_with("text/plain"))
            .map(|(_, b)| b.clone())
        {
            for name in TEXT_TYPES {
                if !pick.types.iter().any(|(t, _)| t == name) {
                    pick.types.push((name.to_owned(), text.clone()));
                }
            }
        }
        (!pick.types.is_empty()).then_some(pick)
    }

    /// Keeps `pick`, copied in `app`: a copy of the newest text moves it up instead; the
    /// oldest unpinned past the caps go. The files to write and to remove, for the worker.
    fn keep(
        &mut self,
        pick: &Pick,
        app: String,
    ) -> (
        Vec<(std::path::PathBuf, Arc<Vec<u8>>)>,
        Vec<std::path::PathBuf>,
    ) {
        let now = unix_now();
        let text = pick
            .types
            .iter()
            .find(|(t, _)| t.starts_with("text/plain") || *t == "UTF8_STRING")
            .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let image = pick.types.iter().find(|(t, _)| t.starts_with("image/"));
        if image.is_none()
            && !text.is_empty()
            && let Some(at) = self.items.iter().position(|i| {
                !i.pinned && i.text == text.chars().take(SEARCH_TEXT).collect::<String>()
            })
        {
            let mut item = self.items.remove(at);
            item.at = now;
            self.items.insert(0, item);
            return (Vec::new(), Vec::new());
        }
        let size = image.and_then(|(_, b)| png_size(b));
        let kept: Vec<(String, Arc<Vec<u8>>)> = kept_types(
            &pick
                .types
                .iter()
                .map(|(t, _)| t.clone())
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .filter_map(|t| pick.types.iter().find(|(n, _)| *n == t).cloned())
        .collect();
        let item = Item {
            id: self.next,
            at: now,
            app,
            pinned: false,
            types: kept.iter().map(|(t, _)| t.clone()).collect(),
            text: if image.is_some() {
                String::new()
            } else {
                text.chars().take(SEARCH_TEXT).collect()
            },
            size,
        };
        self.next += 1;
        let writes = kept
            .iter()
            .enumerate()
            .map(|(n, (_, b))| (item.file(&self.dir, n), b.clone()))
            .collect();
        self.items.insert(0, item);
        (writes, self.trim(now))
    }

    /// Drops unpinned copies past the caps; their files.
    fn trim(&mut self, now: u64) -> Vec<std::path::PathBuf> {
        let mut unpinned = 0;
        let mut gone = Vec::new();
        let dir = self.dir.clone();
        self.items.retain(|i| {
            if i.pinned {
                return true;
            }
            unpinned += 1;
            let keep = unpinned <= KEEP_ITEMS && now.saturating_sub(i.at) <= KEEP_FOR;
            if !keep {
                gone.extend((0..i.types.len()).map(|n| i.file(&dir, n)));
            }
            keep
        });
        gone
    }

    fn remove(&mut self, id: u64) -> Vec<std::path::PathBuf> {
        let Some(at) = self.items.iter().position(|i| i.id == id) else {
            return Vec::new();
        };
        let item = self.items.remove(at);
        (0..item.types.len())
            .map(|n| item.file(&self.dir, n))
            .collect()
    }

    /// The copies as Spotlight's clipboard source lists them.
    pub fn clips(
        &self,
        name: impl Fn(&str) -> String,
    ) -> Vec<isle_search::sources::clipboard::Clip> {
        use isle_search::sources::clipboard::{Clip, Kind};
        self.items
            .iter()
            .map(|i| {
                let kind = if let Some(path) = i.image(&self.dir) {
                    Kind::Image { path, size: i.size }
                } else if i.text.is_empty() && i.types.iter().any(|t| t == "text/uri-list") {
                    Kind::Files {
                        first: "Files".into(),
                        count: 1,
                    }
                } else {
                    let mut lines = i.text.lines();
                    Kind::Text {
                        first: lines.next().unwrap_or("").trim().to_owned(),
                        more: lines.count(),
                    }
                };
                Clip {
                    id: i.id,
                    kind,
                    text: i.text.clone(),
                    app: name(&i.app),
                    pinned: i.pinned,
                }
            })
            .collect()
    }
}

/// A PNG's width and height from its header.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    (bytes.len() > 24 && bytes[..8] == *b"\x89PNG\r\n\x1a\n").then(|| {
        (
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        )
    })
}

impl State {
    /// A copy was made: it is read off the loop and kept, unless it is marked a secret or
    /// comes from an excluded app (`platform.md` Clipboard 2).
    pub fn record_copy(&mut self) {
        let offered = self.isle.clipboard.offered.clone();
        if offered.iter().any(|t| t == "x-kde-passwordManagerHint") {
            return;
        }
        let app = self.isle.front_app().unwrap_or_default();
        let excluded = &self.isle.settings.spotlight.clipboard_exclude;
        if excluded.iter().any(|x| x == &app) {
            return;
        }
        self.read_selection(move |state, pick| {
            if let Some(pick) = pick {
                state.keep_copy(&pick, app);
            }
        });
    }

    /// Keeps `pick` in history and writes it out.
    pub fn keep_copy(&mut self, pick: &Pick, app: String) {
        let (writes, gone) = self.isle.clipboard.keep(pick, app);
        self.save_clipboard(writes, gone);
    }

    /// Writes history's index, the new files and removes the old, on the worker.
    fn save_clipboard(
        &mut self,
        writes: Vec<(std::path::PathBuf, Arc<Vec<u8>>)>,
        gone: Vec<std::path::PathBuf>,
    ) {
        let Some(worker) = self.isle.worker.clone() else {
            return;
        };
        let dir = self.isle.clipboard.dir.clone();
        let index = serde_json::to_vec(&self.isle.clipboard.items).unwrap_or_default();
        worker.spawn_blocking(move || {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            let _ = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&dir);
            let write = |path: &std::path::Path, bytes: &[u8]| {
                let tmp = path.with_extension("tmp");
                let done = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&tmp)
                    .and_then(|mut f| f.write_all(bytes))
                    .and_then(|_| std::fs::rename(&tmp, path));
                if let Err(err) = done {
                    warn!(path = %path.display(), "could not keep a copy: {err}");
                }
            };
            for (path, bytes) in writes {
                write(&path, &bytes);
            }
            for path in gone {
                let _ = std::fs::remove_file(path);
            }
            write(&dir.join("index.json"), &index);
        });
    }

    pub fn clipboard_pin(&mut self, id: u64) {
        if let Some(item) = self.isle.clipboard.items.iter_mut().find(|i| i.id == id) {
            item.pinned = !item.pinned;
        }
        self.save_clipboard(Vec::new(), Vec::new());
    }

    pub fn clipboard_delete(&mut self, id: u64) {
        let gone = self.isle.clipboard.remove(id);
        self.save_clipboard(Vec::new(), gone);
    }

    /// Forgets every copy but the pinned.
    pub fn clipboard_clear(&mut self) {
        let dir = self.isle.clipboard.dir.clone();
        let mut gone = Vec::new();
        self.isle.clipboard.items.retain(|i| {
            if !i.pinned {
                gone.extend((0..i.types.len()).map(|n| i.file(&dir, n)));
            }
            i.pinned
        });
        self.save_clipboard(Vec::new(), gone);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("isle-clip-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn copies_are_kept_newest_first_and_capped() {
        let mut history = ClipboardState::load(temp("keep"));
        for n in 0..(KEEP_ITEMS + 5) {
            history.keep(&Pick::text(format!("copy {n}")), "kitty".into());
        }
        assert_eq!(history.items.len(), KEEP_ITEMS);
        assert_eq!(history.items[0].text, format!("copy {}", KEEP_ITEMS + 4));
        let id = history.items[KEEP_ITEMS - 1].id;
        history
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .unwrap()
            .pinned = true;
        history.keep(&Pick::text("one more"), "kitty".into());
        assert!(
            history.items.iter().any(|i| i.id == id),
            "a pin is never evicted"
        );
        assert_eq!(
            history.items.iter().filter(|i| !i.pinned).count(),
            KEEP_ITEMS
        );
    }

    #[test]
    fn copying_the_same_text_again_moves_it_up() {
        let mut history = ClipboardState::load(temp("dup"));
        history.keep(&Pick::text("a"), "kitty".into());
        history.keep(&Pick::text("b"), "kitty".into());
        let (writes, _) = history.keep(&Pick::text("a"), "kitty".into());
        assert!(writes.is_empty());
        let texts: Vec<&str> = history.items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, ["a", "b"]);
    }

    #[test]
    fn only_text_images_and_files_are_kept() {
        let mut history = ClipboardState::load(temp("types"));
        let pick = Pick {
            types: vec![
                ("text/html".into(), Arc::new(b"<b>x</b>".to_vec())),
                ("text/plain;charset=utf-8".into(), Arc::new(b"x".to_vec())),
                ("application/x-secret".into(), Arc::new(b"no".to_vec())),
            ],
        };
        history.keep(&pick, "firefox".into());
        assert_eq!(history.items[0].types, ["text/plain;charset=utf-8"]);
    }
}
