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

/// For `pre_exec`: puts a child at nice 0, so an app takes neither greetd's nice nor the
/// compositor's. Lowering it needs `RLIMIT_NICE` and is left as it was when refused.
pub fn default_nice() -> std::io::Result<()> {
    // SAFETY: setpriority is async-signal-safe.
    unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, 0) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::CommandExt;

    /// The nice of a child spawned from a thread at nice 5.
    fn child_nice(reset: bool) -> i32 {
        std::thread::spawn(move || {
            unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, 5) };
            let mut command = std::process::Command::new("/bin/cat");
            command.arg("/proc/self/stat");
            if reset {
                unsafe { command.pre_exec(super::default_nice) };
            }
            let stat = String::from_utf8(command.output().unwrap().stdout).unwrap();
            // The nice is the 19th field; the list after the name starts at the 3rd.
            let after = &stat[stat.rfind(')').unwrap() + 2..];
            after.split(' ').nth(16).unwrap().parse().unwrap()
        })
        .join()
        .unwrap()
    }

    #[test]
    fn a_child_runs_at_the_default_nice() {
        let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        unsafe { libc::getrlimit(libc::RLIMIT_NICE, &mut limit) };
        assert_eq!(child_nice(false), 5);
        // Below the parent's nice only where `RLIMIT_NICE` reaches 0.
        if limit.rlim_cur >= 20 {
            assert_eq!(child_nice(true), 0);
        }
    }
}
