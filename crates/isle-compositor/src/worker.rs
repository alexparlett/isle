//! The service worker: an async runtime on its own thread for D-Bus and anything that
//! could block, reaching the main loop only through jobs (B9).

use std::{error::Error, future::Future, thread, time::Duration};

use smithay::reexports::calloop::channel::Sender;
use tokio::sync::{broadcast, oneshot};
use tracing::{error, warn};

use crate::{settings::SharedStore, state::State};

/// How long a request waits for the main loop before it fails.
pub const LOOP_TIMEOUT: Duration = Duration::from_secs(1);

pub type Job = Box<dyn FnOnce(&mut State) + Send>;

/// Changes the worker announces on D-Bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    WindowsChanged,
    DisplaysChanged,
    DesktopsChanged,
    KeyboardsChanged,
    NotificationsChanged,
    FocusChanged,
    /// The session was locked or unlocked.
    LockChanged,
    /// The keys whose values changed.
    SettingsChanged(Vec<String>),
    /// A hand edit was rejected, or a rejected one was fixed.
    SettingsStatus,
    /// Makes the D-Bus service panic, to test its restart.
    #[cfg(feature = "test-hooks")]
    PanicService,
}

#[derive(Debug)]
pub struct LoopTimeout;

impl std::fmt::Display for LoopTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the compositor did not answer within {} ms",
            LOOP_TIMEOUT.as_millis()
        )
    }
}

/// The worker's way into the main loop.
#[derive(Clone)]
pub struct ToLoop(Sender<Job>);

impl ToLoop {
    pub fn new(sender: Sender<Job>) -> Self {
        Self(sender)
    }

    /// Runs `f` on the main loop without waiting for it.
    pub fn post(&self, f: impl FnOnce(&mut State) + Send + 'static) {
        let _ = self.0.send(Box::new(f));
    }

    /// Runs `f` on the main loop and returns its result, or fails after `LOOP_TIMEOUT`.
    pub async fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut State) -> T + Send + 'static,
    ) -> Result<T, LoopTimeout> {
        let (tx, rx) = oneshot::channel();
        self.post(move |state| {
            let _ = tx.send(f(state));
        });
        match tokio::time::timeout(LOOP_TIMEOUT, rx).await {
            Ok(Ok(value)) => Ok(value),
            _ => Err(LoopTimeout),
        }
    }
}

/// Starts the worker thread, which serves D-Bus and watches the settings until the
/// process ends. Returns its runtime, for state writes from the loop.
pub fn spawn(
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
    store: SharedStore,
) -> tokio::runtime::Handle {
    let (handle_tx, handle_rx) = std::sync::mpsc::channel();
    thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("worker-task")
                .enable_all()
                .build()
                .expect("the worker's runtime");
            let _ = handle_tx.send(runtime.handle().clone());
            let (loop_w, events_w, store_w) = (to_loop.clone(), events.clone(), store.clone());
            runtime.spawn(supervise("settings-watch", move || {
                crate::settings::watch(store_w.clone(), loop_w.clone(), events_w.clone())
            }));
            let loop_t = to_loop.clone();
            runtime.spawn(supervise("tokens-watch", move || {
                crate::tokens::watch(loop_t.clone())
            }));
            let loop_k = to_loop.clone();
            runtime.spawn(supervise("translation-watch", move || {
                crate::keyboard::watch_tables(loop_k.clone())
            }));
            let loop_g = to_loop.clone();
            runtime.spawn(supervise("gamemode-watch", move || {
                crate::keyboard::games::watch_gamemode(loop_g.clone())
            }));
            let loop_b = to_loop.clone();
            runtime.spawn(supervise("launcher-entry", move || {
                crate::badges::watch(loop_b.clone())
            }));
            let loop_s = to_loop.clone();
            runtime.spawn(supervise("games-watch", move || {
                crate::games::watch(loop_s.clone())
            }));
            let loop_a = to_loop.clone();
            runtime.spawn(supervise("apps-watch", move || {
                crate::wm::watch_apps(loop_a.clone())
            }));
            use crate::services::start;
            start(&runtime, &to_loop, "login", crate::services::login::watch, State::login_changed);
            let (may_sleep, may_sleep_rx) = tokio::sync::watch::channel(false);
            to_loop.post(move |state| state.isle.may_sleep = Some(may_sleep));
            let loop_l = to_loop.clone();
            runtime.spawn(supervise("login-hold", move || {
                crate::services::login::hold(loop_l.clone(), may_sleep_rx.clone())
            }));
            start(&runtime, &to_loop, "power", isle_power::watch_batteries, State::power_changed);
            start(&runtime, &to_loop, "profiles", isle_power::watch_profiles, State::profiles_changed);
            start(&runtime, &to_loop, "audio", isle_audio::watch, State::audio_changed);
            start(&runtime, &to_loop, "network", isle_network::watch, State::network_changed);
            start(&runtime, &to_loop, "bluetooth", isle_bluetooth::watch, State::bluetooth_changed);
            start(&runtime, &to_loop, "media", isle_media::watch, State::media_changed);
            start(&runtime, &to_loop, "disks", isle_disks::watch, State::disks_changed);
            start(&runtime, &to_loop, "cameras", isle_audio::camera::watch, State::cameras_changed);
            let loop_p = to_loop.clone();
            runtime.spawn(supervise("polkit-agent", move || {
                crate::polkit::run(crate::prompts::PromptClient(loop_p.clone()))
            }));
            let loop_bt = to_loop.clone();
            runtime.spawn(supervise("bluetooth-agent", move || {
                crate::bluetooth_agent::run(crate::prompts::PromptClient(loop_bt.clone()))
            }));
            let loop_n = to_loop.clone();
            runtime.spawn(supervise("secret-agent", move || {
                crate::network_secrets::run(crate::prompts::PromptClient(loop_n.clone()))
            }));
            runtime.block_on(supervise("dbus", move || {
                crate::dbus::serve(to_loop.clone(), events.clone(), store.clone())
            }));
        })
        .expect("the worker thread");
    handle_rx.recv().expect("the worker's runtime handle")
}

/// Runs a service, restarting it with a backoff when it fails or panics.
pub async fn supervise<F, Fut>(name: &'static str, mut start: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send + 'static,
{
    let mut backoff = Duration::from_secs(1);
    loop {
        match tokio::spawn(start()).await {
            Ok(Ok(())) => return,
            Ok(Err(err)) => warn!(service = name, "service stopped: {err}"),
            Err(err) if err.is_panic() => error!(service = name, "service panicked"),
            Err(_) => return,
        }
        warn!(service = name, "restarting in {} s", backoff.as_secs());
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}
