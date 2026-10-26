//! The game library on the worker, read again when a launcher's library changes
//! (`p7-switcher.md` choice 13, `p10-spotlight.md` choice 10).

use std::{error::Error, sync::Arc, time::Duration};

use isle_games::{GameId, Library, Roots};
use notify::{RecursiveMode, Watcher};
use tracing::info;

use crate::worker::ToLoop;

pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn Error + Send + Sync>> {
    let roots = Roots::find();
    let read = |roots: Roots| async move {
        tokio::task::spawn_blocking(move || Library::read(&roots))
            .await
            .unwrap_or_default()
    };
    let library = read(roots.clone()).await;
    info!(games = library.games().len(), ?roots, "game library read");
    to_loop.post(move |state| state.isle.games = Arc::new(library));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // Reading a watched file is itself an event; only changes count.
        if event.is_ok_and(|e| !matches!(e.kind, notify::EventKind::Access(_))) {
            let _ = tx.send(());
        }
    })?;
    for dir in roots.watched() {
        if dir.is_dir() {
            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
        }
    }
    while rx.recv().await.is_some() {
        // Steam writes manifests many times as a game installs; read once it settles.
        tokio::time::sleep(Duration::from_secs(2)).await;
        while rx.try_recv().is_ok() {}
        let library = read(roots.clone()).await;
        to_loop.post(move |state| {
            state.isle.games = Arc::new(library);
            state.isle.notify(crate::worker::Event::WindowsChanged);
        });
    }
    Ok(())
}

impl crate::state::Isle {
    /// The name shown for `app`: a game's from its library, else its entry's; an app with
    /// no id is named by its process, as a Mac app by its binary.
    pub fn app_name(&self, app: &str) -> String {
        if let Some(game) = self.games.for_app(app) {
            return game.name.clone();
        }
        let name = self.apps.name(app);
        if !name.is_empty() {
            return name;
        }
        self.wm
            .records
            .iter()
            .filter(|r| r.app == app)
            .find_map(|r| std::fs::read_to_string(format!("/proc/{}/comm", r.pid?)).ok())
            .map(|comm| comm.trim().to_owned())
            .unwrap_or_default()
    }
}

impl crate::state::State {
    /// Starts a library game through its launcher: the launcher's entry given the game's
    /// link, else the link opened by its handler (`gaming.md` Game library 2).
    pub fn launch_game(&mut self, id: &GameId) -> bool {
        let Some(game) = self.isle.games.get(id).cloned() else {
            return false;
        };
        let entry = game.launcher.desktop_id(game.flatpak);
        if self.isle.apps.get(entry).is_some() {
            return self.launch(crate::launch::Launch::App {
                app: entry.to_owned(),
                action: None,
                uris: vec![game.url()],
            });
        }
        self.launch(crate::launch::Launch::Open(game.url()))
    }
}
