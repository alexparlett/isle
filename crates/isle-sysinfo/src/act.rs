//! Acting on a process the user aimed at (`p19a-monitor.md` choices 18 to 20): a signal
//! through a pidfd opened before the start time is checked, so a pid the kernel has given out
//! again is never signalled; a nice level on every thread.

use std::{fs, io, path::Path};

use crate::procs::Key;

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// The process has ended, or its pid now belongs to another.
    Gone,
    /// The kernel needs privilege for it: another user's process, or a raised priority.
    NotPermitted,
    Other(i32),
}

impl Refused {
    fn of(err: io::Error) -> Self {
        match err.raw_os_error() {
            Some(libc::ESRCH) => Refused::Gone,
            Some(libc::EPERM | libc::EACCES) => Refused::NotPermitted,
            other => Refused::Other(other.unwrap_or(0)),
        }
    }
}

/// The start time `/proc/<pid>/stat` gives now, in clock ticks after boot.
fn start_of(root: &Path, pid: i32) -> Option<u64> {
    let stat = fs::read_to_string(root.join(format!("proc/{pid}/stat"))).ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(19)?.parse().ok()
}

/// Sends `signal` to the process `key` names.
pub fn signal(key: Key, signal: i32) -> Result<(), Refused> {
    // SAFETY: pidfd_open takes a pid and flags and returns a descriptor this function owns.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, key.pid, 0) } as i32;
    if fd < 0 {
        return Err(Refused::of(io::Error::last_os_error()));
    }
    let fd = unsafe { <std::os::fd::OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(fd) };
    // The pidfd holds the process it was opened on: checked now, the signal cannot reach
    // another that took the pid since.
    if start_of(Path::new("/"), key.pid) != Some(key.start) {
        return Err(Refused::Gone);
    }
    use std::os::fd::AsRawFd;
    let sent = unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd.as_raw_fd(), signal, std::ptr::null::<libc::siginfo_t>(), 0) };
    if sent < 0 {
        return Err(Refused::of(io::Error::last_os_error()));
    }
    Ok(())
}

/// Sets every thread of the process `key` names to `nice`.
pub fn renice(key: Key, nice: i32) -> Result<(), Refused> {
    let root = Path::new("/");
    if start_of(root, key.pid) != Some(key.start) {
        return Err(Refused::Gone);
    }
    let tasks = fs::read_dir(root.join(format!("proc/{}/task", key.pid))).map_err(Refused::of)?;
    for task in tasks.flatten() {
        let Some(tid) = task.file_name().to_str().and_then(|t| t.parse::<u32>().ok()) else { continue };
        // SAFETY: setpriority takes plain integers.
        let set = unsafe { libc::setpriority(libc::PRIO_PROCESS, tid, nice) };
        if set < 0 {
            let err = io::Error::last_os_error();
            // A thread that ended meanwhile is no matter.
            if err.raw_os_error() != Some(libc::ESRCH) {
                return Err(Refused::of(err));
            }
        }
    }
    if start_of(root, key.pid) != Some(key.start) {
        return Err(Refused::Gone);
    }
    Ok(())
}

/// The process's nice level now.
pub fn nice_of(pid: i32) -> Option<i32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(16)?.parse().ok()
}

/// The signals Send Signal offers, by name.
pub const SIGNALS: [(&str, i32); 10] = [
    ("Hangup (SIGHUP)", libc::SIGHUP),
    ("Interrupt (SIGINT)", libc::SIGINT),
    ("Quit (SIGQUIT)", libc::SIGQUIT),
    ("Abort (SIGABRT)", libc::SIGABRT),
    ("Kill (SIGKILL)", libc::SIGKILL),
    ("Alarm (SIGALRM)", libc::SIGALRM),
    ("Terminate (SIGTERM)", libc::SIGTERM),
    ("User 1 (SIGUSR1)", libc::SIGUSR1),
    ("User 2 (SIGUSR2)", libc::SIGUSR2),
    ("Stop (SIGSTOP)", libc::SIGSTOP),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn child() -> (std::process::Child, Key) {
        let child = std::process::Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id() as i32;
        let start = start_of(Path::new("/"), pid).unwrap();
        (child, Key { pid, start })
    }

    #[test]
    fn a_child_is_signalled_and_a_changed_start_time_is_refused() {
        let (mut stale, key) = child();
        let wrong = Key { start: key.start + 1, ..key };
        assert_eq!(signal(wrong, libc::SIGTERM), Err(Refused::Gone), "another process's start time");
        assert!(stale.try_wait().unwrap().is_none(), "nothing was sent");
        signal(key, libc::SIGTERM).unwrap();
        let status = stale.wait().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        assert_eq!(signal(key, libc::SIGTERM), Err(Refused::Gone), "reaped, it is gone");
    }

    #[test]
    fn a_child_is_reniced_and_a_changed_start_time_is_refused() {
        let (mut c, key) = child();
        renice(key, 10).unwrap();
        assert_eq!(nice_of(key.pid), Some(10));
        assert_eq!(renice(Key { start: key.start + 1, ..key }, 5), Err(Refused::Gone));
        let _ = c.kill();
        let _ = c.wait();
    }
}
