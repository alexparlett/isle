//! The Request object the frontend closes when the app cancels or goes, and the Session
//! object a ScreenCast or an Inhibit monitor lives in.

use std::{future::Future, sync::Arc};

use tokio::sync::watch;
use zbus::{Connection, interface, object_server::SignalEmitter};
use zvariant::ObjectPath;

/// Becomes true when the frontend closes the request.
#[derive(Clone)]
pub struct Closed(watch::Receiver<bool>);

impl Closed {
    /// Waits until the request is closed.
    pub async fn wait(&mut self) {
        let _ = self.0.wait_for(|c| *c).await;
    }

    pub fn is_closed(&self) -> bool {
        *self.0.borrow()
    }

    /// One that never closes, for the tests.
    pub fn never() -> (Self, watch::Sender<bool>) {
        let (tx, rx) = watch::channel(false);
        (Self(rx), tx)
    }
}

struct RequestIface {
    close: watch::Sender<bool>,
}

#[interface(name = "org.freedesktop.impl.portal.Request")]
impl RequestIface {
    async fn close(&self) {
        let _ = self.close.send(true);
    }
}

/// Exports a Request at `handle` while `f` runs, giving it the request's [`Closed`].
pub async fn with_request<F, Fut, T>(conn: &Connection, handle: &ObjectPath<'_>, f: F) -> T
where
    F: FnOnce(Closed) -> Fut,
    Fut: Future<Output = T>,
{
    let (tx, rx) = watch::channel(false);
    let exported = conn
        .object_server()
        .at(handle, RequestIface { close: tx })
        .await
        .unwrap_or(false);
    let out = f(Closed(rx)).await;
    if exported {
        let _ = conn.object_server().remove::<RequestIface, _>(handle).await;
    }
    out
}

/// What a session's Close does on the backend's side.
pub type OnClose = Arc<dyn Fn(String) + Send + Sync>;

pub(crate) struct SessionIface {
    pub id: String,
    pub on_close: OnClose,
}

#[interface(name = "org.freedesktop.impl.portal.Session")]
impl SessionIface {
    async fn close(
        &self,
        #[zbus(object_server)] server: &zbus::ObjectServer,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        (self.on_close)(self.id.clone());
        let path = emitter.path().to_owned();
        let _ = server.remove::<SessionIface, _>(&path).await;
    }

    #[zbus(signal)]
    async fn closed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        1
    }
}

/// Exports a Session at `path`; `on_close` hears the frontend close it.
pub(crate) async fn export_session(
    conn: &Connection,
    path: &ObjectPath<'_>,
    on_close: OnClose,
) -> zbus::Result<bool> {
    conn.object_server()
        .at(
            path,
            SessionIface {
                id: path.to_string(),
                on_close,
            },
        )
        .await
}

/// Ends the session at `path` from Isle's side: its Closed is emitted and it goes.
pub async fn end_session(conn: &Connection, path: &str) {
    let Ok(path) = ObjectPath::try_from(path) else {
        return;
    };
    if let Ok(emitter) = SignalEmitter::new(conn, path.clone()) {
        let _ = SessionIface::closed(&emitter).await;
    }
    let _ = conn.object_server().remove::<SessionIface, _>(&path).await;
}
