//! The greeter mode: the compositor run by greetd as its greeter, the lock and nothing else
//! (`p14b-greeter.md`).

use std::sync::OnceLock;

static ON: OnceLock<bool> = OnceLock::new();

/// Whether this process is the greeter; fixed at start.
pub fn on() -> bool {
    ON.get().copied().unwrap_or(false)
}

/// Sets the mode and, before any thread starts, the greeter's own directories where greetd
/// left none: its caches in `/var/cache/isle-greeter`, its state in `/var/lib/isle-greeter`
/// (both the package's, owned by `greeter`).
pub fn start(greeter: bool) {
    let _ = ON.set(greeter);
    if !greeter {
        return;
    }
    for (key, dir) in [
        ("XDG_CACHE_HOME", "/var/cache/isle-greeter"),
        ("XDG_STATE_HOME", "/var/lib/isle-greeter"),
    ] {
        if std::env::var_os(key).is_none() {
            // SAFETY: called from `run` before any thread starts.
            unsafe { std::env::set_var(key, dir) };
        }
    }
}
