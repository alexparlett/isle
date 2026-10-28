//! Providers' widgets on the dashboard: which are installed, and running each only while it
//! shows (`p17-widgets.md` choices 17 and 24).

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use isle_providers::{Data, Grant, Manifest, run::Runner};
use tokio::task::JoinHandle;

use crate::state::{Isle, State};

/// A provider found on disk.
#[derive(Clone, Debug)]
pub struct Installed {
    pub manifest: Manifest,
    pub folder: PathBuf,
    /// The source it came from; none for the user's own.
    pub source: Option<String>,
    pub version_commit: Option<String>,
}

/// Where installs from sources go: `~/.local/share/isle/widgets/<id>/`.
pub fn installed_folder() -> PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
    });
    data.join("isle/widgets")
}

#[derive(serde::Deserialize)]
struct InstallNote {
    source: String,
    #[serde(default)]
    commit: Option<String>,
}

/// Every provider in the two folders, the user's own winning a clash of ids; a folder whose
/// manifest has problems is left out.
pub fn scan() -> BTreeMap<String, Installed> {
    let mut out = BTreeMap::new();
    let read = |dir: &Path, own: bool, out: &mut BTreeMap<String, Installed>| {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            let Ok(manifest) = Manifest::load(&folder) else {
                continue;
            };
            if isle_shell::dashboard::catalogue::builtin(&manifest.id).is_some() {
                continue;
            }
            let note: Option<InstallNote> = std::fs::read_to_string(folder.join(".isle-install.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok());
            let (source, version_commit) = match (own, note) {
                (false, Some(n)) => (Some(n.source), n.commit),
                _ => (None, None),
            };
            out.insert(
                manifest.id.clone(),
                Installed {
                    manifest,
                    folder,
                    source,
                    version_commit,
                },
            );
        }
    };
    read(&installed_folder(), false, &mut out);
    read(&isle_providers::manifest::own_folder(), true, &mut out);
    out
}

/// A running provider widget.
pub struct Running {
    task: JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// What a provider widget last had.
#[derive(Clone, Debug, Default)]
pub struct Answer {
    pub error: Option<String>,
    /// It kept dying and was left stopped.
    pub stopped: bool,
}

/// How a provider widget stands.
pub enum Standing<'a> {
    NotInstalled,
    NotAllowed(&'a Installed),
    Runs(&'a Installed),
}

impl Isle {
    pub fn provider_of(&self, id: &str) -> Option<&Installed> {
        self.dashboard.providers.get(isle_shell::dashboard::grid::kind_of(id))
    }

    pub fn standing(&self, id: &str) -> Standing<'_> {
        match self.provider_of(id) {
            None => Standing::NotInstalled,
            Some(p) => match &p.source {
                Some(url) if !self.settings.widget_source.get(url).is_some_and(|s| s.allowed) => {
                    Standing::NotAllowed(p)
                }
                _ => Standing::Runs(p),
            },
        }
    }

    /// What the user granted the provider, of what it declares.
    pub fn granted(&self, provider: &str) -> Vec<Grant> {
        self.settings
            .provider
            .get(provider)
            .map(|p| p.granted.iter().filter_map(|g| Grant::parse(g)).collect())
            .unwrap_or_default()
    }

    /// The JSON a provider widget is given on stdin.
    fn provider_input(&self, id: &str, size: (i32, i32)) -> String {
        let settings: serde_json::Map<String, serde_json::Value> = self
            .settings
            .widget
            .get(id)
            .map(|o| {
                o.0.iter()
                    .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();
        serde_json::json!({
            "size": format!("{}x{}", size.0, size.1),
            "settings": settings,
            "instance": id,
        })
        .to_string()
    }

    /// The last answer kept for a provider widget.
    pub fn provider_data(&self, id: &str) -> Option<(Data, u64)> {
        let kept = self.dashboard.memory.data.get(id)?;
        Some((serde_json::from_value(kept.value.clone()).ok()?, kept.at))
    }

    pub fn runner(&self) -> Runner {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let state = isle_state::StateDir::default_dir();
        Runner::new(runtime.join("isle/providers"), state.join("providers"))
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Deaths in this long that leave a stream stopped.
const DEATHS: usize = 5;
const DEATH_WINDOW: Duration = Duration::from_secs(60);

impl State {
    /// Starts every provider widget shown that may run and is not running; stops those no
    /// longer placed.
    pub fn run_providers(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            self.isle.dashboard.running.clear();
            return;
        };
        let placed: Vec<(String, (i32, i32))> = open
            .layout
            .widgets
            .iter()
            .filter(|p| isle_shell::dashboard::catalogue::builtin(p.kind()).is_none())
            .map(|p| (p.id.clone(), (p.at.w, p.at.h)))
            .collect();
        self.isle
            .dashboard
            .running
            .retain(|id, _| placed.iter().any(|(p, _)| p == id));
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        for (id, size) in placed {
            if self.isle.dashboard.running.contains_key(&id)
                || self.isle.dashboard.answers.get(&id).is_some_and(|a| a.stopped)
            {
                continue;
            }
            let Standing::Runs(installed) = self.isle.standing(&id) else {
                continue;
            };
            let installed = installed.clone();
            let granted = self.isle.granted(&installed.manifest.id);
            let input = self.isle.provider_input(&id, size);
            let runner = self.isle.runner();
            let (to_loop, key) = (to_loop.clone(), id.clone());
            let task = runtime.spawn(async move {
                let m = &installed.manifest;
                if m.stream {
                    let mut deaths: Vec<Instant> = Vec::new();
                    let mut wait = Duration::from_secs(1);
                    loop {
                        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
                        let forward = {
                            let (to_loop, key) = (to_loop.clone(), key.clone());
                            tokio::spawn(async move {
                                while let Some(answer) = rx.recv().await {
                                    let key = key.clone();
                                    to_loop.post(move |state| state.provider_answered(&key, answer, Vec::new(), false));
                                }
                            })
                        };
                        let ended = runner.stream(m, &installed.folder, &granted, &input, tx).await;
                        let _ = forward.await;
                        let now = Instant::now();
                        deaths.retain(|d| now.duration_since(*d) < DEATH_WINDOW);
                        deaths.push(now);
                        let stopped = deaths.len() >= DEATHS;
                        let key2 = key.clone();
                        to_loop.post(move |state| {
                            state.provider_answered(&key2, ended.answer, ended.denied, stopped)
                        });
                        if stopped {
                            return;
                        }
                        tokio::time::sleep(wait).await;
                        wait = (wait * 2).min(Duration::from_secs(30));
                    }
                } else {
                    let every = Duration::from_secs(m.interval.unwrap_or(60));
                    loop {
                        let outcome = runner.once(m, &installed.folder, &granted, &input).await;
                        let key2 = key.clone();
                        to_loop.post(move |state| {
                            state.provider_answered(&key2, outcome.answer, outcome.denied, false)
                        });
                        tokio::time::sleep(every).await;
                    }
                }
            });
            self.isle.dashboard.running.insert(id, Running { task });
        }
    }

    fn provider_answered(&mut self, id: &str, answer: Result<Data, String>, denied: Vec<String>, stopped: bool) {
        for host in denied {
            tracing::info!(id, host, "a provider was refused a connection it was not granted");
        }
        let entry = self.isle.dashboard.answers.entry(id.to_owned()).or_default();
        entry.stopped = stopped;
        match answer {
            Ok(data) => {
                entry.error = None;
                let value = serde_json::to_value(&data).unwrap_or_default();
                self.isle.dashboard.memory.data.insert(
                    id.to_owned(),
                    isle_state::domains::WidgetData { value, at: now_secs() },
                );
                self.isle.save_widgets();
            }
            Err(e) if e == "stopped" => {}
            Err(e) => entry.error = Some(e),
        }
        self.put_dashboard_frame();
    }

    /// Try Again: a stopped provider started afresh.
    pub fn provider_try_again(&mut self, id: &str) {
        self.isle.dashboard.answers.remove(id);
        self.isle.dashboard.running.remove(id);
        self.run_providers();
        self.put_dashboard_frame();
    }
}

pub type Runs = HashMap<String, Running>;
pub type Answers = HashMap<String, Answer>;
