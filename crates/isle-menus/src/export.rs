//! An app's menu bar served over DBusMenu (`com.canonical.dbusmenu`), for Isle's own apps
//! (`apps.md` Conventions 8): the tree is replaced whole and announced, and a choice comes
//! back as the item's id.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use zbus::{interface, object_server::SignalEmitter};
use zvariant::{OwnedValue, StructureBuilder, Value};

/// One entry of a menu.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Item(MenuItem),
    Separator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    /// Unique in the tree and stable across updates; 0 is the root's.
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    /// A checkmark's state, for an item that toggles.
    pub checked: Option<bool>,
    /// DBusMenu's chord: modifier names (`Super`, `Control`, `Alt`, `Shift`) then the key.
    pub shortcut: Option<Vec<String>>,
    pub children: Vec<Entry>,
}

impl MenuItem {
    pub fn new(id: i32, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            checked: None,
            shortcut: None,
            children: Vec::new(),
        }
    }
}

/// Called with the id of each item chosen.
pub type OnActivate = Arc<dyn Fn(i32) + Send + Sync>;

struct Menus {
    revision: u32,
    top: Vec<MenuItem>,
}

/// The served object; `update` replaces what it serves.
#[derive(Clone)]
pub struct MenuBar {
    menus: Arc<Mutex<Menus>>,
    on_activate: OnActivate,
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

fn owned(value: Value<'static>) -> OwnedValue {
    OwnedValue::try_from(value).expect("a value without file descriptors")
}

fn props(item: &MenuItem) -> HashMap<String, OwnedValue> {
    let mut props = HashMap::from([("label".to_owned(), owned(Value::from(item.label.clone())))]);
    if !item.enabled {
        props.insert("enabled".into(), owned(Value::from(false)));
    }
    if let Some(checked) = item.checked {
        props.insert("toggle-type".into(), owned(Value::from("checkmark")));
        props.insert("toggle-state".into(), owned(Value::from(i32::from(checked))));
    }
    if let Some(chord) = &item.shortcut {
        props.insert("shortcut".into(), owned(Value::from(vec![chord.clone()])));
    }
    if !item.children.is_empty() {
        props.insert("children-display".into(), owned(Value::from("submenu")));
    }
    props
}

fn layout(id: i32, props: HashMap<String, OwnedValue>, children: &[Entry], separators: &mut i32) -> Layout {
    let kids = children
        .iter()
        .map(|entry| match entry {
            Entry::Item(item) => structure(layout(item.id, self::props(item), &item.children, separators)),
            Entry::Separator => {
                *separators -= 1;
                let props = HashMap::from([("type".to_owned(), owned(Value::from("separator")))]);
                structure((*separators, props, Vec::new()))
            }
        })
        .collect();
    (id, props, kids)
}

/// A child layout as the variant DBusMenu nests it in.
fn structure((id, props, kids): Layout) -> OwnedValue {
    let props: HashMap<String, Value<'static>> =
        props.into_iter().map(|(k, v)| (k, Value::from(v))).collect();
    let kids: Vec<Value<'static>> = kids.into_iter().map(Value::from).collect();
    owned(Value::Structure(
        StructureBuilder::new()
            .add_field(id)
            .add_field(props)
            .append_field(Value::Array(zvariant::Array::from(kids)))
            .build()
            .expect("a layout structure"),
    ))
}

fn find(items: &[MenuItem], id: i32) -> Option<&MenuItem> {
    for item in items {
        if item.id == id {
            return Some(item);
        }
        for entry in &item.children {
            if let Entry::Item(child) = entry
                && let Some(found) = find(std::slice::from_ref(child), id)
            {
                return Some(found);
            }
        }
    }
    None
}

impl MenuBar {
    pub fn new(top: Vec<MenuItem>, on_activate: OnActivate) -> Self {
        Self {
            menus: Arc::new(Mutex::new(Menus { revision: 1, top })),
            on_activate,
        }
    }

    /// Replaces the tree and tells the reader, through the object served at `path`.
    pub async fn update(&self, conn: &zbus::Connection, path: &str, top: Vec<MenuItem>) -> zbus::Result<()> {
        let revision = {
            let mut menus = self.menus.lock().unwrap();
            if menus.top == top {
                return Ok(());
            }
            menus.top = top;
            menus.revision += 1;
            menus.revision
        };
        let emitter = SignalEmitter::new(conn, path)?;
        Self::layout_updated(&emitter, revision, 0).await
    }

    fn layout_of(&self, parent: i32) -> (u32, Layout) {
        let menus = self.menus.lock().unwrap();
        let mut separators = -1_000_000;
        let root: Vec<Entry> = menus.top.iter().cloned().map(Entry::Item).collect();
        let layout = if parent == 0 {
            layout(0, HashMap::new(), &root, &mut separators)
        } else {
            match find(&menus.top, parent) {
                Some(item) => layout(item.id, props(item), &item.children, &mut separators),
                None => (parent, HashMap::new(), Vec::new()),
            }
        };
        (menus.revision, layout)
    }
}

#[interface(name = "com.canonical.dbusmenu")]
impl MenuBar {
    async fn get_layout(&self, parent_id: i32, _depth: i32, _names: Vec<String>) -> (u32, Layout) {
        self.layout_of(parent_id)
    }

    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        let menus = self.menus.lock().unwrap();
        ids.into_iter()
            .filter_map(|id| find(&menus.top, id).map(|item| (id, props(item))))
            .collect()
    }

    async fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    async fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) {
        if event_id == "clicked" {
            (self.on_activate)(id);
        }
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "normal".into()
    }

    #[zbus(signal)]
    async fn layout_updated(emitter: &SignalEmitter<'_>, revision: u32, parent: i32) -> zbus::Result<()>;
}
