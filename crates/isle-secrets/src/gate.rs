//! The reveal gate: Isle's own polkit action at `auth_self`, its grant held by Keychain until
//! idle, the session's lock or the process's end (choice 16).

use std::{
    collections::HashMap,
    os::fd::{AsFd, FromRawFd, OwnedFd},
    time::{Duration, Instant},
};

use zbus::{
    Connection,
    zvariant::{Fd, Value},
};

use crate::service::Error;

pub const ACTION: &str = "io.github.alexparlett.Isle.Keychain.reveal";

/// How long without input in Keychain's windows before the grant ends.
pub const IDLE: Duration = Duration::from_secs(5 * 60);

/// The grant, once polkit has given it.
#[derive(Debug, Default)]
pub struct Gate {
    granted: bool,
    last_input: Option<Instant>,
}

impl Gate {
    pub fn held(&self, now: Instant) -> bool {
        self.granted
            && self
                .last_input
                .is_some_and(|t| now.duration_since(t) < IDLE)
    }

    /// Input in one of Keychain's windows; the idle period runs from the last.
    pub fn touch(&mut self, now: Instant) {
        self.last_input = Some(now);
    }

    pub fn grant(&mut self, now: Instant) {
        self.granted = true;
        self.last_input = Some(now);
    }

    /// Ends the grant, as the session's lock does.
    pub fn end(&mut self) {
        self.granted = false;
    }

    /// When the grant ends by idle, if it is held.
    pub fn ends_at(&self) -> Option<Instant> {
        self.granted.then_some(self.last_input? + IDLE)
    }
}

fn start_time() -> u64 {
    // Field 22 of /proc/self/stat, counted after the command's closing parenthesis.
    std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|s| {
            let rest = &s[s.rfind(')')? + 2..];
            rest.split_whitespace().nth(19)?.parse().ok()
        })
        .unwrap_or(0)
}

fn pidfd() -> Option<OwnedFd> {
    // SAFETY: pidfd_open takes a pid and flags and returns a new descriptor or -1.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, libc::getpid(), 0) };
    // SAFETY: a non-negative return is a descriptor this function now owns.
    (fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd as i32) })
}

/// Asks polkit whether Keychain's own process may reveal, letting the desktop's dialog ask
/// for the password. Waits as long as the dialog is open.
pub async fn ask(system: &Connection) -> Result<bool, Error> {
    let fd = pidfd();
    let mut details: HashMap<&str, Value<'_>> = HashMap::from([
        ("pid", Value::from(std::process::id())),
        ("start-time", Value::from(start_time())),
        // SAFETY: getuid cannot fail.
        ("uid", Value::from(unsafe { libc::getuid() } as i32)),
    ]);
    if let Some(fd) = &fd {
        details.insert("pidfd", Value::from(Fd::from(fd.as_fd())));
    }
    let reply = system
        .call_method(
            Some("org.freedesktop.PolicyKit1"),
            "/org/freedesktop/PolicyKit1/Authority",
            Some("org.freedesktop.PolicyKit1.Authority"),
            "CheckAuthorization",
            &(
                ("unix-process", details),
                ACTION,
                HashMap::<&str, &str>::new(),
                1u32,
                "",
            ),
        )
        .await
        .map_err(Error::from)?;
    let (authorized, _, _): (bool, bool, HashMap<String, String>) =
        reply.body().deserialize().map_err(zbus::Error::from)?;
    Ok(authorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grant_ends_after_idle_and_at_lock() {
        let t = Instant::now();
        let mut gate = Gate::default();
        assert!(!gate.held(t));
        gate.grant(t);
        assert!(gate.held(t + IDLE - Duration::from_secs(1)));
        assert!(!gate.held(t + IDLE));
        gate.touch(t + Duration::from_secs(200));
        assert!(
            gate.held(t + IDLE + Duration::from_secs(100)),
            "input keeps it"
        );
        gate.end();
        assert!(!gate.held(t + Duration::from_secs(201)));
        gate.touch(t + Duration::from_secs(202));
        assert!(
            !gate.held(t + Duration::from_secs(202)),
            "input does not bring it back"
        );
    }

    #[test]
    fn its_own_start_time_is_read() {
        assert!(start_time() > 0);
    }
}
