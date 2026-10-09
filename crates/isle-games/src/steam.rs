//! Steam's installed games: the app manifests in every library folder, and their covers.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Game, GameId, Launcher, vdf};

/// Every installed game under `root`, with its cover.
pub fn read(root: &Path, flatpak: bool) -> Vec<Game> {
    let mut games = Vec::new();
    for folder in library_folders(root) {
        let Ok(dir) = fs::read_dir(folder.join("steamapps")) else {
            continue;
        };
        for file in dir.flatten() {
            let name = file.file_name();
            let Some(id) = name
                .to_str()
                .and_then(|n| n.strip_prefix("appmanifest_"))
                .and_then(|n| n.strip_suffix(".acf"))
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let Some(name) = fs::read_to_string(file.path())
                .ok()
                .and_then(|t| vdf::parse(&t))
                .and_then(|v| v.get("AppState")?.get("name")?.as_str().map(str::to_owned))
            else {
                continue;
            };
            if is_tool(&name) {
                continue;
            }
            games.push(Game {
                id: GameId::Steam(id),
                name,
                cover: cover(root, id),
                launcher: Launcher::Steam,
                flatpak,
            });
        }
    }
    games
}

/// Steam's runtimes and Proton install as apps; they are not games.
fn is_tool(name: &str) -> bool {
    name.starts_with("Proton")
        || name.starts_with("Steam Linux Runtime")
        || name.starts_with("Steamworks Common")
}

/// The files whose change means the library should be read again.
pub fn watched(root: &Path) -> Vec<PathBuf> {
    library_folders(root)
        .into_iter()
        .map(|f| f.join("steamapps"))
        .chain([root.join("steamapps"), root.join("appcache/librarycache")])
        .collect()
}

/// The library folders `libraryfolders.vdf` lists, the root's own first.
fn library_folders(root: &Path) -> Vec<PathBuf> {
    let mut folders = vec![root.to_owned()];
    let text = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")).unwrap_or_default();
    if let Some(list) = vdf::parse(&text).and_then(|v| v.get("libraryfolders").cloned()) {
        for (_, folder) in list.entries() {
            if let Some(path) = folder.get("path").and_then(|p| p.as_str()) {
                let path = PathBuf::from(path);
                if !folders.contains(&path) {
                    folders.push(path);
                }
            }
        }
    }
    folders
}

/// The portrait cover: one the user set in Steam (`userdata/*/config/grid/<id>p`), else the
/// library cache's `<id>/library_600x900.jpg`, the same in one of its hashed folders,
/// `library_capsule.jpg` likewise, or the flat `<id>_library_600x900.jpg` of before 2024.
fn cover(root: &Path, id: u32) -> Option<PathBuf> {
    let users = fs::read_dir(root.join("userdata"))
        .into_iter()
        .flatten()
        .flatten();
    let mut custom: Vec<PathBuf> = users
        .flat_map(|u| ["jpg", "png"].map(|ext| u.path().join(format!("config/grid/{id}p.{ext}"))))
        .filter(|p| p.is_file())
        .collect();
    custom.sort();
    if let Some(path) = custom.into_iter().next() {
        return Some(path);
    }
    let cache = root.join("appcache/librarycache");
    let dir = cache.join(id.to_string());
    for name in ["library_600x900.jpg", "library_capsule.jpg"] {
        if dir.join(name).is_file() {
            return Some(dir.join(name));
        }
        if let Ok(read) = fs::read_dir(&dir) {
            let mut found: Vec<PathBuf> = read
                .flatten()
                .map(|e| e.path().join(name))
                .filter(|p| p.is_file())
                .collect();
            found.sort();
            if let Some(path) = found.into_iter().next() {
                return Some(path);
            }
        }
    }
    Some(cache.join(format!("{id}_library_600x900.jpg"))).filter(|p| p.is_file())
}
