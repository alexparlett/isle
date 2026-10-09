//! Settings: the `org.freedesktop.appearance` namespace from Isle's own appearance.

use std::{collections::HashMap, sync::Arc};

use zbus::{Connection, interface, message::Header, object_server::SignalEmitter};
use zvariant::{OwnedValue, Value};

use crate::{PATH, from_frontend};

pub const APPEARANCE: &str = "org.freedesktop.appearance";

pub trait Settings: Send + Sync {
    /// Every key of every namespace served, by namespace.
    fn read_all(&self) -> HashMap<String, HashMap<String, OwnedValue>>;
}

pub struct SettingsIface {
    handler: Arc<dyn Settings>,
}

impl SettingsIface {
    pub fn new(handler: Arc<dyn Settings>) -> Self {
        Self { handler }
    }
}

/// Whether `namespace` matches one of the frontend's patterns: exact, or a prefix ending `*`.
fn wanted(patterns: &[String], namespace: &str) -> bool {
    patterns.is_empty()
        || patterns.iter().any(|p| match p.strip_suffix('*') {
            Some(prefix) => namespace.starts_with(prefix),
            None => p == namespace || p.is_empty(),
        })
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.portal.Error")]
pub enum Error {
    #[zbus(error)]
    ZBus(zbus::Error),
    NotFound(String),
    NotAllowed(String),
}

impl From<zbus::fdo::Error> for Error {
    fn from(err: zbus::fdo::Error) -> Self {
        Error::NotAllowed(err.to_string())
    }
}

#[interface(name = "org.freedesktop.impl.portal.Settings")]
impl SettingsIface {
    async fn read_all(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        namespaces: Vec<String>,
    ) -> Result<HashMap<String, HashMap<String, OwnedValue>>, Error> {
        from_frontend(conn, &header).await?;
        Ok(self
            .handler
            .read_all()
            .into_iter()
            .filter(|(ns, _)| wanted(&namespaces, ns))
            .collect())
    }

    async fn read(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        namespace: String,
        key: String,
    ) -> Result<OwnedValue, Error> {
        from_frontend(conn, &header).await?;
        self.handler
            .read_all()
            .remove(&namespace)
            .and_then(|mut keys| keys.remove(&key))
            .ok_or_else(|| Error::NotFound(format!("{namespace}.{key}")))
    }

    #[zbus(signal)]
    async fn setting_changed(
        emitter: &SignalEmitter<'_>,
        namespace: &str,
        key: &str,
        value: Value<'_>,
    ) -> zbus::Result<()>;

    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        2
    }
}

/// Tells the frontend one key changed.
pub async fn changed(conn: &Connection, namespace: &str, key: &str, value: Value<'_>) {
    if let Ok(emitter) = SignalEmitter::new(conn, PATH) {
        let _ = SettingsIface::setting_changed(&emitter, namespace, key, value).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_match_exactly_or_by_prefix() {
        let p = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(wanted(&p(&[]), APPEARANCE));
        assert!(wanted(&p(&["org.freedesktop.*"]), APPEARANCE));
        assert!(wanted(&p(&[APPEARANCE]), APPEARANCE));
        assert!(!wanted(&p(&["org.gnome.*"]), APPEARANCE));
    }
}
