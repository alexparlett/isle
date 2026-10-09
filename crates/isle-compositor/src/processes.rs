//! Apps' processes, so an app whose windows are gone and that still runs is known as running
//! in the background (`p7-switcher.md` choice 7).

use std::{
    collections::HashMap,
    os::fd::{FromRawFd, OwnedFd},
    path::{Path, PathBuf},
};

use smithay::reexports::calloop::{
    Interest, Mode, PostAction, RegistrationToken, generic::Generic,
};
use tracing::{debug, info};

use crate::state::State;

/// The processes each app has had a window from, and the scopes Isle launched apps in.
#[derive(Default)]
pub struct Processes {
    /// Each live pid, the app whose window last had it, and its pidfd's source.
    pids: HashMap<i32, (String, RegistrationToken)>,
    /// Each launched scope's cgroup directory and its app, while it has processes.
    scopes: HashMap<PathBuf, String>,
    /// Apps that have had a window, most recently windowless first.
    seen: Vec<String>,
}

impl Processes {
    /// Whether `app` has a live process it once had a window from, or a live launched scope.
    pub fn running(&self, app: &str) -> bool {
        self.seen.iter().any(|a| a == app)
            && (self.pids.values().any(|(a, _)| a == app) || self.scopes.values().any(|a| a == app))
    }

    /// Apps that have had a window and still run, most recently windowless first.
    pub fn seen(&self) -> impl Iterator<Item = &String> {
        self.seen.iter().filter(|a| self.running(a))
    }

    /// The app whose window last had `pid`.
    pub fn app_of(&self, pid: i32) -> Option<String> {
        self.pids.get(&pid).map(|(app, _)| app.clone())
    }

    /// The app `pid` belongs to: its own window's, the nearest ancestor's, or the scope Isle
    /// launched it in. `pkexec` in a terminal is the terminal's.
    pub fn app_of_tree(&self, pid: i32) -> Option<String> {
        let mut at = pid;
        for _ in 0..32 {
            if let Some(app) = self.app_of(at) {
                return Some(app);
            }
            if let Some(app) = cgroup_of(at as u32).and_then(|dir| self.scopes.get(&dir)) {
                return Some(app.clone());
            }
            at = parent_of(at).filter(|p| *p > 1)?;
        }
        None
    }

    pub fn pids(&self, app: &str) -> Vec<i32> {
        let mut pids: Vec<i32> = self
            .pids
            .iter()
            .filter(|(_, (a, _))| a == app)
            .map(|(pid, _)| *pid)
            .collect();
        for (dir, _) in self.scopes.iter().filter(|(_, a)| *a == app) {
            pids.extend(cgroup_pids(dir));
        }
        pids.sort_unstable();
        pids.dedup();
        pids
    }
}

/// The pids in a cgroup directory.
fn cgroup_pids(dir: &Path) -> Vec<i32> {
    std::fs::read_to_string(dir.join("cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

/// `pid`'s parent, from the field after the command in `/proc/{pid}/stat`.
fn parent_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
}

/// The cgroup directory `pid` is in, under the unified hierarchy.
pub fn cgroup_of(pid: u32) -> Option<PathBuf> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    let path = text.lines().find_map(|l| l.strip_prefix("0::"))?;
    Some(Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/')))
}

/// Whether a cgroup directory still holds a process.
pub fn populated(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("cgroup.events"))
        .map(|t| t.lines().any(|l| l == "populated 1"))
        .unwrap_or(false)
}

fn pidfd_open(pid: i32) -> Option<OwnedFd> {
    // Safety: pidfd_open takes a pid and flags and returns a new descriptor or -1.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    // Safety: a non-negative result is a descriptor this process now owns.
    (fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd as i32) })
}

impl State {
    /// A window of `app` from `pid` mapped: the pid is the app's until another app's window
    /// has it, and is watched until it exits.
    pub fn track_process(&mut self, app: &str, pid: Option<i32>) {
        let processes = &mut self.isle.processes;
        processes.seen.retain(|a| a != app);
        processes.seen.push(app.to_owned());
        let Some(pid) = pid.filter(|p| *p > 1 && *p != std::process::id() as i32) else {
            return;
        };
        if let Some((owner, _)) = processes.pids.get_mut(&pid) {
            if owner != app {
                *owner = app.to_owned();
            }
            return;
        }
        let Some(fd) = pidfd_open(pid) else {
            return;
        };
        let token = self.isle.loop_handle.insert_source(
            Generic::new(fd, Interest::READ, Mode::Level),
            move |_, _, state| {
                state.process_exited(pid);
                Ok(PostAction::Remove)
            },
        );
        if let Ok(token) = token {
            self.isle
                .processes
                .pids
                .insert(pid, (app.to_owned(), token));
        }
    }

    fn process_exited(&mut self, pid: i32) {
        if let Some((app, _)) = self.isle.processes.pids.remove(&pid) {
            debug!(pid, app, "process exited");
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }

    /// A launched app's scope, watched by the worker until it empties.
    pub fn track_scope(&mut self, app: String, dir: PathBuf) {
        self.isle.processes.scopes.insert(dir, app);
        self.isle.notify(crate::worker::Event::WindowsChanged);
    }

    pub fn scope_emptied(&mut self, dir: &Path) {
        if self.isle.processes.scopes.remove(dir).is_some() {
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }

    /// The last window of `app` went: it is the most recently windowless app.
    pub fn app_windowless(&mut self, app: &str) {
        let processes = &mut self.isle.processes;
        if processes.seen.iter().any(|a| a == app) {
            processes.seen.retain(|a| a != app);
            processes.seen.insert(0, app.to_owned());
        }
    }

    /// Q on a background app: the polite signal to every process it has.
    pub fn stop_app(&mut self, app: &str) -> bool {
        if self
            .isle
            .wm
            .records
            .iter()
            .any(|r| r.app == app && r.mapped)
        {
            return false;
        }
        let pids = self.isle.processes.pids(app);
        info!(app, ?pids, "stop running in background");
        for pid in &pids {
            // Safety: kill sends a signal to a pid; a gone pid only returns an error.
            unsafe { libc::kill(*pid, libc::SIGTERM) };
        }
        !pids.is_empty()
    }
}

/// Watches a launched scope's `cgroup.events` until it reports no process.
pub async fn watch_scope(to_loop: crate::worker::ToLoop, dir: PathBuf) {
    use notify::Watcher;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = tx.send(());
        }
    });
    let Ok(mut watcher) = watcher else {
        return;
    };
    if watcher
        .watch(
            &dir.join("cgroup.events"),
            notify::RecursiveMode::NonRecursive,
        )
        .is_err()
    {
        return;
    }
    loop {
        if !populated(&dir) {
            break;
        }
        if rx.recv().await.is_none() {
            break;
        }
    }
    to_loop.post(move |state| state.scope_emptied(&dir));
}
