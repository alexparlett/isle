//! Login Items: the XDG autostart entries Isle starts at sign-in, read by the rules systemd's
//! generator uses (`p14a-session.md` choice 17).

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Entry, installed, parse};

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
}
