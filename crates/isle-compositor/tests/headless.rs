//! The compositor run headless on a private bus, driven through the Test interface with
//! real Wayland clients. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use isle_ipc::{
    displays::DisplaysProxy, settings::SettingsProxy, test::TestProxy, version::VersionProxy,
    windows::WindowsProxy,
};

struct Isle {
    dir: PathBuf,
    bus_pid: i32,
    compositor: Child,
    clients: Vec<Child>,
    bus: zbus::Connection,
}

struct Client {
    lines: Arc<Mutex<Vec<String>>>,
}

impl Client {
    async fn wait_for(&self, wanted: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.lines.lock().unwrap().iter().any(|l| l == wanted) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!(
            "the client never printed {wanted:?}; it printed {:?}",
            self.lines.lock().unwrap()
        );
    }
}

impl Isle {
    async fn start(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("isle-headless-{name}-{}", std::process::id()));
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
            .expect("dbus-daemon");
        let text = String::from_utf8(out.stdout).unwrap();
        let mut lines = text.lines();
        let bus_address = lines.next().unwrap().to_owned();
        let bus_pid = lines.next().unwrap().trim().parse().unwrap();

        let log = fs::File::create(dir.join("compositor.log")).unwrap();
        let compositor = Command::new(env!("CARGO_BIN_EXE_isle-compositor"))
            .arg("--headless")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .env("XDG_RUNTIME_DIR", dir.join("runtime"))
            .env("ISLE_CONFIG_DIR", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("DBUS_SESSION_BUS_ADDRESS", &bus_address)
            .env("RUST_LOG", "warn,isle_compositor=info")
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .expect("isle-compositor");

        let bus = zbus::connection::Builder::address(bus_address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap();
        let isle = Self {
            dir,
            bus_pid,
            compositor,
            clients: Vec::new(),
            bus,
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let ready = match VersionProxy::new(&isle.bus).await {
                Ok(proxy) => proxy.commit().await.is_ok(),
                Err(_) => false,
            };
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the compositor never came up: {}",
                isle.log()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        isle
    }

    fn log(&self) -> String {
        fs::read_to_string(self.dir.join("compositor.log")).unwrap_or_default()
    }

    fn socket(&self) -> String {
        fs::read_dir(self.dir.join("runtime"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
            .expect("a Wayland socket")
    }

    fn client(&mut self, app_id: &str, colour: &str) -> Client {
        self.client_with(&["--app-id", app_id, "--colour", colour])
    }

    fn client_with(&mut self, args: &[&str]) -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_isle-test-client"))
            .args(args)
            .env("XDG_RUNTIME_DIR", self.dir.join("runtime"))
            .env("WAYLAND_DISPLAY", self.socket())
            .stdout(Stdio::piped())
            .spawn()
            .expect("isle-test-client");
        let stdout = child.stdout.take().unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        self.clients.push(child);
        Client { lines }
    }

    async fn windows(&self) -> Vec<isle_ipc::WindowInfo> {
        WindowsProxy::new(&self.bus)
            .await
            .unwrap()
            .list()
            .await
            .unwrap()
    }

    async fn wait_for_window(&self, app_id: &str) -> isle_ipc::WindowInfo {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(w) = self
                .windows()
                .await
                .into_iter()
                .find(|w| w.app_id == app_id)
            {
                return w;
            }
            assert!(
                Instant::now() < deadline,
                "{app_id} never mapped: {}",
                self.log()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    async fn test(&self) -> TestProxy<'static> {
        TestProxy::new(&self.bus).await.unwrap()
    }
}

impl Drop for Isle {
    fn drop(&mut self) {
        for child in self
            .clients
            .iter_mut()
            .chain(std::iter::once(&mut self.compositor))
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = Command::new("kill").arg(self.bus_pid.to_string()).status();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

async fn wait_until<F, Fut>(mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + Duration::from_secs(5);
    while !check().await {
        assert!(Instant::now() < deadline, "timed out");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_maps_focused_and_closes_by_its_id() {
    let mut isle = Isle::start("map").await;
    let client = isle.client("isle.test.Map", "3366ff");
    let window = isle.wait_for_window("isle.test.Map").await;
    assert!(window.focused);
    assert_eq!(window.output, "HEADLESS-1");
    client.wait_for("keyboard-enter").await;

    let windows = WindowsProxy::new(&isle.bus).await.unwrap();
    assert!(windows.close(window.id).await.unwrap());
    client.wait_for("closed").await;
    assert!(!windows.close(9999).await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_and_clicks_reach_the_window_under_the_pointer() {
    let mut isle = Isle::start("input").await;
    let first = isle.client("isle.test.First", "ff0000");
    isle.wait_for_window("isle.test.First").await;
    let second = isle.client("isle.test.Second", "00ff00");
    isle.wait_for_window("isle.test.Second").await;
    second.wait_for("keyboard-enter").await;

    // Both open centred, the second on top, so a click at the centre is on the second.
    let test = isle.test().await;
    test.pointer_to(960.0, 540.0).await.unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    second.wait_for("button 272 pressed").await;
    second.wait_for("button 272 released").await;

    test.key(30, true).await.unwrap();
    test.key(30, false).await.unwrap();
    second.wait_for("key 30 pressed").await;
    second.wait_for("key 30 released").await;
    assert!(
        !first
            .lines
            .lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("key "))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_still_desktop_presents_no_frames() {
    let mut isle = Isle::start("idle").await;
    let _client = isle.client("isle.test.Idle", "888888");
    isle.wait_for_window("isle.test.Idle").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let test = isle.test().await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    assert!(before > 0);
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_request_during_a_stall_fails_within_a_second_and_the_stall_is_logged() {
    let isle = Isle::start("stall").await;
    let test = isle.test().await;
    test.stall(2500).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    let err = WindowsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
    assert!(err.to_string().contains("TimedOut"), "{err}");
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(isle.log().contains("main loop stalled"), "{}", isle.log());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_hand_edit_keeps_the_last_good_settings_and_refuses_sets() {
    let isle = Isle::start("settings").await;
    let file = isle.dir.join("config/settings.kdl");
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let rate = || async {
        let v = settings.get("keyboard.repeat-rate").await.unwrap();
        i64::try_from(&*v).unwrap()
    };

    fs::write(&file, "// mine\nkeyboard {\n    repeat-rate 40\n}\n").unwrap();
    wait_until(|| async { rate().await == 40 }).await;

    fs::write(&file, "// mine\nkeyboard {\n    repeat-rate 50\n").unwrap();
    wait_until(|| async { !settings.status().await.unwrap().ok }).await;
    assert_eq!(settings.status().await.unwrap().line, 2);
    assert_eq!(rate().await, 40);
    let refused = settings
        .set("keyboard.repeat-rate", &zvariant::Value::from(60i64))
        .await;
    assert!(refused.is_err());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "// mine\nkeyboard {\n    repeat-rate 50\n"
    );

    fs::write(
        &file,
        "// mine\nkeyboard {\n    repeat-rate 50 // fast\n}\n",
    )
    .unwrap();
    wait_until(|| async { rate().await == 50 }).await;
    settings
        .set("keyboard.repeat-rate", &zvariant::Value::from(60i64))
        .await
        .unwrap();
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "// mine\nkeyboard {\n    repeat-rate 60 // fast\n}\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_screenshot_shows_the_window_over_the_backdrop() {
    let mut isle = Isle::start("shot").await;
    let _client = isle.client("isle.test.Shot", "3366ff");
    isle.wait_for_window("isle.test.Shot").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let path = isle.dir.join("shot.png");
    isle.test()
        .await
        .screenshot("HEADLESS-1", path.to_str().unwrap())
        .await
        .unwrap();
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(&path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (1920, 1080));
    let at = |x: usize, y: usize| {
        let i = (y * 1920 + x) * 4;
        [buf[i], buf[i + 1], buf[i + 2]]
    };
    assert_eq!(at(960, 540), [0x33, 0x66, 0xff], "the window at the centre");
    assert_eq!(at(10, 10), [28, 28, 31], "the backdrop in the corner");
}

#[tokio::test(flavor = "multi_thread")]
async fn outputs_come_and_go_through_the_test_interface() {
    let isle = Isle::start("outputs").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Headless", "2", 1280, 720)
        .await
        .unwrap();
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let list = displays.list().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!((list[1].x, list[1].width), (1920, 1280));
    assert!(test.remove_output("HEADLESS-2").await.unwrap());
    assert_eq!(displays.list().await.unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_leaves_with_its_output_whole_and_returns_with_it() {
    let mut isle = Isle::start("kvm").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "AOC", "U34G2G4R3", "7", 1280, 720)
        .await
        .unwrap();
    // New windows open on the output under the pointer.
    test.pointer_to(1920.0 + 640.0, 360.0).await.unwrap();
    let _client = isle.client("isle.test.Kvm", "ffaa00");
    let before = isle.wait_for_window("isle.test.Kvm").await;
    assert_eq!(before.output, "HEADLESS-2");

    assert!(test.remove_output("HEADLESS-2").await.unwrap());
    let moved = isle.wait_for_window("isle.test.Kvm").await;
    assert_eq!(moved.output, "HEADLESS-1");
    assert!(moved.x >= 0 && moved.y >= 0, "{moved:?}");
    assert!(
        moved.x + moved.width <= 1920 && moved.y + moved.height <= 1080,
        "{moved:?}"
    );

    test.add_output("HEADLESS-3", "AOC", "U34G2G4R3", "7", 1280, 720)
        .await
        .unwrap();
    let back = isle.wait_for_window("isle.test.Kvm").await;
    assert_eq!(
        back.output, "HEADLESS-3",
        "the same display, by its identity"
    );
    assert_eq!((back.x - 1920, back.y), (before.x - 1920, before.y));
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_output_the_session_continues_and_windows_return() {
    let mut isle = Isle::start("dark").await;
    let _client = isle.client("isle.test.Dark", "00aaff");
    isle.wait_for_window("isle.test.Dark").await;
    let test = isle.test().await;
    assert!(test.remove_output("HEADLESS-1").await.unwrap());
    let dark = isle.wait_for_window("isle.test.Dark").await;
    assert_eq!(dark.output, "");
    test.add_output("HEADLESS-9", "Isle", "Other", "9", 800, 600)
        .await
        .unwrap();
    let lit = isle.wait_for_window("isle.test.Dark").await;
    assert_eq!(lit.output, "HEADLESS-9", "{lit:?}");
    assert!(lit.x >= 0 && lit.x + lit.width <= 800, "{lit:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_layer_with_an_exclusive_zone_keeps_new_windows_clear_and_takes_clicks() {
    let mut isle = Isle::start("layer").await;
    let bar = isle.client_with(&[
        "--layer",
        "top",
        "--anchor",
        "top",
        "--size",
        "10x100",
        "--exclusive",
        "100",
        "--colour",
        "ffffff",
        "--app-id",
        "isle.test.Bar",
    ]);
    bar.wait_for("configured 1920 100").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _client = isle.client("isle.test.Below", "3366ff");
    let window = isle.wait_for_window("isle.test.Below").await;
    assert_eq!(window.y, 100 + (980 - window.height) / 2, "{window:?}");

    let path = isle.dir.join("layer.png");
    let test = isle.test().await;
    test.screenshot("HEADLESS-1", path.to_str().unwrap())
        .await
        .unwrap();
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(&path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    let i = (50 * 1920 + 960) * 4;
    assert_eq!(&buf[i..i + 3], &[255, 255, 255], "the bar across the top");

    test.pointer_to(960.0, 50.0).await.unwrap();
    test.button(272, true).await.unwrap();
    bar.wait_for("button 272 pressed").await;
    bar.wait_for("keyboard-enter").await;
}
