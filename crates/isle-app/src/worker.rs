//! The app's worker: a tokio runtime on a thread of its own for D-Bus and the daemons, so the
//! UI thread never waits on them (`apps.md` Conventions 22).

use std::{future::Future, sync::Arc};

use tokio::{runtime::Runtime, task::JoinHandle};

#[derive(Clone)]
pub struct Worker {
    runtime: Arc<Runtime>,
}

impl Worker {
    pub fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("isle-app-worker")
            .enable_all()
            .build()
            .expect("a tokio runtime");
        Self {
            runtime: Arc::new(runtime),
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// Runs `future` to its end on the worker; only before the window exists.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}
