//! Apps that crash: `systemd-coredump`'s records followed in the journal, matched to an app,
//! and "{app} quit unexpectedly." once it is gone (`p20b-session.md` choices 15–17).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use tokio::io::AsyncBufReadExt;
use tracing::{debug, info, warn};

use crate::{desktop_notes::Note, restore::Slot, state::State, worker::ToLoop};

/// `systemd-coredump`'s message id for a process that dumped core.
const COREDUMP: &str = "fc2e22bc6ee647b6b90729ab34a250b1";
/// How long after the record the app must be gone for the notice.
const SETTLE: Duration = Duration::from_secs(1);
/// One notice per app in this long.
const ONCE_PER: Duration = Duration::from_secs(60);
/// The reports kept in `crashes/apps`.
const KEEP: usize = 20;

/// One core dump, as the journal records it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub pid: i32,
    pub cgroup: String,
    pub command: String,
    pub executable: String,
    pub signal: String,
    /// The summary and stack `systemd-coredump` wrote.
    pub message: String,
}

/// A journal field's text: a string, or bytes for one that is not UTF-8.
fn field(entry: &serde_json::Value, name: &str) -> Option<String> {
    match entry.get(name)? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(bytes) => {
            let bytes: Option<Vec<u8>> =
                bytes.iter().map(|b| b.as_u64().map(|b| b as u8)).collect();
            Some(String::from_utf8_lossy(&bytes?).into_owned())
        }
        _ => None,
    }
}

/// A journal line as a core dump of `uid`'s, or `None` for anything else.
pub fn parse(line: &str, uid: u32) -> Option<Record> {
    let entry: serde_json::Value = serde_json::from_str(line).ok()?;
    if field(&entry, "MESSAGE_ID")? != COREDUMP
        || field(&entry, "COREDUMP_UID")?.parse::<u32>().ok()? != uid
    {
        return None;
    }
    Some(Record {
        pid: field(&entry, "COREDUMP_PID")?.parse().ok()?,
        cgroup: field(&entry, "COREDUMP_CGROUP").unwrap_or_default(),
        command: field(&entry, "COREDUMP_COMM").unwrap_or_default(),
        executable: field(&entry, "COREDUMP_EXE").unwrap_or_default(),
        signal: field(&entry, "COREDUMP_SIGNAL_NAME").unwrap_or_default(),
        message: field(&entry, "MESSAGE").unwrap_or_default(),
    })
}

/// The journal's core dumps of this user from now on, or the lines of `ISLE_CRASH_JOURNAL`,
/// a file a test appends to.
fn source(uid: u32) -> tokio::process::Command {
    let mut command = match std::env::var_os("ISLE_CRASH_JOURNAL") {
        Some(file) => {
            let mut tail = tokio::process::Command::new("tail");
            tail.args(["-n", "+1", "-F"]).arg(file);
            tail
        }
        None => {
            let mut journal = tokio::process::Command::new("journalctl");
            journal
                .args(["--follow", "--lines=0", "--output=json", "--all"])
                .arg(format!("MESSAGE_ID={COREDUMP}"))
                .arg(format!("COREDUMP_UID={uid}"));
            journal
        }
    };
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    command
}

/// Follows the records on the worker, started again if the source ends.
pub fn watch(runtime: &tokio::runtime::Handle, to_loop: ToLoop) {
    // Safety: getuid has no failure and touches no memory.
    let uid = unsafe { libc::getuid() };
    runtime.spawn(async move {
        loop {
            let child = source(uid).spawn();
            let mut child = match child {
                Ok(child) => child,
                Err(err) => {
                    warn!("no crash records: {err}");
                    return;
                }
            };
            let Some(out) = child.stdout.take() else {
                return;
            };
            let mut lines = tokio::io::BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(record) = parse(&line, uid) {
                    to_loop.post(move |state| state.crash_recorded(record));
                }
            }
            let _ = child.wait().await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

/// What a crash notice's actions act on.
#[derive(Clone, Debug)]
pub struct Crash {
    pub app: String,
    pub name: String,
    pub record: Record,
    /// Unix seconds it was recorded.
    pub at: u64,
}

#[derive(Default)]
pub struct CrashState {
    /// When each app's last notice was posted.
    noticed: HashMap<String, Instant>,
}

/// The folder Report… writes into.
pub fn reports_dir() -> PathBuf {
    crate::crash::crashes_dir().join("apps")
}

/// Writes `crash`'s report into `dir`, keeping the newest `KEEP`; its path.
pub fn write_report(dir: &Path, crash: &Crash) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let r = &crash.record;
    let text = format!(
        "{} quit unexpectedly.\n\nApp: {}\nProcess: {} [{}]\nExecutable: {}\nSignal: {}\nTime: {}\n\n{}\n",
        crash.name, crash.app, r.command, r.pid, r.executable, r.signal, crash.at, r.message
    );
    let path = dir.join(format!("{}-{}.txt", crash.app, crash.at));
    std::fs::write(&path, text)?;
    let mut reports: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    reports.sort();
    let extra = reports.len().saturating_sub(KEEP);
    for (_, old) in reports.into_iter().take(extra) {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

impl State {
    /// Starts following crash records, under the supervisor or for a test.
    pub fn watch_crashes(&mut self) {
        if let (Some(runtime), Some(to_loop)) = (&self.isle.worker, &self.isle.to_loop) {
            watch(runtime, to_loop.clone());
        }
    }

    /// A record arrived: if it names an app, the app is looked at again a moment later.
    pub fn crash_recorded(&mut self, record: Record) {
        let Some(app) = self.isle.processes.app_of_crash(record.pid, &record.cgroup) else {
            debug!(
                pid = record.pid,
                command = record.command,
                "a crash of no app's"
            );
            return;
        };
        info!(app, pid = record.pid, command = record.command, "a crash");
        let _ = self.isle.loop_handle.insert_source(
            Timer::from_duration(SETTLE),
            move |_, _, state| {
                state.crash_settled(&app, record.clone());
                TimeoutAction::Drop
            },
        );
    }

    /// Only an app with no window and no process left gets the notice, once a minute.
    fn crash_settled(&mut self, app: &str, record: Record) {
        let windowed = self
            .isle
            .wm
            .records
            .iter()
            .any(|r| r.mapped && r.app == app);
        if windowed || self.isle.processes.running(app) {
            info!(
                app,
                pid = record.pid,
                "a helper crashed; the app still runs"
            );
            return;
        }
        let noticed = &mut self.isle.crashes.noticed;
        if noticed.get(app).is_some_and(|at| at.elapsed() < ONCE_PER) {
            return;
        }
        noticed.insert(app.to_owned(), Instant::now());
        let name = self.isle.app_name(app);
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let summary = format!("{name} quit unexpectedly.");
        self.post_desktop(
            app,
            &name,
            summary,
            String::new(),
            vec![("reopen", "Reopen"), ("report", "Report…")],
            Note::AppCrash(Box::new(Crash {
                app: app.to_owned(),
                name: name.clone(),
                record,
                at,
            })),
        );
    }

    /// Reopen: the app launched with its windows' last places pending.
    pub fn reopen_crashed(&mut self, app: &str) {
        let slots = self
            .isle
            .restore
            .last_windows
            .get(app)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(n, window)| Slot {
                window,
                focus: true,
                rank: (usize::MAX, n),
            })
            .collect();
        self.reopen_with(app, slots, true);
    }

    /// Report…: the record's summary and stack written to a file and opened.
    pub fn report_crash(&mut self, crash: &Crash) {
        match write_report(&reports_dir(), crash) {
            Ok(path) => {
                self.launch(crate::launch::Launch::Open(
                    path.to_string_lossy().into_owned(),
                ));
            }
            Err(err) => warn!("could not write the crash report: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_core_dump_of_this_user_is_read_and_others_are_not() {
        let line = serde_json::json!({
            "MESSAGE_ID": COREDUMP,
            "COREDUMP_UID": "1000",
            "COREDUMP_PID": "4242",
            "COREDUMP_CGROUP": "/user.slice/user-1000.slice/user@1000.service/app.slice/app-isle-x-1.scope",
            "COREDUMP_COMM": "x",
            "COREDUMP_SIGNAL_NAME": "SIGSEGV",
            "MESSAGE": [80, 255, 33],
        })
        .to_string();
        let record = parse(&line, 1000).unwrap();
        assert_eq!(record.pid, 4242);
        assert_eq!(record.signal, "SIGSEGV");
        assert_eq!(record.message, "P\u{fffd}!");
        assert!(parse(&line, 1001).is_none());
        assert!(parse(r#"{"MESSAGE_ID":"other","COREDUMP_UID":"1000"}"#, 1000).is_none());
        assert!(parse("not json", 1000).is_none());
    }

    #[test]
    fn reports_keep_the_newest_twenty() {
        let dir = std::env::temp_dir().join(format!("isle-crash-reports-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for n in 0..25 {
            let crash = Crash {
                app: "x".into(),
                name: "X".into(),
                record: Record::default(),
                at: n,
            };
            write_report(&dir, &crash).unwrap();
        }
        let left = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(left, KEEP);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
