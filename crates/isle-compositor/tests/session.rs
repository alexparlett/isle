//! `isle-session` supervising the headless compositor on a private bus. Needs
//! `--features test-hooks` and `isle-session` built beside the compositor.
#![cfg(feature = "test-hooks")]

use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use isle_ipc::{
    session::SessionProxy, settings::SettingsProxy, test::TestProxy, version::VersionProxy,
};

struct Session {
    dir: PathBuf,
    bus_pid: String,
    child: Child,
    bus: zbus::Connection,
}

impl Session {
    async fn start(name: &str) -> Option<Self> {
        let compositor = PathBuf::from(env!("CARGO_BIN_EXE_isle-compositor"));
        let supervisor = compositor.with_file_name("isle-session");
        if !supervisor.exists() {
            eprintln!("skipped: build isle-session (cargo test --workspace)");
            return None;
        }
        let dir = std::env::temp_dir().join(format!("isle-session-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in ["runtime", "config", "state"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
        let out = Command::new("dbus-daemon")
            .args([
                "--session",
                "--fork",
                "--nopidfile",
                "--print-address=1",
                "--print-pid=1",
            ])
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        let mut lines = text.lines();
        let address = lines.next().unwrap().to_owned();
        let bus_pid = lines.next().unwrap().trim().to_owned();
        let child = Command::new(&supervisor)
            .args(["--", "--headless"])
            .env("ISLE_COMPOSITOR", &compositor)
            .env("ISLE_SESSION_EXPORT", "0")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .env("XDG_RUNTIME_DIR", dir.join("runtime"))
            .env("ISLE_CONFIG_DIR", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("DBUS_SESSION_BUS_ADDRESS", &address)
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let bus = zbus::connection::Builder::address(address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap();
        Some(Self {
            dir,
            bus_pid,
            child,
            bus,
        })
    }

    /// The compositor's pid once it answers, waiting for one other than `not`.
    async fn compositor_pid(&self, not: u32) -> u32 {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Ok(proxy) = VersionProxy::builder(&self.bus)
                .cache_properties(zbus::proxy::CacheProperties::No)
                .build()
                .await
                && let Ok(pid) = proxy.pid().await
                && pid != not
            {
                return pid;
            }
            assert!(Instant::now() < deadline, "the compositor did not come up");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn records(&self) -> Vec<String> {
        let mut paths: Vec<PathBuf> = fs::read_dir(self.dir.join("state/isle/crashes"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        paths.sort();
        paths
            .iter()
            .map(|p| fs::read_to_string(p).unwrap())
            .collect()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .arg(self.child.id().to_string())
            .status();
        let _ = self.child.wait();
        let _ = Command::new("kill").arg(&self.bus_pid).status();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_core_panic_restarts_the_compositor_and_leaves_a_record() {
    let Some(session) = Session::start("panic").await else {
        return;
    };
    let first = session.compositor_pid(0).await;
    let _ = TestProxy::new(&session.bus)
        .await
        .unwrap()
        .panic_core()
        .await;
    let second = session.compositor_pid(first).await;
    assert_ne!(first, second);
    let records = session.records();
    assert_eq!(records.len(), 1, "{records:?}");
    let record = &records[0];
    assert!(record.contains("kind: panic"), "{record}");
    assert!(record.contains("a core panic injected"), "{record}");
    assert!(record.contains("exit: exit code 101"), "{record}");
    assert!(record.contains("\nlog:\n"), "{record}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_killed_compositor_is_restarted_and_recorded() {
    let Some(session) = Session::start("kill").await else {
        return;
    };
    let first = session.compositor_pid(0).await;
    Command::new("kill")
        .args(["-9", &first.to_string()])
        .status()
        .unwrap();
    session.compositor_pid(first).await;
    let records = session.records();
    assert!(
        records
            .iter()
            .any(|r| r.contains("kind: exit") && r.contains("exit: signal 9")),
        "{records:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_compositor_killed_while_locked_comes_back_locked() {
    let Some(session) = Session::start("kill-locked").await else {
        return;
    };
    let first = session.compositor_pid(0).await;
    SessionProxy::new(&session.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    let marker = match std::env::var("XDG_SESSION_ID") {
        Ok(id) if !id.is_empty() => format!("locked-{id}"),
        _ => "locked".to_owned(),
    };
    assert!(session.dir.join("runtime/isle").join(marker).exists());
    Command::new("kill")
        .args(["-9", &first.to_string()])
        .status()
        .unwrap();
    session.compositor_pid(first).await;
    let locked = SessionProxy::builder(&session.bus)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .unwrap()
        .locked()
        .await
        .unwrap();
    assert!(locked, "the restarted compositor is not locked");
    let test = TestProxy::new(&session.bus).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let kinds = test.frame_kinds("HEADLESS-1").await.unwrap();
        if !kinds.is_empty() {
            assert!(!kinds.iter().any(|k| k == "client"), "{kinds:?}");
            break;
        }
        assert!(Instant::now() < deadline, "no frame after the restart");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn three_quick_deaths_start_the_next_compositor_on_the_defaults() {
    let Some(session) = Session::start("safe").await else {
        return;
    };
    fs::write(
        session.dir.join("config/settings.kdl"),
        "keyboard { repeat-rate 40; }\n",
    )
    .unwrap();
    let mut pid = session.compositor_pid(0).await;
    for _ in 0..3 {
        let _ = TestProxy::new(&session.bus)
            .await
            .unwrap()
            .panic_core()
            .await;
        pid = session.compositor_pid(pid).await;
    }
    let settings = SettingsProxy::new(&session.bus).await.unwrap();
    let status = settings.status().await.unwrap();
    assert!(
        !status.ok && status.message.contains("default settings"),
        "{status:?}"
    );
    let rate = settings.get("keyboard.repeat-rate").await.unwrap();
    assert_eq!(i64::try_from(&*rate).unwrap(), 25);

    settings.use_mine().await.unwrap();
    let rate = settings.get("keyboard.repeat-rate").await.unwrap();
    assert_eq!(i64::try_from(&*rate).unwrap(), 40);
    assert!(settings.status().await.unwrap().ok);
}

#[tokio::test(flavor = "multi_thread")]
async fn quitting_ends_the_session_cleanly() {
    let Some(mut session) = Session::start("quit").await else {
        return;
    };
    session.compositor_pid(0).await;
    isle_ipc::session::SessionProxy::new(&session.bus)
        .await
        .unwrap()
        .quit()
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = session.child.try_wait().unwrap() {
            assert!(status.success(), "{status:?}");
            break;
        }
        assert!(Instant::now() < deadline, "isle-session did not end");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(session.records().is_empty());
}
