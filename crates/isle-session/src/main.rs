//! `isle-session`: starts the compositor, restarts it when it dies, keeps crash records
//! and the session log, and exports the session's environment (B9; `platform.md`,
//! Supervisor and crash records).

use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{net::UnixDatagram, process::CommandExt, process::ExitStatusExt},
    },
    path::{Path, PathBuf},
    process::{Command, ExitCode, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use isle_state::{StateDir, domains::Crashes};

/// A death this soon after a start counts towards the backoff and the safe start.
const QUICK: Duration = Duration::from_secs(60);
/// Quick deaths within `QUICK` of each other before the next start ignores the user's
/// settings.
const SAFE_AFTER: usize = 3;
const RECORDS_KEPT: usize = 10;
const LOG_TAIL: usize = 200;
/// The fd the compositor writes `WAYLAND_DISPLAY=…` to once its socket is up.
const READY_FD: i32 = 3;

static CHILD: AtomicI32 = AtomicI32::new(0);
static STOPPING: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    STOPPING.store(true, Ordering::SeqCst);
    let pid = CHILD.load(Ordering::SeqCst);
    if pid > 0 {
        unsafe { libc::kill(pid, libc::SIGTERM) };
    }
}

fn say(message: impl AsRef<str>) {
    eprintln!("isle-session: {}", message.as_ref());
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `ISLE_COMPOSITOR` for development, else the compositor beside this binary, else `PATH`'s.
fn compositor_path() -> PathBuf {
    if let Some(path) = std::env::var_os("ISLE_COMPOSITOR") {
        return path.into();
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("isle-compositor")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| "isle-compositor".into())
}

/// The variables every program of the session gets (`platform.md`, Session start 6).
fn session_environment() -> Vec<(String, String)> {
    let mut vars = vec![
        ("XDG_CURRENT_DESKTOP", "Isle".to_owned()),
        ("XDG_SESSION_DESKTOP", "Isle".to_owned()),
        ("XDG_SESSION_TYPE", "wayland".to_owned()),
        ("QT_QPA_PLATFORM", "wayland;xcb".to_owned()),
        ("QT_QPA_PLATFORMTHEME", "gtk3".to_owned()),
        ("GDK_BACKEND", "wayland,x11,*".to_owned()),
        ("ELECTRON_OZONE_PLATFORM_HINT", "auto".to_owned()),
    ];
    if Path::new("/proc/driver/nvidia/version").exists() {
        vars.push(("LIBVA_DRIVER_NAME", "nvidia".to_owned()));
        vars.push(("__GLX_VENDOR_LIBRARY_NAME", "nvidia".to_owned()));
    }
    if std::env::var_os("SSH_AUTH_SOCK").is_none()
        && let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR")
    {
        // gcr-ssh-agent.socket's, when it is enabled.
        let agent = Path::new(&runtime).join("gcr/ssh");
        if agent.exists() {
            vars.push(("SSH_AUTH_SOCK", agent.display().to_string()));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let local = Path::new(&home).join(".local/bin");
        let path = std::env::var("PATH").unwrap_or_default();
        if !path.split(':').any(|p| Path::new(p) == local) {
            vars.push(("PATH", format!("{}:{path}", local.display())));
        }
    }
    vars.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// Pushes the environment into the systemd user manager and D-Bus activation and starts
/// `isle-session.target`; `ISLE_SESSION_EXPORT=0` skips it, for tests.
fn export(vars: &[(String, String)]) {
    if std::env::var("ISLE_SESSION_EXPORT").as_deref() == Ok("0") {
        return;
    }
    let assignments: Vec<String> = vars.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let updated = Command::new("dbus-update-activation-environment")
        .arg("--systemd")
        .args(&assignments)
        .status();
    if !updated.is_ok_and(|s| s.success()) {
        say("could not export the session's environment");
    }
    let started = Command::new("systemctl")
        .args(["--user", "start", "isle-session.target"])
        .status();
    if !started.is_ok_and(|s| s.success()) {
        say("isle-session.target did not start; graphical-session.target units will not run");
    }
}

fn unexport(vars: &[(String, String)]) {
    if std::env::var("ISLE_SESSION_EXPORT").as_deref() == Ok("0") {
        return;
    }
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "isle-session.target"])
        .status();
    let Some(manager) = manager_environment() else {
        say("could not read the user manager's environment; leaving it as it is");
        return;
    };
    let names = still_ours(vars, &manager);
    if !names.is_empty() {
        let _ = Command::new("systemctl")
            .args(["--user", "unset-environment"])
            .args(&names)
            .status();
    }
}

/// The user manager's environment, as `KEY=VALUE` entries.
fn manager_environment() -> Option<Vec<String>> {
    let out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "get-property",
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "Environment",
        ])
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let entries = value.get("data")?.as_array()?;
    entries
        .iter()
        .map(|e| e.as_str().map(str::to_owned))
        .collect()
}

/// The names to unset at the end: none unless this session exported and the manager's
/// display is still its own, since a session started since then owns them; then those whose
/// value is still the one exported.
fn still_ours<'a>(exported: &'a [(String, String)], manager: &[String]) -> Vec<&'a str> {
    let set = |name: &str| {
        manager
            .iter()
            .find_map(|e| e.strip_prefix(name)?.strip_prefix('='))
    };
    let ours = exported
        .iter()
        .find(|(k, _)| k == "WAYLAND_DISPLAY")
        .is_some_and(|(_, v)| set("WAYLAND_DISPLAY") == Some(v.as_str()));
    if !ours {
        return Vec::new();
    }
    exported
        .iter()
        .filter(|(k, v)| set(k) == Some(v.as_str()))
        .map(|(k, _)| k.as_str())
        .collect()
}

/// The user journal, when the session's output does not already reach it.
struct Journal(Option<UnixDatagram>);

impl Journal {
    fn open() -> Self {
        if std::env::var_os("JOURNAL_STREAM").is_some() {
            return Self(None);
        }
        let socket = UnixDatagram::unbound()
            .and_then(|s| s.connect("/run/systemd/journal/socket").map(|()| s))
            .ok();
        Self(socket)
    }

    fn send(&self, line: &str) {
        let Some(socket) = &self.0 else {
            return;
        };
        let priority = if line.contains(" ERROR ") {
            3
        } else if line.contains(" WARN ") {
            4
        } else {
            6
        };
        let entry = format!(
            "SYSLOG_IDENTIFIER=isle-compositor\nPRIORITY={priority}\nMESSAGE={}\n",
            line.replace('\n', " ")
        );
        let _ = socket.send(entry.as_bytes());
    }
}

struct Log {
    file: Mutex<fs::File>,
    journal: Journal,
    tail: Mutex<VecDeque<String>>,
}

impl Log {
    /// `logs/session.log`, the previous session's kept as `session.log.old`; v1 writes
    /// `session.log` beside `logs/`.
    fn open(dir: &Path) -> std::io::Result<Self> {
        let dir = dir.join("logs");
        fs::create_dir_all(&dir)?;
        let path = dir.join("session.log");
        let _ = fs::rename(&path, dir.join("session.log.old"));
        Ok(Self {
            file: Mutex::new(fs::File::create(path)?),
            journal: Journal::open(),
            tail: Mutex::new(VecDeque::new()),
        })
    }

    fn line(&self, line: &str) {
        let _ = writeln!(self.file.lock().unwrap(), "{line}");
        eprintln!("{line}");
        self.journal.send(line);
        let mut tail = self.tail.lock().unwrap();
        if tail.len() == LOG_TAIL {
            tail.pop_front();
        }
        tail.push_back(line.to_owned());
    }

    fn take_tail(&self) -> Vec<String> {
        self.tail.lock().unwrap().drain(..).collect()
    }
}

fn pump(reader: impl Read + Send + 'static, log: Arc<Log>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            log.line(&line);
        }
    })
}

/// Adds the exit status and the log's tail to the compositor's own record, or writes one
/// when it could not (a signal, an abort), then keeps the newest `RECORDS_KEPT`.
fn record_crash(dir: &Path, pid: u32, status: ExitStatus, tail: &[String]) {
    let _ = fs::create_dir_all(dir);
    let marker = format!("-{pid}-");
    let existing = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().contains(&marker))
        });
    let exit = match (status.code(), status.signal()) {
        (Some(code), _) => format!("exit code {code}"),
        (_, Some(signal)) => format!("signal {signal}"),
        _ => "unknown".into(),
    };
    let appendix = format!("\nexit: {exit}\n\nlog:\n{}\n", tail.join("\n"));
    let path = match existing {
        Some(path) => path,
        None => {
            let path = dir.join(format!("{}-{pid}-exit.txt", now_secs()));
            let head = format!(
                "isle crash record 1\nkind: exit\ntime: {}\npid: {pid}\nmessage: the compositor stopped with {exit}\n",
                humantime::format_rfc3339_seconds(SystemTime::now())
            );
            let _ = fs::write(&path, head);
            path
        }
    };
    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&path) {
        let _ = file.write_all(appendix.as_bytes());
    }
    say(format!("crash record: {}", path.display()));

    let mut records: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    records.sort();
    let excess = records.len().saturating_sub(RECORDS_KEPT);
    for old in &records[..excess] {
        let _ = fs::remove_file(old);
    }
}

fn main() -> ExitCode {
    let extra: Vec<String> = std::env::args().skip_while(|a| a != "--").skip(1).collect();
    let state = StateDir::new(StateDir::default_dir());
    let crashes_dir = StateDir::default_dir().join("crashes");
    let log = match Log::open(&StateDir::default_dir()) {
        Ok(log) => Arc::new(log),
        Err(err) => {
            say(format!("cannot write the session log: {err}"));
            return ExitCode::FAILURE;
        }
    };
    unsafe {
        let handler = on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGHUP, handler);
    }

    let environment = session_environment();
    for (key, value) in &environment {
        unsafe { std::env::set_var(key, value) };
    }
    let compositor = compositor_path();
    say(format!("compositor: {}", compositor.display()));

    loop {
        let (mut crashes, _) = state.load::<Crashes>();
        let now = now_secs();
        crashes
            .quick_deaths
            .retain(|t| now.saturating_sub(*t) < QUICK.as_secs() * SAFE_AFTER as u64);
        let safe = crashes.quick_deaths.len() >= SAFE_AFTER;
        if safe {
            say("starting on the default settings after repeated quick crashes");
        }

        let (ready_read, ready_write) = match std::io::pipe() {
            Ok(pipe) => pipe,
            Err(err) => {
                say(format!("cannot make a pipe: {err}"));
                return ExitCode::FAILURE;
            }
        };
        let write_fd = ready_write.as_raw_fd();
        let mut command = Command::new(&compositor);
        command
            .args(&extra)
            .env("ISLE_READY_FD", READY_FD.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if safe {
            command.env("ISLE_SAFE_START", "1");
        } else {
            command.env_remove("ISLE_SAFE_START");
        }
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(write_fd, READY_FD) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let started = Instant::now();
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                say(format!("cannot start {}: {err}", compositor.display()));
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        drop(ready_write);
        let pid = child.id();
        CHILD.store(pid as i32, Ordering::SeqCst);
        let out = pump(child.stdout.take().unwrap(), log.clone());
        let err = pump(child.stderr.take().unwrap(), log.clone());

        let ready_env = environment.clone();
        let ready = thread::spawn(move || {
            let mut vars = ready_env;
            for line in BufReader::new(ready_read).lines().map_while(Result::ok) {
                if let Some((key, value)) = line.split_once('=') {
                    vars.push((key.to_owned(), value.to_owned()));
                }
            }
            if vars.iter().any(|(k, _)| k == "WAYLAND_DISPLAY") {
                export(&vars);
            }
            vars
        });

        let status = child.wait();
        CHILD.store(0, Ordering::SeqCst);
        let _ = out.join();
        let _ = err.join();
        let exported = ready.join().unwrap_or_default();
        let status = match status {
            Ok(status) => status,
            Err(err) => {
                say(format!("lost the compositor: {err}"));
                return ExitCode::FAILURE;
            }
        };

        if status.success() || STOPPING.load(Ordering::SeqCst) {
            say("the session ended");
            unexport(&exported);
            return ExitCode::SUCCESS;
        }

        let tail = log.take_tail();
        record_crash(&crashes_dir, pid, status, &tail);
        let quick = started.elapsed() < QUICK;
        if quick {
            crashes.quick_deaths.push(now_secs());
        } else {
            crashes.quick_deaths.clear();
        }
        if let Err(err) = state.save(&crashes) {
            say(format!("cannot keep the crash history: {err}"));
        }
        let delay = if quick {
            let n = crashes.quick_deaths.len().saturating_sub(1).min(5) as u32;
            Duration::from_secs(1 << n).min(Duration::from_secs(30))
        } else {
            Duration::from_secs(1)
        };
        say(format!(
            "the compositor stopped; starting it again in {} s",
            delay.as_secs()
        ));
        let deadline = Instant::now() + delay;
        while Instant::now() < deadline && !STOPPING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
        if STOPPING.load(Ordering::SeqCst) {
            unexport(&exported);
            return ExitCode::SUCCESS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exported() -> Vec<(String, String)> {
        [
            ("XDG_CURRENT_DESKTOP", "Isle"),
            ("QT_QPA_PLATFORMTHEME", "gtk3"),
            ("WAYLAND_DISPLAY", "wayland-1"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .to_vec()
    }

    fn manager(entries: &[&str]) -> Vec<String> {
        entries.iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn what_is_still_ours_is_unset() {
        let manager = manager(&[
            "XDG_CURRENT_DESKTOP=Isle",
            "QT_QPA_PLATFORMTHEME=qt6ct",
            "WAYLAND_DISPLAY=wayland-1",
            "HOME=/home/a",
        ]);
        assert_eq!(
            still_ours(&exported(), &manager),
            ["XDG_CURRENT_DESKTOP", "WAYLAND_DISPLAY"]
        );
    }

    #[test]
    fn a_session_started_since_keeps_even_the_values_it_shares() {
        let manager = manager(&[
            "XDG_CURRENT_DESKTOP=Hyprland",
            "QT_QPA_PLATFORMTHEME=gtk3",
            "WAYLAND_DISPLAY=wayland-0",
        ]);
        assert!(still_ours(&exported(), &manager).is_empty());
    }

    #[test]
    fn a_session_that_never_exported_unsets_nothing() {
        let mut vars = exported();
        vars.retain(|(k, _)| k != "WAYLAND_DISPLAY");
        let manager = manager(&["XDG_CURRENT_DESKTOP=Isle", "QT_QPA_PLATFORMTHEME=gtk3"]);
        assert!(still_ours(&vars, &manager).is_empty());
    }
}
