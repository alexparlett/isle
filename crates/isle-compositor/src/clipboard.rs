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
        current_data_device_selection_userdata, request_data_device_client_selection,
        set_data_device_selection,
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
        let dh = self.isle.display_handle.clone();
        let types = pick.types.iter().map(|(t, _)| t.clone()).collect();
        set_data_device_selection(&dh, &self.isle.seat, types, Owner::Isle(Arc::new(pick)));
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
            }
        }
    }

    /// Puts `pick` into the window `target` by making it the clipboard and sending that app's
    /// paste chord; `copy_only`, or no window, leaves it on the clipboard and says so.
    pub fn insert_pick(&mut self, target: Option<u64>, pick: Pick, copy_only: bool) {
        self.set_clipboard(pick);
        match target.filter(|_| !copy_only) {
            Some(id) => {
                let chord = isle_keys::chord::Chord::parse("Cmd+V").expect("a chord");
                self.send_chord_to(id, chord);
            }
            None => self.island_copied(),
        }
    }

    /// A clipboard item picked from history.
    pub fn insert_clip(&mut self, target: Option<u64>, id: u64, copy_only: bool) {
        if let Some(pick) = self.isle.clipboard.pick(id) {
            self.insert_pick(target, pick, copy_only);
        }
    }
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
}

impl ClipboardState {
    pub fn pick(&self, id: u64) -> Option<Pick> {
        self.items
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, p)| p.clone())
    }
}
