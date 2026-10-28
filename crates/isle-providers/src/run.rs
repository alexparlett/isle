//! Running a provider: once on an interval, or as a stream, sandboxed, with its time limit
//! and output cap (`platform.md` Providers 4; `p17-widgets.md` choice 17).

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::mpsc,
};

use crate::{
    grant::Grant,
    manifest::Manifest,
    proxy::{self, Proxy},
    sandbox::{self, Plan},
    template::{self, Data},
};

/// What runs providers, and the limits each run has.
#[derive(Clone)]
pub struct Runner {
    /// `isle-provider-init`, beside the running build.
    pub init: PathBuf,
    /// Where the proxies' sockets go.
    pub sockets: PathBuf,
    /// Where each provider's own folder goes, by id.
    pub states: PathBuf,
    /// The ports a granted host is reached on.
    pub ports: Vec<u16>,
    pub limit: Duration,
    /// The most an answer may be, in bytes.
    pub cap: usize,
    /// The user's home, against which `~` is read.
    pub home: PathBuf,
}

/// One run's result: its answer or why there is none, and the hosts it was refused.
#[derive(Debug)]
pub struct Outcome {
    pub answer: Result<Data, String>,
    pub denied: Vec<String>,
}

static RUNS: AtomicU64 = AtomicU64::new(0);

/// The helper beside the running program, or as the environment names it for tests.
pub fn init_helper() -> PathBuf {
    std::env::var_os("ISLE_PROVIDER_INIT")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok().map(|e| e.with_file_name("isle-provider-init")))
        .unwrap_or_else(|| PathBuf::from("isle-provider-init"))
}

/// A memory limit for the sandbox: generous, so a runtime that reserves address space
/// still starts, and a runaway still stops.
const DATA_LIMIT: u64 = 1 << 30;

impl Runner {
    pub fn new(sockets: PathBuf, states: PathBuf) -> Self {
        Self {
            init: init_helper(),
            sockets,
            states,
            ports: vec![80, 443],
            limit: Duration::from_secs(20),
            cap: 64 * 1024,
            home: PathBuf::from(std::env::var_os("HOME").unwrap_or_default()),
        }
    }

    /// What a run of `manifest` in `folder` needs open around it: the network proxy and bus
    /// proxies for what is granted.
    async fn prepare(&self, manifest: &Manifest, folder: &Path, granted: &[Grant]) -> Result<(Plan, Guards), String> {
        let declared = manifest.declared();
        let grants: Vec<Grant> = declared.into_iter().filter(|g| granted.contains(g)).collect();
        let n = RUNS.fetch_add(1, Ordering::Relaxed);
        std::fs::create_dir_all(&self.sockets).map_err(|e| e.to_string())?;
        let state = self.states.join(&manifest.id);
        std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
        let hosts: Vec<String> = grants
            .iter()
            .filter_map(|g| match g {
                Grant::Connect(h) => Some(h.clone()),
                _ => None,
            })
            .collect();
        let mut guards = Guards::default();
        let proxy_path = self.sockets.join(format!("{}-{n}.proxy", manifest.id));
        if !hosts.is_empty() {
            guards.proxy = Some(proxy::serve(&proxy_path, hosts, self.ports.clone()).map_err(|e| e.to_string())?);
        }
        let names = |system: bool| -> Vec<String> {
            grants
                .iter()
                .filter_map(|g| match (g, system) {
                    (Grant::Bus(n), false) | (Grant::SystemBus(n), true) => Some(n.clone()),
                    _ => None,
                })
                .collect()
        };
        let mut bus = None;
        let mut system_bus = None;
        for (system, names) in [(false, names(false)), (true, names(true))] {
            if names.is_empty() {
                continue;
            }
            let address = if system {
                std::env::var("DBUS_SYSTEM_BUS_ADDRESS").unwrap_or_else(|_| "unix:path=/run/dbus/system_bus_socket".into())
            } else {
                std::env::var("DBUS_SESSION_BUS_ADDRESS").map_err(|_| "No session bus".to_owned())?
            };
            let path = self.sockets.join(format!("{}-{n}.{}", manifest.id, if system { "system" } else { "bus" }));
            let child = bus_proxy(&address, &path, &names).await?;
            guards.buses.push((child, path.clone()));
            if system {
                system_bus = Some(path);
            } else {
                bus = Some(path);
            }
        }
        Ok((
            Plan {
                folder: folder.to_owned(),
                state,
                home: self.home.clone(),
                grants,
                init: self.init.clone(),
                proxy: guards.proxy.as_ref().map(|p| p.path.clone()),
                bus,
                system_bus,
                argv: manifest.argv(folder),
                python: manifest.script.is_some(),
            },
            guards,
        ))
    }

    fn spawn(&self, plan: &Plan) -> Result<Child, String> {
        let mut command = Command::new("bwrap");
        command
            .args(sandbox::args(plan))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // SAFETY: setrlimit is async-signal-safe and touches nothing of the parent's.
        unsafe {
            command.pre_exec(|| {
                let limit = rustix::process::Rlimit {
                    current: Some(DATA_LIMIT),
                    maximum: Some(DATA_LIMIT),
                };
                let _ = rustix::process::setrlimit(rustix::process::Resource::Data, limit);
                Ok(())
            });
        }
        command.spawn().map_err(|e| format!("bwrap could not start: {e}"))
    }

    /// Runs it once with `input` on stdin and reads its one answer.
    pub async fn once(&self, manifest: &Manifest, folder: &Path, granted: &[Grant], input: &str) -> Outcome {
        let (plan, guards) = match self.prepare(manifest, folder, granted).await {
            Ok(p) => p,
            Err(e) => return Outcome { answer: Err(e), denied: Vec::new() },
        };
        let answer = async {
            let mut child = self.spawn(&plan)?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(input.as_bytes()).await;
            }
            let (mut out, mut err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
            let cap = self.cap;
            let run = async {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                let read_out = async {
                    let mut buf = [0u8; 8192];
                    loop {
                        let n = out.read(&mut buf).await.unwrap_or(0);
                        if n == 0 {
                            return Ok(());
                        }
                        stdout.extend_from_slice(&buf[..n]);
                        if stdout.len() > cap {
                            return Err(format!("{} answered more than {} KB", manifest.name, cap / 1024));
                        }
                    }
                };
                let read_err = async {
                    let _ = (&mut err).take(cap as u64).read_to_end(&mut stderr).await;
                };
                let (read, _) = tokio::join!(read_out, read_err);
                read?;
                let status = child.wait().await.map_err(|e| e.to_string())?;
                Ok::<_, String>((status, stdout, stderr))
            };
            match tokio::time::timeout(self.limit, run).await {
                Err(_) => Err(format!("{} didn't answer. Try again later.", manifest.name)),
                Ok(Err(e)) => Err(e),
                Ok(Ok((status, stdout, stderr))) => {
                    if !status.success() {
                        return Err(reason(&stderr, status.code()));
                    }
                    let text = String::from_utf8_lossy(&stdout);
                    let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
                    template::read(manifest.template, last)
                }
            }
        }
        .await;
        Outcome {
            answer,
            denied: guards.denied(),
        }
    }

    /// Runs it as a stream: each line it writes becomes an answer sent on `answers`, until it
    /// ends or `stop` is dropped. Ends with why it stopped.
    pub async fn stream(
        &self,
        manifest: &Manifest,
        folder: &Path,
        granted: &[Grant],
        input: &str,
        answers: mpsc::Sender<Result<Data, String>>,
    ) -> Outcome {
        let (plan, guards) = match self.prepare(manifest, folder, granted).await {
            Ok(p) => p,
            Err(e) => return Outcome { answer: Err(e), denied: Vec::new() },
        };
        let ended = async {
            let mut child = self.spawn(&plan)?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(input.as_bytes()).await;
            }
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            let mut err = child.stderr.take().unwrap();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) if line.trim().is_empty() => {}
                    Ok(Some(line)) if line.len() > self.cap => {
                        let _ = answers.send(Err(format!("{} answered more than {} KB", manifest.name, self.cap / 1024))).await;
                    }
                    Ok(Some(line)) => {
                        if answers.send(template::read(manifest.template, &line)).await.is_err() {
                            return Err("stopped".to_owned());
                        }
                    }
                    _ => break,
                }
            }
            let mut stderr = Vec::new();
            let _ = (&mut err).take(self.cap as u64).read_to_end(&mut stderr).await;
            let status = child.wait().await.map_err(|e| e.to_string())?;
            Err(reason(&stderr, status.code()))
        }
        .await;
        Outcome {
            answer: ended,
            denied: guards.denied(),
        }
    }
}

/// Why it failed, as it said: the last line it wrote to stderr.
fn reason(stderr: &[u8], code: Option<i32>) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_owned())
        .unwrap_or_else(|| match code {
            Some(c) => format!("It stopped with status {c}"),
            None => "It was stopped".to_owned(),
        })
}

/// What lives as long as one run: the proxies.
#[derive(Default)]
struct Guards {
    proxy: Option<Proxy>,
    buses: Vec<(Child, PathBuf)>,
}

impl Guards {
    fn denied(&self) -> Vec<String> {
        self.proxy.as_ref().map(Proxy::denied).unwrap_or_default()
    }
}

impl Drop for Guards {
    fn drop(&mut self) {
        for (child, path) in &mut self.buses {
            let _ = child.start_kill();
            let _ = std::fs::remove_file(path);
        }
    }
}

/// `xdg-dbus-proxy` filtering `address` to `names`, ready once its socket exists.
async fn bus_proxy(address: &str, path: &Path, names: &[String]) -> Result<Child, String> {
    let _ = std::fs::remove_file(path);
    let mut command = Command::new("xdg-dbus-proxy");
    command.arg(address).arg(path).arg("--filter");
    for name in names {
        command.arg(format!("--talk={name}"));
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("xdg-dbus-proxy could not start: {e}"))?;
    for _ in 0..100 {
        if path.exists() {
            return Ok(child);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("xdg-dbus-proxy did not start".into())
}
