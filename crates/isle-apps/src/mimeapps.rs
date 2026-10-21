//! Which app opens a file, by the MIME applications spec: the desktop's own list first, then
//! the user's, then the system's, each with its defaults, added and removed associations,
//! over the types the apps' entries declare (`p15-files.md` choice 12). Writes go to the
//! user's `mimeapps.list`, as GTK writes them, except the folder handler, which is Isle's own.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{Index, mime};

/// One list's three groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct List {
    pub defaults: HashMap<String, Vec<String>>,
    pub added: HashMap<String, Vec<String>>,
    pub removed: HashMap<String, Vec<String>>,
}

pub fn parse(text: &str) -> List {
    let mut list = List::default();
    let mut group = "";
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            group = match name {
                "Default Applications" => "defaults",
                "Added Associations" => "added",
                "Removed Associations" => "removed",
                _ => "",
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let apps: Vec<String> = value
            .split(';')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(|a| a.strip_suffix(".desktop").unwrap_or(a).to_owned())
            .collect();
        let into = match group {
            "defaults" => &mut list.defaults,
            "added" => &mut list.added,
            "removed" => &mut list.removed,
            _ => continue,
        };
        into.entry(key.trim().to_owned()).or_default().extend(apps);
    }
    list
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

fn split_env(key: &str, fallback: &[&str]) -> Vec<PathBuf> {
    std::env::var_os(key)
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_else(|| fallback.iter().map(PathBuf::from).collect())
}

/// The lists in the spec's order of precedence, the first winning; Isle's own list always
/// among them, as the compositor resolves for Isle whatever its own environment says.
pub fn list_paths() -> Vec<PathBuf> {
    let mut desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|d| !d.is_empty())
        .map(str::to_lowercase)
        .collect();
    if !desktops.iter().any(|d| d == "isle") {
        desktops.insert(0, "isle".into());
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.extend(config_home());
    dirs.extend(split_env("XDG_CONFIG_DIRS", &["/etc/xdg"]));
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    dirs.extend(data_home.map(|d| d.join("applications")));
    dirs.extend(split_env("XDG_DATA_DIRS", &["/usr/local/share", "/usr/share"]).into_iter().map(|d| d.join("applications")));
    let mut paths = Vec::new();
    for dir in dirs {
        for desktop in &desktops {
            paths.push(dir.join(format!("{desktop}-mimeapps.list")));
        }
        paths.push(dir.join("mimeapps.list"));
    }
    paths
}

/// Every list that could be read, best first.
pub fn read_lists() -> Vec<List> {
    list_paths().iter().filter_map(|p| std::fs::read_to_string(p).ok()).map(|t| parse(&t)).collect()
}

/// The apps that open `kind`, the default first, as the lists and the entries say; `kind`'s
/// parent types follow it, so a Markdown file is offered the text editors.
pub fn apps_for(kind: &str, lists: &[List], index: &Index) -> Vec<String> {
    let types = mime::types();
    let mut chain = vec![kind.to_owned()];
    if kind.starts_with("text/") && kind != mime::PLAIN {
        chain.push(mime::PLAIN.to_owned());
    }
    let mut out: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let exists = |id: &str| index.get(id).is_some();
    let push = |id: &String, out: &mut Vec<String>, removed: &[String]| {
        if exists(id) && !removed.contains(id) && !out.contains(id) {
            out.push(id.clone());
        }
    };
    for t in &chain {
        for list in lists {
            removed.extend(list.removed.get(t).cloned().unwrap_or_default());
            for id in list.defaults.get(t).into_iter().flatten() {
                push(id, &mut out, &removed);
            }
        }
        for list in lists {
            for id in list.added.get(t).into_iter().flatten() {
                push(id, &mut out, &removed);
            }
        }
        for entry in index.entries().iter().filter(|e| e.mime_types.iter().any(|m| m == t || types.is_a(t, m))) {
            push(&entry.id, &mut out, &removed);
        }
    }
    out
}

/// How a file opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opening {
    /// In this app, by desktop id.
    App(String),
    /// Nothing opens it by itself: ask which app (Open With › Other…). Scripts, programs and
    /// AppImages are never run by opening them.
    Ask,
}

/// The file's type, by name and, when the name says nothing, by its content.
pub fn kind_of(path: &Path) -> String {
    if path.is_dir() {
        return mime::DIRECTORY.to_owned();
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    mime::types().by_name(&name).unwrap_or_else(|| mime::types().sniff(path))
}

/// What opening `path` does; reads the file and the lists, so not on a thread that draws.
pub fn opening(path: &Path, lists: &[List], index: &Index) -> Opening {
    let kind = kind_of(path);
    let types = mime::types();
    let runs = ["application/x-executable", "application/x-sharedlib", "application/vnd.appimage", "application/x-shellscript"];
    let default = apps_for(&kind, lists, index).into_iter().next();
    match default {
        Some(app) => Opening::App(app),
        None if runs.iter().any(|r| types.is_a(&kind, r)) => Opening::Ask,
        None if kind.starts_with("text/") || types.is_a(&kind, mime::PLAIN) => {
            apps_for(mime::PLAIN, lists, index).into_iter().next().map_or(Opening::Ask, Opening::App)
        }
        None => Opening::Ask,
    }
}

/// The scheme handler for a link (`x-scheme-handler/https`).
pub fn handler_for_url(url: &str, lists: &[List], index: &Index) -> Option<String> {
    let scheme = url.split_once(':')?.0;
    apps_for(&format!("x-scheme-handler/{scheme}"), lists, index).into_iter().next()
}

/// Makes `app` the default for `kinds` in the user's `mimeapps.list` (or the desktop's own
/// list with `isle_only`), keeping everything else in it.
pub fn set_default(kinds: &[&str], app: &str, isle_only: bool) -> std::io::Result<()> {
    let dir = config_home().ok_or_else(|| std::io::Error::other("no home"))?;
    let path = dir.join(if isle_only { "isle-mimeapps.list" } else { "mimeapps.list" });
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::create_dir_all(&dir)?;
    let next = with_default(&text, kinds, &format!("{app}.desktop"));
    let part = path.with_extension("list.part");
    std::fs::write(&part, next)?;
    std::fs::rename(part, path)
}

/// `text` with each of `kinds` set to `desktop_file` under `[Default Applications]`.
fn with_default(text: &str, kinds: &[&str], desktop_file: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let start = match lines.iter().position(|l| l.trim() == "[Default Applications]") {
        Some(i) => i,
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("[Default Applications]".into());
            lines.len() - 1
        }
    };
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim().starts_with('['))
        .map_or(lines.len(), |i| start + 1 + i);
    for kind in kinds {
        let line = format!("{kind}={desktop_file}");
        match lines[start + 1..end].iter().position(|l| l.split_once('=').is_some_and(|(k, _)| k.trim() == *kind)) {
            Some(i) => lines[start + 1 + i] = line,
            None => lines.insert(end, line),
        }
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_parse_their_three_groups() {
        let list = parse(
            "[Default Applications]\ntext/plain=org.gnome.TextEditor.desktop;\n[Added Associations]\ntext/plain=code.desktop;zed.desktop;\n[Removed Associations]\ntext/plain=vim.desktop\n",
        );
        assert_eq!(list.defaults["text/plain"], ["org.gnome.TextEditor"]);
        assert_eq!(list.added["text/plain"], ["code", "zed"]);
        assert_eq!(list.removed["text/plain"], ["vim"]);
    }

    #[test]
    fn a_default_is_set_and_the_rest_kept() {
        let text = "[Added Associations]\nimage/png=gimp.desktop;\n\n[Default Applications]\ntext/plain=old.desktop\nimage/png=eog.desktop\n";
        let out = with_default(text, &["text/plain", "text/markdown"], "zed.desktop");
        let list = parse(&out);
        assert_eq!(list.defaults["text/plain"], ["zed"]);
        assert_eq!(list.defaults["text/markdown"], ["zed"]);
        assert_eq!(list.defaults["image/png"], ["eog"]);
        assert_eq!(list.added["image/png"], ["gimp"]);
        assert_eq!(parse(&with_default("", &["inode/directory"], "files.desktop")).defaults["inode/directory"], ["files"]);
    }
}

#[cfg(test)]
mod opening_tests {
    use super::*;

    fn index_with(entries: &[(&str, &str)]) -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("applications");
        std::fs::create_dir_all(&apps).unwrap();
        for (id, mimes) in entries {
            std::fs::write(
                apps.join(format!("{id}.desktop")),
                format!("[Desktop Entry]\nType=Application\nName={id}\nExec={id} %F\nMimeType={mimes}\n"),
            )
            .unwrap();
        }
        let index = Index::scan(&[apps]);
        (dir, index)
    }

    #[test]
    fn a_file_opens_in_its_default_text_falls_back_and_programs_ask() {
        let (dir, index) = index_with(&[("viewer", "image/png;"), ("editor", "text/plain;"), ("painter", "image/png;")]);
        let lists = vec![parse("[Default Applications]\nimage/png=painter.desktop\n")];
        let file = |name: &str, body: &[u8]| {
            let p = dir.path().join(name);
            std::fs::write(&p, body).unwrap();
            p
        };
        assert_eq!(opening(&file("a.png", b"x"), &lists, &index), Opening::App("painter".into()));
        assert_eq!(apps_for("image/png", &lists, &index), ["painter", "viewer"]);
        // Markdown has no app of its own here: the text editor takes it.
        assert_eq!(opening(&file("notes.md", b"# x"), &lists, &index), Opening::App("editor".into()));
        // A script opens as the text it is, never run; a program with no app asks.
        assert_eq!(opening(&file("run", b"#!/bin/sh\necho\n"), &lists, &index), Opening::App("editor".into()));
        assert_eq!(opening(&file("tool.AppImage", b"\x7fELF\x02\x01"), &lists, &index), Opening::Ask);
    }
}
