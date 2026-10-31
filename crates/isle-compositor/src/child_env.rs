//! The variables Isle's own children get on top of its environment: Isle's sockets, named
//! once they exist, so the process environment is never written while threads run.

use std::sync::RwLock;

static VARS: RwLock<Vec<(&'static str, String)>> = RwLock::new(Vec::new());

pub fn set(key: &'static str, value: String) {
    let mut vars = VARS.write().unwrap();
    vars.retain(|(k, _)| *k != key);
    vars.push((key, value));
}

/// For `Command::envs`.
pub fn vars() -> Vec<(&'static str, String)> {
    VARS.read().unwrap().clone()
}

/// For `pre_exec` where an app is started: nice 0, so an app takes neither greetd's nice nor
/// the compositor's, and transparent huge pages back on, which the compositor turns off for
/// itself and every child would inherit (B137). Lowering nice needs `RLIMIT_NICE` and is left
/// as it was when refused.
pub fn as_an_app() -> std::io::Result<()> {
    // SAFETY: setpriority and prctl are async-signal-safe.
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 0);
        libc::prctl(libc::PR_SET_THP_DISABLE, 0, 0, 0, 0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::CommandExt;

    /// The nice and `THP_enabled` of a child spawned from a thread at nice 5 in a process
    /// with huge pages off.
    fn child(as_an_app: bool) -> (i32, String) {
        std::thread::spawn(move || {
            unsafe {
                libc::setpriority(libc::PRIO_PROCESS, 0, 5);
                libc::prctl(libc::PR_SET_THP_DISABLE, 1, 0, 0, 0);
            }
            let mut command = std::process::Command::new("/bin/cat");
            command.args(["/proc/self/stat", "/proc/self/status"]);
            if as_an_app {
                unsafe { command.pre_exec(super::as_an_app) };
            }
            let out = String::from_utf8(command.output().unwrap().stdout).unwrap();
            unsafe { libc::prctl(libc::PR_SET_THP_DISABLE, 0, 0, 0, 0) };
            // The nice is the 19th field of the first line; the list after the name starts
            // at the 3rd.
            let stat = out.lines().next().unwrap();
            let after = &stat[stat.rfind(')').unwrap() + 2..];
            let nice = after.split(' ').nth(16).unwrap().parse().unwrap();
            let thp = out
                .lines()
                .find_map(|l| l.strip_prefix("THP_enabled:"))
                .unwrap()
                .trim()
                .to_owned();
            (nice, thp)
        })
        .join()
        .unwrap()
    }

    #[test]
    fn an_app_runs_at_the_default_nice_with_huge_pages() {
        let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        unsafe { libc::getrlimit(libc::RLIMIT_NICE, &mut limit) };
        assert_eq!(child(false), (5, "0".into()));
        let (nice, thp) = child(true);
        assert_eq!(thp, "1");
        // Below the parent's nice only where `RLIMIT_NICE` reaches 0.
        if limit.rlim_cur >= 20 {
            assert_eq!(nice, 0);
        }
    }
}
