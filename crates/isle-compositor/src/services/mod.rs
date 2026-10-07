//! The system's daemons as the desktop knows them: each service's last snapshot, taken in on
//! the loop, where the island's glyphs, Control Centre and the OSDs read them
//! (`p8-services.md` choice 2).

pub mod audio;
pub mod brightness;
pub mod login;
pub mod power;

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

#[derive(Default)]
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
    /// When Control Centre last changed a level itself.
    pub own_change: Option<std::time::Instant>,
    /// The last failed action of a module or row, by `module` or `module/row`, shown in its
    /// place until the next action there.
    pub errors: std::collections::HashMap<String, String>,
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
        self.audio_privacy();
        self.refresh_island();
    }
}
