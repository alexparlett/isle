//! Login Items: the autostart entries started at every compositor start, once Isle's session
//! carries the user manager (`p14a-session.md` choice 17).

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use isle_apps::Entry;
use tracing::info;
use zbus::{proxy::CacheProperties, zvariant::OwnedObjectPath};

use crate::{launch::Launch, launch::unit_part, state::State};

/// How long the supervisor has to export the session and start its target.
const EXPORT_WAIT: Duration = Duration::from_secs(30);

impl State {
    /// Called once the supervisor has the display; `ISLE_LOGIN_ITEMS=0` turns it off for a
    /// session run beside another.
    pub fn start_login_items_when_exported(&mut self) {
        if std::env::var("ISLE_LOGIN_ITEMS").as_deref() == Ok("0") {
            info!("Login Items are off for this run");
            return;
        }
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return;
        };
        let socket = self.isle.socket_name.to_string_lossy().into_owned();
        runtime.spawn(async move {
            if !session_carries_manager(&socket).await {
                info!("the user manager is not this session's; Login Items not started");
                return;
            }
            let items = due().await;
            to_loop.post(move |state| state.start_login_items(items));
        });
    }

    pub fn start_login_items(&mut self, items: Vec<Entry>) {
        for item in items {
            info!(item = item.id, "starting a Login Item");
            self.launch(Launch::Item(Box::new(item)));
        }
    }
}

/// The items to start: every one that applies, but those still running from the compositor
/// before this one.
pub async fn due() -> Vec<Entry> {
    let items = tokio::task::spawn_blocking(|| {
        isle_apps::autostart::items(&isle_apps::autostart::dirs(), "Isle")
    })
    .await
    .unwrap_or_default();
    let bus = zbus::Connection::session().await.ok();
    let slice = app_slice();
    let mut due = Vec::new();
    for item in items {
        let named = match (&bus, item.dbus_activatable) {
            (Some(bus), true) => name_owned(bus, &item.id).await,
            _ => false,
        };
        if !named && !scope_running(&slice, &item.id) {
            due.push(item);
        }
    }
    due
}

/// The user manager's slice that the launch path's scopes go in.
fn app_slice() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!(
        "/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service/app.slice"
    ))
}

/// Whether a scope the launch path started `app` in still has processes.
fn scope_running(slice: &Path, app: &str) -> bool {
    let prefix = format!("app-isle-{}-", unit_part(app));
    std::fs::read_dir(slice)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&prefix)
                && name.ends_with(".scope")
                && crate::processes::populated(&entry.path())
        })
}

async fn name_owned(bus: &zbus::Connection, name: &str) -> bool {
    let Ok(name) = zbus::names::BusName::try_from(name) else {
        return false;
    };
    match zbus::fdo::DBusProxy::new(bus).await {
        Ok(proxy) => proxy.name_has_owner(name).await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Whether the user manager names this session's display and its graphical target is up.
async fn session_carries_manager(socket: &str) -> bool {
    let Ok(bus) = zbus::Connection::session().await else {
        return false;
    };
    let want = format!("WAYLAND_DISPLAY={socket}");
    let deadline = tokio::time::Instant::now() + EXPORT_WAIT;
    while tokio::time::Instant::now() < deadline {
        if manager_carries(&bus, &want).await.unwrap_or(false) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

async fn manager_carries(bus: &zbus::Connection, want: &str) -> zbus::Result<bool> {
    let proxy = |path: OwnedObjectPath, interface: &'static str| {
        zbus::proxy::Builder::<zbus::Proxy>::new(bus)
            .destination("org.freedesktop.systemd1")
            .and_then(|b| b.path(path))
            .and_then(|b| b.interface(interface))
            .map(|b| b.cache_properties(CacheProperties::No))
    };
    let root = OwnedObjectPath::try_from("/org/freedesktop/systemd1").unwrap();
    let manager = proxy(root, "org.freedesktop.systemd1.Manager")?
        .build()
        .await?;
    let environment: Vec<String> = manager.get_property("Environment").await?;
    if !environment.iter().any(|e| e == want) {
        return Ok(false);
    }
    let target: OwnedObjectPath = manager
        .call("GetUnit", &("graphical-session.target",))
        .await?;
    let target = proxy(target, "org.freedesktop.systemd1.Unit")?
        .build()
        .await?;
    Ok(target.get_property::<String>("ActiveState").await? == "active")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_whose_scope_has_processes_is_still_running() {
        let slice = std::env::temp_dir().join(format!("isle-slice-{}", std::process::id()));
        let scope = |name: &str, populated: bool| {
            let dir = slice.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let events = format!("populated {}\nfrozen 0\n", populated as u8);
            std::fs::write(dir.join("cgroup.events"), events).unwrap();
        };
        scope("app-isle-claude_desktop-0000abcd.scope", true);
        scope("app-isle-proton_mail-00001234.scope", false);
        assert!(scope_running(&slice, "claude-desktop"));
        assert!(!scope_running(&slice, "proton-mail"));
        assert!(!scope_running(&slice, "claude"));
        std::fs::remove_dir_all(&slice).unwrap();
    }
}
