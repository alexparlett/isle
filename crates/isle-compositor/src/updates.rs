//! Software Update on the desktop: the hourly check, the run's progress file, the restart owed,
//! and the run started and stopped (`p14d-updates.md` choices 6–24).

use std::time::Duration;

use isle_shell::island::{
    grid::TileSize,
    panel::Module,
};
use isle_updates::{Check, Progress, Reason, RunState, Source};
use smithay::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use tracing::{info, warn};

use crate::{
    desktop_notes::Note,
    state::{Isle, State},
    worker::{Event, ToLoop},
};

/// The first check after the session starts or the computer wakes, then the rest.
const FIRST: Duration = Duration::from_secs(30);
const EVERY: Duration = Duration::from_secs(60 * 60);
/// How long a finished run stays shown.
const SHOWN: Duration = Duration::from_secs(30);
/// The row of Settings' Software Update page its links open.
pub const ROW: &str = "updates-status";

#[derive(Default)]
pub struct Updates {
    pub check: Option<Check>,
    pub checking: bool,
    pub check_error: Option<String>,
    /// The run as its file last said, until a finished one is cleared.
    pub progress: Option<Progress>,
    pub restart: Vec<Reason>,
    /// The last run whose end was told, by when it started.
    told: Option<u64>,
    /// When this compositor started, so a run that ended before it is not told again.
    since: u64,
    clear: Option<RegistrationToken>,
    schedule: Option<RegistrationToken>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn alive(pid: u32) -> bool {
    pid != 0 && unsafe { libc::kill(pid as libc::pid_t, 0) } == 0
}

fn release() -> String {
    let mut name: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut name) } != 0 {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(name.release.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

impl Updates {
    pub fn running(&self) -> bool {
        self.progress
            .as_ref()
            .is_some_and(|p| p.state == RunState::Running)
    }

    /// Updates waiting that are Store's: anything but Isle's own build.
    pub fn for_store(&self) -> bool {
        self.waiting() > self.count(Source::Isle)
    }

    pub fn waiting(&self) -> usize {
        self.check.as_ref().map_or(0, |c| c.pending.len())
    }

    fn count(&self, source: Source) -> usize {
        self.check.as_ref().map_or(0, |c| {
            c.pending.iter().filter(|p| p.source == source).count()
        })
    }
}

/// Reads the run's file whenever it changes, and once at the start.
pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use notify::{RecursiveMode, Watcher};
    let path = isle_updates::progress_path();
    let dir = path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    std::fs::create_dir_all(&dir)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(e) = event
            && !matches!(e.kind, notify::EventKind::Access(_))
            && e.paths
                .iter()
                .any(|p| p.file_name() == Some("update.json".as_ref()))
        {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    loop {
        let progress = isle_updates::read_progress(&path);
        to_loop.post(move |state| state.progress_read(progress));
        if rx.recv().await.is_none() {
            return Ok(());
        }
        // A run rewrites the file a line at a time; take the latest of a burst.
        tokio::time::sleep(Duration::from_millis(100)).await;
        while rx.try_recv().is_ok() {}
    }
}

impl State {
    /// The session has started: the checks begin, and the restart owed is read.
    pub fn start_updates(&mut self) {
        self.isle.updates.since = now();
        self.refresh_restart();
        self.schedule_updates();
    }

    /// Checks again soon, then every hour. The timer stands still while the computer sleeps,
    /// so waking starts it over.
    pub fn schedule_updates(&mut self) {
        if let Some(old) = self.isle.updates.schedule.take() {
            self.isle.loop_handle.remove(old);
        }
        let token = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(FIRST), |_, _, state| {
                state.refresh_restart();
                if state.isle.settings.updates.check {
                    state.check_updates(false);
                }
                TimeoutAction::ToDuration(EVERY)
            })
            .ok();
        self.isle.updates.schedule = token;
    }

    /// Looks for updates unless a check or a run is going. `by_user`, a failed or stopped run
    /// is put away first.
    pub fn check_updates(&mut self, by_user: bool) {
        let u = &mut self.isle.updates;
        if u.checking || u.running() || isle_updates::lock() == isle_updates::Lock::Held {
            return;
        }
        if by_user
            && u.progress
                .as_ref()
                .is_some_and(|p| p.state != RunState::Running)
        {
            self.clear_progress();
        }
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return;
        };
        self.isle.updates.checking = true;
        self.updates_changed();
        runtime.spawn(async move {
            let tools = isle_updates::Tools::from_env();
            let result = isle_updates::check(&tools, now())
                .await
                .map_err(|e| e.to_string());
            to_loop.post(move |state| state.updates_checked(result));
        });
    }

    fn updates_checked(&mut self, result: Result<Check, String>) {
        let u = &mut self.isle.updates;
        u.checking = false;
        match result {
            Ok(check) => {
                info!(waiting = check.pending.len(), "updates checked");
                u.check = Some(check);
                u.check_error = None;
            }
            Err(e) => {
                warn!("couldn't check for updates: {e}");
                u.check_error = Some(e);
            }
        }
        self.updates_changed();
    }

    /// The run's file as it now stands, or none.
    pub fn progress_read(&mut self, progress: Option<Progress>) {
        let mut progress = progress;
        // A run whose process is gone without saying how it ended.
        if let Some(p) = &mut progress
            && p.state == RunState::Running
            && !alive(p.pid)
        {
            p.state = RunState::Failed;
            p.reason = Some("The update stopped unexpectedly.".into());
            p.finished_at = Some(now());
        }
        let ended = progress
            .as_ref()
            .filter(|p| p.state != RunState::Running)
            .filter(|p| self.isle.updates.told != Some(p.started_at))
            .cloned();
        self.isle.updates.progress = progress;
        if let Some(p) = ended {
            self.isle.updates.told = Some(p.started_at);
            if p.finished_at.unwrap_or(0) >= self.isle.updates.since {
                self.run_ended(&p);
            }
        }
        self.updates_changed();
    }

    fn run_ended(&mut self, p: &Progress) {
        info!(state = ?p.state, reason = ?p.reason, "update run ended");
        self.refresh_restart();
        let settings_in_front =
            self.isle.front_app().as_deref() == Some(crate::settings_link::SETTINGS_APP);
        let store_in_front = self.isle.front_app().as_deref() == Some(STORE_APP);
        // Store's runs (Get, Remove, and apps' updates): what was done, and what failed.
        use isle_updates::Part;
        let store = match Part::parse(&p.part) {
            Some(Part::Install { id, .. }) => {
                Some((format!("{id} was installed"), format!("Couldn't install {id}")))
            }
            Some(Part::Remove { id, .. }) => {
                Some((format!("{id} was removed"), format!("Couldn't remove {id}")))
            }
            Some(Part::Flathub(Some(id)) | Part::Aur(Some(id))) => {
                Some((format!("{id} was updated"), format!("Couldn't update {id}")))
            }
            Some(Part::AppImage(path)) => {
                let name = std::path::Path::new(&path)
                    .file_stem()
                    .map_or(path.clone(), |s| s.to_string_lossy().into_owned());
                Some((format!("{name} was updated"), format!("Couldn't update {name}")))
            }
            Some(Part::Flathub(None) | Part::Aur(None)) => {
                Some(("Apps were updated".into(), "Couldn't update apps".into()))
            }
            Some(Part::System) => {
                Some(("The system was updated".into(), "Couldn't update the system".into()))
            }
            _ => None,
        };
        // The system's run updated every package the check found waiting in the repositories.
        if p.state == RunState::Done
            && matches!(Part::parse(&p.part), Some(Part::System | Part::Both))
            && let Some(check) = &mut self.isle.updates.check
        {
            check.pending.retain(|w| w.source != Source::Repositories);
            check.system = Default::default();
        }
        match p.state {
            RunState::Done if store.is_some() => {
                if let Some((done, _)) = store.filter(|_| !store_in_front) {
                    let owed = !self.isle.updates.restart.is_empty();
                    let (body, actions) = if owed {
                        ("Restart to finish updating.".to_owned(), vec![("restart", "Restart…")])
                    } else {
                        (packages(p.total), vec![])
                    };
                    self.post_desktop("", "Store", done, body, actions, Note::Store);
                }
            }
            RunState::Failed | RunState::Refused if store.is_some() => {
                if let Some((_, failed)) = store {
                    self.post_desktop(
                        "",
                        "Store",
                        failed,
                        p.reason.clone().unwrap_or_default(),
                        vec![],
                        Note::Store,
                    );
                }
            }
            RunState::Done => {
                if !settings_in_front {
                    let owed = !self.isle.updates.restart.is_empty();
                    let (body, actions) = if owed {
                        (
                            "Restart to finish updating.".to_owned(),
                            vec![("restart", "Restart…")],
                        )
                    } else {
                        (packages(p.total), vec![])
                    };
                    self.post_desktop(
                        "",
                        "Software Update",
                        "Update installed".into(),
                        body,
                        actions,
                        Note::Update,
                    );
                }
                let token = self
                    .isle
                    .loop_handle
                    .insert_source(Timer::from_duration(SHOWN), |_, _, state| {
                        state.isle.updates.clear = None;
                        if state
                            .isle
                            .updates
                            .progress
                            .as_ref()
                            .is_some_and(|p| p.state == RunState::Done)
                        {
                            state.clear_progress();
                            state.updates_changed();
                        }
                        TimeoutAction::Drop
                    })
                    .ok();
                if let Some(old) = std::mem::replace(&mut self.isle.updates.clear, token) {
                    self.isle.loop_handle.remove(old);
                }
            }
            RunState::Failed | RunState::Refused if !settings_in_front => {
                let reason = p.reason.clone().unwrap_or_default();
                self.post_desktop(
                    "",
                    "Software Update",
                    "Couldn't install updates".into(),
                    reason,
                    vec![("details", "Show Details…")],
                    Note::Update,
                );
            }
            _ => {}
        }
        self.check_updates(false);
    }

    fn clear_progress(&mut self) {
        self.isle.updates.progress = None;
        let _ = std::fs::remove_file(isle_updates::progress_path());
    }

    pub fn refresh_restart(&mut self) {
        let pids = [std::process::id(), std::os::unix::process::parent_id()];
        // `ISLE_UPDATES_ROOT` stands a tree in for `/` in tests.
        let root = std::env::var_os("ISLE_UPDATES_ROOT")
            .map_or_else(|| "/".into(), std::path::PathBuf::from);
        let owed = isle_updates::restart_owed(&root, &release(), &pids);
        if owed != self.isle.updates.restart {
            info!(?owed, "restart owed");
            self.isle.updates.restart = owed;
            self.updates_changed();
        }
    }

    /// Why a run of `part` cannot start now; else the part to run. No part runs everything
    /// waiting: the system, Isle's build, or both in one run.
    pub fn may_install(&self, part: &str) -> Result<isle_updates::Part, String> {
        use isle_updates::Part;
        if self.isle.updates.running() {
            return Err("An update is already running.".into());
        }
        match isle_updates::lock() {
            isle_updates::Lock::Free => {}
            isle_updates::Lock::Held => {
                return Err(
                    "Another program is installing packages. Try again when it has finished."
                        .into(),
                );
            }
            isle_updates::Lock::Left => {
                return Err(
                    "An install that didn't finish left pacman locked. With no package manager \
                     open, run sudo rm /var/lib/pacman/db.lck in Terminal, then try again."
                        .into(),
                );
            }
        }
        let isle = self.isle.updates.count(Source::Isle) > 0;
        let system = self.isle.updates.count(Source::Repositories) > 0;
        match part {
            "isle" if !isle => Err("Isle is up to date.".into()),
            "" if isle && system => Ok(Part::Both),
            "" if isle => Ok(Part::Isle),
            "" => Ok(Part::System),
            other => match Part::parse(other) {
                // An AUR package is built against the system as it is; one still waiting for
                // its own update would leave the build behind it.
                Some(Part::Aur(_)) if system => Err("Update the system first.".into()),
                Some(Part::Install { catalogue, .. }) if catalogue == "aur" && system => {
                    Err("Update the system first.".into())
                }
                Some(Part::Install { catalogue, id, .. }) if catalogue == "cachyos" => {
                    Ok(Part::Install { catalogue, id, with_updates: system })
                }
                Some(part) => Ok(part),
                None => Err(format!("No part called {other}.")),
            },
        }
    }

    /// Stops the run: refused while pacman holds its lock, since a half transaction is the
    /// one thing a run must never leave (choice 14).
    pub fn stop_updates(&mut self) -> Option<String> {
        let Some(p) = self
            .isle
            .updates
            .progress
            .as_ref()
            .filter(|p| p.state == RunState::Running)
        else {
            return Some("No update is running.".into());
        };
        if isle_updates::lock() == isle_updates::Lock::Held {
            return Some("Can't stop while packages are being installed.".into());
        }
        // The runner leads its own session, in its unit or started directly; the group's
        // signal reaches paru and makepkg as well.
        let pid = p.pid as libc::pid_t;
        info!(pid, "stopping the update");
        unsafe {
            if libc::kill(-pid, libc::SIGTERM) != 0 {
                libc::kill(pid, libc::SIGTERM);
            }
        }
        None
    }

    pub fn updates_changed(&mut self) {
        self.isle.updates_glyph();
        self.isle.notify(Event::UpdatesChanged);
        self.isle.refresh_island();
        self.refresh_bar();
    }

    pub fn open_store_updates(&mut self) {
        self.launch(crate::launch::Launch::App {
            app: STORE_APP.into(),
            action: None,
            uris: vec!["isle-store://updates".into()],
        });
    }

    /// Where the updates are: Store's Updates for anything but Isle's build, else Settings.
    pub fn open_updates(&mut self) {
        let isle_run = self.isle.updates.running()
            && self.isle.updates.progress.as_ref().is_some_and(|p| matches!(p.part.as_str(), "isle" | "both" | ""));
        if !isle_run && (self.isle.updates.for_store() || self.isle.updates.running()) {
            self.open_store_updates();
        } else {
            self.open_settings(Some("general"), Some(ROW));
        }
    }
}

const STORE_APP: &str = "io.github.alexparlett.Isle.Store";

fn packages(n: u32) -> String {
    match n {
        0 => String::new(),
        1 => "1 package was updated.".into(),
        n => format!("{n} packages were updated."),
    }
}

/// Starts the run in its own unit, so it outlives the compositor (choice 11).
/// `ISLE_UPDATE_DIRECT=1` starts it as a detached process instead, for tests without a user
/// manager; `ISLE_CLI` names the `isle` that runs it.
pub async fn start_run(part: isle_updates::Part) -> Result<(), String> {
    let cli = std::env::var("ISLE_CLI").unwrap_or_else(|_| "isle".into());
    let word = part.word();
    let args = ["update", "run", "--part", word.as_str()];
    if std::env::var("ISLE_UPDATE_DIRECT").as_deref() == Ok("1") {
        let mut command = tokio::process::Command::new(&cli);
        command.args(args).stdin(std::process::Stdio::null());
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(|e| format!("{cli}: {e}"))?;
        tokio::spawn(async move { child.wait().await });
        return Ok(());
    }
    let out = tokio::process::Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--quiet",
            "--unit",
            isle_updates::UNIT,
            "--description",
            "Software Update",
            "--",
        ])
        .arg(&cli)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("systemd-run: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err
            .lines()
            .next()
            .unwrap_or("The update couldn't start.")
            .to_owned())
    }
}

impl Isle {
    /// The island's glyph while a run is going, a restart is owed or updates wait (B150).
    pub fn updates_glyph(&mut self) {
        use isle_shell::island::model::{Glyph, GlyphKind};
        let glyph = if self.updates.running() {
            Some(Glyph {
                active: true,
                symbol: "arrow-circle-up".into(),
                alt: "Updating".into(),
                ..Default::default()
            })
        } else if !self.updates.restart.is_empty() {
            Some(Glyph {
                active: true,
                symbol: "arrows-clockwise".into(),
                alt: "Restart to finish updating".into(),
                ..Default::default()
            })
        } else if self.updates.waiting() > 0 {
            Some(Glyph {
                active: true,
                symbol: "download-simple".into(),
                alt: "Updates available".into(),
                ..Default::default()
            })
        } else {
            None
        };
        self.island.model.set_glyph(GlyphKind::Updates, glyph);
    }

    /// Control Centre's Updates module: a click opens where the updates are, with no page.
    pub fn updates_module(&self, size: TileSize) -> Option<Module> {
        let u = &self.updates;
        let (state, glyph) =
            if let Some(p) = u.progress.as_ref().filter(|p| p.state == RunState::Running) {
                (installing(p), "arrow-circle-up")
            } else if !u.restart.is_empty() {
                ("Restart to finish updating".to_owned(), "arrows-clockwise")
            } else if u.waiting() > 0 {
                (count(u.waiting()), "arrow-circle-up")
            } else if u.checking {
                ("Checking…".to_owned(), "arrow-circle-up")
            } else if u.check_error.is_some() {
                ("Couldn't check".to_owned(), "arrow-circle-up")
            } else {
                ("Up to date".to_owned(), "arrow-circle-up")
            };
        let progress = u
            .progress
            .as_ref()
            .filter(|p| p.state == RunState::Running)
            .map(|p| {
                if p.total == 0 {
                    0.0
                } else {
                    p.done as f32 / p.total as f32
                }
            });
        let badge = (progress.is_none() && u.restart.is_empty() && u.waiting() > 0)
            .then(|| u.waiting().to_string());
        Some(Module {
            id: "updates".into(),
            size,
            name: "Updates".into(),
            state,
            glyph: glyph.into(),
            page: None,
            pane: "Software Update".into(),
            badge,
            progress,
            ..Default::default()
        })
    }

    /// Store's count in the system menu, as its Updates page counts: the system's row and each
    /// app from CachyOS, then Flathub's and the AUR's updates.
    pub fn store_count(&self) -> Option<usize> {
        let system = match &self.updates.check {
            Some(c) if self.updates.count(Source::Repositories) > 0 => 1 + c.system.apps.len(),
            _ => 0,
        };
        let n = system + self.updates.count(Source::Flathub) + self.updates.count(Source::Aur);
        (n > 0 && !self.updates.running()).then_some(n)
    }

    /// System Settings' count in the system menu: Isle's build waiting, none while a run is
    /// going.
    pub fn updates_count(&self) -> Option<usize> {
        let n = usize::from(self.updates.count(Source::Isle) > 0);
        (n > 0 && !self.updates.running()).then_some(n)
    }

    pub fn updates_status(&self) -> String {
        let u = &self.updates;
        serde_json::json!({
            "pending": u.check.as_ref().map(|c| &c.pending),
            "system": u.check.as_ref().map(|c| &c.system),
            "checked_at": u.check.as_ref().map(|c| c.checked_at),
            "checking": u.checking,
            "check_error": u.check_error,
            "progress": u.progress,
            "restart": u.restart.iter().map(|r| r.sentence()).collect::<Vec<_>>(),
            "can_stop": u.running() && isle_updates::lock() != isle_updates::Lock::Held,
            "auto": self.settings.updates.check,
        })
        .to_string()
    }
}

fn count(n: usize) -> String {
    if n == 1 {
        "1 update".into()
    } else {
        format!("{n} updates")
    }
}

fn installing(p: &Progress) -> String {
    if p.total > 0 {
        format!("Installing {} of {}", p.done.min(p.total), p.total)
    } else {
        "Installing".into()
    }
}
