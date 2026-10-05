//! Which windows are games, until P12 detects them properly: Steam's own labels, Feral
//! GameMode's registrations, Wine's `.exe` classes, and the user's corrections.

use std::collections::HashSet;

use futures_util::StreamExt;
use tracing::{debug, info};

use crate::{
    state::{Isle, State},
    worker::{Event, ToLoop},
};

/// Whether a process was started by Steam as a game: Steam gives it `SteamGameId`.
fn steam_game_env(pid: i32) -> bool {
    let Ok(environ) = std::fs::read(format!("/proc/{pid}/environ")) else {
        return false;
    };
    environ.split(|b| *b == 0).any(|var| {
        var.strip_prefix(b"SteamGameId=")
            .is_some_and(|id| !id.is_empty() && id != b"0")
    })
}

impl Isle {
    /// Works out again whether a window is a game.
    pub fn refresh_game(&mut self, id: u64) {
        let windows = &self.settings.windows;
        let gamemode = &self.gamemode_pids;
        let Some(record) = self.wm.get_mut(id) else {
            return;
        };
        let (app_id, _) = record.names();
        let names = [record.app.as_str(), app_id.as_str()];
        let matches = |list: &[String]| list.iter().any(|p| isle_keys::tables::matches(p, &names));
        let labelled = names.iter().any(|n| {
            let n = n.to_lowercase();
            n.starts_with("steam_app_") || n.starts_with("gamescope")
        });
        let wine = record
            .x11()
            .is_some_and(|x| x.class().to_lowercase().ends_with(".exe"));
        let registered = record.pid.is_some_and(|pid| gamemode.contains(&pid));
        let game = !matches(&windows.not_games)
            && (matches(&windows.games) || labelled || wine || record.steam || registered);
        if record.game != game {
            info!(id, app = record.app, game, "game");
            record.game = game;
            self.notify(Event::WindowsChanged);
        }
    }

    pub fn refresh_games(&mut self) {
        let ids: Vec<u64> = self.wm.records.iter().map(|r| r.id).collect();
        for id in ids {
            self.refresh_game(id);
        }
    }
}

impl State {
    /// Reads a newly mapped window's process environment off the loop for Steam's label.
    pub fn read_steam_env(&mut self, id: u64) {
        let (Some(pid), Some(worker), Some(to_loop)) = (
            self.isle.wm.get(id).and_then(|r| r.pid),
            self.isle.worker.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        worker.spawn_blocking(move || {
            if steam_game_env(pid) {
                to_loop.post(move |state| {
                    if let Some(record) = state.isle.wm.get_mut(id) {
                        record.steam = true;
                    }
                    state.isle.refresh_game(id);
                });
            }
        });
    }

    /// Whether the window with keyboard focus is a game.
    pub fn focus_is_game(&self) -> bool {
        self.focused_window()
            .and_then(|id| self.isle.wm.get(id))
            .is_some_and(|r| r.game)
    }
}

#[zbus::proxy(
    interface = "com.feralinteractive.GameMode",
    default_service = "com.feralinteractive.GameMode",
    default_path = "/com/feralinteractive/GameMode"
)]
trait GameMode {
    fn list_games(&self) -> zbus::Result<Vec<(i32, zvariant::OwnedObjectPath)>>;
    #[zbus(signal)]
    fn game_registered(&self, pid: i32, path: zvariant::OwnedObjectPath) -> zbus::Result<()>;
    #[zbus(signal)]
    fn game_unregistered(&self, pid: i32, path: zvariant::OwnedObjectPath) -> zbus::Result<()>;
}

const GAMEMODE: &str = "com.feralinteractive.GameMode";

/// Follows GameMode's registered games, so a game started through it counts as one. It is
/// asked only while it runs, so asking never starts it.
pub async fn watch_gamemode(
    to_loop: ToLoop,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bus = zbus::Connection::session().await?;
    let dbus = zbus::fdo::DBusProxy::new(&bus).await?;
    let mut owners = dbus
        .receive_name_owner_changed_with_args(&[(0, GAMEMODE)])
        .await?;
    let proxy = GameModeProxy::builder(&bus)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?;
    let mut registered = proxy.receive_game_registered().await?;
    let mut unregistered = proxy.receive_game_unregistered().await?;
    loop {
        let running = dbus
            .name_has_owner(GAMEMODE.try_into()?)
            .await
            .unwrap_or(false);
        let pids: HashSet<i32> = if running {
            match proxy.list_games().await {
                Ok(games) => games.into_iter().map(|(pid, _)| pid).collect(),
                Err(err) => {
                    debug!("GameMode is not answering: {err}");
                    HashSet::new()
                }
            }
        } else {
            HashSet::new()
        };
        to_loop.post(move |state| {
            state.isle.gamemode_pids = pids;
            state.isle.refresh_games();
        });
        tokio::select! {
            Some(_) = registered.next() => {}
            Some(_) = unregistered.next() => {}
            Some(_) = owners.next() => {}
            else => return Ok(()),
        }
    }
}
