//! App icons by name through the freedesktop icon theme lookup: the theme, the themes it
//! inherits, hicolor, then pixmaps (`p1/system/icons.md`: Papirus, each app's own as
//! fallback).

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Icon theme directories: `$XDG_DATA_HOME/icons`, `~/.icons`, each `$XDG_DATA_DIRS/icons`.
pub fn base_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
    let dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    data_home
        .map(|d| d.join("icons"))
        .into_iter()
        .chain(home.map(|h| h.join(".icons")))
        .chain(
            dirs.split(':')
                .filter(|d| !d.is_empty())
                .map(|d| PathBuf::from(d).join("icons")),
        )
        .collect()
}

/// One theme's directories that hold app icons, with their nominal size; 0 is scalable.
#[derive(Clone, Debug, Default)]
struct Theme {
    dirs: Vec<(String, u32)>,
    inherits: Vec<String>,
}

/// Finds icons in a theme and its parents, remembering what it found.
#[derive(Debug)]
pub struct Icons {
    bases: Vec<PathBuf>,
    theme: String,
    themes: HashMap<String, Option<Theme>>,
    found: HashMap<(String, u32), Option<PathBuf>>,
}

impl Icons {
    pub fn new(theme: impl Into<String>, bases: Vec<PathBuf>) -> Self {
        Self {
            bases,
            theme: theme.into(),
            themes: HashMap::new(),
            found: HashMap::new(),
        }
    }

    /// Papirus in the light appearance, Papirus-Dark in the dark.
    pub fn papirus(dark: bool) -> Self {
        Self::new(if dark { "Papirus-Dark" } else { "Papirus" }, base_dirs())
    }

    /// The best file for `name` at `size` points: the nearest size at or above it, else the
    /// largest, SVG preferred at equal fit. An absolute path is taken as it is.
    pub fn lookup(&mut self, name: &str, size: u32) -> Option<PathBuf> {
        if name.starts_with('/') {
            return Path::new(name).exists().then(|| PathBuf::from(name));
        }
        let key = (name.to_owned(), size);
        if let Some(found) = self.found.get(&key) {
            return found.clone();
        }
        let mut order = Vec::new();
        let mut seen = HashSet::new();
        self.chain(&self.theme.clone(), &mut order, &mut seen);
        if seen.insert("hicolor".to_owned()) {
            order.push("hicolor".to_owned());
        }
        let found = order
            .iter()
            .find_map(|theme| self.in_theme(theme, name, size))
            .or_else(|| self.pixmap(name));
        self.found.insert(key, found.clone());
        found
    }

    fn chain(&mut self, theme: &str, order: &mut Vec<String>, seen: &mut HashSet<String>) {
        if !seen.insert(theme.to_owned()) {
            return;
        }
        order.push(theme.to_owned());
        let inherits = self
            .theme(theme)
            .map(|t| t.inherits.clone())
            .unwrap_or_default();
        for parent in inherits {
            self.chain(&parent, order, seen);
        }
    }

    fn theme(&mut self, name: &str) -> Option<&Theme> {
        if !self.themes.contains_key(name) {
            let theme = self
                .bases
                .iter()
                .map(|b| b.join(name).join("index.theme"))
                .find_map(|p| fs::read_to_string(p).ok())
                .map(|text| parse_index(&text));
            self.themes.insert(name.to_owned(), theme);
        }
        self.themes.get(name).and_then(Option::as_ref)
    }

    fn in_theme(&mut self, theme: &str, name: &str, size: u32) -> Option<PathBuf> {
        let dirs = self.theme(theme)?.dirs.clone();
        let mut best: Option<(u32, PathBuf)> = None;
        for (dir, nominal) in dirs {
            // Scalable fits any size; otherwise prefer the smallest at or above the size.
            let fit = match nominal {
                0 => 1,
                n if n >= size => n - size + 2,
                n => 10_000 + size - n,
            };
            if best.as_ref().is_some_and(|(b, _)| *b <= fit) {
                continue;
            }
            for base in &self.bases {
                for ext in ["svg", "png"] {
                    let path = base.join(theme).join(&dir).join(format!("{name}.{ext}"));
                    if path.exists() {
                        best = Some((fit, path));
                        break;
                    }
                }
                if best.as_ref().is_some_and(|(b, _)| *b == fit) {
                    break;
                }
            }
        }
        best.map(|(_, p)| p)
    }

    fn pixmap(&self, name: &str) -> Option<PathBuf> {
        self.bases
            .iter()
            .filter_map(|b| b.parent().map(|p| p.join("pixmaps")))
            .flat_map(|d| ["svg", "png", "xpm"].map(|e| d.join(format!("{name}.{e}"))))
            .find(|p| p.exists())
    }
}

/// The app directories of an `index.theme`, and what it inherits.
fn parse_index(text: &str) -> Theme {
    let mut theme = Theme::default();
    let mut group = String::new();
    let mut dirs: Vec<String> = Vec::new();
    let mut sizes: HashMap<String, (u32, bool, String)> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            group = name.to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if group == "Icon Theme" {
            match key {
                "Inherits" => {
                    theme.inherits = value.split(',').map(|s| s.trim().to_owned()).collect()
                }
                "Directories" | "ScaledDirectories" => dirs.extend(
                    value
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                ),
                _ => {}
            }
        } else {
            let entry = sizes.entry(group.clone()).or_default();
            match key {
                "Size" => entry.0 = value.parse().unwrap_or(0),
                "Scale" => entry.1 = value != "1",
                "Context" => entry.2 = value.to_owned(),
                "Type" if value == "Scalable" && entry.0 == 0 => {}
                _ => {}
            }
        }
    }
    for dir in dirs {
        let Some((size, scaled, context)) = sizes.get(&dir) else {
            continue;
        };
        if *scaled || (context != "Applications" && !dir.ends_with("/apps")) {
            continue;
        }
        let nominal = if dir.starts_with("scalable") {
            0
        } else {
            *size
        };
        theme.dirs.push((dir, nominal));
    }
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_falls_back_through_what_it_inherits_and_hicolor() {
        let root = std::env::temp_dir().join(format!("isle-icons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let put = |rel: &str, text: &str| {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        put(
            "Child/index.theme",
            "[Icon Theme]\nInherits=Parent\nDirectories=16x16/apps,64x64/apps\n[16x16/apps]\nSize=16\nContext=Applications\n[64x64/apps]\nSize=64\nContext=Applications\n",
        );
        put("Child/16x16/apps/kitty.svg", "");
        put("Child/64x64/apps/kitty.svg", "");
        put(
            "Parent/index.theme",
            "[Icon Theme]\nDirectories=scalable/apps\n[scalable/apps]\nSize=48\nType=Scalable\nContext=Applications\n",
        );
        put("Parent/scalable/apps/firefox.svg", "");
        put(
            "hicolor/index.theme",
            "[Icon Theme]\nDirectories=48x48/apps\n[48x48/apps]\nSize=48\nContext=Applications\n",
        );
        put("hicolor/48x48/apps/steam.png", "");

        let mut icons = Icons::new("Child", vec![root.clone()]);
        assert_eq!(
            icons.lookup("kitty", 32),
            Some(root.join("Child/64x64/apps/kitty.svg"))
        );
        assert_eq!(
            icons.lookup("kitty", 16),
            Some(root.join("Child/16x16/apps/kitty.svg"))
        );
        assert_eq!(
            icons.lookup("firefox", 32),
            Some(root.join("Parent/scalable/apps/firefox.svg"))
        );
        assert_eq!(
            icons.lookup("steam", 32),
            Some(root.join("hicolor/48x48/apps/steam.png"))
        );
        assert_eq!(icons.lookup("nothing", 32), None);
        fs::remove_dir_all(root).unwrap();
    }
}
