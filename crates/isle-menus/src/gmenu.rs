//! A GMenuModel client: GTK's `org.gtk.Menus` for the menus and `org.gtk.Actions` for what
//! their items do, read into a [`Tree`]. No Rust crate reads them; this follows KDE's
//! gmenu-dbusmenu-proxy in resolving `app.`, `win.` and `unity.` (`global-menu-2026.md`).

use std::collections::{BTreeSet, HashMap};

use futures_util::{Stream, StreamExt};
use zvariant::{OwnedValue, Value};

use crate::{Activation, Item, Node, Shortcut, Source, Toggle, Tree, limited, strip_mnemonic};

type Entry = (u32, u32, Vec<HashMap<String, OwnedValue>>);
/// An action's description: enabled, parameter type, and state if it has one.
type Description = (bool, zvariant::Signature, Vec<OwnedValue>);

/// The action groups an item's prefix names.
struct Groups<'a> {
    app: Option<&'a str>,
    win: Option<&'a str>,
    unity: Option<&'a str>,
}

impl Groups<'_> {
    fn path(&self, prefix: &str) -> Option<String> {
        match prefix {
            "app" => self.app,
            "win" => self.win,
            "unity" => self.unity,
            _ => None,
        }
        .map(str::to_owned)
    }
}

/// The app's menu bar from `source`, a [`Source::GMenu`].
pub async fn fetch(conn: &zbus::Connection, source: &Source) -> zbus::Result<Tree> {
    let Source::GMenu {
        service,
        menubar,
        application,
        window,
        unity,
    } = source
    else {
        return Err(zbus::Error::Failure("not a GMenu source".into()));
    };
    let menus = zbus::Proxy::new(conn, service.clone(), menubar.clone(), "org.gtk.Menus").await?;
    let mut groups: BTreeSet<u32> = BTreeSet::new();
    let mut wanted = vec![0u32];
    let mut content: HashMap<(u32, u32), Vec<HashMap<String, OwnedValue>>> = HashMap::new();
    while !wanted.is_empty() {
        let ask: Vec<u32> = wanted.drain(..).filter(|g| groups.insert(*g)).collect();
        if ask.is_empty() {
            break;
        }
        let entries: Vec<Entry> = limited(menus.call("Start", &(ask.clone(),))).await?;
        for (group, menu, items) in entries {
            for item in &items {
                for link in [":submenu", ":section"] {
                    if let Some((g, _)) = item.get(link).and_then(pair) {
                        wanted.push(g);
                    }
                }
            }
            content.insert((group, menu), items);
        }
    }
    let _ =
        limited(menus.call::<_, _, ()>("End", &(groups.iter().copied().collect::<Vec<u32>>(),)))
            .await;

    // The actions' state, from each group the items name.
    let unity_path = unity.clone().or_else(|| Some(menubar.clone()));
    let paths = Groups {
        app: application.as_deref(),
        win: window.as_deref(),
        unity: unity_path.as_deref(),
    };
    let mut described: HashMap<String, HashMap<String, Description>> = HashMap::new();
    for path in [paths.app, paths.win, paths.unity].into_iter().flatten() {
        if described.contains_key(path) {
            continue;
        }
        let actions =
            zbus::Proxy::new(conn, service.clone(), path.to_owned(), "org.gtk.Actions").await?;
        let all: HashMap<String, Description> = limited(actions.call("DescribeAll", &()))
            .await
            .unwrap_or_default();
        described.insert(path.to_owned(), all);
    }

    let root = build(&content, &described, &paths, (0, 0), 0);
    let menus = root
        .into_iter()
        .filter_map(|n| match n {
            Node::Item(item) => Some(item),
            _ => None,
        })
        .collect();
    Ok(Tree { menus })
}

fn pair(value: &OwnedValue) -> Option<(u32, u32)> {
    let Value::Structure(s) = &**value else {
        return None;
    };
    match s.fields() {
        [Value::U32(a), Value::U32(b)] => Some((*a, *b)),
        _ => None,
    }
}

fn text(item: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    item.get(key).and_then(|v| match &**v {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    })
}

fn build(
    content: &HashMap<(u32, u32), Vec<HashMap<String, OwnedValue>>>,
    described: &HashMap<String, HashMap<String, Description>>,
    groups: &Groups<'_>,
    at: (u32, u32),
    depth: usize,
) -> Vec<Node> {
    if depth > 16 {
        return Vec::new();
    }
    let Some(items) = content.get(&at) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        if let Some(section) = item.get(":section").and_then(pair) {
            let inner = build(content, described, groups, section, depth + 1);
            if inner.is_empty() {
                continue;
            }
            match text(item, "label") {
                Some(label) => out.push(Node::Section(strip_mnemonic(&label))),
                None if !out.is_empty() => out.push(Node::Separator),
                None => {}
            }
            out.extend(inner);
            continue;
        }
        let label = strip_mnemonic(&text(item, "label").unwrap_or_default());
        let children = item
            .get(":submenu")
            .and_then(pair)
            .map(|sub| build(content, described, groups, sub, depth + 1));
        let action = text(item, "action");
        let target = item.get("target").and_then(|t| t.try_clone().ok());
        let (activation, enabled, toggle) = match action.as_deref().and_then(|a| a.split_once('.'))
        {
            Some((prefix, name)) => match groups.path(prefix) {
                Some(path) => {
                    let description = described.get(&path).and_then(|d| d.get(name));
                    let enabled = description.is_none_or(|d| d.0);
                    let state = description.and_then(|d| d.2.first());
                    let toggle = match (state.map(|s| &**s), &target) {
                        (Some(Value::Bool(on)), _) => Toggle::Check(Some(*on)),
                        (Some(state), Some(target)) => Toggle::Radio(state == &**target),
                        _ => Toggle::None,
                    };
                    (
                        Activation::GAction {
                            path,
                            name: name.to_owned(),
                            target: target.as_ref().and_then(|t| t.try_clone().ok()),
                        },
                        enabled,
                        toggle,
                    )
                }
                None => (Activation::None, false, Toggle::None),
            },
            None => (Activation::None, children.is_some(), Toggle::None),
        };
        let fill = text(item, "submenu-action").and_then(|a| {
            let (prefix, name) = a.split_once('.')?;
            Some(Activation::GAction {
                path: groups.path(prefix)?,
                name: name.to_owned(),
                target: None,
            })
        });
        out.push(Node::Item(Item {
            label,
            enabled,
            toggle,
            shortcut: text(item, "accel")
                .or_else(|| text(item, "x-canonical-accel"))
                .map(Shortcut::Accel),
            children,
            activation,
            fill,
        }));
    }
    out
}

/// Runs an item's action.
pub async fn activate(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
    name: &str,
    target: Option<&OwnedValue>,
) -> zbus::Result<()> {
    let actions =
        zbus::Proxy::new(conn, service.to_owned(), path.to_owned(), "org.gtk.Actions").await?;
    let parameter: Vec<Value<'_>> = target
        .and_then(|t| t.try_clone().ok())
        .map(Value::from)
        .into_iter()
        .collect();
    limited(actions.call::<_, _, ()>(
        "Activate",
        &(name, parameter, HashMap::<String, Value<'_>>::new()),
    ))
    .await
}

/// Sets a submenu's `submenu-action` on or off, so the app fills it or may let it go.
pub async fn set_submenu(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
    name: &str,
    open: bool,
) -> zbus::Result<()> {
    let actions =
        zbus::Proxy::new(conn, service.to_owned(), path.to_owned(), "org.gtk.Actions").await?;
    limited(actions.call::<_, _, ()>(
        "SetState",
        &(
            name,
            Value::Bool(open),
            HashMap::<String, Value<'_>>::new(),
        ),
    ))
    .await
}

/// A stream that yields whenever the app says its menus or actions changed.
pub async fn changes(
    conn: &zbus::Connection,
    source: &Source,
) -> zbus::Result<impl Stream<Item = ()> + 'static> {
    let Source::GMenu {
        service, menubar, ..
    } = source
    else {
        return Err(zbus::Error::Failure("not a GMenu source".into()));
    };
    let menus = zbus::Proxy::new(conn, service.clone(), menubar.clone(), "org.gtk.Menus").await?;
    let actions =
        zbus::Proxy::new(conn, service.clone(), menubar.clone(), "org.gtk.Actions").await?;
    let a = menus.receive_signal("Changed").await?;
    let b = actions.receive_signal("Changed").await?;
    Ok(futures_util::stream::select(a.map(|_| ()), b.map(|_| ())))
}
