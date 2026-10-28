//! What Files remembers between launches, in its `isle-state` domain (`conventions.md`,
//! What an app remembers): the windows and their tabs, views and saved searches.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{places::Place, view::Shown};

/// Folders whose view is kept; the longest unvisited are forgotten first.
const VIEWS: usize = 2000;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memory {
    /// Each open window's tabs, by the window's number.
    pub windows: BTreeMap<u64, WindowMemory>,
    /// Each folder's view as last set, the most recently set last.
    #[serde(default)]
    pub views: Vec<(Place, Shown)>,
    /// The view of a folder never set (View Options' Use as Defaults).
    #[serde(default)]
    pub default_view: Shown,
    /// Saved searches, as Favourites lists them.
    #[serde(default)]
    pub saved: Vec<crate::search::Search>,
    /// Get Info's sections closed.
    #[serde(default)]
    pub info_collapsed: Vec<String>,
    /// The Open and Save panels, by the app asking.
    #[serde(default)]
    pub panels: BTreeMap<String, PanelMemory>,
    /// Hidden files shown (View › Show Hidden Files).
    #[serde(default)]
    pub hidden: bool,
    /// The tab bar shown with one tab (View › Show Tab Bar).
    #[serde(default)]
    pub tab_bar: bool,
    /// Folders gone to, the latest first (Go › Recent Folders).
    #[serde(default)]
    pub recent_folders: Vec<std::path::PathBuf>,
    /// The sidebar's groups collapsed, by heading.
    #[serde(default)]
    pub collapsed: Vec<String>,
    /// The preview pane's width as dragged; none for its own.
    #[serde(default)]
    pub preview_width: Option<f32>,
    /// Favourites in the user's order, by folder (D76).
    #[serde(default)]
    pub favourites: Vec<std::path::PathBuf>,
}

/// What a panel keeps for one app.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PanelMemory {
    pub folder: Option<std::path::PathBuf>,
    pub view: Option<crate::view::View>,
    /// Save shows the browser.
    pub expanded: bool,
    /// The browser's size as last left.
    #[serde(default)]
    pub size: Option<(f64, f64)>,
}

impl Memory {
    pub fn shown(&self, place: &Place) -> Shown {
        let found = self.views.iter().rev().find(|(p, _)| p == place).map(|(_, s)| *s);
        found.unwrap_or_else(|| match place {
            // Gatherings show where each item is, so List by default; Recents newest first by day.
            Place::Recents => Shown {
                view: crate::view::View::List,
                sort: isle_fs::SortBy::Modified,
                reverse: false,
                group: Some(crate::view::GroupBy::Modified),
                ..self.default_view
            },
            Place::Trash | Place::Search(_) => Shown { view: crate::view::View::List, ..self.default_view },
            _ => self.default_view,
        })
    }

    pub fn keep(&mut self, place: &Place, shown: Shown) {
        if matches!(place, Place::Search(s) if s.name.is_none()) {
            return;
        }
        self.views.retain(|(p, _)| p != place);
        self.views.push((place.clone(), shown));
        if self.views.len() > VIEWS {
            let extra = self.views.len() - VIEWS;
            self.views.drain(..extra);
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowMemory {
    pub tabs: Vec<Place>,
    pub current: usize,
}

impl isle_state::Domain for Memory {
    const NAME: &'static str = "files";
    const VERSION: u32 = 1;
}

fn dir() -> isle_state::StateDir {
    isle_state::StateDir::new(isle_state::StateDir::default_dir())
}

pub fn load() -> Memory {
    dir().load::<Memory>().0
}

/// Saves from any thread, one save at a time, so two never write the same temporary file.
pub fn save(memory: &Memory) {
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(err) = dir().save(memory) {
        tracing::warn!("Files' windows are not kept: {err}");
    }
}
