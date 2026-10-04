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

use isle_ipc::{test::TestProxy, version::VersionProxy, windows::WindowsProxy};

pub struct Isle {
    pub dir: PathBuf,
    bus_pid: i32,
    compositor: Child,
    clients: Vec<Child>,
    pub bus: zbus::Connection,
}

pub struct Client {
    pub lines: Arc<Mutex<Vec<String>>>,
}

impl Client {
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
    pub async fn start(name: &str) -> Self {
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
            .env("RUST_LOG", std::env::var("ISLE_TEST_LOG").unwrap_or_else(|_| "warn,isle_compositor=info".into()))
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

    pub fn client_with(&mut self, args: &[&str]) -> Client {
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
        for child in self
            .clients
            .iter_mut()
            .chain(std::iter::once(&mut self.compositor))
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = Command::new("kill").arg(self.bus_pid.to_string()).status();
        if std::env::var_os("ISLE_TEST_KEEP").is_none() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
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

