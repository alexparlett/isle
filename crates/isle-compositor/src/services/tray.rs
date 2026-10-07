//! The tray: other programs' status items in Control Centre, their presses and their menus
//! as pages (`p8-services.md` choice 17).

use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::Duration,
};

use isle_bus::{Publisher, Snapshot};
use isle_menus::{Activation, Node, Toggle};
use isle_shell::island::{
    panel::{Module, PanelAction, TrayIcon, TrayPress},
    rows::{Choice, PageRow},
};
use isle_tray::{Address, Icon, Item, Tray};
use tokio::sync::Notify;
use tracing::{debug, warn};

use crate::{
    state::{Isle, State},
    worker::ToLoop,
};

/// The open item's menu: the item's key, and its tree once read.
pub struct TrayMenu {
    pub key: String,
    pub tree: Option<isle_menus::Tree>,
    pub error: Option<String>,
}

type First = Arc<Mutex<Option<(zbus::Connection, isle_tray::Host)>>>;

/// Serves the watcher, from `first` if it was owned before the socket, and owns it again
/// whenever it is lost; `kill` drops it as a crash would.
async fn serve(
    first: First,
    publisher: Publisher<Tray>,
    kill: Arc<Notify>,
) -> Result<(), isle_bus::Error> {
    loop {
        let taken = first.lock().unwrap().take();
        let (conn, host) = match taken {
            Some(owned) => owned,
            None => {
                let conn = zbus::Connection::session().await?;
                match isle_tray::own(&conn).await? {
                    Ok(host) => (conn, host),
                    Err(isle_tray::Taken) => {
                        publisher.unavailable();
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
        };
        tokio::select! {
            result = host.follow(&conn, publisher.clone()) => {
                if let Err(err) = result {
                    warn!("the tray stopped: {err}");
                }
            }
            _ = kill.notified() => debug!("the tray's watcher dropped"),
        }
        drop(host);
        let _ = conn.close().await;
        publisher.publish(Tray::default());
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Starts the tray on the worker; returns what drops its watcher.
pub fn start(
    runtime: &tokio::runtime::Handle,
    to_loop: &ToLoop,
    first: Option<(zbus::Connection, isle_tray::Host)>,
) -> Arc<Notify> {
    let kill = Arc::new(Notify::new());
    let first: First = Arc::new(Mutex::new(first));
    let (publisher, snapshots) = isle_bus::snapshot::<Tray>();
    let k = kill.clone();
    runtime.spawn(crate::worker::supervise("tray", move || {
        serve(first.clone(), publisher.clone(), k.clone())
    }));
    let to_loop = to_loop.clone();
    runtime.spawn(crate::worker::supervise("tray", move || {
        super::forward(
            "tray",
            snapshots.clone(),
            to_loop.clone(),
            State::tray_changed,
        )
    }));
    kill
}

/// An item's key in page paths and actions.
pub fn key(address: &Address) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    address.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// The DBusMenu ids along a page path below `tray/<key>`.
fn page_ids(page: &str) -> Option<(&str, Vec<i32>)> {
    let mut parts = page.strip_prefix("tray/")?.split('/');
    let key = parts.next()?;
    Some((key, parts.filter_map(|p| p.parse().ok()).collect()))
}

fn dbus_id(item: &isle_menus::Item) -> Option<i32> {
    match item.activation {
        Activation::DBusMenu { id } => Some(id),
        _ => match item.fill {
            Some(Activation::DBusMenu { id }) => Some(id),
            _ => None,
        },
    }
}

/// The nodes under the items `ids` lead to; the menu's top for none.
fn nodes_at(tree: &isle_menus::Tree, ids: &[i32]) -> Option<Vec<Node>> {
    let mut nodes: Vec<Node> = tree.menus.iter().cloned().map(Node::Item).collect();
    for id in ids {
        let item = nodes.iter().find_map(|n| match n {
            Node::Item(item) if dbus_id(item) == Some(*id) => Some(item),
            _ => None,
        })?;
        nodes = item.children.clone()?;
    }
    Some(nodes)
}

fn item_at_mut<'a>(
    nodes: &'a mut [isle_menus::Item],
    ids: &[i32],
) -> Option<&'a mut isle_menus::Item> {
    let (first, rest) = ids.split_first()?;
    let item = nodes.iter_mut().find(|i| dbus_id(i) == Some(*first))?;
    if rest.is_empty() {
        return Some(item);
    }
    let (next, _) = rest.split_first()?;
    let children = item.children.as_mut()?;
    let child = children.iter_mut().find_map(|n| match n {
        Node::Item(i) if dbus_id(i) == Some(*next) => Some(i),
        _ => None,
    })?;
    item_at_mut(std::slice::from_mut(child), rest)
}

fn menu_rows(nodes: &[Node]) -> Vec<PageRow> {
    nodes
        .iter()
        .filter_map(|node| match node {
            Node::Item(item) => Some(PageRow::Choice(Choice {
                id: dbus_id(item).map(|i| i.to_string()).unwrap_or_default(),
                label: item.label.clone(),
                checked: matches!(item.toggle, Toggle::Check(Some(true)) | Toggle::Radio(true)),
                sub_page: item.children.is_some() || item.fill.is_some(),
                enabled: item.enabled,
                ..Default::default()
            })),
            Node::Section(label) if !label.is_empty() => Some(PageRow::Heading(label.clone())),
            _ => None,
        })
        .collect()
}

impl Isle {
    /// The items in the user's order: those named by `control-centre.tray`, then the rest
    /// as they came.
    pub fn tray_items(&self) -> Vec<&Item> {
        if !self.services.tray.available {
            return Vec::new();
        }
        let order = &self.settings.control_centre.tray;
        let mut items: Vec<&Item> = self.services.tray.state.items.iter().collect();
        items.sort_by_key(|item| {
            order
                .iter()
                .position(|id| *id == item.id)
                .unwrap_or(usize::MAX)
        });
        items
    }

    fn tray_item(&self, key: &str) -> Option<&Item> {
        self.services
            .tray
            .state
            .items
            .iter()
            .find(|i| self::key(&i.address) == key)
    }

    fn tray_icon_path(&self, icon: &Icon) -> Option<std::path::PathBuf> {
        match icon {
            Icon::File(path) => Some(path.clone()),
            Icon::Name(name, theme) => theme
                .as_deref()
                .and_then(|t| isle_tray::theme_icon(name, t))
                .or_else(|| {
                    let dark = self.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
                    let mut icons = self.services.tray_icons.borrow_mut();
                    if icons.as_ref().is_none_or(|(d, _)| *d != dark) {
                        *icons = Some((dark, isle_apps::icons::Icons::status(dark)));
                    }
                    icons.as_mut().unwrap().1.lookup(name, 24)
                }),
        }
    }

    pub fn tray_icons(&self) -> Vec<TrayIcon> {
        self.tray_items()
            .into_iter()
            .filter(|item| !item.passive)
            .map(|item| TrayIcon {
                key: key(&item.address),
                title: [&item.title, &item.tooltip, &item.id]
                    .into_iter()
                    .find(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_default(),
                icon: item.icon.as_ref().and_then(|i| self.tray_icon_path(i)),
                attention: item.attention,
            })
            .collect()
    }

    /// The page of the open item's menu at `page`.
    pub fn tray_menu_module(&self, page: &str) -> Option<Module> {
        let (key, ids) = page_ids(page)?;
        let item = self.tray_item(key)?;
        let menu = self.services.tray_menu.as_ref().filter(|m| m.key == key);
        let rows = match menu.and_then(|m| m.tree.as_ref()) {
            Some(tree) => match nodes_at(tree, &ids) {
                Some(nodes) if !nodes.is_empty() => menu_rows(&nodes),
                Some(_) => vec![PageRow::Status("Nothing in this menu".into())],
                None => vec![PageRow::Status(String::new())],
            },
            None => match menu.and_then(|m| m.error.clone()) {
                Some(err) => vec![PageRow::Status(err)],
                None => vec![PageRow::Status(String::new())],
            },
        };
        let title = |tree: &isle_menus::Tree| -> Option<String> {
            let (last, parents) = ids.split_last()?;
            nodes_at(tree, parents)?.into_iter().find_map(|n| match n {
                Node::Item(i) if dbus_id(&i) == Some(*last) => Some(i.label),
                _ => None,
            })
        };
        let name = [&item.title, &item.tooltip, &item.id]
            .into_iter()
            .find(|s| !s.is_empty())
            .cloned()
            .unwrap_or_default();
        Some(Module {
            id: "tray".into(),
            name,
            glyph: "app-window".into(),
            icon: item.icon.as_ref().and_then(|i| self.tray_icon_path(i)),
            page: Some(rows),
            page_title: menu.and_then(|m| m.tree.as_ref()).and_then(title),
            plain_head: true,
            ..Default::default()
        })
    }
}

impl State {
    pub fn tray_changed(&mut self, snapshot: Snapshot<Tray>) {
        self.isle.services.tray = snapshot;
        // An item that left takes its open menu with it.
        if let Some(panel) = &self.isle.island.panel
            && let Some((key, _)) = panel.page.as_deref().and_then(page_ids)
            && self.isle.tray_item(key).is_none()
        {
            let panel = panel.clone();
            self.isle.island.panel = Some(crate::control_centre::PanelState {
                page: None,
                ..panel
            });
            self.isle.services.tray_menu = None;
        }
        self.isle.refresh_island();
    }

    /// Reads the open item's menu, and a submenu the app fills only when asked.
    pub fn follow_tray_menu(&mut self) {
        let page = self.isle.island.panel.as_ref().and_then(|p| p.page.clone());
        let Some((key, ids)) = page.as_deref().and_then(page_ids) else {
            self.isle.services.tray_menu = None;
            return;
        };
        let Some(item) = self.isle.tray_item(key).cloned() else {
            return;
        };
        let Some(menu_path) = item.menu.clone() else {
            return;
        };
        let service = item.address.service.clone();
        let key = key.to_owned();
        match &self.isle.services.tray_menu {
            Some(menu) if menu.key == key => {
                let Some(tree) = &menu.tree else {
                    return;
                };
                if ids.is_empty() {
                    return;
                }
                let mut menus = tree.menus.clone();
                let Some(item) = item_at_mut(&mut menus, &ids) else {
                    return;
                };
                // A submenu the app fills only once asked, and not asked yet.
                let filled = item.children.as_ref().is_some_and(|c| !c.is_empty());
                if item.fill.is_none() || filled {
                    return;
                }
                let Some(id) = dbus_id(item) else {
                    return;
                };
                self.isle.run_service(
                    async move {
                        let conn = isle_bus::session().await?;
                        Ok(
                            isle_menus::dbusmenu::fetch_submenu(&conn, &service, &menu_path, id)
                                .await?,
                        )
                    },
                    move |state, result| {
                        let Some(menu) = state
                            .isle
                            .services
                            .tray_menu
                            .as_mut()
                            .filter(|m| m.key == key)
                        else {
                            return;
                        };
                        if let (Ok(children), Some(tree)) = (result, menu.tree.as_mut())
                            && let Some(item) = item_at_mut(&mut tree.menus, &ids)
                        {
                            item.children = Some(children);
                            item.fill = None;
                        }
                        state.isle.refresh_island();
                    },
                );
            }
            _ => {
                self.isle.services.tray_menu = Some(TrayMenu {
                    key: key.clone(),
                    tree: None,
                    error: None,
                });
                self.isle.run_service(
                    async move {
                        let conn = isle_bus::session().await?;
                        Ok(isle_menus::dbusmenu::fetch(&conn, &service, &menu_path).await?)
                    },
                    move |state, result| {
                        let Some(menu) = state
                            .isle
                            .services
                            .tray_menu
                            .as_mut()
                            .filter(|m| m.key == key)
                        else {
                            return;
                        };
                        match result {
                            Ok(tree) => menu.tree = Some(tree),
                            Err(err) => {
                                debug!("a tray menu: {err}");
                                menu.error = Some("The menu could not be read".into());
                            }
                        }
                        state.follow_tray_menu();
                        state.isle.refresh_island();
                    },
                );
            }
        }
    }

    pub fn tray_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::Tray { key, press } => self.tray_press(&key, press),
            PanelAction::TrayMove { key, to } => {
                let cmd = self.isle.seat.get_keyboard().unwrap().modifier_state().logo;
                if cmd {
                    self.tray_reorder(&key, to);
                }
            }
            PanelAction::Choose { row, .. } => self.tray_choose(&row),
            _ => {}
        }
    }

    fn open_tray_menu(&mut self, key: &str) {
        if let Some(panel) = self.isle.island.panel.clone() {
            self.isle.island.panel = Some(crate::control_centre::PanelState {
                page: Some(format!("tray/{key}")),
                ..panel
            });
        }
    }

    fn tray_press(&mut self, key: &str, press: TrayPress) {
        let Some(item) = self.isle.tray_item(key).cloned() else {
            return;
        };
        let address = item.address.clone();
        let has_menu = item.menu.is_some();
        match press {
            TrayPress::Primary if item.item_is_menu && has_menu => self.open_tray_menu(key),
            TrayPress::Primary => {
                let key = key.to_owned();
                self.isle.run_service(
                    async move { isle_tray::activate(&address, 0, 0).await },
                    move |state, result| match result {
                        Ok(()) => state.close_control_centre(),
                        // An item that does not activate shows its menu instead.
                        Err(_) if has_menu => {
                            state.open_tray_menu(&key);
                            state.follow_pages();
                            state.isle.refresh_island();
                        }
                        Err(err) => debug!("a tray item's activation: {err}"),
                    },
                );
            }
            TrayPress::Secondary if has_menu => self.open_tray_menu(key),
            TrayPress::Secondary | TrayPress::Middle => {
                self.isle.run_service(
                    async move { isle_tray::secondary_activate(&address, 0, 0).await },
                    |_, result| {
                        if let Err(err) = result {
                            debug!("a tray item's secondary activation: {err}");
                        }
                    },
                );
            }
            TrayPress::Scroll(delta) => {
                self.isle.run_service(
                    async move { isle_tray::scroll(&address, delta).await },
                    |_, _| {},
                );
            }
        }
    }

    fn tray_choose(&mut self, row: &str) {
        let page = self.isle.island.panel.as_ref().and_then(|p| p.page.clone());
        let Some((key, _)) = page.as_deref().and_then(page_ids) else {
            return;
        };
        let (Some(item), Ok(id)) = (self.isle.tray_item(key).cloned(), row.parse::<i32>()) else {
            return;
        };
        let Some(menu_path) = item.menu.clone() else {
            return;
        };
        let service = item.address.service.clone();
        self.isle.run_service(
            async move {
                let conn = isle_bus::session().await?;
                Ok(isle_menus::dbusmenu::activate(&conn, &service, &menu_path, id).await?)
            },
            |_, result| {
                if let Err(err) = result {
                    debug!("a tray menu item: {err}");
                }
            },
        );
        self.close_control_centre();
    }

    /// Moves an item to place `to` among those shown and writes the order.
    pub fn tray_reorder(&mut self, key: &str, to: usize) {
        let (shown, passive): (Vec<&Item>, Vec<&Item>) =
            self.isle.tray_items().into_iter().partition(|i| !i.passive);
        let Some(from) = shown.iter().position(|i| self::key(&i.address) == key) else {
            return;
        };
        let mut ids: Vec<String> = shown.iter().map(|i| i.id.clone()).collect();
        let rest: Vec<String> = passive.iter().map(|i| i.id.clone()).collect();
        let id = ids.remove(from);
        ids.insert(to.min(ids.len()), id);
        ids.extend(rest);
        ids.retain(|id| !id.is_empty());
        // In force before the write comes back.
        self.isle.settings.control_centre.tray = ids.clone();
        self.isle
            .write_setting("control-centre.tray", isle_settings::Value::List(ids));
        self.isle.refresh_island();
    }
}
