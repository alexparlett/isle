//! Quit, stop, signal and prioritise: directly where the kernel allows, else through
//! `isle-process-helper` under pkexec, which asks in the desktop's polkit dialog
//! (`p19a-monitor.md` choices 18 to 20). Refused, nothing changes and nothing is said.

use isle_sysinfo::{
    act::{self, Refused},
    procs::Key,
};
use tracing::{info, warn};

const HELPER: &str = "/usr/lib/isle/isle-process-helper";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Signal(i32),
    Priority(Priority),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    VeryLow,
    Low,
    Normal,
    High,
    VeryHigh,
}

impl Priority {
    pub const ALL: [Priority; 5] = [Self::VeryLow, Self::Low, Self::Normal, Self::High, Self::VeryHigh];

    pub fn label(self) -> &'static str {
        match self {
            Self::VeryLow => "Very low",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
            Self::VeryHigh => "Very high",
        }
    }

    pub fn nice(self) -> i32 {
        match self {
            Self::VeryLow => 19,
            Self::Low => 10,
            Self::Normal => 0,
            Self::High => -5,
            Self::VeryHigh => -10,
        }
    }

    /// The app scope's share of the CPU beside other apps' (choice 20).
    pub fn weight(self) -> u64 {
        match self {
            Self::VeryLow => 10,
            Self::Low => 50,
            Self::Normal => 100,
            Self::High => 200,
            Self::VeryHigh => 500,
        }
    }

    /// The level a nice value stands nearest.
    pub fn of(nice: i32) -> Self {
        *Self::ALL.iter().min_by_key(|p| (p.nice() - nice).abs()).unwrap_or(&Self::Normal)
    }
}

/// Carries out `act` on `key`; runs on the worker.
pub async fn run(bus: Option<zbus::Connection>, key: Key, cgroup: String, act: Act) {
    let direct = tokio::task::spawn_blocking(move || match act {
        Act::Signal(s) => act::signal(key, s),
        Act::Priority(p) => act::renice(key, p.nice()),
    })
    .await
    .unwrap_or(Err(Refused::Other(0)));
    match direct {
        Ok(()) => {}
        Err(Refused::NotPermitted) => helper(key, act).await,
        Err(Refused::Gone) => info!(pid = key.pid, "the process had ended"),
        Err(err) => warn!(pid = key.pid, ?err, "could not act on the process"),
    }
    if let (Act::Priority(p), Some(bus)) = (act, bus.as_ref())
        && let Some(unit) = led_scope(key.pid, &cgroup)
        && let Err(err) = set_weight(bus, &unit, p.weight()).await
    {
        warn!(unit, "the scope's CPU weight: {err}");
    }
}

/// Asks pkexec to run the helper; a refusal or a dismissed dialog ends here quietly.
async fn helper(key: Key, act: Act) {
    let (verb, value) = match act {
        Act::Signal(s) => ("signal", s),
        Act::Priority(p) => ("nice", p.nice()),
    };
    let helper = std::env::var("ISLE_PROCESS_HELPER").unwrap_or_else(|_| HELPER.into());
    let status = tokio::process::Command::new("pkexec")
        .args([helper.as_str(), verb, &key.pid.to_string(), &key.start.to_string(), &value.to_string()])
        .status()
        .await;
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => info!(pid = key.pid, code = ?s.code(), "the helper did not act"),
        Err(err) => warn!("pkexec: {err}"),
    }
}

/// The app scope `pid` leads: its cgroup is an `app-*.scope` whose first process it is.
fn led_scope(pid: i32, cgroup: &str) -> Option<String> {
    let unit = cgroup.rsplit('/').next()?;
    if !(unit.starts_with("app-") && unit.ends_with(".scope")) {
        return None;
    }
    let procs = std::fs::read_to_string(format!("/sys/fs/cgroup{cgroup}/cgroup.procs")).ok()?;
    let first: i32 = procs.lines().next()?.trim().parse().ok()?;
    (first == pid).then(|| unit.to_owned())
}

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Systemd {
    fn set_unit_properties(&self, name: &str, runtime: bool, properties: &[(&str, zbus::zvariant::Value<'_>)]) -> zbus::Result<()>;
}

/// Sets the user manager's `CPUWeight` on `unit`, for as long as the scope lasts.
async fn set_weight(bus: &zbus::Connection, unit: &str, weight: u64) -> zbus::Result<()> {
    SystemdProxy::new(bus).await?.set_unit_properties(unit, true, &[("CPUWeight", weight.into())]).await
}

/// Processes Monitor will not quit: Isle's own and the user's service manager (choice 21).
pub fn protected(name: &str, ppid: i32) -> bool {
    matches!(name, "isle-compositor" | "isle-session") || (name == "systemd" && ppid == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_map_to_nice_and_weight_and_back() {
        assert_eq!(Priority::High.nice(), -5);
        assert_eq!(Priority::VeryLow.weight(), 10);
        assert_eq!(Priority::of(0), Priority::Normal);
        assert_eq!(Priority::of(7), Priority::Low);
        assert_eq!(Priority::of(-20), Priority::VeryHigh);
        assert!(protected("isle-compositor", 900));
        assert!(protected("systemd", 1));
        assert!(!protected("systemd", 900), "a user's own nested manager is not the session's");
    }

    #[tokio::test]
    async fn a_child_is_quit_directly_without_the_helper() {
        let mut child = std::process::Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id() as i32;
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let start = stat.rsplit_once(')').unwrap().1.split_whitespace().nth(19).unwrap().parse().unwrap();
        // SAFETY: the variable is read only by `helper`, which this test must not reach.
        unsafe { std::env::set_var("ISLE_PROCESS_HELPER", "/nonexistent") };
        run(None, Key { pid, start }, String::new(), Act::Signal(libc::SIGTERM)).await;
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGTERM));
    }
}
