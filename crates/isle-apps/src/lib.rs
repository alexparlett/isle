//! Desktop entries and the apps they name: an index read off the main loop, and the
//! resolution of a window's app id to the app it belongs to (`compositor.md` Apps 1).

pub mod appimage;
pub mod badges;
pub mod exec;
pub mod icons;
pub mod metainfo;

use std::{
    collections::HashMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// One application's desktop entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The desktop id: the file's path under `applications/`, `/` as `-`, without `.desktop`.
    pub id: String,
    /// In the session's language where the entry has it, as are the other strings.
    pub name: String,
    pub generic_name: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
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
    pub only_show_in: Vec<String>,
    pub not_show_in: Vec<String>,
    /// Started over D-Bus (`org.freedesktop.Application`) on the bus name that is its id.
    pub dbus_activatable: bool,
    /// Has one main window, so a second launch raises it (`SingleMainWindow`).
    pub single_main_window: bool,
    pub try_exec: Option<String>,
    /// Its `TryExec` program is not installed, so launchers leave it out.
    pub missing: bool,
    /// The file it was read from.
    pub path: PathBuf,
}

impl Entry {
    /// Whether launchers list it in a session of `desktops` (`XDG_CURRENT_DESKTOP`'s names).
    pub fn listed_in(&self, desktops: &[String]) -> bool {
        let named = |list: &[String]| list.iter().any(|d| desktops.contains(d));
        !self.no_display
            && !self.missing
            && (self.only_show_in.is_empty() || named(&self.only_show_in))
            && !named(&self.not_show_in)
    }

    /// Whether launchers list it in this session.
    pub fn listed(&self) -> bool {
        self.listed_in(current_desktops())
    }

    /// Where the Apps view files it (`surfaces/spotlight.md` Categories): its first main
    /// category that maps, a bare `AudioVideo` as video.
    pub fn category(&self) -> Category {
        let has = |c: &str| self.categories.iter().any(|x| x == c);
        for c in &self.categories {
            let found = match c.as_str() {
                "Development" => Category::Developer,
                "Game" => Category::Games,
                "Graphics" => Category::Graphics,
                "Network" => Category::Internet,
                "Audio" => Category::Music,
                "Video" => Category::Video,
                "AudioVideo" if has("Audio") && !has("Video") => Category::Music,
                "AudioVideo" => Category::Video,
                "Office" => Category::Productivity,
                "Education" | "Science" => Category::Education,
                "Utility" | "System" | "Settings" => Category::Utilities,
                _ => continue,
            };
            return found;
        }
        Category::Other
    }
}

/// The Apps view's categories, in the order its filter lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Developer,
    Games,
    Graphics,
    Internet,
    Music,
    Video,
    Productivity,
    Education,
    Utilities,
    Other,
}

impl Category {
    pub const ALL: [Category; 10] = [
        Category::Developer,
        Category::Games,
        Category::Graphics,
        Category::Internet,
        Category::Music,
        Category::Video,
        Category::Productivity,
        Category::Education,
        Category::Utilities,
        Category::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Developer => "Developer tools",
            Category::Games => "Games",
            Category::Graphics => "Graphics and design",
            Category::Internet => "Internet",
            Category::Music => "Music",
            Category::Video => "Video",
            Category::Productivity => "Productivity",
            Category::Education => "Education",
            Category::Utilities => "Utilities",
            Category::Other => "Other",
        }
    }
}

/// `XDG_CURRENT_DESKTOP`'s names; Isle's own when it is unset.
pub fn current_desktops() -> &'static [String] {
    static DESKTOPS: OnceLock<Vec<String>> = OnceLock::new();
    DESKTOPS.get_or_init(|| {
        std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "Isle".into())
            .split(':')
            .map(str::to_owned)
            .collect()
    })
}

/// The session's language as the spec matches it, best first: `lang_COUNTRY@MODIFIER`,
/// `lang_COUNTRY`, `lang@MODIFIER`, `lang`.
pub fn locales() -> &'static [String] {
    static LOCALES: OnceLock<Vec<String>> = OnceLock::new();
    LOCALES.get_or_init(|| {
        let value = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find(|v| !v.is_empty())
            .unwrap_or_default();
        locales_of(&value)
    })
}

fn locales_of(value: &str) -> Vec<String> {
    let (rest, modifier) = match value.split_once('@') {
        Some((r, m)) => (r, Some(m)),
        None => (value, None),
    };
    let base = rest.split('.').next().unwrap_or("");
    if base.is_empty() || base == "C" || base == "POSIX" {
        return Vec::new();
    }
    let (lang, country) = match base.split_once('_') {
        Some((l, c)) => (l, Some(c)),
        None => (base, None),
    };
    let mut out = Vec::new();
    if let (Some(c), Some(m)) = (country, modifier) {
        out.push(format!("{lang}_{c}@{m}"));
    }
    if let Some(c) = country {
        out.push(format!("{lang}_{c}"));
    }
    if let Some(m) = modifier {
        out.push(format!("{lang}@{m}"));
    }
    out.push(lang.to_owned());
    out
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
    /// `$XDG_DATA_HOME/applications`, then each of `$XDG_DATA_DIRS`, then Flatpak's exports
    /// where the session's data path lacks them.
    pub fn data_dirs() -> Vec<PathBuf> {
        let home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        let dirs = std::env::var("XDG_DATA_DIRS")
            .ok()
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        let mut all: Vec<PathBuf> = home
            .iter()
            .cloned()
            .chain(dirs.split(':').filter(|d| !d.is_empty()).map(PathBuf::from))
            .collect();
        let flatpak = home
            .map(|h| h.join("flatpak/exports/share"))
            .into_iter()
            .chain([PathBuf::from("/var/lib/flatpak/exports/share")]);
        for dir in flatpak {
            if !all.contains(&dir) {
                all.push(dir);
            }
        }
        all.into_iter().map(|d| d.join("applications")).collect()
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
                    entry.missing = entry.try_exec.as_deref().is_some_and(|p| !installed(p));
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

/// Whether `program` is an executable file, by path or on `$PATH`.
fn installed(program: &str) -> bool {
    let executable = |p: &Path| {
        fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if program.contains('/') {
        return executable(Path::new(program));
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| executable(&dir.join(program))))
}

/// A string value with the spec's escapes undone.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// A list value: split on `;` but not on `\;`, each item unescaped, empty ones dropped.
fn list(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut item = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                item.push('\\');
                if let Some(next) = chars.next() {
                    item.push(next);
                }
            }
            ';' => out.push(std::mem::take(&mut item)),
            c => item.push(c),
        }
    }
    out.push(item);
    out.into_iter()
        .map(|i| unescape(i.trim()))
        .filter(|i| !i.is_empty())
        .collect()
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

/// The `[Desktop Entry]` group of an application entry in the session's language; `None` for
/// hidden entries and other types.
pub fn parse(id: &str, text: &str) -> Option<Entry> {
    parse_in(id, text, locales())
}

/// As `parse`, preferring the translations for `locales`, best first.
pub fn parse_in(id: &str, text: &str, locales: &[String]) -> Option<Entry> {
    // How well each translated key's value so far suits `locales`: lower is better, the
    // untranslated value last.
    let untranslated = locales.len();
    let mut best: HashMap<String, usize> = HashMap::new();
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
        let (key, rank) = match key.split_once('[') {
            Some((k, locale)) => match locales
                .iter()
                .position(|l| Some(l.as_str()) == locale.strip_suffix(']'))
            {
                Some(rank) => (k, rank),
                None => continue,
            },
            None => (key, untranslated),
        };
        let translatable = match &group {
            Group::Entry if matches!(key, "Name" | "GenericName" | "Comment" | "Keywords") => {
                Some(key.to_owned())
            }
            Group::Action(id) if key == "Name" => Some(format!("{id}/Name")),
            _ => None,
        };
        match translatable {
            Some(slot) => {
                let was = best.entry(slot).or_insert(usize::MAX);
                if rank > *was {
                    continue;
                }
                *was = rank;
            }
            None if rank != untranslated => continue,
            None => {}
        }
        match &group {
            Group::Entry => match key {
                "Type" => kind = Some(value),
                "Name" => entry.name = unescape(&value),
                "GenericName" => entry.generic_name = Some(unescape(&value)),
                "Keywords" => entry.keywords = list(&value),
                "Categories" => entry.categories = list(&value),
                "OnlyShowIn" => entry.only_show_in = list(&value),
                "NotShowIn" => entry.not_show_in = list(&value),
                "DBusActivatable" => entry.dbus_activatable = value == "true",
                "SingleMainWindow" | "X-GNOME-SingleWindow" => {
                    entry.single_main_window |= value == "true"
                }
                "TryExec" => entry.try_exec = Some(value),
                "Exec" => entry.exec = Some(value),
                "Icon" => entry.icon = Some(value),
                "StartupWMClass" => entry.startup_wm_class = Some(value),
                "NoDisplay" => entry.no_display = value == "true",
                "Comment" => entry.comment = Some(unescape(&value)),
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
                    "Name" => action.name = unescape(&value),
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

    #[test]
    fn the_new_keys_and_their_lists() {
        let text = "[Desktop Entry]\nType=Application\nName=Web\\sBrowser\nGenericName=Browser\nKeywords=internet;www;semi\\;colon;\nCategories=Network;WebBrowser;\nOnlyShowIn=GNOME;Isle;\nNotShowIn=KDE;\nDBusActivatable=true\nX-GNOME-SingleWindow=true\nTryExec=/nowhere/at/all\n";
        let e = parse_in("web", text, &[]).unwrap();
        assert_eq!(e.name, "Web Browser");
        assert_eq!(e.generic_name.as_deref(), Some("Browser"));
        assert_eq!(e.keywords, ["internet", "www", "semi;colon"]);
        assert_eq!(e.categories, ["Network", "WebBrowser"]);
        assert!(e.dbus_activatable && e.single_main_window);
        assert_eq!(e.try_exec.as_deref(), Some("/nowhere/at/all"));
        let isle = ["Isle".to_owned()];
        assert!(e.listed_in(&isle));
        assert!(!e.listed_in(&["KDE".to_owned()]));
        assert!(!e.listed_in(&["Hyprland".to_owned()]));
        assert!(
            !Entry {
                missing: true,
                ..e.clone()
            }
            .listed_in(&isle)
        );
        assert!(
            !Entry {
                no_display: true,
                ..e
            }
            .listed_in(&isle)
        );
    }

    #[test]
    fn translations_follow_the_session_language() {
        let text = "[Desktop Entry]\nType=Application\nName=Files\nName[fr]=Fichiers\nName[fr_CA]=Fichiers (CA)\nKeywords=folder;\nKeywords[fr]=dossier;\nComment[de]=Dateien\nActions=new;\n\n[Desktop Action new]\nName=New Window\nName[fr]=Nouvelle fenêtre\nExec=files --new\n";
        let fr = locales_of("fr_FR.UTF-8");
        assert_eq!(fr, ["fr_FR", "fr"]);
        let e = parse_in("files", text, &fr).unwrap();
        assert_eq!(e.name, "Fichiers");
        assert_eq!(e.keywords, ["dossier"]);
        assert_eq!(e.comment, None);
        assert_eq!(e.actions[0].name, "Nouvelle fenêtre");
        assert_eq!(
            parse_in("files", text, &locales_of("fr_CA.UTF-8"))
                .unwrap()
                .name,
            "Fichiers (CA)"
        );
        assert_eq!(
            parse_in("files", text, &locales_of("en_GB.UTF-8"))
                .unwrap()
                .name,
            "Files"
        );
        assert_eq!(
            locales_of("sr_RS@latin"),
            ["sr_RS@latin", "sr_RS", "sr@latin", "sr"]
        );
        assert!(locales_of("C.UTF-8").is_empty());
    }

    #[test]
    fn categories_map_to_the_apps_view() {
        let with = |c: &[&str]| Entry {
            categories: c.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(
            with(&["Network", "WebBrowser"]).category(),
            Category::Internet
        );
        assert_eq!(
            with(&["AudioVideo", "Audio", "Player"]).category(),
            Category::Music
        );
        assert_eq!(with(&["AudioVideo", "Video"]).category(), Category::Video);
        assert_eq!(with(&["AudioVideo"]).category(), Category::Video);
        assert_eq!(
            with(&["GTK", "System", "TerminalEmulator"]).category(),
            Category::Utilities
        );
        assert_eq!(with(&["Science", "Math"]).category(), Category::Education);
        assert_eq!(with(&[]).category(), Category::Other);
    }

    #[test]
    fn flatpak_exports_join_the_data_path() {
        let dirs = Index::data_dirs();
        assert!(dirs.contains(&PathBuf::from(
            "/var/lib/flatpak/exports/share/applications"
        )));
    }
}
