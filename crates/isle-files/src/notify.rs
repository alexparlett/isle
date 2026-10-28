//! A job's end told by a notification while no Files window is in front
//! (`p1/structure/files.md` § Copying and moving, The end).

use std::{collections::HashMap, path::PathBuf};

use futures_util::StreamExt;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.FileManager1",
    default_service = "org.freedesktop.FileManager1",
    default_path = "/org/freedesktop/FileManager1"
)]
trait FileManager {
    fn show_folders(&self, uris: &[&str], startup_id: &str) -> zbus::Result<()>;
}

/// Tells of a job finished unseen; Show opens `folder` in Files.
pub async fn finished(bus: zbus::Connection, summary: String, folder: PathBuf) {
    let Ok(notifications) = NotificationsProxy::new(&bus).await else { return };
    let (Ok(invoked), Ok(closed)) = (notifications.receive_action_invoked().await, notifications.receive_notification_closed().await) else {
        return;
    };
    // Show pressed, or the notification gone: whichever comes first ends the wait.
    let mut heard = futures_util::stream::select(
        invoked.map(|s| s.args().ok().map(|a| (a.id, true))),
        closed.map(|s| s.args().ok().map(|a| (a.id, false))),
    );
    let mut hints = HashMap::new();
    hints.insert("desktop-entry", zbus::zvariant::Value::from(crate::prefs::APP));
    let id = match notifications
        .notify("Files", 0, "system-file-manager", &summary, "", &["default", "Show", "show", "Show"], hints, -1)
        .await
    {
        Ok(id) => id,
        Err(err) => {
            tracing::warn!("no notification for a job's end: {err}");
            return;
        }
    };
    while let Some(event) = heard.next().await {
        let Some((of, shown)) = event else { continue };
        if of != id {
            continue;
        }
        if shown && let Ok(files) = FileManagerProxy::new(&bus).await {
            let uri = isle_thumbnail::uri(&folder);
            let _ = files.show_folders(&[uri.as_str()], "").await;
        }
        return;
    }
}
