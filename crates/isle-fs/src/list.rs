//! A folder read in batches, so the first screen draws before the last entry is read
//! (`apps.md` Files 1; v1's D66, which v1 never did).

use std::{os::unix::ffi::OsStrExt, path::Path};

use rustix::fs::{Dir, Mode, OFlags};

use crate::Entry;

/// What a listing sends, in order: batches, then `Done` or `Failed`.
#[derive(Debug)]
pub enum Listed {
    Batch(Vec<Entry>),
    Done,
    Failed(ListError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListError {
    /// "You don't have permission to see the contents of …"
    Denied,
    /// "… was moved or deleted."
    Gone,
    NotAFolder,
    Other(i32),
}

impl From<rustix::io::Errno> for ListError {
    fn from(e: rustix::io::Errno) -> Self {
        use rustix::io::Errno;
        match e {
            Errno::ACCESS | Errno::PERM => ListError::Denied,
            Errno::NOENT => ListError::Gone,
            Errno::NOTDIR => ListError::NotAFolder,
            other => ListError::Other(other.raw_os_error()),
        }
    }
}

/// Lists `dir`, sending a batch every `batch` entries; stops when `send` says no more are
/// wanted. Blocks: run it on a thread of its own.
pub fn list(dir: &Path, batch: usize, mut send: impl FnMut(Listed) -> bool) {
    let fd = match rustix::fs::open(
        dir,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            send(Listed::Failed(e.into()));
            return;
        }
    };
    let mut entries = match Dir::read_from(&fd) {
        Ok(entries) => entries,
        Err(e) => {
            send(Listed::Failed(e.into()));
            return;
        }
    };
    let mut pending = Vec::with_capacity(batch);
    while let Some(next) = entries.next() {
        let dirent = match next {
            Ok(d) => d,
            Err(e) => {
                send(Listed::Failed(e.into()));
                return;
            }
        };
        let name = dirent.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        let name = std::ffi::OsStr::from_bytes(name);
        // An entry removed between the read and its stat is simply not there.
        if let Ok(entry) = Entry::read(&fd, dir, name) {
            pending.push(entry);
        }
        if pending.len() == batch
            && !send(Listed::Batch(std::mem::replace(&mut pending, Vec::with_capacity(batch))))
        {
            return;
        }
    }
    if pending.is_empty() || send(Listed::Batch(pending)) {
        send(Listed::Done);
    }
}

/// Lists `dir` on a thread of its own, so a folder on a stalled mount costs that thread and
/// never the caller; dropping the receiver stops it at the next batch.
pub fn spawn(dir: std::path::PathBuf, batch: usize) -> std::sync::mpsc::Receiver<Listed> {
    let (tx, rx) = std::sync::mpsc::channel();
    let started = std::thread::Builder::new()
        .name("isle-fs-list".into())
        .spawn(move || list(&dir, batch, |listed| tx.send(listed).is_ok()));
    if started.is_err() {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Listed::Failed(ListError::Other(rustix::io::Errno::AGAIN.raw_os_error())));
        return rx;
    }
    rx
}
