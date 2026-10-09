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
        self.set_clipboard(Pick::text(text));
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
        if let Some(Owner::Isle(pick)) = owner {
            return then(self, Some((*pick).clone()));
        }
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
            if request_data_device_client_selection(&self.isle.seat, mime.clone(), write).is_ok() {
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

/// Clipboard history; filled in by the selection hook.
#[derive(Default)]
pub struct ClipboardState {
    items: Vec<(u64, Pick)>,
    /// The types the client that set the clipboard offers.
    offered: Vec<String>,
    /// A pick pasted for the user, with the clipboard to put back after.
    restore: Option<Restore>,
}

impl ClipboardState {
    pub fn pick(&self, id: u64) -> Option<Pick> {
        self.items
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| p.clone())
    }
}
