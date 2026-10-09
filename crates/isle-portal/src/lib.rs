//! Isle's xdg-desktop-portal backend: the `org.freedesktop.impl.portal.*` interfaces the
//! frontend calls, their Request and Session objects, and the check that only the frontend
//! calls them (`p11-capture.md` choices 18–20). What each answer is comes from a [`Backend`]
//! the compositor provides; nothing here knows the compositor.

pub mod access;
pub mod inhibit;
pub mod request;
pub mod screencast;
pub mod screenshot;
pub mod settings;

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use zbus::{
    Connection,
    fdo::{RequestNameFlags, RequestNameReply},
    message::Header,
    names::BusName,
};
use zvariant::OwnedValue;

pub use request::Closed;

pub const BUS_NAME: &str = "org.freedesktop.impl.portal.desktop.isle";
pub const PATH: &str = "/org/freedesktop/portal/desktop";
/// The frontend's own name; only its owner may call the backend.
pub const FRONTEND: &str = "org.freedesktop.portal.Desktop";

/// A portal reply: the response code and its results.
pub type Reply = (u32, HashMap<String, OwnedValue>);

/// The responses the frontend takes.
pub mod response {
    pub const SUCCESS: u32 = 0;
    pub const CANCELLED: u32 = 1;
    pub const OTHER: u32 = 2;
}

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Refuses a call from anyone but the frontend: the session bus checks nothing, so a client
/// calling the backend directly would skip the frontend's permission checks.
pub async fn from_frontend(conn: &Connection, header: &Header<'_>) -> zbus::fdo::Result<()> {
    let sender = header
        .sender()
        .ok_or_else(|| zbus::fdo::Error::AccessDenied("no sender".into()))?;
    let dbus = zbus::fdo::DBusProxy::new(conn).await?;
    let owner = dbus
        .get_name_owner(BusName::try_from(FRONTEND).unwrap())
        .await
        .map_err(|_| zbus::fdo::Error::AccessDenied("the portal frontend is not running".into()))?;
    if owner.as_str() != sender.as_str() {
        return Err(zbus::fdo::Error::AccessDenied(
            "only xdg-desktop-portal calls the backend".into(),
        ));
    }
    Ok(())
}

/// What the backend answers with: the compositor's side of every interface served.
pub struct Handlers {
    pub access: Arc<dyn access::Access>,
    pub screencast: Arc<dyn screencast::ScreenCast>,
    pub screenshot: Arc<dyn screenshot::Screenshot>,
    pub settings: Arc<dyn settings::Settings>,
    pub inhibit: Arc<dyn inhibit::Inhibit>,
}

/// Serves every interface on `conn` and takes the backend's name; `false` when another
/// process owns it.
pub async fn serve(conn: &Connection, handlers: Handlers) -> zbus::Result<bool> {
    let server = conn.object_server();
    server
        .at(PATH, access::AccessIface::new(handlers.access))
        .await?;
    server
        .at(PATH, screencast::ScreenCastIface::new(handlers.screencast))
        .await?;
    server
        .at(PATH, screenshot::ScreenshotIface::new(handlers.screenshot))
        .await?;
    let settings = settings::SettingsIface::new(handlers.settings);
    server.at(PATH, settings).await?;
    server
        .at(PATH, inhibit::InhibitIface::new(handlers.inhibit))
        .await?;
    match conn
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(true),
        Ok(_) | Err(zbus::Error::NameTaken) => Ok(false),
        Err(err) => Err(err),
    }
}

/// The frontend reads a backend's version and cursor modes once, at its start; restarting it
/// once Isle owns the name makes it read Isle's. A frontend not running is left alone.
pub async fn restart_frontend(conn: &Connection) {
    let reply = conn
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "TryRestartUnit",
            &("xdg-desktop-portal.service", "replace"),
        )
        .await;
    if let Err(err) = reply {
        tracing::debug!("the portal frontend was not restarted: {err}");
    }
}
