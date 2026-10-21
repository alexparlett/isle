//! File jobs: copy, move, trash, put back, delete, duplicate, new folders and files, rename,
//! compress and expand (`apps.md` Files 4–9; `p15-files.md` choices 16–20).
//!
//! Nothing is removed before what replaces it is in place: a copy is written to a part
//! file beside its destination and renamed in with `RENAME_NOREPLACE`, or exchanged with
//! what it replaces and the old then dropped. A job asks about a name already taken through
//! its control, which the job's thread waits on; failures are gathered and the rest goes
//! on; every step records how to undo it as it goes, so a stopped or failed job can be
//! undone, unless it replaced something.

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use rustix::fs::{CWD, RenameFlags, renameat_with};

use crate::{
    copy,
    trash::{self, Trashed},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Work {
    Copy { items: Vec<PathBuf>, into: PathBuf },
    Move { items: Vec<PathBuf>, into: PathBuf },
    Duplicate { items: Vec<PathBuf> },
    Trash { items: Vec<PathBuf> },
    PutBack { items: Vec<Trashed> },
    /// Erases for good.
    Delete { items: Vec<PathBuf> },
    EmptyTrash { items: Vec<Trashed> },
    /// A new folder, the items moved into it (New Folder with Items).
    NewFolder { into: PathBuf, name: String, with: Vec<PathBuf> },
    NewFile { into: PathBuf, name: String, template: Option<PathBuf> },
    Rename { item: PathBuf, to: String },
    /// `#` in the pattern is each item's number from `start`.
    RenameMany { items: Vec<PathBuf>, pattern: String, start: u64 },
    Compress { items: Vec<PathBuf> },
    Expand { archive: PathBuf },
    Undo { steps: Vec<Step> },
}

/// How to take back one thing a job did.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// It made this: to the Trash with it.
    Made(PathBuf),
    /// It moved this here from there.
    Moved { now: PathBuf, was: PathBuf },
    /// It trashed this.
    Trashed(Trashed),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Undo {
    Steps(Vec<Step>),
    /// It replaced or erased something, which cannot be brought back.
    CantUndo,
}

/// A name already taken in the destination.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub name: String,
    pub folder: PathBuf,
    /// Both are folders, so Merge is offered.
    pub folders: bool,
    /// The one already there is the newer.
    pub existing_newer: Option<bool>,
    /// More items follow, so Apply to all is offered.
    pub more: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Stop,
    KeepBoth,
    Replace,
    Merge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answer {
    pub choice: Choice,
    /// For the rest of the job.
    pub all: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub bytes: u64,
    pub total_bytes: u64,
    pub items: u64,
    pub total_items: u64,
}

/// What a job hears from and tells its owner, from the job's own thread.
pub trait Control: Sync {
    fn progress(&self, progress: Progress);
    /// Waits for the user.
    fn ask(&self, conflict: Conflict) -> Answer;
    fn stopped(&self) -> bool;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// What the job made or moved, where it is now.
    pub made: Vec<PathBuf>,
    /// Each item that failed and why.
    pub failed: Vec<(PathBuf, String)>,
    pub undo: Option<Undo>,
    pub stopped: bool,
}

/// The job's running state.
struct Run<'a> {
    control: &'a dyn Control,
    progress: Progress,
    answer_all: Option<Choice>,
    steps: Vec<Step>,
    cant_undo: bool,
    report: Report,
}

impl Run<'_> {
    fn bytes(&mut self, n: u64) -> bool {
        self.progress.bytes += n;
        self.control.progress(self.progress);
        !self.control.stopped()
    }

    fn item_done(&mut self) {
        self.progress.items += 1;
        self.control.progress(self.progress);
    }

    fn fail(&mut self, item: &Path, why: impl ToString) {
        self.report.failed.push((item.to_owned(), why.to_string()));
    }

    fn ask(&mut self, conflict: Conflict) -> Choice {
        if let Some(all) = self.answer_all {
            if all != Choice::Merge || conflict.folders {
                return all;
            }
        }
        let answer = self.control.ask(conflict);
        if answer.all {
            self.answer_all = Some(answer.choice);
        }
        answer.choice
    }
}

/// Why an error happened, in the copy's words.
fn why(e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::PermissionDenied => "You don't have permission.".into(),
        io::ErrorKind::NotFound => "It was moved or deleted.".into(),
        io::ErrorKind::StorageFull => "There isn't enough space.".into(),
        io::ErrorKind::ReadOnlyFilesystem => "The volume is read only.".into(),
        _ => e.to_string(),
    }
}

/// A name in `dir` like `name` that is free: "name 2.ext", "name 3.ext"; with `copy`,
/// "name copy.ext", "name copy 2.ext" (Finder's Duplicate).
pub fn free_name(dir: &Path, name: &str, copy: bool) -> PathBuf {
    let (stem, ext) = split_ext(name);
    let candidates = (1..).map(|n| match (copy, n) {
        (false, 1) => name.to_owned(),
        (false, n) => format!("{stem} {n}{ext}"),
        (true, 1) => format!("{stem} copy{ext}"),
        (true, n) => format!("{stem} copy {n}{ext}"),
    });
    candidates.map(|c| dir.join(c)).find(|p| p.symlink_metadata().is_err()).unwrap()
}

/// A name's stem and extension with its dot, a leading dot and `.tar.gz` kept whole.
pub fn split_ext(name: &str) -> (&str, &str) {
    for double in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if let Some(stem) = name.strip_suffix(double).filter(|s| !s.is_empty()) {
            return (stem, &name[stem.len()..]);
        }
    }
    match name.rfind('.') {
        Some(0) | None => (name, ""),
        Some(i) => (&name[..i], &name[i..]),
    }
}

fn part_for(dest: &Path) -> PathBuf {
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    dest.with_file_name(format!(".{name}.isle-part-{}", std::process::id()))
}

fn rename(from: &Path, to: &Path, flags: RenameFlags) -> io::Result<()> {
    renameat_with(CWD, from, CWD, to, flags).map_err(io::Error::from)
}

/// Copies a file, folder or link to the new path `to`, without progress.
pub fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    copy_with(from, to, &mut |_| true)
}

fn copy_with(from: &Path, to: &Path, step: &mut dyn FnMut(u64) -> bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(from)?, to)?;
        return Ok(());
    }
    if meta.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_with(&entry.path(), &to.join(entry.file_name()), step)?;
        }
        fs::set_permissions(to, meta.permissions())?;
        copy::keep_attributes(from, to, &meta)?;
        return Ok(());
    }
    if !meta.is_file() {
        return Err(io::Error::other("Only files, folders and links are copied."));
    }
    copy::file(from, to, step)
}

/// Copies `src` into `into` as `dest`, by a part file renamed in; `replace` exchanges it with
/// what is there and drops the old.
fn place_copy(run: &mut Run, src: &Path, dest: &Path, replace: bool) -> io::Result<()> {
    let part = part_for(dest);
    let result = {
        let mut step = |n| run.bytes(n);
        copy_with(src, &part, &mut step)
    };
    if let Err(e) = result {
        let _ = trash::remove_tree(&part);
        return Err(e);
    }
    if replace {
        rename(&part, dest, RenameFlags::EXCHANGE)?;
        // The part's name now holds what was replaced.
        trash::remove_tree(&part)?;
    } else if let Err(e) = rename(&part, dest, RenameFlags::NOREPLACE) {
        let _ = trash::remove_tree(&part);
        return Err(e);
    }
    Ok(())
}

/// Resolves where an item goes when its name is taken: `None` stops the job or skips it.
enum Target {
    New(PathBuf),
    Replace(PathBuf),
    Merge(PathBuf),
    Stop,
}

fn target(run: &mut Run, src: &Path, into: &Path, more: bool) -> Target {
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dest = into.join(&name);
    let Ok(there) = fs::symlink_metadata(&dest) else {
        return Target::New(dest);
    };
    let here = fs::symlink_metadata(src).ok();
    let folders = there.is_dir() && here.as_ref().is_some_and(|m| m.is_dir());
    let existing_newer = here.and_then(|h| Some(there.modified().ok()? > h.modified().ok()?));
    let choice = run.ask(Conflict {
        name,
        folder: into.to_owned(),
        folders,
        existing_newer,
        more,
    });
    match choice {
        Choice::Stop => Target::Stop,
        Choice::KeepBoth => Target::New(free_name(into, &dest.file_name().unwrap().to_string_lossy(), false)),
        Choice::Replace => Target::Replace(dest),
        Choice::Merge if folders => Target::Merge(dest),
        Choice::Merge => Target::New(free_name(into, &dest.file_name().unwrap().to_string_lossy(), false)),
    }
}

fn copy_items(run: &mut Run, items: &[PathBuf], into: &Path) {
    for (n, src) in items.iter().enumerate() {
        if run.control.stopped() || run.report.stopped {
            run.report.stopped = true;
            return;
        }
        if into.starts_with(src) && into != src.parent().unwrap_or(src) {
            run.fail(src, "A folder can't be copied into itself.");
            continue;
        }
        let more = n + 1 < items.len();
        let done = match target(run, src, into, more) {
            Target::Stop => {
                run.report.stopped = true;
                return;
            }
            Target::New(dest) => place_copy(run, src, &dest, false).map(|_| {
                run.steps.push(Step::Made(dest.clone()));
                dest
            }),
            Target::Replace(dest) => place_copy(run, src, &dest, true).map(|_| {
                run.cant_undo = true;
                dest
            }),
            Target::Merge(dest) => {
                let children: Vec<PathBuf> = fs::read_dir(src)
                    .map(|r| r.flatten().map(|e| e.path()).collect())
                    .unwrap_or_default();
                copy_items(run, &children, &dest);
                Ok(dest)
            }
        };
        match done {
            Ok(dest) => run.report.made.push(dest),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                run.report.stopped = true;
                return;
            }
            Err(e) => run.fail(src, why(&e)),
        }
        run.item_done();
    }
}

fn move_items(run: &mut Run, items: &[PathBuf], into: &Path) {
    for (n, src) in items.iter().enumerate() {
        if run.control.stopped() || run.report.stopped {
            run.report.stopped = true;
            return;
        }
        if into.starts_with(src) {
            run.fail(src, "A folder can't be moved into itself.");
            continue;
        }
        if src.parent() == Some(into) {
            continue;
        }
        let more = n + 1 < items.len();
        let done = match target(run, src, into, more) {
            Target::Stop => {
                run.report.stopped = true;
                return;
            }
            Target::New(dest) => match rename(src, &dest, RenameFlags::NOREPLACE) {
                Ok(()) => Ok(dest),
                Err(e) if e.raw_os_error() == Some(rustix::io::Errno::XDEV.raw_os_error()) => {
                    // Another filesystem: copied in whole, then the source removed.
                    place_copy(run, src, &dest, false).and_then(|_| trash::remove_tree(src)).map(|_| dest)
                }
                Err(e) => Err(e),
            }
            .map(|dest| {
                run.steps.push(Step::Moved { now: dest.clone(), was: src.clone() });
                dest
            }),
            Target::Replace(dest) => match rename(src, &dest, RenameFlags::EXCHANGE) {
                Ok(()) => {
                    // The source's name now holds what was replaced.
                    run.cant_undo = true;
                    trash::remove_tree(src).map(|_| dest)
                }
                Err(_) => place_copy(run, src, &dest, true).and_then(|_| trash::remove_tree(src)).map(|_| {
                    run.cant_undo = true;
                    dest
                }),
            },
            Target::Merge(dest) => {
                let children: Vec<PathBuf> = fs::read_dir(src)
                    .map(|r| r.flatten().map(|e| e.path()).collect())
                    .unwrap_or_default();
                move_items(run, &children, &dest);
                if fs::read_dir(src).is_ok_and(|mut r| r.next().is_none()) {
                    let _ = fs::remove_dir(src);
                }
                Ok(dest)
            }
        };
        match done {
            Ok(dest) => run.report.made.push(dest),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                run.report.stopped = true;
                return;
            }
            Err(e) => run.fail(src, why(&e)),
        }
        run.item_done();
    }
}

fn bsdtar(args: &[&std::ffi::OsStr]) -> io::Result<()> {
    let status = Command::new("bsdtar").args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("The archive couldn't be read or written."))
    }
}

/// What a job will need, and what its destination has, when it will not fit.
pub fn short_of_space(work: &Work) -> Option<(u64, u64, PathBuf)> {
    let (items, into) = match work {
        Work::Copy { items, into } => (items, into),
        Work::Move { items, into } => {
            let same = items.first().and_then(|i| fs::symlink_metadata(i.parent()?).ok()).map(|m| {
                use std::os::unix::fs::MetadataExt;
                m.dev()
            }) == fs::metadata(into).ok().map(|m| {
                use std::os::unix::fs::MetadataExt;
                m.dev()
            });
            if same {
                return None;
            }
            (items, into)
        }
        _ => return None,
    };
    let needed: u64 = items.iter().map(|i| copy::measure(i).0).sum();
    let stat = rustix::fs::statvfs(into).ok()?;
    let free = stat.f_bavail * stat.f_frsize;
    (needed > free).then(|| (needed, free, into.clone()))
}

/// Runs a job to its end on the caller's thread.
pub fn run(work: Work, control: &dyn Control) -> Report {
    let mut run = Run {
        control,
        progress: Progress::default(),
        answer_all: None,
        steps: Vec::new(),
        cant_undo: false,
        report: Report::default(),
    };
    let measure = |items: &[PathBuf]| -> (u64, u64) {
        items.iter().map(|i| copy::measure(i)).fold((0, 0), |(b, n), (b2, n2)| (b + b2, n + n2))
    };
    match work {
        Work::Copy { items, into } => {
            (run.progress.total_bytes, run.progress.total_items) = measure(&items);
            copy_items(&mut run, &items, &into);
        }
        Work::Move { items, into } => {
            run.progress.total_items = items.len() as u64;
            move_items(&mut run, &items, &into);
        }
        Work::Duplicate { items } => {
            (run.progress.total_bytes, run.progress.total_items) = measure(&items);
            for src in &items {
                let Some(dir) = src.parent() else { continue };
                let name = src.file_name().unwrap().to_string_lossy().into_owned();
                let dest = free_name(dir, &name, true);
                match place_copy(&mut run, src, &dest, false) {
                    Ok(()) => {
                        run.steps.push(Step::Made(dest.clone()));
                        run.report.made.push(dest);
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                        run.report.stopped = true;
                        break;
                    }
                    Err(e) => run.fail(src, why(&e)),
                }
            }
        }
        Work::Trash { items } => {
            run.progress.total_items = items.len() as u64;
            for item in &items {
                if run.control.stopped() {
                    run.report.stopped = true;
                    break;
                }
                match trash::trash(item) {
                    Ok(t) => {
                        run.report.made.push(t.file.clone());
                        run.steps.push(Step::Trashed(t));
                    }
                    Err(e) => run.fail(item, why(&e)),
                }
                run.item_done();
            }
        }
        Work::PutBack { items } => {
            run.progress.total_items = items.len() as u64;
            for item in &items {
                match trash::put_back(item) {
                    Ok(back) => {
                        run.report.made.push(back.clone());
                        run.steps.push(Step::Moved { now: back, was: item.file.clone() });
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        run.fail(&item.original, "Something with its name is there now.")
                    }
                    Err(e) => run.fail(&item.original, why(&e)),
                }
                run.item_done();
            }
        }
        Work::Delete { items } => {
            run.cant_undo = true;
            for item in &items {
                if let Err(e) = trash::remove_tree(item) {
                    run.fail(item, why(&e));
                }
                run.item_done();
            }
        }
        Work::EmptyTrash { items } => {
            run.cant_undo = true;
            run.progress.total_items = items.len() as u64;
            for item in &items {
                if let Err(e) = trash::erase(item) {
                    run.fail(&item.file, why(&e));
                }
                run.item_done();
            }
        }
        Work::NewFolder { into, name, with } => {
            let dest = free_name(&into, &name, false);
            match fs::create_dir(&dest) {
                Ok(()) => {
                    run.steps.push(Step::Made(dest.clone()));
                    run.report.made.push(dest.clone());
                    if !with.is_empty() {
                        move_items(&mut run, &with, &dest);
                        run.report.made.retain(|p| *p == dest);
                    }
                }
                Err(e) => run.fail(&dest, why(&e)),
            }
        }
        Work::NewFile { into, name, template } => {
            let dest = free_name(&into, &name, false);
            let made = match template {
                Some(t) => copy_tree(&t, &dest),
                None => fs::OpenOptions::new().write(true).create_new(true).open(&dest).map(|_| ()),
            };
            match made {
                Ok(()) => {
                    run.steps.push(Step::Made(dest.clone()));
                    run.report.made.push(dest);
                }
                Err(e) => run.fail(&dest, why(&e)),
            }
        }
        Work::Rename { item, to } => {
            let dest = item.with_file_name(&to);
            match rename(&item, &dest, RenameFlags::NOREPLACE) {
                Ok(()) => {
                    run.steps.push(Step::Moved { now: dest.clone(), was: item.clone() });
                    run.report.made.push(dest);
                }
                Err(e) => run.fail(&item, why(&e)),
            }
        }
        Work::RenameMany { items, pattern, start } => {
            let pattern = if pattern.contains('#') { pattern } else { format!("{pattern} #") };
            for (n, item) in items.iter().enumerate() {
                let name = item.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let ext = if item.is_dir() { "" } else { split_ext(&name).1 };
                let to = format!("{}{ext}", pattern.replace('#', &(start + n as u64).to_string()));
                let dest = item.with_file_name(to);
                match rename(item, &dest, RenameFlags::NOREPLACE) {
                    Ok(()) => {
                        run.steps.push(Step::Moved { now: dest.clone(), was: item.clone() });
                        run.report.made.push(dest);
                    }
                    Err(e) => run.fail(item, why(&e)),
                }
                run.item_done();
            }
        }
        Work::Compress { items } => {
            let Some(dir) = items.first().and_then(|i| i.parent()).map(Path::to_owned) else {
                return run.report;
            };
            let name = match items.as_slice() {
                [one] => format!("{}.zip", one.file_name().unwrap().to_string_lossy()),
                _ => "Archive.zip".into(),
            };
            let dest = free_name(&dir, &name, false);
            let part = part_for(&dest);
            let mut args: Vec<&std::ffi::OsStr> = vec!["-a".as_ref(), "-cf".as_ref(), part.as_os_str(), "-C".as_ref(), dir.as_os_str()];
            let names: Vec<_> = items.iter().filter_map(|i| i.file_name()).collect();
            args.extend(names.iter().copied());
            match bsdtar(&args).and_then(|_| rename(&part, &dest, RenameFlags::NOREPLACE)) {
                Ok(()) => {
                    run.steps.push(Step::Made(dest.clone()));
                    run.report.made.push(dest);
                }
                Err(e) => {
                    let _ = fs::remove_file(&part);
                    run.fail(&dest, why(&e));
                }
            }
        }
        Work::Expand { archive } => {
            let Some(dir) = archive.parent() else { return run.report };
            let name = archive.file_name().unwrap().to_string_lossy().into_owned();
            let dest = free_name(dir, split_ext(&name).0, false);
            let part = part_for(&dest);
            let made = fs::create_dir(&part)
                .and_then(|_| bsdtar(&["-xf".as_ref(), archive.as_os_str(), "-C".as_ref(), part.as_os_str()]))
                .and_then(|_| rename(&part, &dest, RenameFlags::NOREPLACE));
            match made {
                Ok(()) => {
                    run.steps.push(Step::Made(dest.clone()));
                    run.report.made.push(dest);
                }
                Err(e) => {
                    let _ = trash::remove_tree(&part);
                    run.fail(&archive, why(&e));
                }
            }
        }
        Work::Undo { steps } => {
            for step in steps.into_iter().rev() {
                let done = match &step {
                    Step::Made(path) => trash::trash(path).map(|_| ()),
                    // A merge removes the folder it emptied; undoing it makes the folder again.
                    Step::Moved { now, was } => was
                        .parent()
                        .map_or(Ok(()), fs::create_dir_all)
                        .and_then(|_| rename(now, was, RenameFlags::NOREPLACE)),
                    Step::Trashed(t) => trash::put_back(t).map(|_| ()),
                };
                if let Err(e) = done {
                    let item = match &step {
                        Step::Made(p) | Step::Moved { now: p, .. } => p.clone(),
                        Step::Trashed(t) => t.original.clone(),
                    };
                    run.fail(&item, why(&e));
                }
            }
            return run.report;
        }
    }
    run.report.undo = Some(if run.cant_undo { Undo::CantUndo } else { Undo::Steps(run.steps) });
    run.report
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use super::*;

    /// Answers every question the same way, and can stop after a number of progress reports.
    struct Fixed {
        answer: Answer,
        asked: Mutex<Vec<Conflict>>,
        stop_after: Option<usize>,
        reports: AtomicUsize,
        stopped: AtomicBool,
    }

    impl Fixed {
        fn new(choice: Choice) -> Self {
            Self {
                answer: Answer { choice, all: false },
                asked: Mutex::default(),
                stop_after: None,
                reports: AtomicUsize::new(0),
                stopped: AtomicBool::new(false),
            }
        }
    }

    impl Control for Fixed {
        fn progress(&self, _: Progress) {
            let n = self.reports.fetch_add(1, Ordering::SeqCst) + 1;
            if self.stop_after.is_some_and(|s| n >= s) {
                self.stopped.store(true, Ordering::SeqCst);
            }
        }
        fn ask(&self, conflict: Conflict) -> Answer {
            self.asked.lock().unwrap().push(conflict);
            self.answer
        }
        fn stopped(&self) -> bool {
            self.stopped.load(Ordering::SeqCst)
        }
    }

    fn tree(base: &Path) -> (PathBuf, PathBuf) {
        let src = base.join("src");
        let dst = base.join("dst");
        fs::create_dir_all(src.join("folder/inner")).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("a.txt"), "a").unwrap();
        fs::write(src.join("folder/inner/b.txt"), "b").unwrap();
        (src, dst)
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut n: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        n.sort();
        n
    }

    #[test]
    fn a_copy_lands_whole_and_undoes_to_the_trash() {
        let base = tempfile::tempdir().unwrap();
        // The home trash, on the same filesystem as the test's folder.
        fs::create_dir_all(base.path().join("data")).unwrap();
        unsafe { std::env::set_var("XDG_DATA_HOME", base.path().join("data")) };
        let (src, dst) = tree(base.path());
        let report = run(
            Work::Copy { items: vec![src.join("a.txt"), src.join("folder")], into: dst.clone() },
            &Fixed::new(Choice::Stop),
        );
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(names(&dst), ["a.txt", "folder"]);
        assert_eq!(fs::read_to_string(dst.join("folder/inner/b.txt")).unwrap(), "b");
        let Some(Undo::Steps(steps)) = report.undo else { panic!() };
        let undone = run(Work::Undo { steps }, &Fixed::new(Choice::Stop));
        assert!(undone.failed.is_empty(), "{:?}", undone.failed);
        assert!(names(&dst).is_empty());
    }

    #[test]
    fn a_taken_name_asks_and_keep_both_numbers() {
        let base = tempfile::tempdir().unwrap();
        let (src, dst) = tree(base.path());
        fs::write(dst.join("a.txt"), "old").unwrap();
        let control = Fixed::new(Choice::KeepBoth);
        run(Work::Copy { items: vec![src.join("a.txt")], into: dst.clone() }, &control);
        assert_eq!(names(&dst), ["a 2.txt", "a.txt"]);
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "old");
        assert_eq!(control.asked.lock().unwrap()[0].name, "a.txt");
    }

    #[test]
    fn replace_swaps_in_whole_and_cannot_be_undone() {
        let base = tempfile::tempdir().unwrap();
        let (src, dst) = tree(base.path());
        fs::write(dst.join("a.txt"), "old").unwrap();
        let report = run(Work::Copy { items: vec![src.join("a.txt")], into: dst.clone() }, &Fixed::new(Choice::Replace));
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "a");
        assert_eq!(names(&dst), ["a.txt"], "no part file left");
        assert_eq!(report.undo, Some(Undo::CantUndo));
    }

    #[test]
    fn a_stopped_copy_leaves_nothing_half_written() {
        let base = tempfile::tempdir().unwrap();
        let (src, dst) = tree(base.path());
        fs::write(src.join("big"), vec![1u8; 20 * 1024 * 1024]).unwrap();
        let mut control = Fixed::new(Choice::Stop);
        control.stop_after = Some(1);
        let report = run(Work::Copy { items: vec![src.join("big"), src.join("a.txt")], into: dst.clone() }, &control);
        assert!(report.stopped);
        assert!(names(&dst).iter().all(|n| !n.contains("isle-part")), "{:?}", names(&dst));
    }

    #[test]
    fn a_move_merges_folders_and_undoes_back() {
        let base = tempfile::tempdir().unwrap();
        let (src, dst) = tree(base.path());
        fs::create_dir_all(dst.join("folder")).unwrap();
        fs::write(dst.join("folder/kept.txt"), "k").unwrap();
        let report = run(Work::Move { items: vec![src.join("folder"), src.join("a.txt")], into: dst.clone() }, &Fixed::new(Choice::Merge));
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(names(&dst.join("folder")), ["inner", "kept.txt"]);
        assert!(!src.join("folder").exists(), "an emptied folder goes");
        let Some(Undo::Steps(steps)) = report.undo else { panic!() };
        run(Work::Undo { steps }, &Fixed::new(Choice::Stop));
        assert!(src.join("a.txt").exists());
        assert!(src.join("folder/inner/b.txt").exists());
    }

    #[test]
    fn nothing_goes_into_itself() {
        let base = tempfile::tempdir().unwrap();
        let (src, _) = tree(base.path());
        let report = run(Work::Move { items: vec![src.join("folder")], into: src.join("folder/inner") }, &Fixed::new(Choice::Stop));
        assert_eq!(report.failed.len(), 1);
        assert!(src.join("folder/inner").exists());
    }

    #[test]
    fn names_are_made_free_and_split_as_finder_does() {
        let base = tempfile::tempdir().unwrap();
        fs::write(base.path().join("a.txt"), "").unwrap();
        fs::write(base.path().join("a copy.txt"), "").unwrap();
        assert_eq!(free_name(base.path(), "a.txt", false), base.path().join("a 2.txt"));
        assert_eq!(free_name(base.path(), "a.txt", true), base.path().join("a copy 2.txt"));
        assert_eq!(split_ext("x.tar.gz"), ("x", ".tar.gz"));
        assert_eq!(split_ext(".bashrc"), (".bashrc", ""));
    }

    #[test]
    fn rename_many_numbers_and_keeps_extensions() {
        let base = tempfile::tempdir().unwrap();
        for n in ["b.jpg", "a.jpg"] {
            fs::write(base.path().join(n), "").unwrap();
        }
        let items = vec![base.path().join("a.jpg"), base.path().join("b.jpg")];
        run(Work::RenameMany { items, pattern: "Holiday".into(), start: 1 }, &Fixed::new(Choice::Stop));
        assert_eq!(names(base.path()), ["Holiday 1.jpg", "Holiday 2.jpg"]);
    }

    #[test]
    fn compress_and_expand_beside() {
        if Command::new("bsdtar").arg("--version").output().is_err() {
            return;
        }
        let base = tempfile::tempdir().unwrap();
        let (src, _) = tree(base.path());
        let made = run(Work::Compress { items: vec![src.join("folder")] }, &Fixed::new(Choice::Stop));
        let archive = made.made[0].clone();
        assert_eq!(archive.file_name().unwrap(), "folder.zip");
        let out = run(Work::Expand { archive }, &Fixed::new(Choice::Stop));
        assert!(out.failed.is_empty(), "{:?}", out.failed);
        assert!(out.made[0].join("folder/inner/b.txt").exists(), "{:?}", out.made);
    }
}
