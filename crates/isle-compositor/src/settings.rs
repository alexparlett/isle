//! The settings store on the worker, its file watcher, and settings applied on the loop.

use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use isle_settings::{Reload, Store, schema::Settings};
use notify::{EventKind, RecursiveMode, Watcher};
use smithay::output::{Output, Scale};
use tokio::sync::{Mutex, broadcast, mpsc};
use tracing::{info, warn};

use crate::{
    state::{Isle, State},
    worker::{Event, ToLoop},
};

pub type SharedStore = Arc<Mutex<Store<Settings>>>;

/// `$ISLE_CONFIG_DIR`, for development and tests, else `$XDG_CONFIG_HOME/isle`.
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ISLE_CONFIG_DIR") {
        return dir.into();
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("isle")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.kdl")
}

/// Hands a change to the loop and announces it on D-Bus.
pub fn publish(
    to_loop: &ToLoop,
    events: &broadcast::Sender<Event>,
    settings: Settings,
    keys: Vec<String>,
) {
    if keys.is_empty() {
        return;
    }
    info!(?keys, "settings changed");
    let applied = keys.clone();
    to_loop.post(move |state| state.apply_settings(settings, Some(&applied)));
    let _ = events.send(Event::SettingsChanged(keys));
}

/// Watches the file through its directory, and through the directory of a symlink's
/// target, so editors that replace it by rename and dotfile links are both seen.
pub async fn watch(
    store: SharedStore,
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = store.lock().await.path().to_owned();
    let dir = path.parent().unwrap_or(Path::new("/")).to_owned();
    std::fs::create_dir_all(&dir)?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            let _ = tx.send(event);
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    let mut watched = vec![dir];
    let mut targets = watched_names(&path);

    while let Some(event) = rx.recv().await {
        // Reading the file is itself an event; only writes, renames and removals count.
        if matches!(event.kind, EventKind::Access(_))
            || !event
                .paths
                .iter()
                .any(|p| targets.iter().any(|t| p.ends_with(t)))
        {
            continue;
        }
        // Editors write in several steps; read once they settle.
        tokio::time::sleep(Duration::from_millis(100)).await;
        while rx.try_recv().is_ok() {}

        if let Some(target_dir) = std::fs::canonicalize(&path)
            .ok()
            .and_then(|t| t.parent().map(Path::to_owned))
            && !watched.contains(&target_dir)
        {
            watcher.watch(&target_dir, RecursiveMode::NonRecursive)?;
            watched.push(target_dir);
        }
        targets = watched_names(&path);

        let mut store = store.lock().await;
        match store.reload() {
            Reload::Unchanged => {}
            Reload::Applied(keys) => {
                publish(&to_loop, &events, store.current().clone(), keys);
                to_loop.post(|state| state.island_settings_status(None));
                let _ = events.send(Event::SettingsStatus);
            }
            Reload::Rejected(err) => {
                warn!("settings.kdl rejected, the last good settings stay: {err}");
                let error = store.error().map(|e| {
                    (e.file.display().to_string(), e.line as u32, e.column as u32, e.message.clone())
                });
                to_loop.post(move |state| state.island_settings_status(error));
                let _ = events.send(Event::SettingsStatus);
            }
        }
    }
    Ok(())
}

/// The file names a change to the settings can arrive under: its own, and its target's.
fn watched_names(path: &Path) -> Vec<PathBuf> {
    let mut names = vec![PathBuf::from(path.file_name().unwrap_or_default())];
    if let Ok(target) = std::fs::canonicalize(path)
        && let Some(name) = target.file_name()
    {
        names.push(PathBuf::from(name));
    }
    names
}

impl State {
    /// Puts `settings` in force; `changed` names the keys that differ, `None` all of them.
    pub fn apply_settings(&mut self, settings: Settings, changed: Option<&[String]>) {
        let touched =
            |prefix: &str| changed.is_none_or(|keys| keys.iter().any(|k| k.starts_with(prefix)));
        self.isle.settings = settings;
        if touched("lock-screen") {
            self.arm_idle();
        }
        if touched("spotlight")
            && let Some(tx) = &self.isle.files_reach
        {
            let reach = crate::files::reach(&self.isle.settings.spotlight);
            tx.send_if_modified(|old| {
                let changed = old.as_ref() != Some(&reach);
                *old = Some(reach);
                changed
            });
        }

        if touched("keyboard") {
            self.apply_keyboard_settings();
        }
        if touched("windows.games") || touched("windows.not-games") {
            self.isle.refresh_games();
        }
        if touched("keyboard.layouts") || touched("shortcuts") {
            let layouts = self.layouts().layouts.len();
            self.isle.bindings = crate::binds::Bindings::new(&self.isle.settings, layouts);
        }
        if touched("output[") {
            let outputs: Vec<Output> = self.isle.space.outputs().cloned().collect();
            for output in &outputs {
                self.isle.apply_output_settings(output);
            }
        }
        if touched("pointer[") {
            self.reapply_pointer_settings();
        }
        if touched("windows.title-bars") && changed.is_some() {
            self.renegotiate_decorations();
            self.isle.sync_bars();
        }
        if touched("appearance.") {
            self.isle.refresh_theme();
        }
        if touched("appearance.mode") || touched("wallpaper") {
            self.isle.queue_redraw_all();
        }
        if touched("island") || touched("shell.") || touched("appearance.") {
            self.sync_island();
        }
        if touched("debug.") {
            self.isle
                .heartbeat
                .set_abort(self.isle.settings.debug.watchdog_abort);
        }
    }
}

impl Isle {
    /// The key an output's settings are filed under: make, model and serial.
    pub fn output_identity(output: &Output) -> String {
        let p = output.physical_properties();
        format!("{} {} {}", p.make, p.model, p.serial_number)
    }

    pub fn apply_output_settings(&mut self, output: &Output) {
        let Some(config) = self
            .settings
            .output
            .get(&Self::output_identity(output))
            .cloned()
        else {
            return;
        };
        if output.current_scale().fractional_scale() != config.scale {
            output.change_current_state(None, None, Some(Scale::Fractional(config.scale)), None);
        }
        if let (Some(x), Some(y)) = (config.x, config.y) {
            self.space.map_output(output, (x, y));
        }
        self.sync_shell_outputs();
        self.queue_redraw(output);
        self.notify(Event::DisplaysChanged);
    }
}
