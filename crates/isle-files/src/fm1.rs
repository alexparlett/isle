//! `org.freedesktop.FileManager1`: other apps' Show in folder (`files-2026.md` §2). Each
//! call becomes a request to the app, its startup id the activation token.

use isle_app::{Request, Requester};
use tracing::{info, warn};
use zbus::interface;

pub const SHOW_ITEMS: &str = "show-items";
pub const SHOW_FOLDERS: &str = "show-folders";
pub const SHOW_PROPERTIES: &str = "show-properties";

const NAME: &str = "org.freedesktop.FileManager1";
const PATH: &str = "/org/freedesktop/FileManager1";

struct FileManager {
    requests: Requester,
}

impl FileManager {
    fn send(&self, name: &str, uris: Vec<String>, startup_id: String) {
        self.requests.send(Request::Action {
            name: name.to_owned(),
            params: uris,
            token: Some(startup_id).filter(|t| !t.is_empty()),
        });
    }
}

#[interface(name = "org.freedesktop.FileManager1")]
impl FileManager {
    fn show_folders(&self, uris: Vec<String>, startup_id: String) {
        self.send(SHOW_FOLDERS, uris, startup_id);
    }

    fn show_items(&self, uris: Vec<String>, startup_id: String) {
        self.send(SHOW_ITEMS, uris, startup_id);
    }

    fn show_item_properties(&self, uris: Vec<String>, startup_id: String) {
        self.send(SHOW_PROPERTIES, uris, startup_id);
    }
}

/// Serves the interface and takes its name, replacing no other owner.
pub async fn serve(bus: zbus::Connection, requests: Requester) {
    if let Err(err) = bus.object_server().at(PATH, FileManager { requests }).await {
        warn!("FileManager1 could not be served: {err}");
        return;
    }
    match bus
        .request_name_with_flags(NAME, zbus::fdo::RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(_) => info!("serving FileManager1"),
        Err(err) => warn!("FileManager1 is another app's: {err}"),
    }
}
