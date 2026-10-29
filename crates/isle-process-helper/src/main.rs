//! Signals or renices one process as root for Monitor, run by pkexec under Isle's own policy
//! (`p19a-monitor.md` choice 19): `signal <pid> <start> <signal>` or `nice <pid> <start>
//! <level>`. It refuses pid 1, Isle's compositor and session, a user's systemd manager and its
//! own callers; the pidfd check is `isle_sysinfo::act`'s.

use std::process::ExitCode;

use isle_sysinfo::{
    act::{self, Refused},
    procs::Key,
};

/// The signals Monitor sends: its Send Signal list, Stop and Continue.
fn allowed_signal(n: i32) -> bool {
    n == libc::SIGCONT || act::SIGNALS.iter().any(|(_, s)| *s == n)
}

/// A process's command name and parent.
fn comm_and_parent(pid: i32) -> Option<(String, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (head, rest) = stat.rsplit_once(')')?;
    let comm = head.split_once('(')?.1.to_owned();
    let ppid = rest.split_whitespace().nth(1)?.parse().ok()?;
    Some((comm, ppid))
}

/// Why `pid` may not be acted on from here, if it may not.
fn protected(pid: i32) -> Option<&'static str> {
    if pid <= 1 {
        return Some("the system's first process");
    }
    let (comm, ppid) = comm_and_parent(pid)?;
    if matches!(comm.as_str(), "isle-compositor" | "isle-session") {
        return Some("Isle's own session");
    }
    if comm == "systemd" && ppid == 1 {
        return Some("a user's service manager");
    }
    // Its own callers: pkexec, Monitor and whatever started them.
    let mut at = std::os::unix::process::parent_id() as i32;
    while at > 1 {
        if at == pid {
            return Some("its own caller");
        }
        at = comm_and_parent(at).map_or(0, |(_, p)| p);
    }
    None
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [verb, pid, start, value] = args.as_slice() else {
        eprintln!("usage: isle-process-helper signal|nice <pid> <start> <value>");
        return ExitCode::from(2);
    };
    let (Ok(pid), Ok(start), Ok(value)) = (pid.parse::<i32>(), start.parse::<u64>(), value.parse::<i32>()) else {
        eprintln!("isle-process-helper: the pid, start and value are numbers");
        return ExitCode::from(2);
    };
    if let Some(why) = protected(pid) {
        eprintln!("isle-process-helper: {pid} is {why}");
        return ExitCode::from(2);
    }
    let key = Key { pid, start };
    let done = match verb.as_str() {
        "signal" if allowed_signal(value) => act::signal(key, value),
        "nice" if (-20..=19).contains(&value) => act::renice(key, value),
        _ => {
            eprintln!("isle-process-helper: no {verb} {value}");
            return ExitCode::from(2);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(Refused::Gone) => ExitCode::from(1),
        Err(err) => {
            eprintln!("isle-process-helper: {err:?}");
            ExitCode::from(3)
        }
    }
}
