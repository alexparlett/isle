//! Fake system daemons for tests, served on a private bus. Each answers the calls Isle's
//! proxies make, can change its state as the daemon would, and can leave the bus and return.
//! `tree` is a fake `/proc` and `/sys` for readers that take a root; `greetd` a fake greetd on
//! a socket.

pub mod accounts;
pub mod bluez;
pub mod greetd;
pub mod logind;
pub mod mpris;
pub mod network;
pub mod polkit;
pub mod secrets;
pub mod tree;
pub mod udisks;
pub mod upower;

use std::process::Command;

use zbus::Connection;

/// A private session bus that activates nothing, so a name no fake owns is missing, never
/// started from the host's service files. The daemon ends when this is dropped.
pub struct Bus {
    pub address: String,
    pid: i32,
    config: std::path::PathBuf,
}

impl Bus {
    pub fn start() -> Self {
        let config = std::env::temp_dir().join(format!("isle-fakes-bus-{}-{}.conf", std::process::id(), next()));
        std::fs::write(
            &config,
            r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=/tmp</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
        )
        .expect("the bus's config");
        let out = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", config.display()))
            .args(["--fork", "--nopidfile", "--print-address=1", "--print-pid=1"])
            .output()
            .expect("dbus-daemon");
        let text = String::from_utf8(out.stdout).unwrap();
        let mut lines = text.lines();
        Self {
            address: lines.next().expect("the bus's address").to_owned(),
            pid: lines.next().unwrap().trim().parse().unwrap(),
            config,
        }
    }
}

fn next() -> u32 {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Drop for Bus {
    fn drop(&mut self) {
        // SAFETY: the daemon this bus started.
        unsafe { libc::kill(self.pid, libc::SIGTERM) };
        let _ = std::fs::remove_file(&self.config);
    }
}

/// Connects to the bus at `address`.
pub async fn connect(address: &str) -> zbus::Result<Connection> {
    zbus::connection::Builder::address(address)?.build().await
}
