//! The system's daemons as the desktop knows them: each service's last snapshot, taken in on
//! the loop, where the island's glyphs, Control Centre and the OSDs read them
//! (`p8-services.md` choice 2).

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod login;
pub mod media;
pub mod network;
pub mod power;
pub mod tray;

use std::{collections::BTreeMap, error::Error, time::Duration};

use isle_bus::{Publisher, Snapshot};
use serde::Serialize;
use tokio::sync::watch;

use crate::{
    state::{Isle, State},
    worker::ToLoop,
};

/// The least time between two snapshots of one service reaching the loop, about a frame.
const FRAME: Duration = Duration::from_millis(8);

pub struct Services {
    pub login: Snapshot<login::Login>,
    pub power: Snapshot<isle_power::Power>,
    pub profiles: Snapshot<isle_power::Profiles>,
    pub audio: Snapshot<isle_audio::Audio>,
    pub cameras: Snapshot<isle_audio::camera::Cameras>,
    /// The default input's level while Sound's page measures it.
    pub input_level: f32,
    pub meter: Option<tokio::task::JoinHandle<()>>,
    pub screens: brightness::Screens,
    pub disks: Snapshot<isle_disks::Disks>,
    pub media: Snapshot<isle_media::Media>,
    /// When the media snapshot was taken in, from which a playing track's position runs on.
    pub media_read: std::time::Instant,
    /// Art by its URL: a local file, or none yet or none to be had.
    pub art: std::collections::HashMap<String, Option<std::path::PathBuf>>,
    /// A player chosen on Now Playing's page, by bus name.
    pub chosen_player: Option<String>,
    pub media_clock: Option<tokio::task::JoinHandle<()>>,
    pub bluetooth: Snapshot<isle_bluetooth::Bluetooth>,
    /// Devices being connected or paired, by path.
    pub connecting: std::collections::HashSet<String>,
    pub discovering: bool,
    pub network: Snapshot<isle_network::State>,
    /// The network being joined: its name and access point.
    pub joining: Option<(String, String)>,
    pub scan: Option<tokio::task::JoinHandle<()>>,
    /// The VPN last connected, which VPN's switch connects again.
    pub last_vpn: Option<String>,
    /// Folded groups of rows opened, by `module/row`.
    pub expanded: std::collections::HashSet<String>,
    /// When Control Centre last changed a level itself.
    pub own_change: Option<std::time::Instant>,
    /// The last failed action of a module or row, by `module` or `module/row`, shown in its
    /// place until the next action there.
    pub errors: std::collections::HashMap<String, String>,
    pub tray: Snapshot<isle_tray::Tray>,
    pub tray_menu: Option<tray::TrayMenu>,
    pub tray_icons: std::cell::RefCell<Option<(bool, isle_apps::icons::Icons)>>,
    /// Drops the tray's watcher as a crash would.
    pub tray_kill: Option<std::sync::Arc<tokio::sync::Notify>>,
    /// Snapshots taken in, by service.
    pub applied: BTreeMap<&'static str, u64>,
}

impl Services {
    /// Every service's availability and state, with how many snapshots it has delivered.
    pub fn status(&self) -> String {
        fn entry<T: Serialize>(s: &Snapshot<T>, applied: u64) -> serde_json::Value {
            serde_json::json!({ "available": s.available, "state": s.state, "applied": applied })
        }
        let applied = |name| self.applied.get(name).copied().unwrap_or(0);
        serde_json::json!({
            "login": entry(&self.login, applied("login")),
            "power": entry(&self.power, applied("power")),
            "profiles": entry(&self.profiles, applied("profiles")),
            "audio": entry(&self.audio, applied("audio")),
            "cameras": entry(&self.cameras, applied("cameras")),
            "network": entry(&self.network, applied("network")),
            "bluetooth": entry(&self.bluetooth, applied("bluetooth")),
            "media": entry(&self.media, applied("media")),
            "disks": entry(&self.disks, applied("disks")),
            "tray": entry(&self.tray, applied("tray")),
        })
        .to_string()
    }
}

/// Carries a service's snapshots to the loop: the latest of a burst, once a frame at most.
pub async fn forward<T>(
    name: &'static str,
    mut snapshots: watch::Receiver<Snapshot<T>>,
    to_loop: ToLoop,
    apply: fn(&mut State, Snapshot<T>),
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    T: Clone + Send + Sync + 'static,
{
    loop {
        snapshots.changed().await?;
        let snapshot = snapshots.borrow_and_update().clone();
        to_loop.post(move |state| {
            *state.isle.services.applied.entry(name).or_default() += 1;
            apply(state, snapshot);
            state.refresh_lock_face();
        });
        tokio::time::sleep(FRAME).await;
    }
}

/// Starts a service on the worker under the supervisor, and carries its snapshots to the
/// loop through `apply`.
pub fn start<T, W, Fut>(
    runtime: &tokio::runtime::Runtime,
    to_loop: &ToLoop,
    name: &'static str,
    watch: W,
    apply: fn(&mut State, Snapshot<T>),
) where
    T: Default + Clone + PartialEq + Send + Sync + 'static,
    W: Fn(Publisher<T>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), isle_bus::Error>> + Send + 'static,
{
    let (publisher, snapshots) = isle_bus::snapshot::<T>();
    runtime.spawn(crate::worker::supervise(name, move || {
        watch(publisher.clone())
    }));
    let to_loop = to_loop.clone();
    runtime.spawn(crate::worker::supervise(name, move || {
        forward(name, snapshots.clone(), to_loop.clone(), apply)
    }));
}

impl Isle {
    /// Runs a command against a daemon on the worker, then `done` with its result on the loop.
    pub fn run_service<T, Fut>(
        &self,
        command: Fut,
        done: impl FnOnce(&mut State, Result<T, String>) + Send + 'static,
    ) where
        T: Send + 'static,
        Fut: std::future::Future<Output = Result<T, isle_bus::Error>> + Send + 'static,
    {
        let (Some(runtime), Some(to_loop)) = (&self.worker, &self.to_loop) else {
            return;
        };
        let to_loop = to_loop.clone();
        runtime.spawn(async move {
            let result = command.await.map_err(|e| e.to_string());
            to_loop.post(move |state| done(state, result));
        });
    }

    /// A snapshot changed: the glyphs drawn from services, then the island and Control
    /// Centre.
    pub fn services_changed(&mut self) {
        use isle_shell::island::model::GlyphKind;
        let battery = self.battery_glyph();
        self.island.model.set_glyph(GlyphKind::Battery, battery);
        let sound = self.sound_glyph();
        self.island.model.set_glyph(GlyphKind::Sound, sound);
        let wifi = self.wifi_glyph();
        self.island.model.set_glyph(GlyphKind::Wifi, wifi);
        let vpn = self.vpn_glyph();
        self.island.model.set_glyph(GlyphKind::Vpn, vpn);
        let bluetooth = self.bluetooth_glyph();
        self.island.model.set_glyph(GlyphKind::Bluetooth, bluetooth);
        self.audio_privacy();
        self.refresh_island();
    }
}

impl State {
    /// Work a Control Centre page wants while it shows: the input meter, Wi-Fi scans.
    pub fn follow_pages(&mut self) {
        self.follow_meter();
        self.follow_scan();
        self.follow_discovery();
        self.follow_media_clock();
        self.follow_tray_menu();
    }
}

impl Default for Services {
    fn default() -> Self {
        Self {
            login: Default::default(),
            power: Default::default(),
            profiles: Default::default(),
            errors: Default::default(),
            audio: Default::default(),
            cameras: Default::default(),
            input_level: 0.0,
            meter: None,
            own_change: None,
            screens: Default::default(),
            disks: Default::default(),
            media: Default::default(),
            media_read: std::time::Instant::now(),
            art: Default::default(),
            chosen_player: None,
            media_clock: None,
            bluetooth: Default::default(),
            connecting: Default::default(),
            discovering: false,
            network: Default::default(),
            joining: None,
            scan: None,
            last_vpn: None,
            expanded: Default::default(),
            tray: Default::default(),
            tray_menu: None,
            tray_icons: Default::default(),
            tray_kill: None,
            applied: Default::default(),
        }
    }
}
