//! A DBusMenu client: `com.canonical.dbusmenu` read into a [`Tree`], as Qt, Chromium and
//! Electron export menus. Properties left at their default are filled in; a submenu is
//! asked for with both `AboutToShow` and the `opened` event, since Qt, Chromium and Firefox
//! each fill on a different one (`global-menu-2026.md`).

use std::collections::HashMap;

use futures_util::{Stream, StreamExt};
use zvariant::{OwnedValue, Value};

use crate::{Activation, Item, Node, Shortcut, Toggle, Tree, limited, strip_mnemonic};

const INTERFACE: &str = "com.canonical.dbusmenu";

async fn proxy<'a>(
    conn: &zbus::Connection,
    service: &'a str,
    path: &'a str,
) -> zbus::Result<zbus::Proxy<'a>> {
    zbus::Proxy::new(conn, service.to_owned(), path.to_owned(), INTERFACE).await
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// The app's whole tree, the root's children its menus.
pub async fn fetch(conn: &zbus::Connection, service: &str, path: &str) -> zbus::Result<Tree> {
    let proxy = proxy(conn, service, path).await?;
    // Some apps fill the root only once asked.
    let _ = limited(proxy.call::<_, _, bool>("AboutToShow", &(0i32,))).await;
    let (_, root): (u32, Layout) =
        limited(proxy.call("GetLayout", &(0i32, -1i32, Vec::<String>::new()))).await?;
    let menus = children(&root.2)
        .into_iter()
        .filter_map(|n| match n {
            Node::Item(item) => Some(item),
            _ => None,
        })
        .collect();
    Ok(Tree { menus })
}

/// One submenu, by its parent's id, after asking the app to fill it.
pub async fn fetch_submenu(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
    id: i32,
) -> zbus::Result<Vec<Node>> {
    let proxy = proxy(conn, service, path).await?;
    let _ = limited(proxy.call::<_, _, bool>("AboutToShow", &(id,))).await;
    let _ = event(&proxy, id, "opened").await;
    let (_, layout): (u32, Layout) =
        limited(proxy.call("GetLayout", &(id, -1i32, Vec::<String>::new()))).await?;
    // Isle shows its own copy: the app may let its submenu go again.
    let _ = event(&proxy, id, "closed").await;
    Ok(children(&layout.2))
}

/// Tells the app a submenu it filled has closed.
pub async fn closed(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
    id: i32,
) -> zbus::Result<()> {
    let proxy = proxy(conn, service, path).await?;
    event(&proxy, id, "closed").await
}

/// Chooses item `id`.
pub async fn activate(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
    id: i32,
) -> zbus::Result<()> {
    let proxy = proxy(conn, service, path).await?;
    event(&proxy, id, "clicked").await
}

async fn event(proxy: &zbus::Proxy<'_>, id: i32, kind: &str) -> zbus::Result<()> {
    limited(proxy.call::<_, _, ()>("Event", &(id, kind, Value::I32(0), 0u32))).await
}

/// A stream that yields whenever the app says its menus changed.
pub async fn changes(
    conn: &zbus::Connection,
    service: &str,
    path: &str,
) -> zbus::Result<impl Stream<Item = ()> + 'static> {
    let proxy = zbus::Proxy::new(conn, service.to_owned(), path.to_owned(), INTERFACE).await?;
    let layout = proxy.receive_signal("LayoutUpdated").await?;
    let props = proxy.receive_signal("ItemsPropertiesUpdated").await?;
    Ok(futures_util::stream::select(
        layout.map(|_| ()),
        props.map(|_| ()),
    ))
}

fn children(values: &[OwnedValue]) -> Vec<Node> {
    values
        .iter()
        .filter_map(layout_of)
        .filter_map(node)
        .collect()
}

/// A child of a layout: a variant holding `(ia{sv}av)`.
fn layout_of(value: &OwnedValue) -> Option<Layout> {
    let value: &Value = value;
    let value = match value {
        Value::Value(inner) => inner.as_ref(),
        other => other,
    };
    let Value::Structure(s) = value else {
        return None;
    };
    let fields = s.fields();
    let id = match fields.first()? {
        Value::I32(id) => *id,
        _ => return None,
    };
    let props: HashMap<String, OwnedValue> = match fields.get(1)? {
        Value::Dict(dict) => dict
            .iter()
            .filter_map(|(k, v)| {
                let Value::Str(key) = k else { return None };
                Some((
                    key.to_string(),
                    OwnedValue::try_from(v.try_clone().ok()?).ok()?,
                ))
            })
            .collect(),
        _ => return None,
    };
    let kids: Vec<OwnedValue> = match fields.get(2)? {
        Value::Array(array) => array
            .iter()
            .filter_map(|v| OwnedValue::try_from(v.try_clone().ok()?).ok())
            .collect(),
        _ => Vec::new(),
    };
    Some((id, props, kids))
}

/// A property's value, out of the variant a dictionary of variants may wrap it in.
fn inner<'a>(value: &'a Value<'a>) -> &'a Value<'a> {
    match value {
        Value::Value(v) => inner(v),
        other => other,
    }
}

fn node((id, props, kids): Layout) -> Option<Node> {
    let text = |key: &str| {
        props.get(key).and_then(|v| match inner(v) {
            Value::Str(s) => Some(s.to_string()),
            _ => None,
        })
    };
    let flag = |key: &str, default: bool| {
        props.get(key).map_or(default, |v| match inner(v) {
            Value::Bool(b) => *b,
            _ => default,
        })
    };
    if !flag("visible", true) {
        return None;
    }
    if text("type").as_deref() == Some("separator") {
        return Some(Node::Separator);
    }
    let state = props.get("toggle-state").and_then(|v| match inner(v) {
        Value::I32(s) => Some(*s),
        _ => None,
    });
    let toggle = match text("toggle-type").as_deref() {
        Some("checkmark") => Toggle::Check(match state {
            Some(1) => Some(true),
            Some(0) | None => Some(false),
            _ => None,
        }),
        Some("radio") => Toggle::Radio(state == Some(1)),
        _ => Toggle::None,
    };
    let shortcut = props.get("shortcut").and_then(|v| {
        let Value::Array(chords) = inner(v) else {
            return None;
        };
        let keys: Vec<Vec<String>> = chords
            .iter()
            .filter_map(|c| match inner(c) {
                Value::Array(names) => Some(
                    names
                        .iter()
                        .filter_map(|n| match inner(n) {
                            Value::Str(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect();
        (!keys.is_empty()).then_some(Shortcut::Keys(keys))
    });
    let submenu = text("children-display").as_deref() == Some("submenu") || !kids.is_empty();
    Some(Node::Item(Item {
        label: strip_mnemonic(&text("label").unwrap_or_default()),
        enabled: flag("enabled", true),
        toggle,
        shortcut,
        children: submenu.then(|| children(&kids)),
        activation: Activation::DBusMenu { id },
        fill: (submenu && kids.is_empty()).then_some(Activation::DBusMenu { id }),
    }))
}
