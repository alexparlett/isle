//! Which windows are games (`gaming.md` Games 1, 2): Steam's own labels, the launcher that
//! started the process, Feral GameMode's registrations, the surface's content type, Wine's
//! `.exe` classes, and the user's corrections.

use std::collections::HashSet;

use futures_util::StreamExt;
use tracing::{debug, info};

use crate::{
    state::{Isle, State},
    worker::{Event, ToLoop},
};

/// The `SteamGameId` Steam gives a process it starts as a game, if it has one.
fn steam_game_env(pid: i32) -> Option<String> {
    let environ = std::fs::read(format!("/proc/{pid}/environ")).ok()?;
    environ.split(|b| *b == 0).find_map(|var| {
        var.strip_prefix(b"SteamGameId=")
            .filter(|id| !id.is_empty() && *id != b"0")
            .map(|id| String::from_utf8_lossy(id).into_owned())
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
            n.starts_with("steam_app_") || n.starts_with("gamescope") || n == "steam_proton"
        });
        let wine = record
            .x11()
            .is_some_and(|x| x.class().to_lowercase().ends_with(".exe"));
        let registered = record
            .pid
            .into_iter()
            .chain(record.ancestors.iter().copied())
            .any(|pid| gamemode.contains(&pid));
        let steam_window = record.x11().and_then(|x| x.steam_game()).is_some();
        let game = !matches(&windows.not_games)
            && (matches(&windows.games)
                || labelled
                || steam_window
                || wine
                || record.steam
                || record.launched
                || record.content_game
                || registered);
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
    /// Reads a newly mapped window's process off the loop: Steam's label in its
    /// environment, and the launcher it descends from.
    pub fn read_process(&mut self, id: u64) {
        let (Some(pid), Some(worker), Some(to_loop)) = (
            self.isle.wm.get(id).and_then(|r| r.pid),
            self.isle.worker.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        worker.spawn_blocking(move || {
            let lineage = isle_games::process::lineage(pid);
            let steam = steam_game_env(pid);
            to_loop.post(move |state| {
                if let (Some((own, pids, names)), Some(record)) = (lineage, state.isle.wm.get_mut(id)) {
                    record.launched = isle_games::process::launched_game(&own, &names);
                    record.ancestors = pids;
                }
                if let Some(game) = steam
                    && let Some(record) = state.isle.wm.get_mut(id)
                {
                    record.steam = true;
                    // gamescope started by Steam hosts that game, so it is that game's app
                    // (`gaming.md` Games as apps 2).
                    let steam_app = format!("steam_app_{game}");
                    if record.app == "gamescope" && state.isle.games.for_app(&steam_app).is_some() {
                        info!(id, app = steam_app, "gamescope hosts a library game");
                        record.app = steam_app;
                        state.isle.notify(Event::WindowsChanged);
                    }
                }
                state.isle.refresh_game(id);
            });
        });
    }

    /// A window whose surface says it now shows a game, or no longer does.
    pub fn check_content_type(&mut self, id: u64, surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        use smithay::{
            reexports::wayland_protocols::wp::content_type::v1::server::wp_content_type_v1::Type,
            wayland::{compositor::with_states, content_type::ContentTypeSurfaceCachedState},
        };
        let game = with_states(surface, |states| {
            *states
                .cached_state
                .get::<ContentTypeSurfaceCachedState>()
                .current()
                .content_type()
                == Type::Game
        });
        if let Some(record) = self.isle.wm.get_mut(id)
            && record.content_game != game
        {
            record.content_game = game;
            self.isle.refresh_game(id);
        }
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
