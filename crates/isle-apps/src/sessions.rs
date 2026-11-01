//! The sessions a greeter offers: `wayland-sessions` entries on the data path, the first
//! found by file id winning.

use std::{fs, path::PathBuf};

use crate::{exec::split, installed, unescape};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// The file's name without `.desktop`.
    pub id: String,
    pub name: String,
    /// `Exec`, split by the entry's quoting.
    pub exec: Vec<String>,
    /// `DesktopNames`, which become `XDG_CURRENT_DESKTOP`.
    pub desktop_names: Vec<String>,
}

impl Session {
    /// The variables a session is started with: they reach `pam_systemd` and the session.
    pub fn env(&self) -> Vec<String> {
        let mut env = vec!["XDG_SESSION_TYPE=wayland".to_owned()];
        if !self.desktop_names.is_empty() {
            env.push(format!(
                "XDG_CURRENT_DESKTOP={}",
                self.desktop_names.join(":")
            ));
            env.push(format!("XDG_SESSION_DESKTOP={}", self.desktop_names[0]));
        }
        env
    }
}

/// `wayland-sessions` under `$XDG_DATA_DIRS` (`/usr/local/share:/usr/share` when unset).
pub fn dirs() -> Vec<PathBuf> {
    let data = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    data.split(':')
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join("wayland-sessions"))
        .collect()
}

/// Every session in `dirs`, by name; hidden, undisplayed and uninstalled ones left out.
pub fn sessions(dirs: &[PathBuf]) -> Vec<Session> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(read) = fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = read.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        for path in files {
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".desktop"))
            else {
                continue;
            };
            if !seen.insert(id.to_owned()) {
                continue;
            }
            if let Some(session) = fs::read_to_string(&path).ok().and_then(|t| parse(id, &t)) {
                out.push(session);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// One entry's `[Desktop Entry]` group, untranslated.
pub fn parse(id: &str, text: &str) -> Option<Session> {
    let mut in_entry = false;
    let (mut name, mut exec, mut names) = (None, None, Vec::new());
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        let Some((key, value)) = line.split_once('=').filter(|_| in_entry) else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" => name = Some(unescape(value)),
            "Exec" => exec = Some(value.to_owned()),
            "DesktopNames" => {
                names = value
                    .split(';')
                    .filter(|n| !n.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            "TryExec" if !installed(value) => return None,
            "Hidden" | "NoDisplay" if value == "true" => return None,
            _ => {}
        }
    }
    let exec = split(&exec?)?;
    (!exec.is_empty()).then_some(())?;
    Some(Session {
        id: id.to_owned(),
        name: name.filter(|n| !n.is_empty())?,
        exec,
        desktop_names: names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_gives_its_command_and_variables() {
        let s = parse(
            "hyprland-uwsm",
            "[Desktop Entry]\nName=Hyprland (uwsm-managed)\nExec=uwsm start -e -D Hyprland hyprland.desktop\nDesktopNames=Hyprland\nType=Application\n",
        )
        .unwrap();
        assert_eq!(s.name, "Hyprland (uwsm-managed)");
        assert_eq!(
            s.exec,
            ["uwsm", "start", "-e", "-D", "Hyprland", "hyprland.desktop"]
        );
        assert_eq!(
            s.env(),
            [
                "XDG_SESSION_TYPE=wayland",
                "XDG_CURRENT_DESKTOP=Hyprland",
                "XDG_SESSION_DESKTOP=Hyprland"
            ]
        );
    }

    #[test]
    fn quoting_and_names_are_the_entrys() {
        let s = parse(
            "x",
            "[Desktop Entry]\nName=Two\\sWords\nExec=\"/opt/my session/run\" --flag\nDesktopNames=A;B;\n",
        )
        .unwrap();
        assert_eq!(s.name, "Two Words");
        assert_eq!(s.exec, ["/opt/my session/run", "--flag"]);
        assert_eq!(s.env()[1], "XDG_CURRENT_DESKTOP=A:B");
    }

    #[test]
    fn hidden_missing_and_broken_entries_are_left_out() {
        for text in [
            "[Desktop Entry]\nName=A\nExec=a\nHidden=true\n",
            "[Desktop Entry]\nName=A\nExec=a\nNoDisplay=true\n",
            "[Desktop Entry]\nName=A\nExec=a\nTryExec=/nowhere/at/all\n",
            "[Desktop Entry]\nName=A\n",
            "[Desktop Entry]\nExec=a\n",
            "[Other]\nName=A\nExec=a\n",
        ] {
            assert_eq!(parse("a", text), None, "{text}");
        }
    }

    #[test]
    fn the_first_directory_wins_by_file_id() {
        let root = std::env::temp_dir().join(format!("isle-sessions-{}", std::process::id()));
        let (first, second) = (root.join("a"), root.join("b"));
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(
            first.join("isle.desktop"),
            "[Desktop Entry]\nName=Isle\nExec=isle-session\nDesktopNames=Isle\n",
        )
        .unwrap();
        fs::write(
            second.join("isle.desktop"),
            "[Desktop Entry]\nName=Old\nExec=old\n",
        )
        .unwrap();
        fs::write(
            second.join("hyprland.desktop"),
            "[Desktop Entry]\nName=Hyprland\nExec=start-hyprland\n",
        )
        .unwrap();
        let found = sessions(&[first, second]);
        let names: Vec<_> = found.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Hyprland", "Isle"]);
        let _ = fs::remove_dir_all(root);
    }
}
