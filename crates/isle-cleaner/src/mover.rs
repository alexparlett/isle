//! Moving the basket (`p19b-cleaner.md` choices 16 and 17): each item checked again, then one
//! job through `isle-fs`, its progress sent back, its steps kept for one undo.

use std::{os::unix::fs::MetadataExt, path::PathBuf};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use isle_fs::jobs::{self, Answer, Choice, Conflict, Control, Progress, Report, Work};

use crate::basket::Item;

/// Why an item may not go now, if it may not: it has gone, it is no longer what the scan saw,
/// or it has become a link.
pub fn refused(item: &Item) -> Option<&'static str> {
    let Ok(m) = std::fs::symlink_metadata(&item.path) else {
        return Some("It's no longer there.");
    };
    if m.file_type().is_symlink() {
        return Some("It's now a link.");
    }
    if (item.dev, item.ino) != (0, 0) && (m.dev(), m.ino()) != (item.dev, item.ino) {
        return Some("It changed since the scan.");
    }
    None
}

/// The items that may go, and those that may not with why.
pub fn check(items: &[Item]) -> (Vec<PathBuf>, Vec<(PathBuf, String)>) {
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    for i in items {
        match refused(i) {
            None => ok.push(i.path.clone()),
            Some(why) => failed.push((i.path.clone(), why.to_owned())),
        }
    }
    (ok, failed)
}

pub enum Moving {
    /// A share done, 0 to 1.
    Progress(f64),
    Done(Report),
}

struct Sender(UnboundedSender<Moving>);

impl Control for Sender {
    fn progress(&self, p: Progress) {
        let share = if p.total_items > 0 {
            p.items as f64 / p.total_items as f64
        } else {
            0.
        };
        let _ = self.0.unbounded_send(Moving::Progress(share));
    }

    // Moving to the Trash names each item afresh, so nothing conflicts.
    fn ask(&self, _: Conflict) -> Answer {
        Answer {
            choice: Choice::KeepBoth,
            all: true,
        }
    }

    fn stopped(&self) -> bool {
        false
    }
}

/// Runs `work` on a thread of its own.
pub fn run(work: Work) -> UnboundedReceiver<Moving> {
    let (tx, rx) = unbounded();
    std::thread::Builder::new()
        .name("cleaner-move".into())
        .spawn(move || {
            let control = Sender(tx);
            let report = jobs::run(work, &control);
            let _ = control.0.unbounded_send(Moving::Done(report));
        })
        .expect("the move's thread");
    rx
}

/// What the basket frees on one item: its bytes less what other files share, read for files
/// of a megabyte or more (choice 15); a folder is taken from the scan as it is.
pub fn frees(item: &Item) -> (u64, bool) {
    let is_file = std::fs::symlink_metadata(&item.path).is_ok_and(|m| m.is_file());
    if !is_file || item.bytes < isle_scan::walk::CANDIDATE_BYTES {
        return (item.bytes, false);
    }
    match isle_scan::duplicates::sharing(&item.path) {
        Some(s) if s.shared > 0 => (item.bytes.saturating_sub(s.shared), true),
        _ => (item.bytes, false),
    }
}
