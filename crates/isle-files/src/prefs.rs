//! Files' settings: the record its settings window and Spotlight's index are made from, and the
//! values in force (`p1/structure/files.md` § Files' settings; B97).

use std::sync::atomic::{AtomicBool, Ordering};

use isle_app::Desktop;
use isle_fs::Entry;
use isle_settings::Value;

use crate::search::Scope;

pub const APP: &str = "io.github.alexparlett.Isle.Files";

/// A setting's key in Files' own section.
pub fn key(name: &str) -> String {
    format!("app[{APP}].{name}")
}

/// What a row edits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    /// On or off, with its default.
    Switch(bool),
    /// A pop-up of the choice's values, with their labels.
    Choice(&'static [(&'static str, &'static str)]),
}

pub struct RowDef {
    pub id: &'static str,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
    /// The key under Files' section.
    pub key: &'static str,
    pub control: Control,
}

pub struct PaneDef {
    pub id: &'static str,
    pub name: &'static str,
    pub glyph: &'static str,
    pub keywords: &'static [&'static str],
    pub rows: &'static [RowDef],
}

const fn switch(id: &'static str, label: &'static str, key: &'static str, keywords: &'static [&'static str]) -> RowDef {
    RowDef { id, label, keywords, key, control: Control::Switch(true) }
}

pub const NEW_WINDOW: &[(&str, &str)] = &[
    ("home", "Home"),
    ("desktop", "Desktop"),
    ("documents", "Documents"),
    ("downloads", "Downloads"),
    ("recents", "Recents"),
    ("computer", "Computer"),
];

pub const SEARCH_SCOPE: &[(&str, &str)] = &[
    ("this-folder", "This folder"),
    ("under", "Everything under it"),
    ("home", "Home"),
    ("previous", "Use the previous scope"),
];

pub static PANES: &[PaneDef] = &[
    PaneDef {
        id: "general",
        name: "General",
        glyph: "app-window",
        keywords: &["files preferences"],
        rows: &[
            RowDef {
                id: "new-window",
                label: "New windows open",
                keywords: &["start folder", "default folder"],
                key: "new-window",
                control: Control::Choice(NEW_WINDOW),
            },
            switch("open-folders-in-tabs", "Open folders in tabs instead of new windows", "open-folders-in-tabs", &["tabs"]),
        ],
    },
    PaneDef {
        id: "sidebar",
        name: "Sidebar",
        glyph: "sidebar-simple",
        keywords: &["favourites", "locations"],
        rows: &[
            switch("recents", "Recents", "sidebar.recents", &[]),
            switch("home", "Home", "sidebar.home", &[]),
            switch("applications", "Applications", "sidebar.applications", &[]),
            switch("desktop", "Desktop", "sidebar.desktop", &[]),
            switch("documents", "Documents", "sidebar.documents", &[]),
            switch("downloads", "Downloads", "sidebar.downloads", &[]),
            switch("music", "Music", "sidebar.music", &[]),
            switch("pictures", "Pictures", "sidebar.pictures", &[]),
            switch("videos", "Videos", "sidebar.videos", &[]),
            switch("computer", "Computer", "sidebar.computer", &[]),
            switch("volumes", "External disks and shares", "sidebar.volumes", &["drives", "usb", "network"]),
            switch("trash", "Trash", "sidebar.trash", &[]),
        ],
    },
    PaneDef {
        id: "advanced",
        name: "Advanced",
        glyph: "gear-six",
        keywords: &[],
        rows: &[
            switch("show-extensions", "Show all filename extensions", "show-extensions", &["file type", "suffix"]),
            switch("warn-extension-change", "Show warning before changing an extension", "warn-extension-change", &[]),
            switch("warn-empty-trash", "Show warning before emptying the Trash", "warn-empty-trash", &[]),
            RowDef {
                id: "purge-trash",
                label: "Remove items from the Trash after 30 days",
                keywords: &["delete", "clean"],
                key: "purge-trash",
                control: Control::Switch(false),
            },
            switch("folders-on-top", "Keep folders on top", "folders-on-top", &["sort"]),
            RowDef {
                id: "search-scope",
                label: "When performing a search",
                keywords: &["find", "scope"],
                key: "search-scope",
                control: Control::Choice(SEARCH_SCOPE),
            },
        ],
    },
];

/// The rows Spotlight indexes as "Files Settings › …"; `isle-files --write-index` writes it.
#[cfg(any(test, debug_assertions))]
pub fn index() -> String {
    let quote = isle_settings::quote;
    let mut out = String::from("// Written by `isle-files --write-index` from its settings record; a test keeps it current.\n");
    for pane in PANES {
        let mut write = |id: &str, label: &str, also: &[&str]| {
            out.push_str(&format!(
                "row app={} app-name=\"Files\" pane={} id={} label={} pane-label={}",
                quote(APP),
                quote(pane.id),
                quote(id),
                quote(label),
                quote(pane.name)
            ));
            if !also.is_empty() {
                out.push_str(&format!(" also={}", quote(&also.join(";"))));
            }
            out.push('\n');
        };
        write("", pane.name, pane.keywords);
        for row in pane.rows {
            write(row.id, row.label, row.keywords);
        }
    }
    out
}

/// The values in force, each its default until the compositor has said.
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    pub new_window: String,
    pub open_folders_in_tabs: bool,
    /// The sidebar rows turned off, by id.
    pub hidden_rows: Vec<String>,
    pub show_extensions: bool,
    pub warn_extension_change: bool,
    pub warn_empty_trash: bool,
    pub purge_trash: bool,
    pub folders_on_top: bool,
    pub search_scope: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            new_window: "home".into(),
            open_folders_in_tabs: true,
            hidden_rows: Vec::new(),
            show_extensions: true,
            warn_extension_change: true,
            warn_empty_trash: true,
            purge_trash: false,
            folders_on_top: true,
            search_scope: "under".into(),
        }
    }
}

impl Prefs {
    pub fn read(desktop: &Desktop) -> Self {
        let d = Self::default();
        let flag = |name: &str, default: bool| match desktop.value(&key(name)) {
            Some(Value::Bool(b)) => b,
            _ => default,
        };
        let word = |name: &str, default: String| match desktop.value(&key(name)) {
            Some(Value::Str(s)) => s,
            _ => default,
        };
        let sidebar = PANES.iter().find(|p| p.id == "sidebar").map_or(&[][..], |p| p.rows);
        Self {
            new_window: word("new-window", d.new_window),
            open_folders_in_tabs: flag("open-folders-in-tabs", d.open_folders_in_tabs),
            hidden_rows: sidebar.iter().filter(|r| !flag(r.key, true)).map(|r| r.id.to_owned()).collect(),
            show_extensions: flag("show-extensions", d.show_extensions),
            warn_extension_change: flag("warn-extension-change", d.warn_extension_change),
            warn_empty_trash: flag("warn-empty-trash", d.warn_empty_trash),
            purge_trash: flag("purge-trash", d.purge_trash),
            folders_on_top: flag("folders-on-top", d.folders_on_top),
            search_scope: word("search-scope", d.search_scope),
        }
    }

    /// Puts the values every view reads in force; whether either changed.
    pub fn apply(&self) -> bool {
        let a = SHOW_EXTENSIONS.swap(self.show_extensions, Ordering::Relaxed) != self.show_extensions;
        let b = FOLDERS_ON_TOP.swap(self.folders_on_top, Ordering::Relaxed) != self.folders_on_top;
        a || b
    }

    pub fn shows_row(&self, id: &str) -> bool {
        !self.hidden_rows.iter().any(|r| r == id)
    }

    /// The scope Return searches in, given the last one used.
    pub fn scope(&self, previous: Option<Scope>) -> Scope {
        match self.search_scope.as_str() {
            "home" => Scope::Home,
            "previous" => previous.unwrap_or(Scope::Under),
            _ => Scope::Under,
        }
    }
}

static SHOW_EXTENSIONS: AtomicBool = AtomicBool::new(true);
static FOLDERS_ON_TOP: AtomicBool = AtomicBool::new(true);

pub fn folders_on_top() -> bool {
    FOLDERS_ON_TOP.load(Ordering::Relaxed)
}

/// An item's name as the views show it.
pub fn display_name(entry: &Entry) -> &str {
    shown_name(&entry.name, entry.is_dir(), SHOW_EXTENSIONS.load(Ordering::Relaxed))
}

/// A name as shown: without its extension while extensions are hidden, a folder's in full.
pub fn shown_name<'a>(name: &'a str, dir: bool, prefs_show: bool) -> &'a str {
    if prefs_show || dir || name.starts_with('.') {
        return name;
    }
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_installed_index_is_the_record() {
        let installed = include_str!("../../../data/files/settings-index.kdl");
        assert_eq!(installed, index(), "run `isle-files --write-index`");
    }

    #[test]
    fn every_key_is_in_the_schema() {
        let keys: Vec<String> = isle_settings::schema_of::<isle_settings::schema::Settings>().into_iter().map(|k| k.key).collect();
        for pane in PANES {
            for row in pane.rows {
                assert!(keys.contains(&key(row.key)), "{} is not in the schema", key(row.key));
            }
        }
    }

    #[test]
    fn hidden_extensions_leave_folders_and_dot_names_whole() {
        assert_eq!(shown_name("notes.md", false, false), "notes");
        assert_eq!(shown_name("notes.md", false, true), "notes.md");
        assert_eq!(shown_name("src.d", true, false), "src.d");
        assert_eq!(shown_name(".bashrc", false, false), ".bashrc");
    }
}
