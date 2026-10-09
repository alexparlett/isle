//! ScreenCast: a session per share, the sources chosen at Start, and the streams' nodes.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use zbus::{Connection, interface, message::Header};
use zvariant::{ObjectPath, OwnedValue, Value};

use crate::{BoxFuture, Closed, Reply, from_frontend, request, response};

pub const MONITOR: u32 = 1;
pub const WINDOW: u32 = 2;
pub const CURSOR_HIDDEN: u32 = 1;
pub const CURSOR_EMBEDDED: u32 = 2;
pub const CURSOR_METADATA: u32 = 4;

/// What SelectSources asked for.
#[derive(Clone, Debug)]
pub struct Selection {
    pub types: u32,
    pub multiple: bool,
    pub cursor_mode: u32,
    pub persist_mode: u32,
    /// The data of a share the app keeps, as Isle wrote it: vendor, version, payload.
    pub restore: Option<(String, u32, OwnedValue)>,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            types: MONITOR,
            multiple: false,
            cursor_mode: CURSOR_HIDDEN,
            persist_mode: 0,
            restore: None,
        }
    }
}

/// One stream started.
#[derive(Clone, Debug)]
pub struct StreamOut {
    pub node: u32,
    pub serial: Option<u64>,
    pub position: Option<(i32, i32)>,
    pub size: (i32, i32),
    pub source_type: u32,
    pub mapping_id: Option<String>,
}

#[derive(Debug)]
pub struct Started {
    pub streams: Vec<StreamOut>,
    pub persist_mode: u32,
    pub restore: Option<(String, u32, OwnedValue)>,
}

pub trait ScreenCast: Send + Sync {
    fn source_types(&self) -> u32;
    fn cursor_modes(&self) -> u32;
    /// Chooses the sources (by the restore data, or in the picker) and starts their streams;
    /// `None` is a cancel.
    fn start(
        &self,
        session: String,
        app_id: String,
        parent_window: String,
        selection: Selection,
        closed: Closed,
    ) -> BoxFuture<Option<Started>>;
    /// The session closed: its streams end.
    fn close(&self, session: String);
}

pub struct ScreenCastIface {
    handler: Arc<dyn ScreenCast>,
    selections: Arc<Mutex<HashMap<String, Selection>>>,
}

impl ScreenCastIface {
    pub fn new(handler: Arc<dyn ScreenCast>) -> Self {
        Self {
            handler,
            selections: Arc::default(),
        }
    }
}

fn get<T: TryFrom<OwnedValue>>(options: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
    options
        .get(key)
        .and_then(|v| T::try_from(v.try_clone().ok()?).ok())
}

fn restore_of(options: &HashMap<String, OwnedValue>) -> Option<(String, u32, OwnedValue)> {
    let value = options.get("restore_data")?;
    let structure = zvariant::Structure::try_from(value.try_clone().ok()?).ok()?;
    let fields = structure.into_fields();
    let vendor = String::try_from(fields.first()?.try_clone().ok()?).ok()?;
    let version = u32::try_from(fields.get(1)?.try_clone().ok()?).ok()?;
    let payload = match fields.get(2)? {
        Value::Value(inner) => OwnedValue::try_from(inner.as_ref().try_clone().ok()?).ok()?,
        other => OwnedValue::try_from(other.try_clone().ok()?).ok()?,
    };
    Some((vendor, version, payload))
}

#[interface(name = "org.freedesktop.impl.portal.ScreenCast")]
impl ScreenCastIface {
    async fn create_session(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        _handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        _app_id: String,
        _options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let handler = self.handler.clone();
        let selections = self.selections.clone();
        request::export_session(
            conn,
            &session_handle,
            Arc::new(move |session| {
                selections.lock().unwrap().remove(&session);
                handler.close(session);
            }),
        )
        .await?;
        self.selections
            .lock()
            .unwrap()
            .insert(session_handle.to_string(), Selection::default());
        let mut results = HashMap::new();
        results.insert(
            "session_id".to_owned(),
            OwnedValue::try_from(Value::from(session_handle.to_string())).unwrap(),
        );
        Ok((response::SUCCESS, results))
    }

    async fn select_sources(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        _handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        _app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let cursor_mode = get::<u32>(&options, "cursor_mode").unwrap_or(CURSOR_HIDDEN);
        if cursor_mode & self.handler.cursor_modes() == 0 {
            return Ok((response::OTHER, HashMap::new()));
        }
        let selection = Selection {
            types: get::<u32>(&options, "types").unwrap_or(MONITOR) & self.handler.source_types(),
            multiple: get::<bool>(&options, "multiple").unwrap_or(false),
            cursor_mode,
            persist_mode: get::<u32>(&options, "persist_mode").unwrap_or(0),
            restore: restore_of(&options),
        };
        let mut selections = self.selections.lock().unwrap();
        match selections.get_mut(session_handle.as_str()) {
            Some(slot) => {
                *slot = selection;
                Ok((response::SUCCESS, HashMap::new()))
            }
            None => Ok((response::OTHER, HashMap::new())),
        }
    }

    async fn start(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        _options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, &header).await?;
        let Some(selection) = self
            .selections
            .lock()
            .unwrap()
            .get(session_handle.as_str())
            .cloned()
        else {
            return Ok((response::OTHER, HashMap::new()));
        };
        let handler = self.handler.clone();
        let session = session_handle.to_string();
        let started = request::with_request(conn, &handle, |closed| {
            handler.start(session, app_id, parent_window, selection, closed)
        })
        .await;
        let Some(started) = started else {
            return Ok((response::CANCELLED, HashMap::new()));
        };
        let streams: Vec<(u32, HashMap<String, OwnedValue>)> = started
            .streams
            .iter()
            .map(|s| {
                let mut props: HashMap<String, OwnedValue> = HashMap::new();
                let mut put = |k: &str, v: Value<'_>| {
                    if let Ok(v) = OwnedValue::try_from(v) {
                        props.insert(k.into(), v);
                    }
                };
                if let Some(p) = s.position {
                    put("position", Value::from(p));
                }
                put("size", Value::from(s.size));
                put("source_type", Value::from(s.source_type));
                if let Some(id) = &s.mapping_id {
                    put("mapping_id", Value::from(id.clone()));
                }
                if let Some(serial) = s.serial {
                    put("pipewire-serial", Value::from(serial));
                }
                (s.node, props)
            })
            .collect();
        let mut results = HashMap::new();
        if let Ok(v) = OwnedValue::try_from(Value::from(streams)) {
            results.insert("streams".to_owned(), v);
        }
        if started.persist_mode > 0
            && let Some((vendor, version, payload)) = started.restore
        {
            if let Ok(v) = OwnedValue::try_from(Value::from(started.persist_mode)) {
                results.insert("persist_mode".to_owned(), v);
            }
            let data = zvariant::StructureBuilder::new()
                .add_field(vendor)
                .add_field(version)
                .append_field(Value::Value(Box::new(Value::from(payload))))
                .build();
            if let Ok(data) = data
                && let Ok(v) = OwnedValue::try_from(Value::from(data))
            {
                results.insert("restore_data".to_owned(), v);
            }
        }
        Ok((response::SUCCESS, results))
    }

    #[zbus(property)]
    fn available_source_types(&self) -> u32 {
        self.handler.source_types()
    }

    #[zbus(property)]
    fn available_cursor_modes(&self) -> u32 {
        self.handler.cursor_modes()
    }

    // The frontend reads `version` in lower case; zbus's `Version` reads as nought.
    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        5
    }
}
