//! Crash records in `~/.local/state/isle/crashes/`, written by the panic hook and the watchdog.

use std::{
    backtrace::Backtrace,
    fmt::Write as _,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::watchdog::{Heartbeat, Stall};

pub fn crashes_dir() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    state.join("isle/crashes")
}

/// Writes one record. `kind` is `panic` or `hang`.
pub fn write(
    kind: &str,
    thread: &str,
    message: &str,
    backtrace: &str,
    stalls: &[Stall],
) -> io::Result<PathBuf> {
    write_in(&crashes_dir(), kind, thread, message, backtrace, stalls)
}

fn write_in(
    dir: &Path,
    kind: &str,
    thread: &str,
    message: &str,
    backtrace: &str,
    stalls: &[Stall],
) -> io::Result<PathBuf> {
    let now = SystemTime::now();
    let mut text = String::new();
    let _ = writeln!(text, "isle crash record 1");
    let _ = writeln!(text, "kind: {kind}");
    let _ = writeln!(text, "time: {}", humantime::format_rfc3339_seconds(now));
    let _ = writeln!(text, "build: {}", crate::build_info::describe());
    let _ = writeln!(text, "pid: {}", std::process::id());
    let _ = writeln!(text, "thread: {thread}");
    let _ = writeln!(text, "message: {message}");
    let _ = writeln!(text, "\nstalls:");
    for stall in stalls {
        let _ = writeln!(
            text,
            "{} {} {:.1} ms",
            humantime::format_rfc3339_millis(stall.at),
            stall.source,
            stall.took.as_secs_f64() * 1000.0
        );
    }
    let _ = writeln!(text, "\nbacktrace:\n{backtrace}");

    fs::create_dir_all(dir)?;
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("{secs}-{}-{kind}.txt", std::process::id()));
    let mut file = fs::File::create(&path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(path)
}

/// A panic on the main thread ends the session and writes a record; elsewhere the
/// panicking service is restarted (B9). The default hook prints it either way.
pub fn install_panic_hook(heartbeat: Heartbeat) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        if thread.name() != Some("main") {
            default(info);
            return;
        }
        // A shell surface's panic is caught at its boundary and the surface rebuilt.
        if let Some(surface) = isle_shell::boundary::panicking_surface() {
            tracing::error!(
                surface,
                "surface panicked: {info}\n{}",
                Backtrace::force_capture()
            );
            return;
        }
        let message = match info.payload().downcast_ref::<&str>() {
            Some(s) => (*s).to_owned(),
            None => info
                .payload()
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "(no message)".into()),
        };
        let message = match info.location() {
            Some(l) => format!("{message} at {}:{}", l.file(), l.line()),
            None => message,
        };
        let backtrace = Backtrace::force_capture().to_string();
        match write(
            "panic",
            thread.name().unwrap_or("unnamed"),
            &message,
            &backtrace,
            &heartbeat.recent_stalls(),
        ) {
            Ok(path) => eprintln!("isle: crash record written to {}", path.display()),
            Err(err) => eprintln!("isle: could not write the crash record: {err}"),
        }
        default(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_record_names_its_kind_message_and_stalls() {
        let dir = std::env::temp_dir().join(format!("isle-crash-test-{}", std::process::id()));
        let stall = Stall {
            source: "wayland",
            took: Duration::from_millis(40),
            at: SystemTime::now(),
        };
        let path = write_in(&dir, "panic", "main", "boom at x.rs:1", "frames", &[stall]).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("-panic.txt")
        );
        assert!(text.starts_with("isle crash record 1\nkind: panic\n"));
        assert!(text.contains("message: boom at x.rs:1\n"));
        assert!(text.contains(" wayland 40.0 ms\n"));
        assert!(text.contains("backtrace:\nframes"));
    }
}
