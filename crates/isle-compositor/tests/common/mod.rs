//! The compositor run headless on a private bus, for the integration tests.
#![allow(dead_code)]

use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use isle_ipc::{test::TestProxy, windows::WindowsProxy};

pub struct Isle {
    pub dir: PathBuf,
    bus_address: String,
    bus_pid: i32,
    system_address: String,
    system_pid: i32,
    compositor: Child,
    clients: Vec<Arc<Mutex<Child>>>,
    /// The private PipeWire's processes, when a test started it.
    audio: Vec<Arc<Mutex<Child>>>,
    pub bus: zbus::Connection,
}

pub struct Client {
    pub lines: Arc<Mutex<Vec<String>>>,
    stdin: Mutex<std::process::ChildStdin>,
    child: Arc<Mutex<Child>>,
}

impl Client {
    /// Sends the client a command (`src/bin/test_client.rs` lists them).
    pub fn send(&self, command: &str) {
        use std::io::Write;
        let mut stdin = self.stdin.lock().unwrap();
        writeln!(stdin, "{command}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn pid(&self) -> i32 {
        self.child.lock().unwrap().id() as i32
    }

    /// Whether the client's process has ended.
    pub fn exited(&self) -> bool {
        matches!(self.child.lock().unwrap().try_wait(), Ok(Some(_)))
    }

    pub fn printed(&self, line: &str) -> bool {
        self.lines.lock().unwrap().iter().any(|l| l == line)
    }

    /// Forgets what the client printed so far, so a later wait sees only what follows.
    pub fn clear(&self) {
        self.lines.lock().unwrap().clear();
    }

    pub async fn wait_for(&self, wanted: &str) {
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
    /// A compositor on the plain backdrop the older tests sample, `#1c1c1f`.
    pub async fn start(name: &str) -> Self {
        Self::start_with(name, "wallpaper {\n    source \"#1c1c1f\"\n}\n").await
    }

    /// A compositor started with `settings` as its settings.kdl.
    pub async fn start_with(name: &str, settings: &str) -> Self {
        Self::start_prepared(name, settings, |_| {}).await
    }

    /// As `start_with`, with `prepare` writing into the test's directory before it starts.
    pub async fn start_prepared(
        name: &str,
        settings: &str,
        prepare: impl FnOnce(&std::path::Path),
    ) -> Self {
        Self::start_env(name, settings, prepare, |_| Vec::new()).await
    }

    /// As `start_prepared`, with variables for the compositor from the test's directory.
    pub async fn start_env(
        name: &str,
        settings: &str,
        prepare: impl FnOnce(&std::path::Path),
        env: impl FnOnce(&std::path::Path) -> Vec<(String, String)>,
    ) -> Self {
        let dir = std::env::temp_dir().join(format!("isle-headless-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in [
            "runtime",
            "config",
            "state",
            "data/applications",
            "data-system",
        ] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
        if !settings.is_empty() {
            fs::write(dir.join("config/settings.kdl"), settings).unwrap();
        }
        prepare(&dir);

        let (bus_address, bus_pid) = private_bus();
        // A second private bus stands in for the system bus, where the fake daemons live.
        let (system_address, system_pid) = private_bus();

        let log = fs::File::create(dir.join("compositor.log")).unwrap();
        let extra = env(&dir);
        let compositor = Command::new(env!("CARGO_BIN_EXE_isle-compositor"))
            .envs(extra)
            .arg("--headless")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .env("XDG_RUNTIME_DIR", dir.join("runtime"))
            .env("ISLE_CONFIG_DIR", dir.join("config"))
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .env("XDG_DATA_DIRS", dir.join("data-system"))
            // The host's own layout must not reach the tests.
            .env("XKB_DEFAULT_LAYOUT", "us")
            .env("DBUS_SESSION_BUS_ADDRESS", &bus_address)
            .env("DBUS_SYSTEM_BUS_ADDRESS", &system_address)
            .env(
                "RUST_LOG",
                std::env::var("ISLE_TEST_LOG")
                    .unwrap_or_else(|_| "warn,isle_compositor=info".into()),
            )
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
            bus_address: bus_address.clone(),
            bus_pid,
            system_address,
            system_pid,
            compositor,
            clients: Vec::new(),
            audio: Vec::new(),
            bus,
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            // The bus answers from the worker before the loop has its Wayland socket; a call
            // answered by the loop shows both are up.
            let ready = match WindowsProxy::new(&isle.bus).await {
                Ok(proxy) => proxy.list().await.is_ok(),
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
        // Screenshots taken as a window maps see it whole; the motion tests turn it back on.
        isle.test().await.window_motion(false).await.unwrap();
        isle
    }

    pub fn compositor_pid(&self) -> i32 {
        self.compositor.id() as i32
    }

    pub fn bus_address(&self) -> String {
        self.bus_address.clone()
    }

    /// The private bus the compositor takes for the system bus.
    pub fn system_address(&self) -> String {
        self.system_address.clone()
    }

    pub fn log(&self) -> String {
        fs::read_to_string(self.dir.join("compositor.log")).unwrap_or_default()
    }

    pub fn socket(&self) -> String {
        fs::read_dir(self.dir.join("runtime"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
            .expect("a Wayland socket")
    }

    pub fn client(&mut self, app_id: &str, colour: &str) -> Client {
        self.client_with(&["--app-id", app_id, "--colour", colour])
    }

    /// An X11 client on Xwayland, once its window manager runs.
    pub async fn x11_client(&mut self, args: &[&str]) -> Client {
        let test = self.test().await;
        let deadline = Instant::now() + Duration::from_secs(10);
        let display = loop {
            let display = test.x_display().await.unwrap();
            if display >= 0 {
                break display;
            }
            assert!(
                Instant::now() < deadline,
                "Xwayland never came up: {}",
                self.log()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        self.spawn(
            env!("CARGO_BIN_EXE_isle-test-x11-client"),
            args,
            &[("DISPLAY", format!(":{display}"))],
        )
    }

    pub fn client_with(&mut self, args: &[&str]) -> Client {
        let socket = self.socket();
        self.spawn(
            env!("CARGO_BIN_EXE_isle-test-client"),
            args,
            &[("WAYLAND_DISPLAY", socket)],
        )
    }

    /// A Wayland test client with more in its environment.
    pub fn client_env(&mut self, app_id: &str, env: &[(&str, &str)]) -> Client {
        let socket = self.socket();
        let mut envs = vec![("WAYLAND_DISPLAY", socket)];
        envs.extend(env.iter().map(|(k, v)| (*k, v.to_string())));
        self.spawn(
            env!("CARGO_BIN_EXE_isle-test-client"),
            &["--app-id", app_id, "--colour", "3366ff"],
            &envs,
        )
    }

    fn spawn(&mut self, program: &str, args: &[&str], envs: &[(&str, String)]) -> Client {
        let mut child = Command::new(program)
            .args(args)
            .env("XDG_RUNTIME_DIR", self.dir.join("runtime"))
            .env("DBUS_SESSION_BUS_ADDRESS", &self.bus_address)
            .env("DBUS_SYSTEM_BUS_ADDRESS", &self.system_address)
            .envs(envs.iter().map(|(k, v)| (*k, v.as_str())))
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("isle-test-client");
        let stdout = child.stdout.take().unwrap();
        let stdin = Mutex::new(child.stdin.take().unwrap());
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        let child = Arc::new(Mutex::new(child));
        self.clients.push(child.clone());
        Client {
            lines,
            stdin,
            child,
        }
    }

    pub async fn windows(&self) -> Vec<isle_ipc::WindowInfo> {
        WindowsProxy::new(&self.bus)
            .await
            .unwrap()
            .list()
            .await
            .unwrap()
    }

    pub async fn wait_for_window(&self, app_id: &str) -> isle_ipc::WindowInfo {
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

    pub async fn test(&self) -> TestProxy<'static> {
        TestProxy::new(&self.bus).await.unwrap()
    }
}

impl Drop for Isle {
    fn drop(&mut self) {
        for child in &self.clients {
            let mut child = child.lock().unwrap();
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = self.compositor.kill();
        let _ = self.compositor.wait();
        for pid in [self.bus_pid, self.system_pid] {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
        if std::env::var_os("ISLE_TEST_KEEP").is_none() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// A private message bus, as its address and the daemon's pid.
fn private_bus() -> (String, i32) {
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
    let address = lines.next().unwrap().to_owned();
    let pid = lines.next().unwrap().trim().parse().unwrap();
    (address, pid)
}

pub async fn wait_until<F, Fut>(mut check: F)
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

/// A screenshot of one output, read back.
pub struct Shot {
    pub width: usize,
    pub height: usize,
    buf: Vec<u8>,
}

impl Shot {
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.width + x) * 4;
        [self.buf[i], self.buf[i + 1], self.buf[i + 2]]
    }
}

impl Isle {
    pub async fn shot(&self, output: &str, name: &str) -> Shot {
        let path = self.dir.join(format!("{name}.png"));
        self.test()
            .await
            .screenshot(output, path.to_str().unwrap())
            .await
            .unwrap();
        let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(&path).unwrap()));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        Shot {
            width: info.width as usize,
            height: info.height as usize,
            buf,
        }
    }

    pub async fn window(&self, id: u64) -> isle_ipc::WindowInfo {
        self.windows()
            .await
            .into_iter()
            .find(|w| w.id == id)
            .unwrap_or_else(|| panic!("no window {id}"))
    }

    pub async fn windows_proxy(&self) -> WindowsProxy<'static> {
        WindowsProxy::new(&self.bus).await.unwrap()
    }
}

/// evdev codes.
pub mod keys {
    pub const CTRL: u32 = 29;
    pub const ALT: u32 = 56;
    pub const SUPER: u32 = 125;
    pub const SHIFT: u32 = 42;
    pub const LEFT: u32 = 105;
    pub const RIGHT: u32 = 106;
    pub const C: u32 = 46;
    pub const F: u32 = 33;
    pub const R: u32 = 19;
    pub const U: u32 = 22;
    pub const B: u32 = 48;
}

impl Isle {
    /// Presses `keys` in order and releases them in reverse.
    pub async fn chord(&self, keys: &[u32]) {
        let test = self.test().await;
        for key in keys {
            test.key(*key, true).await.unwrap();
        }
        for key in keys.iter().rev() {
            test.key(*key, false).await.unwrap();
        }
    }

    /// Presses at `from`, which the client turns into its own move or resize, moves to `to`
    /// in steps, and releases.
    pub async fn drag(&self, client: &Client, from: (f64, f64), to: (f64, f64)) {
        let test = self.test().await;
        test.pointer_to(from.0, from.1).await.unwrap();
        test.button(272, true).await.unwrap();
        client.wait_for("button 272 pressed").await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        for step in 1..=4 {
            let f = step as f64 / 4.0;
            test.pointer_to(from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f)
                .await
                .unwrap();
        }
        test.button(272, false).await.unwrap();
    }
}

/// Control Centre, read and driven through the island's status and `Test1.Panel`.
impl Isle {
    /// The first output's island as `Island1.Status` gives it.
    pub async fn island_output(&self) -> serde_json::Value {
        let text = isle_ipc::island::IslandProxy::new(&self.bus)
            .await
            .unwrap()
            .status()
            .await
            .unwrap();
        serde_json::from_str::<serde_json::Value>(&text).unwrap()["outputs"][0].clone()
    }

    /// Opens Control Centre pinned, at `page` when given.
    pub async fn open_panel(&self, page: Option<&str>) {
        if self.island_output().await["panel"].is_null() {
            self.chord(&[29, 66]).await;
            wait_until(|| async { !self.island_output().await["panel"].is_null() }).await;
        }
        if let Some(page) = page {
            self.panel(serde_json::json!({ "page": page })).await;
            wait_until(|| async { self.island_output().await["panel"]["page"] == page }).await;
        }
    }

    pub async fn panel(&self, action: serde_json::Value) {
        self.test().await.panel(&action.to_string()).await.unwrap();
    }

    /// A module of the open panel, or Null when it is not in the grid.
    pub async fn module(&self, id: &str) -> serde_json::Value {
        self.island_output().await["panel"]["modules"]
            .as_array()
            .and_then(|m| m.iter().find(|m| m["id"] == id).cloned())
            .unwrap_or_default()
    }

    /// The open page's rows.
    pub async fn rows(&self) -> Vec<serde_json::Value> {
        self.island_output().await["panel"]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// The glyphs the first output's pill shows, by name.
    pub async fn glyphs(&self) -> Vec<String> {
        self.island_output().await["glyphs"]
            .as_array()
            .map(|g| {
                g.iter()
                    .filter_map(|g| g.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Isle {
    /// The power service has reported once.
    pub async fn module_state_ready(&self) -> bool {
        let text = isle_ipc::services::ServicesProxy::new(&self.bus)
            .await
            .unwrap()
            .status()
            .await
            .unwrap();
        let status: serde_json::Value = serde_json::from_str(&text).unwrap();
        status["power"]["available"] == true
    }
}

/// A private PipeWire with pipewire-pulse in the test's runtime directory: null speakers,
/// a microphone, a camera and a call to link it to, with no session manager.
impl Isle {
    /// Starts it; false where PipeWire is not installed, for the test to skip.
    pub async fn start_audio(&mut self) -> bool {
        if Command::new("pipewire").arg("--version").output().is_err() {
            return false;
        }
        let conf = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/pipewire.conf");
        let runtime = self.dir.join("runtime");
        for (program, args) in [("pipewire", vec!["-c", conf]), ("pipewire-pulse", vec![])] {
            let log = fs::File::create(self.dir.join(format!("{program}.log"))).unwrap();
            let child = Command::new(program)
                .args(&args)
                .env("XDG_RUNTIME_DIR", &runtime)
                .env("DBUS_SESSION_BUS_ADDRESS", "disabled:")
                .env_remove("PULSE_SERVER")
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .expect(program);
            let child = Arc::new(Mutex::new(child));
            self.clients.push(child.clone());
            self.audio.push(child);
            if program == "pipewire" {
                let socket = runtime.join("pipewire-0");
                wait_until(|| async { socket.exists() }).await;
            }
        }
        let pulse = runtime.join("pulse/native");
        wait_until(|| async { pulse.exists() }).await;
        true
    }

    /// Stops PipeWire and pipewire-pulse, as a crash or restart would.
    pub fn stop_audio(&mut self) {
        for child in self.audio.drain(..) {
            let mut child = child.lock().unwrap();
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_file(self.dir.join("runtime/pulse/native"));
        let _ = fs::remove_file(self.dir.join("runtime/pipewire-0"));
    }

    /// A PipeWire or Pulse tool run against the private server.
    pub fn audio_tool(&self, program: &str, args: &[&str]) -> std::process::Output {
        Command::new(program)
            .args(args)
            .env("XDG_RUNTIME_DIR", self.dir.join("runtime"))
            .env_remove("PULSE_SERVER")
            .output()
            .expect(program)
    }

    /// A tool left running against the private server, killed with the test.
    pub fn audio_process(&mut self, program: &str, args: &[&str]) -> Arc<Mutex<Child>> {
        let child = Command::new(program)
            .args(args)
            .env("XDG_RUNTIME_DIR", self.dir.join("runtime"))
            .env_remove("PULSE_SERVER")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect(program);
        let child = Arc::new(Mutex::new(child));
        self.clients.push(child.clone());
        child
    }

    pub async fn services(&self) -> serde_json::Value {
        let text = isle_ipc::services::ServicesProxy::new(&self.bus)
            .await
            .unwrap()
            .status()
            .await
            .unwrap();
        serde_json::from_str(&text).unwrap()
    }
}

impl Isle {
    /// A screenshot of the first output, kept in `ISLE_SHOT_DIR` when it is set, for looking at.
    pub async fn keep_shot(&self, name: &str) -> Shot {
        let shot = self.shot("HEADLESS-1", name).await;
        if let Ok(dir) = std::env::var("ISLE_SHOT_DIR") {
            let _ = fs::copy(
                self.dir.join(format!("{name}.png")),
                format!("{dir}/{name}.png"),
            );
        }
        shot
    }

    /// Waits for Control Centre's shape to settle.
    pub async fn panel_settled(&self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let i = self.island_output().await;
            let t = &i["target"];
            if !i["panel"].is_null()
                && (i["height"].as_f64().unwrap() - t[1].as_f64().unwrap()).abs() < 0.5
            {
                return;
            }
            assert!(Instant::now() < deadline, "never settled: {i}");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    }
}

impl Isle {
    /// Main-loop stalls logged since `from` (a length of the log) in work the worker sent the
    /// loop, which is where a daemon's answer would stall it.
    pub fn job_stalls_since(&self, from: usize) -> Vec<String> {
        self.log()[from..]
            .lines()
            .filter(|l| l.contains("main loop stalled") && l.contains("worker-job"))
            .map(str::to_owned)
            .collect()
    }
}
