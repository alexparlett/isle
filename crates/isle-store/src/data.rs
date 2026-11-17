//! Store's worker: follows `Updates1`, reads the installed apps, and carries the window's asks
//! to the compositor.

use std::collections::HashSet;

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use tracing::warn;

use crate::installed::{self, App};

pub enum Event {
    /// `Updates1.Status`, read again on every change.
    Status(serde_json::Value),
    Installed(Vec<App>),
    /// Why the last ask was refused or failed.
    Failed(String),
}

pub enum Ask {
    /// A run of a part: `flathub`, `flathub:<id>`, `aur`, `aur:<package>`.
    Install(String),
    Check,
    Stop,
    Launch(String),
    /// Settings at Software Update, where the system updates.
    SoftwareUpdate,
    Rescan,
}

#[derive(Clone)]
pub struct Data(UnboundedSender<Ask>);

impl PartialEq for Data {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Data {
    pub fn ask(&self, ask: Ask) {
        let _ = self.0.unbounded_send(ask);
    }
}

pub fn start(
    worker: &isle_app::Worker,
    bus: Option<zbus::Connection>,
) -> (Data, UnboundedReceiver<Event>) {
    let (asks, asked) = unbounded();
    let (events, rx) = unbounded();
    worker.spawn(run(asked, events, bus));
    (Data(asks), rx)
}

fn scan() -> Vec<App> {
    let index = isle_apps::Index::scan(&isle_apps::Index::data_dirs());
    let mut icons = isle_apps::icons::Icons::papirus(true);
    let owners = installed::owners(std::path::Path::new("/"));
    let foreign: HashSet<String> = std::process::Command::new("pacman")
        .args(["-Qmq"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    installed::classify(index.entries(), &owners, &foreign, &mut |name| {
        icons.lookup(name, 64)
    })
}

async fn follow(bus: zbus::Connection, events: UnboundedSender<Event>) -> zbus::Result<()> {
    let proxy = isle_ipc::updates::UpdatesProxy::new(&bus).await?;
    let mut changed = proxy.receive_changed().await?;
    loop {
        let status = serde_json::from_str(&proxy.status().await?).unwrap_or_default();
        if events.unbounded_send(Event::Status(status)).is_err() {
            return Ok(());
        }
        if changed.next().await.is_none() {
            return Ok(());
        }
    }
}

async fn act(bus: &zbus::Connection, ask: Ask) -> Result<(), String> {
    let err = |e: zbus::Error| e.to_string();
    let updates = isle_ipc::updates::UpdatesProxy::new(bus)
        .await
        .map_err(err)?;
    let why = match ask {
        Ask::Install(part) => updates.install(&part).await.map_err(err)?,
        Ask::Check => updates.check().await.map(|()| String::new()).map_err(err)?,
        Ask::Stop => updates.stop().await.map_err(err)?,
        Ask::Launch(id) => {
            let windows = isle_ipc::windows::WindowsProxy::new(bus)
                .await
                .map_err(err)?;
            windows.launch(&id, "", &[]).await.map_err(err)?;
            String::new()
        }
        Ask::SoftwareUpdate => {
            let windows = isle_ipc::windows::WindowsProxy::new(bus)
                .await
                .map_err(err)?;
            windows
                .launch(
                    "io.github.alexparlett.Isle.Settings",
                    "",
                    &["isle-settings://general/updates-status"],
                )
                .await
                .map_err(err)?;
            String::new()
        }
        Ask::Rescan => String::new(),
    };
    if why.is_empty() { Ok(()) } else { Err(why) }
}

async fn run(
    mut asked: UnboundedReceiver<Ask>,
    events: UnboundedSender<Event>,
    bus: Option<zbus::Connection>,
) {
    let installed = tokio::task::spawn_blocking(scan).await.unwrap_or_default();
    let _ = events.unbounded_send(Event::Installed(installed));
    let Some(bus) = bus else {
        let _ = events.unbounded_send(Event::Failed("Isle is not running on this session.".into()));
        return;
    };
    {
        let (bus, events) = (bus.clone(), events.clone());
        tokio::spawn(async move {
            if let Err(e) = follow(bus, events).await {
                warn!("updates are not followed: {e}");
            }
        });
    }
    while let Some(ask) = asked.next().await {
        if matches!(ask, Ask::Rescan) {
            let installed = tokio::task::spawn_blocking(scan).await.unwrap_or_default();
            let _ = events.unbounded_send(Event::Installed(installed));
            continue;
        }
        if let Err(why) = act(&bus, ask).await {
            let _ = events.unbounded_send(Event::Failed(why));
        }
    }
}
