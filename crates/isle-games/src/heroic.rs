//! Heroic's installed games: its store caches for Epic, GOG and Amazon, and its sideloaded
//! apps. Covers come from Heroic's own image cache; nothing is fetched.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{Game, GameId, Launcher};

/// The library files and the runner each one's games launch with when they do not say.
const LIBRARIES: [(&str, &str); 4] = [
    ("store_cache/legendary_library.json", "legendary"),
    ("store_cache/gog_library.json", "gog"),
    ("store_cache/nile_library.json", "nile"),
    ("sideload_apps/library.json", "sideload"),
];

/// Every installed game under Heroic's configuration folder `root`.
pub fn read(root: &Path, flatpak: bool) -> Vec<Game> {
    let installed = installed_elsewhere(root);
    let mut games = Vec::new();
    for (file, runner) in LIBRARIES {
        let Some(json) = fs::read(root.join(file))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        else {
            continue;
        };
        let list = json
            .get("library")
            .or_else(|| json.get("games"))
            .and_then(|l| l.as_array());
        for game in list.into_iter().flatten() {
            let text = |k: &str| {
                game.get(k)
                    .and_then(|v| v.as_str())
                    .filter(|v| !v.is_empty())
            };
            let (Some(app), Some(title)) = (text("app_name"), text("title")) else {
                continue;
            };
            let is_installed = game
                .get("is_installed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !is_installed && !installed.contains(app) {
                continue;
            }
            let dlc = game
                .pointer("/install/is_dlc")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if dlc {
                continue;
            }
            let cover = ["art_square", "art_cover"]
                .iter()
                .filter_map(|k| text(k))
                .map(|url| cached(root, url))
                .find(|p| p.is_file())
                .or_else(|| {
                    ["jpg", "png"]
                        .map(|e| root.join(format!("icons/{app}.{e}")))
                        .into_iter()
                        .find(|p| p.is_file())
                });
            games.push(Game {
                id: GameId::Heroic {
                    runner: text("runner").unwrap_or(runner).to_owned(),
                    app: app.to_owned(),
                },
                name: title.to_owned(),
                cover,
                launcher: Launcher::Heroic,
                flatpak,
            });
        }
    }
    games
}

/// Games the store caches may not yet mark installed: legendary's and gogdl's own records.
fn installed_elsewhere(root: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    let read = |p: PathBuf| {
        fs::read(p)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
    };
    if let Some(serde_json::Value::Object(map)) =
        read(root.join("legendaryConfig/legendary/installed.json"))
    {
        out.extend(map.keys().cloned());
    }
    if let Some(list) =
        read(root.join("gog_store/installed.json")).and_then(|j| j.get("installed").cloned())
        && let Some(list) = list.as_array()
    {
        out.extend(
            list.iter()
                .filter_map(|g| g.get("appName")?.as_str().map(str::to_owned)),
        );
    }
    out
}

/// Where Heroic keeps its copy of the picture at `url`: named by the URL's SHA-256.
fn cached(root: &Path, url: &str) -> PathBuf {
    let hash = Sha256::digest(url.as_bytes());
    let name: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    root.join("images-cache").join(name)
}

pub fn watched(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("store_cache"),
        root.join("sideload_apps"),
        root.join("legendaryConfig/legendary"),
        root.join("gog_store"),
    ]
}
