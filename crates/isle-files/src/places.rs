//! The sidebar's places: Recents, Favourites and Locations (`p1/structure/files.md` §
//! Places). Read on a worker; the window draws what was read.

use std::path::{Path, PathBuf};

/// Where a view is: a folder, or a gathering of items from many folders.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Place {
    Folder(PathBuf),
    Recents,
    Trash,
    /// The computer: every volume.
    Computer,
}

impl Place {
    pub fn folder(&self) -> Option<&Path> {
        match self {
            Place::Folder(p) => Some(p),
            _ => None,
        }
    }

    /// The name the title and the tab show.
    pub fn title(&self) -> String {
        match self {
            Place::Folder(p) => folder_name(p),
            Place::Recents => "Recents".into(),
            Place::Trash => "Trash".into(),
            Place::Computer => computer_name(),
        }
    }
}

/// A folder's name as Files shows it: Home's is the user's name, the root the computer's.
pub fn folder_name(path: &Path) -> String {
    if Some(path) == home().as_deref() {
        return std::env::var("USER").unwrap_or_else(|_| "Home".into());
    }
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => computer_name(),
    }
}

pub fn computer_name() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "Computer".into())
}

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home().map(|h| h.join(".config")))
}

/// A row in the sidebar.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub place: Place,
    pub label: String,
    pub glyph: &'static str,
}

/// The sidebar's groups as read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Places {
    pub favourites: Vec<Row>,
    pub locations: Vec<Row>,
}

/// The xdg-user-dirs folders in macOS's order, with their glyphs.
const USER_DIRS: [(&str, &str, &str); 6] = [
    ("XDG_DESKTOP_DIR", "Desktop", "desktop"),
    ("XDG_DOCUMENTS_DIR", "Documents", "file-text"),
    ("XDG_DOWNLOAD_DIR", "Downloads", "download-simple"),
    ("XDG_MUSIC_DIR", "Music", "music-notes"),
    ("XDG_PICTURES_DIR", "Pictures", "image"),
    ("XDG_VIDEOS_DIR", "Videos", "film-strip"),
];

/// `user-dirs.dirs`'s folders by their keys, `$HOME` expanded.
pub fn user_dirs(text: &str, home: &Path) -> Vec<(String, PathBuf)> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once('=')?;
            if key.starts_with('#') {
                return None;
            }
            let value = value.trim().trim_matches('"');
            let path = match value.strip_prefix("$HOME") {
                Some(rest) => home.join(rest.trim_start_matches('/')),
                None => PathBuf::from(value),
            };
            Some((key.to_owned(), path))
        })
        .collect()
}

/// GTK's bookmarks: a `file://` URI and an optional label per line.
pub fn bookmarks(text: &str) -> Vec<(PathBuf, Option<String>)> {
    text.lines()
        .filter_map(|line| {
            let (uri, label) = match line.split_once(' ') {
                Some((uri, label)) => (uri, Some(label.trim().to_owned()).filter(|l| !l.is_empty())),
                None => (line.trim(), None),
            };
            let path = uri.strip_prefix("file://")?;
            Some((PathBuf::from(percent_decode(path)), label))
        })
        .collect()
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reads the places from the user's files; blocks on them, so call it on a worker.
pub fn read() -> Places {
    let Some(home) = home() else {
        return Places::default();
    };
    let mut favourites = vec![Row {
        place: Place::Folder(home.clone()),
        label: "Home".into(),
        glyph: "house",
    }];
    let apps = home.join("Applications");
    if apps.is_dir() {
        favourites.push(Row {
            place: Place::Folder(apps),
            label: "Applications".into(),
            glyph: "app-window",
        });
    }
    let config = config_home().unwrap_or_else(|| home.join(".config"));
    let dirs = std::fs::read_to_string(config.join("user-dirs.dirs"))
        .map(|t| user_dirs(&t, &home))
        .unwrap_or_default();
    for (key, label, glyph) in USER_DIRS {
        let path = dirs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| home.join(label));
        // xdg-user-dirs points a folder it does not use at home.
        if path != home && path.is_dir() {
            favourites.push(Row {
                place: Place::Folder(path),
                label: label.into(),
                glyph,
            });
        }
    }
    let marks = std::fs::read_to_string(config.join("gtk-3.0/bookmarks"))
        .map(|t| bookmarks(&t))
        .unwrap_or_default();
    for (path, label) in marks {
        if favourites.iter().any(|r| r.place == Place::Folder(path.clone())) {
            continue;
        }
        favourites.push(Row {
            label: label.unwrap_or_else(|| folder_name(&path)),
            place: Place::Folder(path),
            glyph: "folder-simple",
        });
    }
    let locations = vec![
        Row {
            place: Place::Computer,
            label: computer_name(),
            glyph: "desktop-tower",
        },
        Row {
            place: Place::Trash,
            label: "Trash".into(),
            glyph: "trash",
        },
    ];
    Places { favourites, locations }
}

/// The recently used files, newest first; blocks, so call it on a worker.
pub fn recents(limit: usize) -> Vec<PathBuf> {
    let xbel = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home().map(|h| h.join(".local/share")))
        .map(|d| d.join("recently-used.xbel"));
    xbel.and_then(|p| std::fs::read_to_string(p).ok())
        .map(|x| isle_search::files::recent(&x, limit))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_dirs_expand_home_and_skip_comments() {
        let text = "# written by xdg-user-dirs-update\nXDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\nXDG_MUSIC_DIR=\"/mnt/music\"\n";
        assert_eq!(
            user_dirs(text, Path::new("/home/me")),
            [
                ("XDG_DOWNLOAD_DIR".to_owned(), PathBuf::from("/home/me/Downloads")),
                ("XDG_MUSIC_DIR".to_owned(), PathBuf::from("/mnt/music")),
            ]
        );
    }

    #[test]
    fn bookmarks_decode_and_keep_labels() {
        let text = "file:///home/me/Development\nfile:///home/me/My%20Games Games\nsftp://host/x\n";
        assert_eq!(
            bookmarks(text),
            [
                (PathBuf::from("/home/me/Development"), None),
                (PathBuf::from("/home/me/My Games"), Some("Games".into())),
            ]
        );
    }
}
