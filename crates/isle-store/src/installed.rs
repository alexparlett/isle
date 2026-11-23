//! Installed apps and the catalogue each came from: a Flatpak export is Flathub's, an
//! AppImage Isle registered is AppImages', and an entry a package installed is CachyOS's or,
//! for a package no repository has, the AUR's (`p14e-store.md` choice 4).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Catalogue {
    Cachyos,
    Flathub,
    Aur,
    AppImages,
}

impl Catalogue {
    pub fn name(self) -> &'static str {
        match self {
            Catalogue::Cachyos => "System",
            Catalogue::Flathub => "Flathub",
            Catalogue::Aur => "AUR",
            Catalogue::AppImages => "AppImages",
        }
    }

    /// The pane hue its badge and tile take.
    pub fn hue(self) -> &'static str {
        match self {
            Catalogue::Cachyos => "teal",
            Catalogue::Flathub => "blue",
            Catalogue::Aur => "indigo",
            Catalogue::AppImages => "purple",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    /// The desktop id.
    pub id: String,
    pub name: String,
    pub summary: String,
    pub icon: Option<PathBuf>,
    pub catalogue: Catalogue,
    /// The package, the Flatpak app id or the AppImage's file, by catalogue.
    pub source: String,
}

/// Desktop entries by the package that installed them, from pacman's local database under
/// `root`: `usr/share/applications/<id>.desktop` → package name.
pub fn owners(root: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let local = root.join("var/lib/pacman/local");
    for dir in std::fs::read_dir(&local).into_iter().flatten().flatten() {
        let Ok(desc) = std::fs::read_to_string(dir.path().join("desc")) else {
            continue;
        };
        let Some(name) = desc.split("%NAME%\n").nth(1).and_then(|r| r.lines().next()) else {
            continue;
        };
        let Ok(files) = std::fs::read_to_string(dir.path().join("files")) else {
            continue;
        };
        for file in files
            .lines()
            .filter(|l| l.starts_with("usr/share/applications/") && l.ends_with(".desktop"))
        {
            out.insert(format!("/{file}"), name.to_string());
        }
    }
    out
}

/// Whether an entry is Isle's own app, which updates with Isle and is not listed in Store.
fn isle_own(package: &str) -> bool {
    package == isle_updates::ISLE_PACKAGE
}

/// The installed apps among `entries`, each with its catalogue. `foreign` holds the packages no
/// repository has (`pacman -Qmq`).
pub fn classify(
    entries: &[isle_apps::Entry],
    owners: &HashMap<String, String>,
    foreign: &HashSet<String>,
    icon: &mut dyn FnMut(&str) -> Option<PathBuf>,
) -> Vec<App> {
    let mut apps: Vec<App> = entries
        .iter()
        .filter(|e| e.listed() && !e.missing)
        .filter_map(|e| {
            let path = e.path.to_string_lossy();
            let (catalogue, source) = if e.id.starts_with("isle-appimage-") {
                (Catalogue::AppImages, e.id.clone())
            } else if path.contains("/flatpak/exports/share/applications/") {
                (Catalogue::Flathub, e.id.clone())
            } else {
                let package = owners.get(path.as_ref())?;
                if isle_own(package) {
                    return None;
                }
                let catalogue = if foreign.contains(package) {
                    Catalogue::Aur
                } else {
                    Catalogue::Cachyos
                };
                (catalogue, package.clone())
            };
            Some(App {
                id: e.id.clone(),
                name: e.name.clone(),
                summary: e.comment.clone().unwrap_or_default(),
                icon: e.icon.as_deref().and_then(&mut *icon),
                catalogue,
                source,
            })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, name: &str, path: &str) -> isle_apps::Entry {
        isle_apps::Entry {
            id: id.into(),
            name: name.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    #[test]
    fn each_app_takes_its_catalogue() {
        let root = std::env::temp_dir().join(format!("isle-store-owners-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, name, file) in [
            (
                "blender-17:4.5.3-1",
                "blender",
                "usr/share/applications/blender.desktop",
            ),
            (
                "xremap-hypr-bin-0.15.9-1",
                "xremap-hypr-bin",
                "usr/share/applications/xremap.desktop",
            ),
            (
                "isle-git-r729.g5268dd662034-1",
                "isle-git",
                "usr/share/applications/io.github.alexparlett.Isle.Files.desktop",
            ),
        ] {
            let d = root.join("var/lib/pacman/local").join(dir);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("desc"), format!("%NAME%\n{name}\n\n%VERSION%\n1\n")).unwrap();
            std::fs::write(d.join("files"), format!("%FILES%\nusr/\n{file}\n")).unwrap();
        }
        let owners = owners(&root);
        assert_eq!(owners["/usr/share/applications/blender.desktop"], "blender");
        let foreign: HashSet<String> = ["xremap-hypr-bin".to_string()].into();
        let entries = [
            entry(
                "blender",
                "Blender",
                "/usr/share/applications/blender.desktop",
            ),
            entry("xremap", "xremap", "/usr/share/applications/xremap.desktop"),
            entry(
                "io.github.alexparlett.Isle.Files",
                "Files",
                "/usr/share/applications/io.github.alexparlett.Isle.Files.desktop",
            ),
            entry(
                "com.spotify.Client",
                "Spotify",
                "/var/lib/flatpak/exports/share/applications/com.spotify.Client.desktop",
            ),
            entry(
                "isle-appimage-obsidian",
                "Obsidian",
                "/home/u/.local/share/applications/isle-appimage-obsidian.desktop",
            ),
            entry(
                "mine",
                "Mine",
                "/home/u/.local/share/applications/mine.desktop",
            ),
        ];
        let apps = classify(&entries, &owners, &foreign, &mut |_| None);
        let got: Vec<(&str, Catalogue, &str)> = apps
            .iter()
            .map(|a| (a.name.as_str(), a.catalogue, a.source.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("Blender", Catalogue::Cachyos, "blender"),
                ("Obsidian", Catalogue::AppImages, "isle-appimage-obsidian"),
                ("Spotify", Catalogue::Flathub, "com.spotify.Client"),
                ("xremap", Catalogue::Aur, "xremap-hypr-bin"),
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
