//! Apps' menus as one tree, whichever way an app exports them: DBusMenu through the
//! registrar or the KDE protocol, or GTK's GMenuModel (`p6-island-and-menus.md` choice 14).

pub mod dbusmenu;
#[cfg(feature = "exporter")]
pub mod exporter;
pub mod gmenu;
pub mod registrar;

use std::time::Duration;

/// How long Isle waits on an app before it gives up on the call.
pub const LIMIT: Duration = Duration::from_secs(2);

/// Where an app's menus are on the bus.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    DBusMenu {
        service: String,
        path: String,
    },
    /// GTK's menu model and the action groups its items name: `app.`, `win.` and
    /// appmenu-gtk-module's `unity.`.
    GMenu {
        service: String,
        menubar: String,
        application: Option<String>,
        window: Option<String>,
        unity: Option<String>,
    },
}

/// How an item is checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Toggle {
    #[default]
    None,
    Check(Option<bool>),
    Radio(bool),
}

/// An item's shortcut as the app declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shortcut {
    /// DBusMenu's: each chord a list of names, `["Control", "c"]` or Qt's `["C"]`.
    Keys(Vec<Vec<String>>),
    /// GTK's accelerator string, `<Primary>c`.
    Accel(String),
}

/// What choosing an item sends its app.
#[derive(Clone, Debug, PartialEq)]
pub enum Activation {
    DBusMenu {
        id: i32,
    },
    /// An action in one of the app's groups, by its object path, with its target.
    GAction {
        path: String,
        name: String,
        target: Option<zvariant::OwnedValue>,
    },
    None,
}

/// One node of a menu.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Node {
    Item(Item),
    Separator,
    Section(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub enabled: bool,
    pub toggle: Toggle,
    pub shortcut: Option<Shortcut>,
    /// A submenu, filled when the app has said what is in it.
    pub children: Option<Vec<Node>>,
    pub activation: Activation,
    /// A submenu an app fills only when asked: its DBusMenu id, or GMenu's
    /// `submenu-action`.
    pub fill: Option<Activation>,
}

/// An app's menu bar: its top-level menus with their items.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub menus: Vec<Item>,
}

impl Tree {
    /// The item at `path`, labels from the top-level menu down: items are found again by
    /// their path, as an app may renumber its tree on every change.
    pub fn find(&self, path: &[&str]) -> Option<&Item> {
        let (first, rest) = path.split_first()?;
        let mut item = self.menus.iter().find(|m| m.label == *first)?;
        for label in rest {
            item = item.children.as_ref()?.iter().find_map(|n| match n {
                Node::Item(i) if i.label == *label => Some(i),
                _ => None,
            })?;
        }
        Some(item)
    }

    /// Every item with its path, top-level menus excluded, for search.
    pub fn walk(&self) -> Vec<(Vec<String>, &Item)> {
        fn go<'a>(
            nodes: &'a [Node],
            path: &mut Vec<String>,
            out: &mut Vec<(Vec<String>, &'a Item)>,
        ) {
            for node in nodes {
                if let Node::Item(item) = node {
                    path.push(item.label.clone());
                    out.push((path.clone(), item));
                    if let Some(children) = &item.children {
                        go(children, path, out);
                    }
                    path.pop();
                }
            }
        }
        let mut out = Vec::new();
        for menu in &self.menus {
            let mut path = vec![menu.label.clone()];
            if let Some(children) = &menu.children {
                go(children, &mut path, &mut out);
            }
        }
        out
    }
}

/// A label with its mnemonic marks taken out: `_File` is File, `__` an underscore.
pub fn strip_mnemonic(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// A call into an app, given up after [`LIMIT`].
pub async fn limited<T>(
    call: impl std::future::Future<Output = zbus::Result<T>>,
) -> zbus::Result<T> {
    match tokio::time::timeout(LIMIT, call).await {
        Ok(result) => result,
        Err(_) => Err(zbus::Error::Failure("the app did not answer".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonics_are_stripped_and_doubled_underscores_kept() {
        assert_eq!(strip_mnemonic("_File"), "File");
        assert_eq!(strip_mnemonic("Save _As…"), "Save As…");
        assert_eq!(strip_mnemonic("snake__case"), "snake_case");
    }
}
