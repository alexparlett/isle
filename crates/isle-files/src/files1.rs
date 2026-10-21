//! `io.github.alexparlett.Isle.Files1`: the portal backend hands Files each FileChooser
//! request and hears its one answer as a signal (`p15-files.md` choice 21).

use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use isle_app::{Request, Requester};
use tracing::{info, warn};
use zbus::{interface, message::Header, object_server::SignalEmitter};
use zvariant::OwnedValue;

use crate::chooser::{self, Answer, Kind};

pub const PATH: &str = "/io/github/alexparlett/Isle/Files1";
/// The portal backend's name; only its owner may ask.
pub const PORTAL: &str = "org.freedesktop.impl.portal.desktop.isle";

pub const CHOOSE: &str = "choose";
pub const CANCEL: &str = "cancel-choose";

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Files1 {
    requests: Requester,
}

/// Whether the caller owns the portal backend's name.
async fn from_portal(bus: &zbus::Connection, header: &Header<'_>) -> bool {
    let Some(sender) = header.sender() else { return false };
    let Ok(dbus) = zbus::fdo::DBusProxy::new(bus).await else { return false };
    let Ok(name) = PORTAL.try_into() else { return false };
    dbus.get_name_owner(name).await.is_ok_and(|owner| owner.as_str() == sender.as_str())
}

#[interface(name = "io.github.alexparlett.Isle.Files1")]
impl Files1 {
    /// Shows the panel for a request; its id, which `Answered` names.
    async fn choose(
        &self,
        kind: String,
        app_id: String,
        parent: String,
        title: String,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] bus: &zbus::Connection,
    ) -> zbus::fdo::Result<u64> {
        if !from_portal(bus, &header).await {
            return Err(zbus::fdo::Error::AccessDenied("only the portal backend asks Files".into()));
        }
        let kind = Kind::from_name(&kind).ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("no kind {kind}")))?;
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let ask = chooser::ask_of(id, kind, &app_id, &parent, &title, &options);
        let json = serde_json::to_string(&ask).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        info!(id, app = app_id, folder = ?ask.current_folder, kind = ?ask.kind, "choose");
        self.requests.send(Request::Action { name: CHOOSE.into(), params: vec![json], token: None });
        Ok(id)
    }

    /// The request was closed by the app: its panel goes, answering cancel.
    async fn cancel(
        &self,
        id: u64,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] bus: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        if !from_portal(bus, &header).await {
            return Err(zbus::fdo::Error::AccessDenied("only the portal backend asks Files".into()));
        }
        self.requests.send(Request::Action { name: CANCEL.into(), params: vec![id.to_string()], token: None });
        Ok(())
    }

    #[zbus(signal)]
    async fn answered(emitter: &SignalEmitter<'_>, id: u64, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
}

pub async fn serve(bus: zbus::Connection, requests: Requester) {
    if let Err(err) = bus.object_server().at(PATH, Files1 { requests }).await {
        warn!("Files1 could not be served: {err}");
    }
}

/// Answers a request, once.
pub async fn answer(bus: zbus::Connection, id: u64, answer: Answer) {
    let Ok(emitter) = SignalEmitter::new(&bus, PATH) else { return };
    info!(id, response = answer.response, "answered");
    if let Err(err) = Files1::answered(&emitter, id, answer.response, answer.results()).await {
        warn!(id, "the answer was not sent: {err}");
    }
}
