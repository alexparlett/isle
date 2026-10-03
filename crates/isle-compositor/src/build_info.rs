//! The running build's identity, reported by `isle version` and in crash records.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const COMMIT: &str = env!("ISLE_COMMIT");

pub fn built() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(env!("ISLE_BUILT").parse().unwrap_or(0))
}

pub fn describe() -> String {
    format!(
        "{} {} built {}",
        env!("CARGO_PKG_VERSION"),
        COMMIT,
        humantime::format_rfc3339_seconds(built())
    )
}
