//! `org.freedesktop.impl.portal.FileChooser`: each request goes to Files over its `Files1`
//! interface, Files started by D-Bus activation, and its one answer comes back as a signal
//! (`p15-files.md` choice 21).

use std::collections::HashMap;

use futures_util::StreamExt;
use tracing::warn;
use zbus::{Connection, interface, message::Header, proxy};
use zvariant::{ObjectPath, OwnedValue};

use crate::{Closed, Reply, from_frontend, request::with_request, response};

pub const FILES: &str = "io.github.alexparlett.Isle.Files";

#[proxy(
    interface = "io.github.alexparlett.Isle.Files1",
    default_service = "io.github.alexparlett.Isle.Files",
    default_path = "/io/github/alexparlett/Isle/Files1"
)]
trait Files1 {
    fn choose(
        &self,
        kind: &str,
        app_id: &str,
        parent: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<u64>;

    fn cancel(&self, id: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    fn answered(&self, id: u64, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
}

/// Asks Files and waits for its answer, the request's Close, or Files leaving the bus.
pub async fn choose(
    conn: &Connection,
    kind: &str,
    app_id: &str,
    parent: &str,
    title: &str,
    options: HashMap<String, OwnedValue>,
    mut closed: Closed,
) -> Reply {
    let cancelled = (response::CANCELLED, HashMap::new());
    let Ok(files) = Files1Proxy::new(conn).await else {
        return (response::OTHER, HashMap::new());
    };
    // Listening first, so an answer that comes at once is not missed.
    let Ok(mut answers) = files.receive_answered().await else {
        return (response::OTHER, HashMap::new());
    };
    // Started by this call, Files owns its name a moment before it serves Files1.
    let mut tries = 0;
    let id = loop {
        let sent = options.iter().map(|(k, v)| v.try_clone().map(|v| (k.clone(), v))).collect::<Result<HashMap<_, _>, _>>();
        let Ok(sent) = sent else { return (response::OTHER, HashMap::new()) };
        match files.choose(kind, app_id, parent, title, sent).await {
            Ok(id) => break id,
            Err(zbus::Error::MethodError(name, _, _))
                if tries < 40 && matches!(name.as_str(), "org.freedesktop.DBus.Error.UnknownObject" | "org.freedesktop.DBus.Error.UnknownMethod") =>
            {
                tries += 1;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            Err(err) => {
                warn!("Files did not take the request: {err}");
                return (response::OTHER, HashMap::new());
            }
        }
    };
    let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await else { return cancelled };
    let Ok(mut owners) = dbus.receive_name_owner_changed_with_args(&[(0, FILES)]).await else {
        return cancelled;
    };
    loop {
        tokio::select! {
            answer = answers.next() => {
                let Some(answer) = answer else { return cancelled };
                let Ok(args) = answer.args() else { continue };
                if args.id == id {
                    return (args.response, args.results);
                }
            }
            changed = owners.next() => {
                let gone = changed.and_then(|c| c.args().ok().map(|a| a.new_owner().is_none())).unwrap_or(true);
                if gone {
                    return cancelled;
                }
            }
            _ = closed.wait() => {
                let _ = files.cancel(id).await;
                return cancelled;
            }
        }
    }
}

pub struct FileChooserIface;

impl FileChooserIface {
    async fn ask(
        conn: &Connection,
        header: &Header<'_>,
        handle: ObjectPath<'_>,
        kind: &str,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        from_frontend(conn, header).await?;
        Ok(with_request(conn, &handle, |closed| async move {
            choose(conn, kind, &app_id, &parent_window, &title, options, closed).await
        })
        .await)
    }
}

#[interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl FileChooserIface {
    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        4
    }

    #[allow(clippy::too_many_arguments)]
    async fn open_file(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        Self::ask(conn, &header, handle, "open", app_id, parent_window, title, options).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn save_file(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        Self::ask(conn, &header, handle, "save", app_id, parent_window, title, options).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn save_files(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        title: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<Reply> {
        Self::ask(conn, &header, handle, "save-files", app_id, parent_window, title, options).await
    }
}
