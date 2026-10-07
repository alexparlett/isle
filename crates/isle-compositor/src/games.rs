//! The game library on the worker, read again when Steam's manifests change (`p7-switcher.md`
//! choice 13).

use std::{error::Error, sync::Arc, time::Duration};

use isle_games::Library;
use notify::{RecursiveMode, Watcher};
use tracing::info;

use crate::worker::ToLoop;

pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(root) = Library::steam_root() else {
        return Ok(());
    };
    let read = |root: std::path::PathBuf| async move {
        tokio::task::spawn_blocking(move || Library::read(&root))
            .await
            .unwrap_or_default()
    };
    let library = read(root.clone()).await;
    info!(games = library.steam.len(), "Steam library read");
    to_loop.post(move |state| state.isle.games = Arc::new(library));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = tx.send(());
        }
    })?;
    for dir in Library::watched(&root) {
        if dir.is_dir() {
            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
        }
    }
    while rx.recv().await.is_some() {
        // Steam writes manifests many times as a game installs; read once it settles.
        tokio::time::sleep(Duration::from_secs(2)).await;
        while rx.try_recv().is_ok() {}
        let library = read(root.clone()).await;
        to_loop.post(move |state| {
            state.isle.games = Arc::new(library);
            state.isle.notify(crate::worker::Event::WindowsChanged);
        });
    }
    Ok(())
}

impl crate::state::Isle {
    /// The name shown for `app`: a game's from its library, else its entry's.
    pub fn app_name(&self, app: &str) -> String {
        match self.games.for_app(app) {
            Some(game) => game.name.clone(),
            None => self.apps.name(app),
        }
    }
}
