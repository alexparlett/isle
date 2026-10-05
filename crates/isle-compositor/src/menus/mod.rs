//! Apps' real menus: where a window's menu is, its tree fetched on the worker and kept by
//! window, refetched when the app says it changed, and what choosing an item sends
//! (`p6-island-and-menus.md` choices 14–16 and 19).

pub mod protocols;
pub mod x11;

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use futures_util::StreamExt;
use isle_menus::{Activation, Shortcut, Source, Tree, registrar::Registrations};
use smithay::reexports::wayland_server::{
    Resource, backend::ObjectId, protocol::wl_surface::WlSurface,
};
use tracing::{debug, info, warn};

use crate::state::{Isle, State};

/// What Isle knows of a window's menus.
#[derive(Clone, Debug)]
pub enum Found {
    Looking,
    /// The app exports none: the fallback stands.
    None,
    Menus(Source, Tree),
}

#[derive(Default)]
pub struct MenuState {
    /// Isle's own connection for reading apps' menus and serving the registrar.
    pub conn: Option<zbus::Connection>,
    pub registrations: Option<Registrations>,
    /// Who owns the registrar's name when Isle could not take it.
    pub taken: Option<String>,
    /// Menu addresses toplevels declared over Wayland, by their surface.
    pub addresses: HashMap<ObjectId, Source>,
    /// Each window's menus, by window id.
    pub trees: HashMap<u64, Found>,
    /// Sources whose changes are watched.
    watching: HashSet<Source>,
}

/// A shortcut as a stroke's words, for the tables' inverse: `[["Control","c"]]` or Qt's
/// `["Ctrl+C"]`, or GTK's `<Primary>c`.
pub fn stroke_words(shortcut: &Shortcut) -> Option<String> {
    let name = |n: &str| -> String {
        match n.to_ascii_lowercase().as_str() {
            "control" | "ctrl" | "primary" => "Ctrl".into(),
            "alt" | "mod1" => "Alt".into(),
            "shift" => "Shift".into(),
            "super" | "meta" | "logo" => "Super".into(),
            "del" => "Delete".into(),
            "ins" => "Insert".into(),
            "pgup" => "Prior".into(),
            "pgdown" => "Next".into(),
            "esc" => "Escape".into(),
            "plus" => "plus".into(),
            "minus" => "minus".into(),
            _ => n.to_owned(),
        }
    };
    match shortcut {
        Shortcut::Keys(chords) => {
            let first = chords.first()?;
            let parts: Vec<String> = first
                .iter()
                .flat_map(|p| {
                    p.split('+')
                        .filter(|s| !s.is_empty())
                        .map(name)
                        .collect::<Vec<_>>()
                })
                .collect();
            (!parts.is_empty()).then(|| parts.join("+"))
        }
        Shortcut::Accel(accel) => {
            let mut parts = Vec::new();
            let mut rest = accel.as_str();
            while let Some(stripped) = rest.strip_prefix('<') {
                let end = stripped.find('>')?;
                parts.push(name(&stripped[..end]));
                rest = &stripped[end + 1..];
            }
            if rest.is_empty() {
                return None;
            }
            parts.push(name(rest));
            Some(parts.join("+"))
        }
    }
}

impl Isle {
    /// The menu address a toplevel declared, by its surface.
    fn declared(&self, id: u64) -> Option<Source> {
        let record = self.wm.get(id)?;
        let surface = smithay::wayland::seat::WaylandFocus::wl_surface(&record.window)?;
        self.menus.addresses.get(&surface.id()).cloned()
    }
}

impl State {
    /// A toplevel told Isle where its menus are.
    pub fn set_menu_address(&mut self, surface: &WlSurface, source: Source) {
        debug!(?source, "menu address");
        self.isle.menus.addresses.insert(surface.id(), source);
        if let Some(id) = self.isle.wm.for_surface(surface).map(|r| r.id) {
            self.isle.menus.trees.remove(&id);
            self.find_menus(id);
        }
    }

    /// Looks for window `id`'s menus once: its own address, then for X11 the registrar's
    /// entry and its properties, then a registration from its process; else none.
    pub fn find_menus(&mut self, id: u64) {
        if self.isle.menus.trees.contains_key(&id) {
            return;
        }
        let Some(record) = self.isle.wm.get(id) else {
            return;
        };
        let (Some(conn), Some(runtime), Some(to_loop)) = (
            self.isle.menus.conn.clone(),
            self.isle.worker.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        self.isle.menus.trees.insert(id, Found::Looking);
        let declared = self.isle.declared(id);
        let xid = record.window.x11_surface().map(|x| x.window_id());
        let display = self.isle.xdisplay;
        let pid = record.pid;
        let registrations = self.isle.menus.registrations.clone();
        runtime.spawn(async move {
            let mut source = declared;
            if source.is_none()
                && let Some(xid) = xid
            {
                source = registrations
                    .as_ref()
                    .and_then(|r| r.for_window(xid))
                    .map(|r| Source::DBusMenu {
                        service: r.sender,
                        path: r.path,
                    });
                if source.is_none()
                    && let Some(display) = display
                {
                    source = tokio::task::spawn_blocking(move || x11::menu_of(display, xid))
                        .await
                        .ok()
                        .flatten();
                }
            }
            // A registration from the window's own process, for a window id nothing matches
            // (Electron on Wayland).
            if source.is_none()
                && let (Some(pid), Some(registrations)) = (pid, registrations.as_ref())
            {
                for r in registrations.all() {
                    if isle_menus::registrar::pid_of(&conn, &r.sender).await == Some(pid as u32) {
                        source = Some(Source::DBusMenu {
                            service: r.sender,
                            path: r.path,
                        });
                        break;
                    }
                }
            }
            let found = match &source {
                Some(source) => match fetch(&conn, source).await {
                    Ok(tree) if !tree.menus.is_empty() => Found::Menus(source.clone(), tree),
                    Ok(_) => Found::None,
                    Err(err) => {
                        warn!(?source, "the app's menus could not be read: {err}");
                        Found::None
                    }
                },
                None => Found::None,
            };
            to_loop.post(move |state| state.menus_found(id, found));
        });
    }

    fn menus_found(&mut self, id: u64, found: Found) {
        if let Found::Menus(source, tree) = &found {
            info!(id, menus = tree.menus.len(), "the app's menus");
            if self.isle.menus.watching.insert(source.clone()) {
                self.watch_menus(source.clone());
            }
        }
        self.isle.menus.trees.insert(id, found);
        if self.focused_window() == Some(id) {
            self.refresh_bar();
        }
    }

    /// Refetches `source`'s tree whenever its app says it changed, a burst at a time.
    fn watch_menus(&mut self, source: Source) {
        let (Some(conn), Some(runtime), Some(to_loop)) = (
            self.isle.menus.conn.clone(),
            self.isle.worker.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        runtime.spawn(async move {
            let changes = match &source {
                Source::DBusMenu { service, path } => {
                    isle_menus::dbusmenu::changes(&conn, service, path)
                        .await
                        .map(|s| s.boxed())
                }
                Source::GMenu { .. } => isle_menus::gmenu::changes(&conn, &source)
                    .await
                    .map(|s| s.boxed()),
            };
            let Ok(mut changes) = changes else {
                return;
            };
            while changes.next().await.is_some() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                while futures_util::FutureExt::now_or_never(changes.next())
                    .flatten()
                    .is_some()
                {}
                let Ok(tree) = fetch(&conn, &source).await else {
                    continue;
                };
                let source = source.clone();
                to_loop.post(move |state| {
                    let ids: Vec<u64> = state
                        .isle
                        .menus
                        .trees
                        .iter()
                        .filter(|(_, f)| matches!(f, Found::Menus(s, _) if *s == source))
                        .map(|(id, _)| *id)
                        .collect();
                    for id in ids {
                        state
                            .isle
                            .menus
                            .trees
                            .insert(id, Found::Menus(source.clone(), tree.clone()));
                    }
                    state.refresh_bar();
                });
            }
        });
    }

    /// Chooses the item at `path` of window `id`'s menus, found again by its labels.
    pub fn activate_remote(&mut self, id: u64, path: &[String]) {
        let Some(Found::Menus(source, tree)) = self.isle.menus.trees.get(&id) else {
            return;
        };
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        let Some(item) = tree.find(&path) else {
            warn!(?path, "the item is gone from the app's menus");
            return;
        };
        let (Some(conn), Some(runtime)) = (self.isle.menus.conn.clone(), self.isle.worker.clone())
        else {
            return;
        };
        let source = source.clone();
        let activation = item.activation.clone();
        runtime.spawn(async move {
            let result = match (&source, &activation) {
                (Source::DBusMenu { service, path }, Activation::DBusMenu { id }) => {
                    isle_menus::dbusmenu::activate(&conn, service, path, *id).await
                }
                (Source::GMenu { service, .. }, Activation::GAction { path, name, target }) => {
                    isle_menus::gmenu::activate(&conn, service, path, name, target.as_ref()).await
                }
                _ => Ok(()),
            };
            if let Err(err) = result {
                warn!("the app did not take the item: {err}");
            }
        });
    }

    /// A submenu of window `id`'s menus is opening: the app is asked to fill it.
    pub fn fill_remote(&mut self, id: u64, path: Vec<String>) {
        let Some(Found::Menus(source, tree)) = self.isle.menus.trees.get(&id) else {
            return;
        };
        let refs: Vec<&str> = path.iter().map(String::as_str).collect();
        let Some(fill) = tree.find(&refs).and_then(|i| i.fill.clone()) else {
            return;
        };
        let (Some(conn), Some(runtime), Some(to_loop)) = (
            self.isle.menus.conn.clone(),
            self.isle.worker.clone(),
            self.isle.to_loop.clone(),
        ) else {
            return;
        };
        let source = source.clone();
        runtime.spawn(async move {
            let filled = match (&source, &fill) {
                (
                    Source::DBusMenu {
                        service,
                        path: object,
                    },
                    Activation::DBusMenu { id: item },
                ) => isle_menus::dbusmenu::fetch_submenu(&conn, service, object, *item)
                    .await
                    .ok(),
                (
                    Source::GMenu { service, .. },
                    Activation::GAction {
                        path: group, name, ..
                    },
                ) => {
                    let _ = isle_menus::gmenu::set_submenu(&conn, service, group, name, true).await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let tree = fetch(&conn, &source).await.ok();
                    let refs: Vec<&str> = path.iter().map(String::as_str).collect();
                    tree.and_then(|t| t.find(&refs).and_then(|i| i.children.clone()))
                }
                _ => None,
            };
            let Some(children) = filled else {
                return;
            };
            to_loop.post(move |state| {
                if let Some(Found::Menus(_, tree)) = state.isle.menus.trees.get_mut(&id) {
                    let refs: Vec<&str> = path.iter().map(String::as_str).collect();
                    if let Some(item) = find_mut(tree, &refs) {
                        item.children = Some(children);
                        item.fill = None;
                    }
                }
                state.refresh_bar_keeping_open();
            });
        });
    }

    /// Window `id`'s menus as JSON, with where they came from.
    pub fn menus_json(&mut self, id: u64) -> String {
        let found = self.isle.menus.trees.get(&id).cloned();
        let (kind, tree) = match found {
            Some(Found::Menus(Source::DBusMenu { .. }, tree)) => ("dbusmenu", Some(tree)),
            Some(Found::Menus(Source::GMenu { .. }, tree)) => ("gmenu", Some(tree)),
            Some(Found::Looking) => ("looking", None),
            Some(Found::None) => ("none", None),
            None => {
                self.find_menus(id);
                ("looking", None)
            }
        };
        let paths: Vec<String> = tree
            .as_ref()
            .map(|t| t.walk().into_iter().map(|(p, _)| p.join(" › ")).collect())
            .unwrap_or_default();
        serde_json::json!({
            "source": kind,
            "menus": tree.map(|t| t.menus.iter().map(|m| m.label.clone()).collect::<Vec<_>>()),
            "items": paths,
            "registrar": self.isle.menus.taken.as_ref().map_or("owned".to_string(), |o| format!("taken by {o}")),
        })
        .to_string()
    }
}

fn find_mut<'a>(tree: &'a mut Tree, path: &[&str]) -> Option<&'a mut isle_menus::Item> {
    let (first, rest) = path.split_first()?;
    let mut item = tree.menus.iter_mut().find(|m| m.label == *first)?;
    for label in rest {
        item = item.children.as_mut()?.iter_mut().find_map(|n| match n {
            isle_menus::Node::Item(i) if i.label == *label => Some(i),
            _ => None,
        })?;
    }
    Some(item)
}

async fn fetch(conn: &zbus::Connection, source: &Source) -> zbus::Result<Tree> {
    match source {
        Source::DBusMenu { service, path } => {
            isle_menus::dbusmenu::fetch(conn, service, path).await
        }
        Source::GMenu { .. } => isle_menus::gmenu::fetch(conn, source).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_from_each_toolkit_become_words() {
        let keys = |v: &[&str]| Shortcut::Keys(vec![v.iter().map(|s| s.to_string()).collect()]);
        assert_eq!(
            stroke_words(&keys(&["Control", "c"])).as_deref(),
            Some("Ctrl+c")
        );
        assert_eq!(
            stroke_words(&keys(&["Control", "Shift", "T"])).as_deref(),
            Some("Ctrl+Shift+T")
        );
        assert_eq!(
            stroke_words(&keys(&["Ctrl+Del"])).as_deref(),
            Some("Ctrl+Delete")
        );
        assert_eq!(
            stroke_words(&Shortcut::Accel("<Primary><Shift>n".into())).as_deref(),
            Some("Ctrl+Shift+n")
        );
        assert_eq!(
            stroke_words(&Shortcut::Accel("F5".into())).as_deref(),
            Some("F5")
        );
    }
}
