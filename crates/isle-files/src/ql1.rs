//! `io.github.alexparlett.Isle.QuickLook1` on Files' name: Spotlight and the capture thumbnail
//! show files in Quick Look and follow which one it shows (`p15-files.md` choice 26).

use std::sync::atomic::{AtomicBool, Ordering};

use isle_app::{Request, Requester};
use serde::{Deserialize, Serialize};
use tracing::warn;
use zbus::{interface, object_server::SignalEmitter};

pub const PATH: &str = "/io/github/alexparlett/Isle/QuickLook1";
pub const SHOW: &str = "quick-look-show";
pub const SELECT: &str = "quick-look-select";
pub const CLOSE: &str = "quick-look-close";

/// Whether Quick Look is open, for the `Visible` property.
pub static VISIBLE: AtomicBool = AtomicBool::new(false);

/// A `Show` call as the app takes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub uris: Vec<String>,
    pub index: u32,
    /// An xdg-foreign handle, or the compositor's `isle:` name for a shell surface.
    pub caller: String,
    /// The item's rectangle in the caller, for the zoom.
    pub origin: (i32, i32, i32, i32),
    /// `none`, `markup` or `trim`.
    pub mode: String,
    pub token: String,
}

struct QuickLook1 {
    requests: Requester,
}

#[interface(name = "io.github.alexparlett.Isle.QuickLook1")]
impl QuickLook1 {
    #[allow(clippy::too_many_arguments)]
    fn show(&self, uris: Vec<String>, index: u32, caller: String, origin: (i32, i32, i32, i32), mode: String, token: String) {
        let show = Show { uris, index, caller, origin, mode, token };
        let Ok(json) = serde_json::to_string(&show) else { return };
        let token = Some(show.token).filter(|t| !t.is_empty());
        self.requests.send(Request::Action { name: SHOW.into(), params: vec![json], token });
    }

    fn select(&self, index: u32) {
        self.requests.send(Request::Action { name: SELECT.into(), params: vec![index.to_string()], token: None });
    }

    fn close(&self) {
        self.requests.send(Request::Action { name: CLOSE.into(), params: Vec::new(), token: None });
    }

    #[zbus(property)]
    fn visible(&self) -> bool {
        VISIBLE.load(Ordering::Relaxed)
    }

    #[zbus(signal)]
    async fn shown(emitter: &SignalEmitter<'_>, index: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn closed(emitter: &SignalEmitter<'_>, index: u32) -> zbus::Result<()>;
}

pub async fn serve(bus: zbus::Connection, requests: Requester) {
    if let Err(err) = bus.object_server().at(PATH, QuickLook1 { requests }).await {
        warn!("QuickLook1 could not be served: {err}");
    }
}

/// Tells the caller which item shows, or that Quick Look closed showing it.
pub async fn tell(bus: zbus::Connection, index: u32, closed: bool) {
    let Ok(emitter) = SignalEmitter::new(&bus, PATH) else { return };
    let sent = if closed { QuickLook1::closed(&emitter, index).await } else { QuickLook1::shown(&emitter, index).await };
    if let Err(err) = sent {
        warn!("Quick Look's caller was not told: {err}");
    }
}
