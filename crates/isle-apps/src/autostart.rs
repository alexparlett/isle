//! Login Items: the XDG autostart entries Isle starts at sign-in, read by the rules systemd's
//! generator uses (`p14a-session.md` choice 17), and Settings' list of them with the user
//! units the user enabled (`p20b-session.md` choice 20).

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{Entry, Index, installed, parse};

/// `$XDG_CONFIG_HOME/autostart`, then each of `$XDG_CONFIG_DIRS`' (`/etc/xdg` when unset).
pub fn dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let system = std::env::var("XDG_CONFIG_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/etc/xdg".into());
    home.into_iter()
        .chain(
            system
                .split(':')
                .filter(|d| !d.is_empty())
                .map(PathBuf::from),
        )
        .map(|d| d.join("autostart"))
        .collect()
}

/// The entries to start in a session of `desktop`, in file-id order. An earlier directory's
/// file hides a later one's of the same id, a hidden file included.
pub fn items(dirs: &[PathBuf], desktop: &str) -> Vec<Entry> {
    let mut seen = Vec::new();
    let mut items = Vec::new();
    for dir in dirs {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
            .collect();
        files.sort();
        for path in files {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else {
                continue;
            };
            if seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            if let Some(entry) = read(&id, &path).filter(|e| starts(e, desktop)) {
                items.push(entry);
            }
        }
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

fn read(id: &str, path: &Path) -> Option<Entry> {
    let mut entry = parse(id, &fs::read_to_string(path).ok()?)?;
    entry.path = path.to_owned();
    entry.missing = entry.try_exec.as_deref().is_some_and(|p| !installed(p));
    Some(entry)
}

/// `NoDisplay` does not matter here: most Login Items are hidden from launchers.
fn starts(entry: &Entry, desktop: &str) -> bool {
    let named = |list: &[String]| list.iter().any(|d| d == desktop);
    !entry.autostart_off
        && !entry.missing
        && entry.exec.is_some()
        && (entry.only_show_in.is_empty() || named(&entry.only_show_in))
        && !named(&entry.not_show_in)
}

/// Where Login Items live: the autostart folders, the user's first, and the user manager's
/// unit folders, the user's own first.
#[derive(Clone, Debug)]
pub struct Places {
    pub autostart: Vec<PathBuf>,
    pub units: Vec<PathBuf>,
}

impl Places {
    /// This session's: `dirs()`, then `$XDG_CONFIG_HOME/systemd/user`, `/etc/systemd/user` and
    /// `/usr/lib/systemd/user`.
    pub fn session() -> Self {
        let autostart = dirs();
        let config = autostart
            .first()
            .and_then(|d| d.parent())
            .map(Path::to_path_buf)
            .unwrap_or_default();
        Self {
            autostart,
            units: vec![
                config.join("systemd/user"),
                "/etc/systemd/user".into(),
                "/usr/lib/systemd/user".into(),
            ],
        }
    }

    fn user(&self) -> &Path {
        &self.autostart[0]
    }

    fn user_units(&self) -> &Path {
        &self.units[0]
    }

    /// The folders whose changes change the list.
    pub fn watched(&self) -> Vec<PathBuf> {
        let mut dirs = self.autostart.clone();
        dirs.push(self.user_units().to_owned());
        dirs
    }
}

/// What starts a Login Item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// An autostart entry, by its file id.
    Entry,
    /// A user unit the user enabled, by its name.
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub source: Source,
    pub on: bool,
    /// Isle's own: shown, never switched.
    pub locked: bool,
    /// Non-empty for an entry another desktop's session starts: its switch is disabled.
    pub only_in: Vec<String>,
}

/// macOS 13's two groups: apps opened at login, and what else runs in the background.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoginItems {
    pub open: Vec<LoginItem>,
    pub background: Vec<LoginItem>,
}

impl LoginItems {
    pub fn is_empty(&self) -> bool {
        self.open.is_empty() && self.background.is_empty()
    }
}

/// Each autostart id's files, in the folders' order.
fn files_by_id(places: &Places) -> Vec<(String, Vec<PathBuf>)> {
    let mut ids: Vec<(String, Vec<PathBuf>)> = Vec::new();
    for dir in &places.autostart {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
            .collect();
        files.sort();
        for path in files {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match ids.iter_mut().find(|(i, _)| i == id) {
                Some((_, paths)) => paths.push(path),
                None => ids.push((id.to_owned(), vec![path])),
            }
        }
    }
    ids
}

/// The Login Items for a session of `desktop`. An app is an entry whose id names one `apps`
/// lists; one the user removed is not shown. `remembered` names units switched off here,
/// shown off while their files remain.
pub fn login_items(
    places: &Places,
    apps: &Index,
    desktop: &str,
    remembered: &[String],
) -> LoginItems {
    let mut items = LoginItems::default();
    for (id, paths) in files_by_id(places) {
        // The first file decides; a hidden one is turned off, described by the next.
        let first = read(&id, &paths[0]);
        let (entry, on) = match first {
            Some(entry) => {
                let on = !entry.autostart_off;
                (entry, on)
            }
            None => match paths[1..].iter().find_map(|p| read(&id, p)) {
                Some(entry) => (entry, false),
                None => continue,
            },
        };
        if entry.missing || entry.exec.is_none() {
            continue;
        }
        let named = |list: &[String]| list.iter().any(|d| d == desktop);
        if named(&entry.not_show_in) {
            continue;
        }
        let only_in = if entry.only_show_in.is_empty() || named(&entry.only_show_in) {
            Vec::new()
        } else {
            entry.only_show_in.clone()
        };
        let app = apps.get(&id).filter(|a| !a.no_display && !a.missing);
        let item = LoginItem {
            id: id.clone(),
            name: app.map_or_else(|| entry.name.clone(), |a| a.name.clone()),
            icon: app.and_then(|a| a.icon.clone()).or(entry.icon.clone()),
            source: Source::Entry,
            on,
            locked: false,
            only_in,
        };
        match (app.is_some(), item.on, item.only_in.is_empty()) {
            (true, true, true) => items.open.push(item),
            (true, false, true) => {}
            _ => items.background.push(item),
        }
    }
    items
        .background
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let mut units = enabled_units(places.user_units());
    for unit in remembered {
        if !units.contains(unit) && unit_file(places, unit).is_some() {
            units.push(unit.clone());
        }
    }
    units.sort();
    let enabled = enabled_units(places.user_units());
    for unit in units {
        let name = unit_file(places, &unit)
            .and_then(|f| fs::read_to_string(f).ok())
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix("Description=").map(|d| d.trim().to_owned()))
            })
            .unwrap_or_else(|| unit.clone());
        items.background.push(LoginItem {
            on: enabled.contains(&unit),
            locked: unit.starts_with("isle-") || unit.starts_with("isle."),
            id: unit,
            name,
            icon: None,
            source: Source::Unit,
            only_in: Vec::new(),
        });
    }
    items
        .open
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

/// The units the user enabled: the links in their unit folder's `*.wants` folders.
fn enabled_units(dir: &Path) -> Vec<String> {
    let mut units: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".wants"))
        .flat_map(|e| fs::read_dir(e.path()).into_iter().flatten().flatten())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_symlink()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    units.sort();
    units.dedup();
    units
}

fn unit_file(places: &Places, unit: &str) -> Option<PathBuf> {
    places
        .units
        .iter()
        .map(|d| d.join(unit))
        .find(|p| p.exists())
}

/// Whether `app` is in Open at Login.
pub fn opens_at_login(places: &Places, apps: &Index, app: &str) -> bool {
    login_items(places, apps, "Isle", &[])
        .open
        .iter()
        .any(|i| i.id == app)
}

/// `text` with `Hidden` set or cleared in its `[Desktop Entry]` group; on, an
/// `X-GNOME-Autostart-enabled=false` goes too.
fn with_hidden(text: &str, hidden: bool) -> String {
    let mut out = String::new();
    let mut in_entry = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_entry = trimmed == "[Desktop Entry]";
            out.push_str(line);
            out.push('\n');
            if in_entry && hidden {
                out.push_str("Hidden=true\n");
            }
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        let dropped = in_entry
            && (key == "Hidden"
                || (!hidden && key == "X-GNOME-Autostart-enabled" && trimmed.ends_with("false")));
        if !dropped {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The text of `id`'s entry in a folder after the user's, if one has it.
fn system_text(places: &Places, id: &str) -> Option<String> {
    places.autostart[1..]
        .iter()
        .map(|d| d.join(format!("{id}.desktop")))
        .find_map(|p| fs::read_to_string(p).ok())
}

fn write_user(places: &Places, id: &str, text: &str) -> io::Result<()> {
    fs::create_dir_all(places.user())?;
    fs::write(places.user().join(format!("{id}.desktop")), text)
}

/// +: `app` opens at login, from a copy of its own entry.
pub fn add(places: &Places, app: &Entry) -> io::Result<()> {
    let text = fs::read_to_string(&app.path)?;
    write_user(places, &app.id, &with_hidden(&text, false))
}

/// −: the user's entry deleted, or a hidden copy written over a system one.
pub fn remove(places: &Places, id: &str) -> io::Result<()> {
    match system_text(places, id) {
        Some(text) => write_user(places, id, &with_hidden(&text, true)),
        None => match fs::remove_file(places.user().join(format!("{id}.desktop"))) {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        },
    }
}

/// An entry's switch: off hides it in the user's folder, on shows it again, dropping a copy
/// that then says no more than the system's.
pub fn set_on(places: &Places, id: &str, on: bool) -> io::Result<()> {
    let user = places.user().join(format!("{id}.desktop"));
    let system = system_text(places, id);
    let current = fs::read_to_string(&user).ok();
    let Some(text) = current.or_else(|| system.clone()) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no entry {id}"),
        ));
    };
    let next = with_hidden(&text, !on);
    if on
        && system
            .as_deref()
            .is_some_and(|s| with_hidden(s, false) == next)
    {
        return match fs::remove_file(&user) {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        };
    }
    write_user(places, id, &next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, id: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join(format!("{id}.desktop")),
            format!("[Desktop Entry]\nType=Application\nName={id}\nExec=true\n{body}"),
        )
        .unwrap();
    }

    #[test]
    fn the_generators_rules() {
        let root = std::env::temp_dir().join(format!("isle-autostart-{}", std::process::id()));
        let (user, system) = (root.join("user"), root.join("system"));
        write(&system, "plain", "NoDisplay=true\n");
        write(&system, "gnome-only", "OnlyShowIn=GNOME;Unity;\n");
        write(&system, "for-isle", "OnlyShowIn=GNOME;Isle;\n");
        write(&system, "not-here", "NotShowIn=Isle;\n");
        write(&system, "turned-off", "X-GNOME-Autostart-enabled=false\n");
        write(&system, "skipped", "X-systemd-skip=true\n");
        write(&system, "uninstalled", "TryExec=/nonexistent/program\n");
        write(&system, "hidden-by-user", "");
        write(&user, "hidden-by-user", "Hidden=true\n");
        write(&system, "edited", "Exec=false\n");
        write(&user, "edited", "Exec=mine\n");
        let items = items(&[user, system], "Isle");
        let ids: Vec<&str> = items.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["edited", "for-isle", "plain"]);
        assert_eq!(items[0].exec.as_deref(), Some("mine"));
        fs::remove_dir_all(&root).unwrap();
    }

    fn places(root: &Path) -> Places {
        Places {
            autostart: vec![root.join("home/autostart"), root.join("etc/autostart")],
            units: vec![root.join("home/systemd"), root.join("lib/systemd")],
        }
    }

    fn app(root: &Path, id: &str, name: &str) -> Entry {
        let dir = root.join("applications");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{id}.desktop"));
        let text =
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec=true\nIcon={id}\n");
        fs::write(&path, &text).unwrap();
        let mut entry = parse(id, &text).unwrap();
        entry.path = path;
        entry
    }

    fn unit(root: &Path, name: &str, enabled: bool) {
        let lib = root.join("lib/systemd");
        fs::create_dir_all(&lib).unwrap();
        fs::write(
            lib.join(name),
            format!("[Unit]\nDescription={name} daemon\n"),
        )
        .unwrap();
        if enabled {
            let wants = root.join("home/systemd/default.target.wants");
            fs::create_dir_all(&wants).unwrap();
            std::os::unix::fs::symlink(lib.join(name), wants.join(name)).unwrap();
        }
    }

    fn ids(items: &[LoginItem]) -> Vec<(&str, bool)> {
        items.iter().map(|i| (i.id.as_str(), i.on)).collect()
    }

    #[test]
    fn login_items_in_two_groups() {
        let root = std::env::temp_dir().join(format!("isle-login-items-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let p = places(&root);
        let apps = Index::from_entries([
            app(&root, "org.app.Mail", "Mail"),
            app(&root, "org.app.Chat", "Chat"),
            app(&root, "org.app.Gnomey", "Gnomey"),
        ]);
        write(&p.autostart[1], "org.app.Mail", "");
        write(&p.autostart[1], "org.app.Chat", "");
        write(&p.autostart[1], "org.app.Gnomey", "OnlyShowIn=GNOME;\n");
        write(&p.autostart[1], "gnome-keyring", "");
        write(&p.autostart[1], "kde-only", "NotShowIn=Isle;\n");
        write(&p.autostart[0], "my-script", "");
        unit(&root, "xremap.service", true);
        unit(&root, "isle-dev-sync.service", true);
        unit(&root, "vendor.service", false);

        let items = login_items(&p, &apps, "Isle", &[]);
        assert_eq!(
            ids(&items.open),
            [("org.app.Chat", true), ("org.app.Mail", true)]
        );
        assert_eq!(
            ids(&items.background),
            [
                ("gnome-keyring", true),
                ("org.app.Gnomey", true),
                ("my-script", true),
                ("isle-dev-sync.service", true),
                ("xremap.service", true),
            ]
        );
        let gnomey = items
            .background
            .iter()
            .find(|i| i.id == "org.app.Gnomey")
            .unwrap();
        assert_eq!(gnomey.only_in, ["GNOME"]);
        assert!(
            items
                .background
                .iter()
                .find(|i| i.id == "isle-dev-sync.service")
                .unwrap()
                .locked
        );
        assert_eq!(items.background[4].name, "xremap.service daemon");

        // − on a system app hides it; on a user one deletes it.
        remove(&p, "org.app.Mail").unwrap();
        assert!(
            fs::read_to_string(p.autostart[0].join("org.app.Mail.desktop"))
                .unwrap()
                .contains("Hidden=true")
        );
        assert!(!opens_at_login(&p, &apps, "org.app.Mail"));
        add(&p, apps.get("org.app.Mail").unwrap()).unwrap();
        assert!(opens_at_login(&p, &apps, "org.app.Mail"));
        remove(&p, "my-script").unwrap();
        assert!(!p.autostart[0].join("my-script.desktop").exists());

        // A switch off and on again leaves no copy behind.
        set_on(&p, "gnome-keyring", false).unwrap();
        let items = login_items(&p, &apps, "Isle", &[]);
        assert!(
            !items
                .background
                .iter()
                .find(|i| i.id == "gnome-keyring")
                .unwrap()
                .on
        );
        assert!(
            !self::items(&p.autostart, "Isle")
                .iter()
                .any(|e| e.id == "gnome-keyring")
        );
        set_on(&p, "gnome-keyring", true).unwrap();
        assert!(!p.autostart[0].join("gnome-keyring.desktop").exists());

        // A unit switched off stays listed while remembered.
        fs::remove_file(root.join("home/systemd/default.target.wants/xremap.service")).unwrap();
        let items = login_items(&p, &apps, "Isle", &["xremap.service".into()]);
        assert!(
            !items
                .background
                .iter()
                .find(|i| i.id == "xremap.service")
                .unwrap()
                .on
        );
        let items = login_items(&p, &apps, "Isle", &[]);
        assert!(!items.background.iter().any(|i| i.id == "xremap.service"));
        fs::remove_dir_all(&root).unwrap();
    }
}
