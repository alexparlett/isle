//! Menus exported as apps export them, for the tests: a small tree over DBusMenu as Qt and
//! Chromium serve it, and over GMenuModel as GTK does. Each activation is reported by label.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use zbus::{interface, object_server::SignalEmitter};
use zvariant::{OwnedValue, StructureBuilder, Value};

/// Called with the label of each item chosen.
pub type OnActivate = Arc<dyn Fn(String) + Send + Sync>;

struct Entry {
    id: i32,
    parent: i32,
    label: &'static str,
    shortcut: Option<&'static [&'static str]>,
    toggle: Option<(&'static str, i32)>,
    submenu: bool,
    separator: bool,
    enabled: bool,
}

fn entry(id: i32, parent: i32, label: &'static str) -> Entry {
    Entry {
        id,
        parent,
        label,
        shortcut: None,
        toggle: None,
        submenu: false,
        separator: false,
        enabled: true,
    }
}

/// File (New, Open Recent ▸ filled on asking, Quit), Edit (Copy, Paste disabled, Word Wrap
/// checked, Light and Dark as a radio pair), Help (About).
fn tree() -> Vec<Entry> {
    vec![
        Entry {
            submenu: true,
            ..entry(1, 0, "_File")
        },
        Entry {
            shortcut: Some(&["Control", "n"]),
            ..entry(10, 1, "_New")
        },
        Entry {
            submenu: true,
            ..entry(11, 1, "Open _Recent")
        },
        Entry {
            separator: true,
            ..entry(12, 1, "")
        },
        Entry {
            shortcut: Some(&["Control", "q"]),
            ..entry(13, 1, "_Quit")
        },
        Entry {
            submenu: true,
            ..entry(2, 0, "_Edit")
        },
        Entry {
            shortcut: Some(&["Control", "c"]),
            ..entry(20, 2, "_Copy")
        },
        Entry {
            enabled: false,
            ..entry(21, 2, "_Paste")
        },
        Entry {
            toggle: Some(("checkmark", 1)),
            ..entry(22, 2, "Word _Wrap")
        },
        Entry {
            toggle: Some(("radio", 1)),
            ..entry(23, 2, "Light")
        },
        Entry {
            toggle: Some(("radio", 0)),
            ..entry(24, 2, "Dark")
        },
        Entry {
            submenu: true,
            ..entry(3, 0, "_Help")
        },
        entry(30, 3, "About Test"),
    ]
}

/// A DBusMenu exporter whose Open Recent fills only once asked, as Qt's do.
pub struct DBusMenuExport {
    filled: AtomicBool,
    on_activate: OnActivate,
}

impl DBusMenuExport {
    pub fn new(on_activate: OnActivate) -> Self {
        Self {
            filled: AtomicBool::new(false),
            on_activate,
        }
    }

    fn layout(&self, id: i32, entries: &[Entry]) -> Layout {
        let mut props: HashMap<String, OwnedValue> = HashMap::new();
        let mut put = |k: &str, v: Value<'static>| {
            props.insert(k.into(), OwnedValue::try_from(v).unwrap());
        };
        if let Some(e) = entries.iter().find(|e| e.id == id) {
            if e.separator {
                put("type", Value::from("separator"));
            } else {
                put("label", Value::from(e.label));
            }
            if !e.enabled {
                put("enabled", Value::from(false));
            }
            if let Some(keys) = e.shortcut {
                let chord: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
                put("shortcut", Value::from(vec![chord]));
            }
            if let Some((kind, state)) = e.toggle {
                put("toggle-type", Value::from(kind));
                put("toggle-state", Value::from(state));
            }
            if e.submenu {
                put("children-display", Value::from("submenu"));
            }
        }
        let filled = self.filled.load(Ordering::Acquire);
        let mut kids: Vec<OwnedValue> = Vec::new();
        if id != 11 || filled {
            for child in entries.iter().filter(|e| e.parent == id) {
                kids.push(structure(self.layout(child.id, entries)));
            }
        }
        if id == 11 && filled {
            let mut recent: HashMap<String, OwnedValue> = HashMap::new();
            recent.insert(
                "label".into(),
                OwnedValue::try_from(Value::from("Recent One")).unwrap(),
            );
            kids.push(structure((110, recent, Vec::new())));
        }
        (id, props, kids)
    }
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// A child layout as the variant DBusMenu nests it in.
fn structure((id, props, kids): Layout) -> OwnedValue {
    let props: HashMap<String, Value<'static>> = props
        .into_iter()
        .map(|(k, v)| (k, Value::from(v)))
        .collect();
    let kids: Vec<Value<'static>> = kids.into_iter().map(Value::from).collect();
    OwnedValue::try_from(Value::Structure(
        StructureBuilder::new()
            .add_field(id)
            .add_field(props)
            .append_field(Value::Array(zvariant::Array::from(kids)))
            .build()
            .unwrap(),
    ))
    .unwrap()
}

#[interface(name = "com.canonical.dbusmenu")]
impl DBusMenuExport {
    async fn get_layout(&self, parent_id: i32, _depth: i32, _names: Vec<String>) -> (u32, Layout) {
        (1, self.layout(parent_id, &tree()))
    }

    async fn about_to_show(&self, id: i32) -> bool {
        if id == 11 {
            self.filled.store(true, Ordering::Release);
        }
        false
    }

    async fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) {
        if event_id != "clicked" {
            return;
        }
        let label = match id {
            110 => "Recent One".to_owned(),
            _ => tree()
                .into_iter()
                .find(|e| e.id == id)
                .map(|e| crate::strip_mnemonic(e.label))
                .unwrap_or_default(),
        };
        (self.on_activate)(label);
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(signal)]
    async fn layout_updated(
        emitter: &SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}

/// GTK's menus: File (New), Edit (Copy, a View section with Word Wrap).
pub struct GMenuExport;

fn link(group: u32, menu: u32) -> OwnedValue {
    OwnedValue::try_from(Value::Structure(
        StructureBuilder::new()
            .add_field(group)
            .add_field(menu)
            .build()
            .unwrap(),
    ))
    .unwrap()
}

fn attrs(pairs: Vec<(&str, OwnedValue)>) -> HashMap<String, OwnedValue> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn s(text: &str) -> OwnedValue {
    OwnedValue::try_from(Value::from(text.to_owned())).unwrap()
}

#[interface(name = "org.gtk.Menus")]
impl GMenuExport {
    async fn start(&self, groups: Vec<u32>) -> Vec<(u32, u32, Vec<HashMap<String, OwnedValue>>)> {
        let mut out = Vec::new();
        for group in groups {
            match group {
                0 => out.push((
                    0,
                    0,
                    vec![
                        attrs(vec![("label", s("_File")), (":submenu", link(1, 0))]),
                        attrs(vec![("label", s("_Edit")), (":submenu", link(2, 0))]),
                    ],
                )),
                1 => out.push((
                    1,
                    0,
                    vec![attrs(vec![
                        ("label", s("_New")),
                        ("action", s("app.new")),
                        ("accel", s("<Primary>n")),
                    ])],
                )),
                2 => {
                    out.push((
                        2,
                        0,
                        vec![
                            attrs(vec![
                                ("label", s("_Copy")),
                                ("action", s("win.copy")),
                                ("accel", s("<Primary>c")),
                            ]),
                            attrs(vec![("label", s("View")), (":section", link(2, 1))]),
                        ],
                    ));
                    out.push((
                        2,
                        1,
                        vec![attrs(vec![
                            ("label", s("Word _Wrap")),
                            ("action", s("win.wrap")),
                        ])],
                    ));
                }
                _ => {}
            }
        }
        out
    }

    async fn end(&self, _groups: Vec<u32>) {}
}

/// An action group: its actions' names with a boolean state for those that toggle.
pub struct ActionsExport {
    actions: Vec<(&'static str, Option<bool>)>,
    state: Mutex<HashMap<String, bool>>,
    on_activate: OnActivate,
}

impl ActionsExport {
    pub fn app(on_activate: OnActivate) -> Self {
        Self {
            actions: vec![("new", None)],
            state: Mutex::default(),
            on_activate,
        }
    }

    pub fn window(on_activate: OnActivate) -> Self {
        Self {
            actions: vec![("copy", None), ("wrap", Some(true))],
            state: Mutex::default(),
            on_activate,
        }
    }
}

#[interface(name = "org.gtk.Actions")]
impl ActionsExport {
    async fn list(&self) -> Vec<String> {
        self.actions.iter().map(|(n, _)| n.to_string()).collect()
    }

    async fn describe_all(&self) -> HashMap<String, (bool, zvariant::Signature, Vec<OwnedValue>)> {
        let state = self.state.lock().unwrap();
        self.actions
            .iter()
            .map(|(name, initial)| {
                let current = state.get(*name).copied().or(*initial);
                let values = current
                    .map(|b| OwnedValue::try_from(Value::from(b)).unwrap())
                    .into_iter()
                    .collect();
                (
                    name.to_string(),
                    (true, zvariant::Signature::try_from("").unwrap(), values),
                )
            })
            .collect()
    }

    async fn activate(
        &self,
        name: String,
        _parameter: Vec<OwnedValue>,
        _platform: HashMap<String, OwnedValue>,
    ) {
        (self.on_activate)(name);
    }

    async fn set_state(
        &self,
        name: String,
        value: OwnedValue,
        _platform: HashMap<String, OwnedValue>,
    ) {
        if let Value::Bool(b) = &*value {
            self.state.lock().unwrap().insert(name, *b);
        }
    }
}
