//! The run: paru's and pacman's lines read into a progress record, rewritten whole on every
//! change so a reader never sees half of one (`p14d-updates.md` choices 12 and 13).

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::ISLE_PACKAGE;

const KEEP_LINES: usize = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    #[default]
    Running,
    Done,
    Failed,
    Stopped,
    /// The password was not given; nothing changed.
    Refused,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub state: RunState,
    pub phase: String,
    pub done: u32,
    pub total: u32,
    pub lines: Vec<String>,
    /// Seconds since the epoch.
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub code: Option<i32>,
    pub reason: Option<String>,
    pub pid: u32,
}

/// Reads the run's lines into its progress.
#[derive(Default)]
pub struct Reader {
    pub progress: Progress,
    announced: BTreeSet<String>,
    finished: BTreeSet<String>,
    /// Under pacman's VerbosePkgLists the packages come as a table, after a blank line and up to
    /// the next.
    in_table: bool,
    table_rows: bool,
    building: Option<String>,
    last_error: Option<String>,
    known: Option<String>,
    refused: bool,
}

fn display(package: &str) -> &str {
    if package == ISLE_PACKAGE {
        "Isle"
    } else {
        package
    }
}

/// `name-pkgver-pkgrel`, or paru's `name-latest-commit` for a devel package, to `name`.
fn bare(entry: &str, fields: usize) -> &str {
    if let Some(name) = entry.strip_suffix("-latest-commit") {
        return name;
    }
    let mut parts = entry.rsplitn(fields + 1, '-');
    let rest: Vec<&str> = parts.by_ref().collect();
    if rest.len() == fields + 1 {
        &entry[..rest[fields].len()]
    } else {
        entry
    }
}

fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else if c != '\r' {
            out.push(c);
        }
    }
    out.trim_end().to_string()
}

const LISTS: &[&str] = &[
    "Packages",
    "Repo",
    "Repo Make",
    "Aur",
    "Aur Make",
    "Pkgbuilds",
    "Pkgbuilds Make",
];

impl Reader {
    pub fn new(started_at: u64, pid: u32) -> Self {
        Reader {
            progress: Progress {
                phase: "Starting".into(),
                started_at,
                pid,
                ..Progress::default()
            },
            ..Reader::default()
        }
    }

    fn count(&mut self) {
        self.progress.total = self.announced.len() as u32;
        self.progress.done = self.finished.intersection(&self.announced).count() as u32;
    }

    pub fn line(&mut self, raw: &str) {
        let line = strip_ansi(raw);
        if line.is_empty() {
            if self.table_rows {
                self.in_table = false;
            }
            return;
        }
        self.progress.lines.push(line.clone());
        if self.progress.lines.len() > KEEP_LINES {
            let over = self.progress.lines.len() - KEEP_LINES;
            self.progress.lines.drain(..over);
        }
        let l = line.trim_start();

        if self.in_table {
            if let Some(first) = l.split_whitespace().next() {
                let name = first.rsplit('/').next().unwrap_or(first);
                self.table_rows = true;
                self.announced.insert(name.to_string());
                self.count();
            }
            return;
        }
        if let Some((head, rest)) = l
            .split_once(") ")
            .or_else(|| l.ends_with(')').then_some((l.trim_end_matches(')'), "")))
            && let Some((kind, n)) = head.rsplit_once(" (")
            && n.parse::<u32>().is_ok()
        {
            if kind == "Package" && rest.contains("Old Version") {
                self.in_table = true;
                self.table_rows = false;
                return;
            }
            if LISTS.contains(&kind) {
                for entry in rest.split_whitespace() {
                    self.announced.insert(bare(entry, 2).to_string());
                }
                self.count();
                return;
            }
        }
        for verb in [
            "upgrading",
            "installing",
            "reinstalling",
            "downgrading",
            "removing",
        ] {
            if let Some(name) = l
                .strip_prefix(verb)
                .and_then(|r| r.strip_prefix(' '))
                .and_then(|r| r.strip_suffix("..."))
            {
                self.announced.insert(name.to_string());
                self.finished.insert(name.to_string());
                self.count();
                let mut word = verb.to_string();
                word[..1].make_ascii_uppercase();
                self.progress.phase = format!("{word} {}", display(name));
                return;
            }
        }
        if let Some(file) = l.strip_suffix(" downloading...") {
            self.progress.phase = format!("Downloading {}", display(bare(file, 3)));
        } else if let Some(rest) = l.strip_prefix("==> Making package: ") {
            let name = rest.split_whitespace().next().unwrap_or(rest).to_string();
            self.progress.phase = format!("Building {}", display(&name));
            self.building = Some(name);
        } else if l.starts_with(":: Synchronizing package databases")
            || l.starts_with(":: Downloading PKGBUILD")
        {
            self.progress.phase = "Updating package lists".into();
        } else if l.starts_with(":: Starting full system upgrade")
            || l.starts_with(":: Looking for")
        {
            self.progress.phase = "Looking for updates".into();
        } else if l.starts_with(":: Retrieving packages") {
            self.progress.phase = "Downloading".into();
        } else if l == "checking package integrity..." || l == "checking keyring..." {
            self.progress.phase = "Checking packages".into();
        } else if l.starts_with(":: Running post-transaction hooks") {
            self.progress.phase = "Finishing".into();
        }

        if l.starts_with("Error executing command as another user") || l.contains("Not authorized")
        {
            self.refused = true;
        }
        let known = if l.contains("unable to lock database") {
            Some(
                "Another program is installing packages. Try again when it has finished."
                    .to_string(),
            )
        } else if l.contains("Could not resolve host")
            || l.contains("Couldn't resolve host")
            || l.contains("Resolving timed out")
        {
            Some("Couldn't reach the package servers. Check your network connection.".into())
        } else if l.contains("not enough free disk space") || l.contains("too full") {
            Some("There isn't enough free disk space.".into())
        } else if l.contains("conflicting files") || l.ends_with("exists in filesystem") {
            Some("A package's files conflict with files already on the computer.".into())
        } else if l.contains("could not satisfy dependencies")
            || l.contains("unable to satisfy dependency")
        {
            Some("A package needs something that can't be installed.".into())
        } else if l.starts_with("==> ERROR: A failure occurred in") {
            Some(format!(
                "{} failed to build.",
                self.building.as_deref().map_or("A package", display)
            ))
        } else {
            None
        };
        if known.is_some() && self.known.is_none() {
            self.known = known;
        }
        if let Some(rest) = l.strip_prefix("error: ") {
            self.last_error = Some(rest.to_string());
        }
    }

    /// The run is over: `code` is its exit code, `signal` the signal that ended it, and
    /// `stopping` whether Stop asked for that.
    pub fn finish(&mut self, code: Option<i32>, signal: Option<i32>, stopping: bool, now: u64) {
        let p = &mut self.progress;
        p.code = code;
        p.finished_at = Some(now);
        p.reason = None;
        p.state = if stopping || signal.is_some() {
            RunState::Stopped
        } else if code == Some(0) {
            RunState::Done
        } else if self.refused || matches!(code, Some(126 | 127)) {
            p.reason = Some("Permission was not given.".into());
            RunState::Refused
        } else {
            let reason = self.known.clone().or_else(|| self.last_error.clone());
            p.reason = Some(
                reason.unwrap_or_else(|| format!("Stopped with code {}.", code.unwrap_or(-1))),
            );
            RunState::Failed
        };
        p.phase = match p.state {
            RunState::Done => "Done",
            RunState::Stopped => "Stopped",
            _ => "Stopped",
        }
        .into();
        if p.state == RunState::Done {
            p.done = p.total;
        }
    }
}

pub fn write_progress(path: &Path, progress: &Progress) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.new");
    std::fs::write(&tmp, serde_json::to_vec(progress)?)?;
    std::fs::rename(tmp, path)
}

pub fn read_progress(path: &Path) -> Option<Progress> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

static STOPPING: AtomicBool = AtomicBool::new(false);

extern "C" fn on_term(_: libc::c_int) {
    STOPPING.store(true, Ordering::SeqCst);
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Runs `argv`, with `envs` added, its output read into `path`, and returns the exit code for
/// the unit.
/// SIGTERM, which Stop sends the whole unit, is noted rather than obeyed: the child ends of it
/// and the record still gets written.
pub fn run(argv: &[OsString], envs: &[(OsString, OsString)], path: &Path) -> i32 {
    // A handler, not SIG_IGN, so the child starts with SIGTERM at its default.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_term as *const () as usize;
        libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut());
    }
    let mut reader = Reader::new(now(), std::process::id());
    let _ = write_progress(path, &reader.progress);

    let mut fds = [0; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        reader.line(&format!("error: {}", std::io::Error::last_os_error()));
        reader.finish(Some(1), None, false, now());
        let _ = write_progress(path, &reader.progress);
        return 1;
    }
    let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    let spawned = match write.try_clone() {
        Ok(write2) => Command::new(&argv[0])
            .args(&argv[1..])
            .env("LC_ALL", "C.UTF-8")
            .envs(envs.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::from(write))
            .stderr(Stdio::from(write2))
            .spawn(),
        Err(e) => Err(e),
    };
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            reader.line(&format!("error: {}: {e}", argv[0].to_string_lossy()));
            reader.finish(Some(1), None, false, now());
            let _ = write_progress(path, &reader.progress);
            return 1;
        }
    };
    // Lines end when every writer has closed: the child and anything it left running.
    for line in BufReader::new(std::fs::File::from(read))
        .split(b'\n')
        .map_while(Result::ok)
    {
        reader.line(&String::from_utf8_lossy(&line));
        let _ = write_progress(path, &reader.progress);
    }
    let status = child.wait();
    let (code, signal) = status
        .as_ref()
        .map_or((None, None), |s| (s.code(), s.signal()));
    reader.finish(code, signal, STOPPING.load(Ordering::SeqCst), now());
    let _ = write_progress(path, &reader.progress);
    match reader.progress.state {
        RunState::Done => 0,
        _ => code.filter(|c| *c != 0).unwrap_or(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");

    fn read(name: &str) -> Reader {
        let mut r = Reader::new(0, 1);
        let text = std::fs::read_to_string(format!("{DATA}/{name}")).unwrap();
        for line in text.lines().filter(|l| !l.starts_with("EXIT=")) {
            r.line(line);
        }
        r
    }

    #[test]
    fn a_full_run_counts_every_package_once() {
        let mut r = read("run-full-en_GB.log");
        // 304 from the repositories, 5 make dependencies, the probe and paru.
        assert_eq!(r.progress.total, 311);
        assert_eq!(r.progress.done, 311);
        assert_eq!(r.progress.phase, "Finishing");
        assert_eq!(r.progress.lines.len(), KEEP_LINES);
        // Mirror errors in a run that succeeds are not its reason.
        r.finish(Some(0), None, false, 9);
        assert_eq!(
            (r.progress.state, r.progress.reason.clone()),
            (RunState::Done, None)
        );
    }

    #[test]
    fn progress_part_way() {
        let mut r = Reader::new(0, 1);
        let text = std::fs::read_to_string(format!("{DATA}/run-full-en_GB.log")).unwrap();
        for line in text.lines().take_while(|l| *l != "upgrading glibc...") {
            r.line(line);
        }
        assert_eq!(r.progress.total, 304);
        assert_eq!(r.progress.done, 4);
        assert_eq!(r.progress.phase, "Upgrading tzdata");
    }

    #[test]
    fn building_isle_is_named() {
        let mut r = Reader::new(0, 1);
        r.line("Pkgbuilds (1) isle-git-latest-commit");
        r.line("==> Making package: isle-git r725.g0f8209c46951-1 (Thu 12 Nov 2026 20:03:54 GMT)");
        assert_eq!(
            (r.progress.phase.as_str(), r.progress.total, r.progress.done),
            ("Building Isle", 1, 0)
        );
        r.line("==> ERROR: A failure occurred in build().");
        r.line("    Aborting...");
        r.line("error: failed to build 'isle-git-r725.g0f8209c46951-1':");
        r.finish(Some(1), None, false, 1);
        assert_eq!(r.progress.reason.as_deref(), Some("Isle failed to build."));
    }

    #[test]
    fn verbose_package_lists() {
        let mut r = Reader::new(0, 1);
        for line in [
            "Package (2)        Old Version  New Version  Net Change  Download Size",
            "",
            "core/glibc         2.44-1       2.44-2         0.01 MiB       9.94 MiB",
            "extra/mesa         1:26.2.2-2   1:26.2.6-1     0.20 MiB      12.00 MiB",
            "",
            "Total Download Size:   21.94 MiB",
            "upgrading glibc...",
        ] {
            r.line(line);
        }
        assert_eq!((r.progress.total, r.progress.done), (2, 1));
    }

    #[test]
    fn a_held_lock() {
        let mut r = read("run-locked.log");
        r.finish(Some(1), None, false, 1);
        assert_eq!(r.progress.state, RunState::Failed);
        assert_eq!(
            r.progress.reason.as_deref(),
            Some("Another program is installing packages. Try again when it has finished.")
        );
    }

    #[test]
    fn a_refused_password_is_not_a_failure() {
        let mut r = read("run-refused.log");
        r.finish(Some(1), None, false, 1);
        assert_eq!(
            (r.progress.state, r.progress.reason.as_deref()),
            (RunState::Refused, Some("Permission was not given."))
        );
    }

    #[test]
    fn stop_and_unknown_failures() {
        let mut r = Reader::new(0, 1);
        r.line("error: something new went wrong");
        r.finish(Some(1), Some(15), true, 1);
        assert_eq!(r.progress.state, RunState::Stopped);
        let mut r = Reader::new(0, 1);
        r.line("error: something new went wrong");
        r.finish(Some(1), None, false, 1);
        assert_eq!(
            r.progress.reason.as_deref(),
            Some("something new went wrong")
        );
        let mut r = Reader::new(0, 1);
        r.finish(Some(3), None, false, 1);
        assert_eq!(r.progress.reason.as_deref(), Some("Stopped with code 3."));
    }

    #[test]
    fn the_file_is_whole() {
        let path =
            std::env::temp_dir().join(format!("isle-update-{}/update.json", std::process::id()));
        let mut p = Progress {
            phase: "Building Isle".into(),
            total: 3,
            ..Progress::default()
        };
        write_progress(&path, &p).unwrap();
        p.done = 2;
        write_progress(&path, &p).unwrap();
        assert_eq!(read_progress(&path), Some(p));
        assert!(!path.with_extension("json.new").exists());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_real_child_and_its_exit() {
        let dir = std::env::temp_dir().join(format!("isle-update-run-{}", std::process::id()));
        let path = dir.join("update.json");
        let argv: Vec<OsString> = [
            "sh",
            "-c",
            "echo 'Packages (1) a-1-1'; echo 'upgrading a...' >&2; exit 0",
        ]
        .map(Into::into)
        .to_vec();
        assert_eq!(run(&argv, &[], &path), 0);
        let p = read_progress(&path).unwrap();
        assert_eq!((p.state, p.done, p.total), (RunState::Done, 1, 1));
        let argv: Vec<OsString> = [
            "sh",
            "-c",
            "echo 'error: failed to synchronize all databases (unable to lock database)'; exit 1",
        ]
        .map(Into::into)
        .to_vec();
        assert_eq!(run(&argv, &[], &path), 1);
        assert_eq!(read_progress(&path).unwrap().state, RunState::Failed);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
