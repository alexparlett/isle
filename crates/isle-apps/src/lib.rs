//! Desktop entries and the apps they name: an index read off the main loop, and the
//! resolution of a window's app id to the app it belongs to (`compositor.md` Apps 1).

pub mod exec;
pub mod icons;
pub mod metainfo;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// One application's desktop entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The desktop id: the file's path under `applications/`, `/` as `-`, without `.desktop`.
    pub id: String,
    pub name: String,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub startup_wm_class: Option<String>,
    /// Launchers do not list it; it still names its windows' app.
    pub no_display: bool,
    pub comment: Option<String>,
    /// Runs in a terminal.
    pub terminal: bool,
    /// The entry's extra actions: New Window, New Private Window and the like.
    pub actions: Vec<EntryAction>,
    /// The file it was read from.
    pub path: PathBuf,
}

/// A `[Desktop Action]` group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryAction {
    pub id: String,
    pub name: String,
    pub exec: Option<String>,
}

/// Every application entry on the data path, the first found winning, as the spec says.
#[derive(Clone, Debug, Default)]
pub struct Index {
    entries: Vec<Entry>,
    by_id: HashMap<String, usize>,
    by_lower_id: HashMap<String, usize>,
    by_wm_class: HashMap<String, usize>,
    by_last_part: HashMap<String, usize>,
}

impl Index {
    /// `$XDG_DATA_HOME/applications`, then each of `$XDG_DATA_DIRS`.
    pub fn data_dirs() -> Vec<PathBuf> {
        let home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        let dirs = std::env::var("XDG_DATA_DIRS")
            .ok()
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        home.into_iter()
            .chain(dirs.split(':').filter(|d| !d.is_empty()).map(PathBuf::from))
            .map(|d| d.join("applications"))
            .collect()
    }

    /// Reads every `.desktop` file under `dirs`; blocking, so never on the main loop.
    pub fn scan(dirs: &[PathBuf]) -> Index {
        let mut index = Index::default();
        for dir in dirs {
            let mut files = Vec::new();
            collect(dir, dir, &mut files);
            files.sort();
            for (id, path) in files {
                if index.by_id.contains_key(&id) {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                if let Some(mut entry) = parse(&id, &text) {
                    entry.path = path.clone();
                    index.insert(entry);
                } else {
                    // A hidden or non-application entry still shadows the same id later on.
                    index.by_id.insert(id, usize::MAX);
                }
            }
        }
        index.by_id.retain(|_, i| *i != usize::MAX);
        index
    }

    pub fn from_entries(entries: impl IntoIterator<Item = Entry>) -> Index {
        let mut index = Index::default();
        for entry in entries {
            index.insert(entry);
        }
        index
    }

    fn insert(&mut self, entry: Entry) {
        let i = self.entries.len();
        self.by_id.insert(entry.id.clone(), i);
        self.by_lower_id.entry(entry.id.to_lowercase()).or_insert(i);
        if let Some(class) = &entry.startup_wm_class {
            self.by_wm_class.entry(class.to_lowercase()).or_insert(i);
        }
        let last = entry
            .id
            .rsplit('.')
            .next()
            .unwrap_or(&entry.id)
            .to_lowercase();
        self.by_last_part.entry(last).or_insert(i);
        self.entries.push(entry);
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.by_id.get(id).map(|i| &self.entries[*i])
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The app a window with `app_id` (or X11 class) belongs to: its desktop id when an entry
    /// claims it, else the id itself. Steam games and gamescope stay apps of their own.
    pub fn resolve(&self, app_id: &str) -> String {
        if app_id.is_empty() || app_id.starts_with("steam_app_") || app_id == "gamescope" {
            return app_id.to_owned();
        }
        let lower = app_id.to_lowercase();
        let last = lower.rsplit('.').next().unwrap_or(&lower).to_owned();
        let found = self
            .by_id
            .get(app_id)
            .or_else(|| self.by_wm_class.get(&lower))
            .or_else(|| self.by_lower_id.get(&lower))
            .or_else(|| self.by_last_part.get(&lower))
            .or_else(|| self.by_lower_id.get(&last));
        match found {
            Some(i) => self.entries[*i].id.clone(),
            None => app_id.to_owned(),
        }
    }

    /// The name shown for `app`: its entry's, else the app id.
    pub fn name(&self, app: &str) -> String {
        self.get(app)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| app.to_owned())
    }
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "desktop")
            && let Ok(relative) = path.strip_prefix(root)
        {
            let id = relative
                .with_extension("")
                .to_string_lossy()
                .replace('/', "-");
            out.push((id, path));
        }
    }
}

/// The `[Desktop Entry]` group of an application entry; `None` for hidden entries and other
/// types.
pub fn parse(id: &str, text: &str) -> Option<Entry> {
    let mut entry = Entry {
        id: id.to_owned(),
        ..Default::default()
    };
    // Which group the lines belong to: the entry, an action by its id, or another.
    enum Group {
        Entry,
        Action(String),
        Other,
    }
    let mut group = Group::Other;
    let mut kind = None;
    let mut listed: Vec<String> = Vec::new();
    let mut actions: HashMap<String, EntryAction> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            group = match line.strip_prefix("[Desktop Action ").and_then(|l| l.strip_suffix(']')) {
                Some(action) => Group::Action(action.to_owned()),
                None if line == "[Desktop Entry]" => Group::Entry,
                None => Group::Other,
            };
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim().to_owned());
        match &group {
            Group::Entry => match key {
                "Type" => kind = Some(value),
                "Name" => entry.name = value,
                "Exec" => entry.exec = Some(value),
                "Icon" => entry.icon = Some(value),
                "StartupWMClass" => entry.startup_wm_class = Some(value),
                "NoDisplay" => entry.no_display = value == "true",
                "Comment" => entry.comment = Some(value),
                "Terminal" => entry.terminal = value == "true",
                "Actions" => {
                    listed = value
                        .split(';')
                        .filter(|a| !a.is_empty())
                        .map(str::to_owned)
                        .collect()
                }
                "Hidden" if value == "true" => return None,
                _ => {}
            },
            Group::Action(id) => {
                let action = actions.entry(id.clone()).or_insert_with(|| EntryAction {
                    id: id.clone(),
                    ..Default::default()
                });
                match key {
                    "Name" => action.name = value,
                    "Exec" => action.exec = Some(value),
                    _ => {}
                }
            }
            Group::Other => {}
        }
    }
    entry.actions = listed
        .iter()
        .filter_map(|id| actions.remove(id))
        .filter(|a| !a.name.is_empty())
        .collect();
    (kind.as_deref() == Some("Application") && !entry.name.is_empty()).then_some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, class: Option<&str>) -> Entry {
        Entry {
            id: id.into(),
            name: id.into(),
            startup_wm_class: class.map(Into::into),
            ..Default::default()
        }
    }

    fn index() -> Index {
        Index::from_entries([
            entry("org.mozilla.firefox", None),
            entry("code", Some("Code")),
            entry("org.gnome.Nautilus", None),
            entry("kitty", None),
            entry("steam", Some("steam")),
            entry("com.discordapp.Discord", None),
        ])
    }

    #[test]
    fn app_ids_resolve_to_their_entries() {
        let index = index();
        assert_eq!(index.resolve("org.mozilla.firefox"), "org.mozilla.firefox");
        assert_eq!(index.resolve("firefox"), "org.mozilla.firefox");
        assert_eq!(index.resolve("Code"), "code");
        assert_eq!(index.resolve("org.gnome.nautilus"), "org.gnome.Nautilus");
        assert_eq!(index.resolve("Nautilus"), "org.gnome.Nautilus");
        assert_eq!(index.resolve("kitty"), "kitty");
        assert_eq!(index.resolve("io.github.someone.Kitty"), "kitty");
        assert_eq!(index.resolve("discord"), "com.discordapp.Discord");
    }

    #[test]
    fn games_and_unknown_ids_are_their_own_apps() {
        let index = index();
        assert_eq!(index.resolve("steam_app_1091500"), "steam_app_1091500");
        assert_eq!(index.resolve("gamescope"), "gamescope");
        assert_eq!(index.resolve("isle.test.Nothing"), "isle.test.Nothing");
        assert_eq!(index.resolve(""), "");
    }

    #[test]
    fn entries_parse_from_their_group_only() {
        let text = "[Desktop Entry]\nType=Application\nName=Files\nName[fr]=Fichiers\nExec=nautilus %U\nIcon=org.gnome.Nautilus\nStartupWMClass=nautilus\n\n[Desktop Action new]\nName=New Window\nExec=nautilus --new-window\n";
        let e = parse("org.gnome.Nautilus", text).unwrap();
        assert_eq!(e.name, "Files");
        assert_eq!(e.exec.as_deref(), Some("nautilus %U"));
        assert_eq!(e.startup_wm_class.as_deref(), Some("nautilus"));
        assert!(
            parse(
                "x",
                "[Desktop Entry]\nType=Application\nName=X\nHidden=true\n"
            )
            .is_none()
        );
        assert!(parse("x", "[Desktop Entry]\nType=Link\nName=X\n").is_none());
    }

    #[test]
    fn the_first_directory_wins_and_subdirectories_join_the_id() {
        let root = std::env::temp_dir().join(format!("isle-apps-{}", std::process::id()));
        let (a, b) = (root.join("a/applications"), root.join("b/applications"));
        fs::create_dir_all(a.join("kde")).unwrap();
        fs::create_dir_all(&b).unwrap();
        let app = |name: &str| format!("[Desktop Entry]\nType=Application\nName={name}\n");
        fs::write(a.join("kitty.desktop"), app("Mine")).unwrap();
        fs::write(b.join("kitty.desktop"), app("System")).unwrap();
        fs::write(a.join("kde/dolphin.desktop"), app("Dolphin")).unwrap();
        fs::write(
            a.join("gone.desktop"),
            "[Desktop Entry]\nType=Application\nName=G\nHidden=true\n",
        )
        .unwrap();
        fs::write(b.join("gone.desktop"), app("Gone")).unwrap();
        let index = Index::scan(&[a, b]);
        assert_eq!(index.get("kitty").unwrap().name, "Mine");
        assert_eq!(index.get("kde-dolphin").unwrap().name, "Dolphin");
        assert!(index.get("gone").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actions_are_read_in_the_order_the_entry_lists_them() {
        let text = "[Desktop Entry]\nType=Application\nName=Files\nComment=Browse\nActions=window;private;\n\n[Desktop Action private]\nName=New Private Window\nExec=files --private\n\n[Desktop Action window]\nName=New Window\nExec=files --new-window\n\n[Desktop Action unlisted]\nName=X\n";
        let entry = parse("files", text).unwrap();
        assert_eq!(entry.comment.as_deref(), Some("Browse"));
        let names: Vec<&str> = entry.actions.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["New Window", "New Private Window"]);
        assert_eq!(entry.actions[0].exec.as_deref(), Some("files --new-window"));
    }
}
