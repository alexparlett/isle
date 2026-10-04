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
/// process ends.
pub fn spawn(to_loop: ToLoop, events: broadcast::Sender<Event>, store: SharedStore) {
    thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("worker-task")
                .enable_all()
                .build()
                .expect("the worker's runtime");
            let (loop_w, events_w, store_w) = (to_loop.clone(), events.clone(), store.clone());
            runtime.spawn(supervise("settings-watch", move || {
                crate::settings::watch(store_w.clone(), loop_w.clone(), events_w.clone())
            }));
            let loop_t = to_loop.clone();
            runtime.spawn(supervise("tokens-watch", move || {
                crate::tokens::watch(loop_t.clone())
            }));
            runtime.block_on(supervise("dbus", move || {
                crate::dbus::serve(to_loop.clone(), events.clone(), store.clone())
            }));
        })
        .expect("the worker thread");
}

/// Runs a service, restarting it with a backoff when it fails or panics.
async fn supervise<F, Fut>(name: &'static str, mut start: F)
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
