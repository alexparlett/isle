//! The design tokens: the shipped file with the user's `tokens.kdl` over it, watched on the
//! worker and resolved on the loop for the appearance settings (B10).

use std::{
    error::Error,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use isle_settings::schema::{AppearanceMode, Settings};
use isle_tokens::{Appearance, Mode, Source, Theme, TokenError};
use notify::{EventKind, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    settings::config_dir,
    state::{Isle, State},
    worker::ToLoop,
};

pub fn tokens_path() -> PathBuf {
    config_dir().join("tokens.kdl")
}

/// The shipped tokens with the user's file over them, if it has one that reads.
pub fn load(path: &Path) -> Result<Source, TokenError> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let (source, unknown) = Source::with_user(&text, path)?;
            if !unknown.is_empty() {
                warn!(?unknown, "tokens.kdl names tokens this Isle does not have");
            }
            Ok(source)
        }
        Err(_) => Ok(Source::shipped()),
    }
}

pub fn appearance(settings: &Settings) -> Appearance {
    let a = &settings.appearance;
    Appearance {
        mode: match a.mode {
            AppearanceMode::Light => Mode::Light,
            AppearanceMode::Dark => Mode::Dark,
            AppearanceMode::Auto if crate::night_light::sun_down(&jiff::Zoned::now()) => Mode::Dark,
            AppearanceMode::Auto => Mode::Light,
        },
        accent: a.accent.clone(),
        glass: a.glass as f32,
        text_scale: a.text_scale as f32,
        increase_contrast: a.increase_contrast,
        reduce_motion: a.reduce_motion,
        reduce_transparency: a.reduce_transparency,
    }
}

/// The theme for `source` and the settings, or the shipped one if the source cannot give one.
pub fn theme(source: &Source, settings: &Settings) -> Rc<Theme> {
    let appearance = appearance(settings);
    let theme = source.theme(&appearance).unwrap_or_else(|err| {
        warn!("the tokens do not resolve, the shipped ones stand in: {err}");
        Source::shipped()
            .theme(&appearance)
            .expect("the shipped tokens resolve")
    });
    Rc::new(theme)
}

/// Watches the user's file and reloads it.
pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = tokens_path();
    watch_file(path.clone(), move || {
        let loaded = load(&path);
        to_loop.post(move |state| state.apply_tokens(loaded));
    })
    .await
}

/// Watches a file through its directory and a symlink's target, as settings are, calling
/// `changed` once an edit's writes settle.
pub async fn watch_file(
    path: PathBuf,
    mut changed: impl FnMut() + Send,
) -> Result<(), Box<dyn Error + Send + Sync>> {
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
    while let Some(event) = rx.recv().await {
        let names = names(&path);
        if matches!(event.kind, EventKind::Access(_))
            || !event
                .paths
                .iter()
                .any(|p| names.iter().any(|n| p.ends_with(n)))
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
        changed();
    }
    Ok(())
}

fn names(path: &Path) -> Vec<PathBuf> {
    let mut names = vec![PathBuf::from(path.file_name().unwrap_or_default())];
    if let Ok(target) = std::fs::canonicalize(path)
        && let Some(name) = target.file_name()
    {
        names.push(PathBuf::from(name));
    }
    names
}

impl Isle {
    /// Resolves the tokens for the settings in force and hands them to every surface.
    pub fn refresh_theme(&mut self) {
        let theme = theme(&self.tokens, &self.settings);
        // Each tree is woken, and the loop's next dispatch re-renders it.
        self.shell.set_theme(theme);
    }
}

impl State {
    /// A reload of the user's file: in force if it reads, else the last good tokens stay.
    pub fn apply_tokens(&mut self, loaded: Result<Source, TokenError>) {
        match loaded {
            Ok(source) => {
                info!("tokens reloaded");
                self.isle.tokens = source;
                self.isle.tokens_error = None;
                self.isle.refresh_theme();
                self.portal_appearance_changed();
                self.sync_island();
            }
            Err(err) => {
                warn!("tokens.kdl rejected, the last good tokens stay: {err}");
                self.isle.tokens_error = Some(err);
                self.sync_island();
            }
        }
    }
}
