//! `com.canonical.AppMenu.Registrar`: apps on X11, and Electron on Wayland, register a
//! window's menu here by window id. Owned before the first client, because Qt asks once per
//! process whether the name has an owner (`p6-island-and-menus.md` choice 15).

use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use tracing::{info, warn};
use zbus::{
    fdo,
    fdo::{RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    zvariant::{ObjectPath, OwnedObjectPath},
};

pub const NAME: &str = "com.canonical.AppMenu.Registrar";
pub const PATH: &str = "/com/canonical/AppMenu/Registrar";

/// One registration: a window id, the connection that made it, and its menu's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    pub window: u32,
    pub sender: String,
    pub path: String,
}

/// The registrations, shared with whoever reads them.
#[derive(Clone, Default)]
pub struct Registrations(Arc<Mutex<Vec<Registration>>>, Arc<tokio::sync::Notify>);

impl Registrations {
    /// Returns once a window has registered since the last return.
    pub async fn registered(&self) {
        self.1.notified().await;
    }

    pub fn all(&self) -> Vec<Registration> {
        self.0.lock().unwrap().clone()
    }

    pub fn for_window(&self, window: u32) -> Option<Registration> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.window == window)
            .cloned()
    }

    fn insert(&self, registration: Registration) {
        let mut all = self.0.lock().unwrap();
        all.retain(|r| r.window != registration.window);
        all.push(registration);
        self.1.notify_one();
    }

    fn drop_sender(&self, sender: &str) {
        self.0.lock().unwrap().retain(|r| r.sender != sender);
    }
}

struct Registrar {
    registrations: Registrations,
}

#[interface(name = "com.canonical.AppMenu.Registrar")]
impl Registrar {
    async fn register_window(
        &self,
        #[zbus(header)] header: Header<'_>,
        window_id: u32,
        menu_object_path: ObjectPath<'_>,
    ) {
        let Some(sender) = header.sender() else {
            return;
        };
        info!(window_id, sender = %sender, path = %menu_object_path, "menu registered");
        self.registrations.insert(Registration {
            window: window_id,
            sender: sender.to_string(),
            path: menu_object_path.to_string(),
        });
    }

    async fn unregister_window(&self, window_id: u32) {
        self.registrations
            .0
            .lock()
            .unwrap()
            .retain(|r| r.window != window_id);
    }

    async fn get_menu_for_window(&self, window_id: u32) -> fdo::Result<(String, OwnedObjectPath)> {
        let found = self
            .registrations
            .for_window(window_id)
            .ok_or_else(|| fdo::Error::Failed(format!("no menu for window {window_id}")))?;
        let path =
            OwnedObjectPath::try_from(found.path).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok((found.sender, path))
    }

    async fn get_menus(&self) -> Vec<(u32, String, OwnedObjectPath)> {
        self.registrations
            .all()
            .into_iter()
            .filter_map(|r| Some((r.window, r.sender, OwnedObjectPath::try_from(r.path).ok()?)))
            .collect()
    }
}

/// Who owns the name when Isle could not take it.
#[derive(Debug)]
pub struct Taken(pub String);

/// Serves the registrar on `conn` and asks for its name without queueing or allowing
/// replacement, so whoever owns it first keeps it. Registrations of a connection that
/// leaves the bus go with it.
pub async fn own(conn: &zbus::Connection) -> zbus::Result<Result<Registrations, Taken>> {
    let registrations = Registrations::default();
    conn.object_server()
        .at(
            PATH,
            Registrar {
                registrations: registrations.clone(),
            },
        )
        .await?;
    let reply = conn
        .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
        .await;
    match reply {
        Ok(RequestNameReply::PrimaryOwner) | Ok(RequestNameReply::AlreadyOwner) => {}
        Ok(_) | Err(zbus::Error::NameTaken) => {
            let dbus = fdo::DBusProxy::new(conn).await?;
            let owner = dbus
                .get_name_owner(NAME.try_into().unwrap())
                .await
                .map(|o| o.to_string())
                .unwrap_or_default();
            warn!(owner, "another program owns the menu registrar");
            return Ok(Err(Taken(owner)));
        }
        Err(err) => return Err(err),
    }
    let dbus = fdo::DBusProxy::new(conn).await?;
    let mut changes = dbus.receive_name_owner_changed().await?;
    let watched = registrations.clone();
    tokio::spawn(async move {
        while let Some(change) = changes.next().await {
            let Ok(args) = change.args() else { continue };
            if args.new_owner().is_none() {
                watched.drop_sender(args.name());
            }
        }
    });
    Ok(Ok(registrations))
}

/// The process behind a bus connection, for matching a registration to a window.
pub async fn pid_of(conn: &zbus::Connection, sender: &str) -> Option<u32> {
    let dbus = fdo::DBusProxy::new(conn).await.ok()?;
    let name = zbus::names::BusName::try_from(sender).ok()?;
    dbus.get_connection_unix_process_id(name).await.ok()
}
