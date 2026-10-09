//! Inhibit: an app holding off idle, sleep or the session's end, and monitors of the
//! session's state.

use std::{collections::HashMap, sync::Arc};

use zbus::{Connection, interface, message::Header, object_server::SignalEmitter};
use zvariant::{ObjectPath, OwnedValue, Value};

use crate::{PATH, from_frontend, request};

pub const LOGOUT: u32 = 1;
pub const USER_SWITCH: u32 = 2;
pub const SUSPEND: u32 = 4;
pub const IDLE: u32 = 8;

/// The session's state for a monitor.
pub const RUNNING: u32 = 1;
pub const QUERY_END: u32 = 2;
pub const ENDING: u32 = 3;

pub trait Inhibit: Send + Sync {
    /// `app` holds `flags` off for `reason` until `release` is called with the same handle.
    fn inhibit(&self, handle: String, app: String, flags: u32, reason: String);
    fn release(&self, handle: String);
    /// A monitor began, at `session`; its state follows through [`state_changed`].
    fn monitor(&self, session: String, app: String);
    fn monitor_closed(&self, session: String);
    /// The app has answered a query-end.
    fn query_end_response(&self, session: String);
}

pub struct InhibitIface {
    handler: Arc<dyn Inhibit>,
}

impl InhibitIface {
    pub fn new(handler: Arc<dyn Inhibit>) -> Self {
        Self { handler }
    }
}

struct InhibitRequest {
    handle: String,
    handler: Arc<dyn Inhibit>,
}

#[interface(name = "org.freedesktop.impl.portal.Request")]
impl InhibitRequest {
    async fn close(
        &self,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.handler.release(self.handle.clone());
        let path = emitter.path().to_owned();
        let _ = server.remove::<InhibitRequest, _>(&path).await;
    }
}

#[interface(name = "org.freedesktop.impl.portal.Inhibit")]
impl InhibitIface {
    async fn inhibit(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        _window: String,
        flags: u32,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<()> {
        from_frontend(conn, &header).await?;
        let reason = options
            .get("reason")
            .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
            .unwrap_or_default();
        conn.object_server()
            .at(
                &handle,
                InhibitRequest {
                    handle: handle.to_string(),
                    handler: self.handler.clone(),
                },
            )
            .await?;
        self.handler
            .inhibit(handle.to_string(), app_id, flags, reason);
        Ok(())
    }

    async fn create_monitor(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        _handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        _window: String,
    ) -> zbus::fdo::Result<u32> {
        from_frontend(conn, &header).await?;
        let handler = self.handler.clone();
        request::export_session(
            conn,
            &session_handle,
            Arc::new(move |session| handler.monitor_closed(session)),
        )
        .await?;
        self.handler.monitor(session_handle.to_string(), app_id);
        Ok(crate::response::SUCCESS)
    }

    async fn query_end_response(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        session_handle: ObjectPath<'_>,
    ) -> zbus::fdo::Result<()> {
        from_frontend(conn, &header).await?;
        self.handler.query_end_response(session_handle.to_string());
        Ok(())
    }

    #[zbus(signal)]
    async fn state_changed(
        emitter: &SignalEmitter<'_>,
        session_handle: ObjectPath<'_>,
        state: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<()>;
}

/// Tells a monitor the session's state.
pub async fn state_changed(conn: &Connection, session: &str, screensaver_active: bool, state: u32) {
    let (Ok(emitter), Ok(path)) = (
        SignalEmitter::new(conn, PATH),
        ObjectPath::try_from(session),
    ) else {
        return;
    };
    let mut map = HashMap::new();
    map.insert("screensaver-active", Value::from(screensaver_active));
    map.insert("session-state", Value::from(state));
    let _ = InhibitIface::state_changed(&emitter, path, map).await;
}
