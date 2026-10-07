//! Games' names and covers from the launchers' own libraries, read and never written
//! (`gaming.md` Games as apps, Game library). Steam's for now.

pub mod vdf;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// One installed game.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    /// A portrait cover, 2:3.
    pub cover: Option<PathBuf>,
}

/// Every installed Steam game, by app id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    pub root: Option<PathBuf>,
    pub steam: HashMap<u32, Game>,
}

impl Library {
    /// Steam's root: `$XDG_DATA_HOME/Steam`, `~/.steam/root`, then the Flatpak's.
    pub fn steam_root() -> Option<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
        data_home
            .map(|d| d.join("Steam"))
            .into_iter()
            .chain(home.iter().map(|h| h.join(".steam/root")))
            .chain(
                home.iter()
                    .map(|h| h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")),
            )
            .find(|root| root.join("steamapps/libraryfolders.vdf").is_file())
    }

    /// Reads every library folder's manifests and the covers in the library cache.
    pub fn read(root: &Path) -> Self {
        let mut steam = HashMap::new();
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
                steam.insert(
                    id,
                    Game {
                        name,
                        cover: cover(root, id),
                    },
                );
            }
        }
        Self {
            root: Some(root.to_owned()),
            steam,
        }
    }

    /// The game a window's app id names: `steam_app_<id>`.
    pub fn for_app(&self, app: &str) -> Option<&Game> {
        let id = app.strip_prefix("steam_app_")?.parse().ok()?;
        self.steam.get(&id)
    }

    /// The files whose change means the library should be read again.
    pub fn watched(root: &Path) -> Vec<PathBuf> {
        library_folders(root)
            .into_iter()
            .map(|f| f.join("steamapps"))
            .chain([root.join("steamapps"), root.join("appcache/librarycache")])
            .collect()
    }
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

/// The portrait cover in the library cache: `<id>/library_600x900.jpg`, the same in one of
/// the hashed folders Steam has used since 2024, `library_capsule.jpg` likewise, or the flat
/// `<id>_library_600x900.jpg` of before.
fn cover(root: &Path, id: u32) -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    #[test]
    fn reads_every_library_folder_and_both_cache_layouts() {
        let root = fixtures().join("steam");
        let folders = root.join("steamapps/libraryfolders.vdf");
        let library = fixtures().join("library");
        let text = fs::read_to_string(&folders)
            .unwrap()
            .replace("LIBRARY", &library.to_string_lossy());
        let tmp = std::env::temp_dir().join(format!("isle-games-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        copy(&root, &tmp);
        fs::write(tmp.join("steamapps/libraryfolders.vdf"), text).unwrap();

        let read = Library::read(&tmp);
        let tainted = read.for_app("steam_app_1466060").unwrap();
        assert_eq!(tainted.name, "Tainted Grail: The Fall of Avalon");
        assert!(
            tainted
                .cover
                .as_ref()
                .unwrap()
                .ends_with("hash/library_600x900.jpg")
        );
        let other = read.for_app("steam_app_281990").unwrap();
        assert_eq!(other.name, "Stellaris", "from the second library folder");
        assert!(
            other
                .cover
                .as_ref()
                .unwrap()
                .ends_with("281990_library_600x900.jpg")
        );
        assert!(read.for_app("steam_app_1").is_none());
        assert!(read.for_app("firefox").is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap().flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
}
