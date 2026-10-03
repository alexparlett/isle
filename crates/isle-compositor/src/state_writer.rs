//! State written from the worker, debounced, so no write runs on the main loop.

use std::{sync::Arc, time::Duration};

use isle_state::{Domain, StateDir};
use tokio::{runtime::Handle, sync::watch};
use tracing::warn;

/// How long a domain's writes wait for more changes.
pub const DEBOUNCE: Duration = Duration::from_millis(500);

/// Takes a domain's latest value from any thread and writes the last of each burst.
pub struct Saver<D> {
    tx: watch::Sender<Option<D>>,
}

impl<D: Domain + Clone + Send + Sync + 'static> Saver<D> {
    pub fn spawn(runtime: &Handle, dir: StateDir) -> Self {
        Self::spawn_with(runtime, DEBOUNCE, move |value: &D| {
            if let Err(err) = dir.save(value) {
                warn!(domain = D::NAME, "could not write state: {err}");
            }
        })
    }

    fn spawn_with(
        runtime: &Handle,
        delay: Duration,
        write: impl Fn(&D) + Send + Sync + 'static,
    ) -> Self {
        let (tx, mut rx) = watch::channel(None::<D>);
        let write = Arc::new(write);
        runtime.spawn(async move {
            while rx.changed().await.is_ok() {
                tokio::time::sleep(delay).await;
                let Some(value) = rx.borrow_and_update().clone() else {
                    continue;
                };
                let write = write.clone();
                let _ = tokio::task::spawn_blocking(move || write(&value)).await;
            }
        });
        Self { tx }
    }

    pub fn save(&self, value: D) {
        self.tx.send_replace(Some(value));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use isle_state::domains::Crashes;

    use super::*;

    #[test]
    fn a_burst_of_saves_is_one_write_of_the_last_value() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let seen = writes.clone();
        let saver = Saver::spawn_with(
            runtime.handle(),
            Duration::from_millis(100),
            move |c: &Crashes| {
                seen.lock().unwrap().push(c.quick_deaths.clone());
            },
        );
        for n in 0..50 {
            saver.save(Crashes {
                quick_deaths: vec![n],
            });
        }
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(*writes.lock().unwrap(), vec![vec![49]]);

        saver.save(Crashes {
            quick_deaths: vec![50],
        });
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(writes.lock().unwrap().len(), 2);
    }
}
