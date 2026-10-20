//! The running build's identity, reported by `isle version` and in crash records.

use std::time::{SystemTime, UNIX_EPOCH};

pub const COMMIT: &str = env!("ISLE_COMMIT");

/// When the running binary was linked, read from its own file: through `/proc/self/exe`,
/// which still reaches it after an update has replaced it on disk.
pub fn built() -> SystemTime {
    std::fs::metadata("/proc/self/exe")
        .or_else(|_| std::env::current_exe().and_then(std::fs::metadata))
        .and_then(|m| m.modified())
        .unwrap_or(UNIX_EPOCH)
}

pub fn features() -> Vec<String> {
    let mut features = Vec::new();
    if cfg!(feature = "test-hooks") {
        features.push("test-hooks".to_owned());
    }
    features
}

pub fn describe() -> String {
    format!(
        "{} {} built {}",
        env!("CARGO_PKG_VERSION"),
        COMMIT,
        humantime::format_rfc3339_seconds(built())
    )
}
