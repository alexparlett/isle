//! The main loop's heartbeat and the thread that watches it (B9).

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use tracing::{error, warn};

/// How many recent stalls a crash record carries.
const RECENT: usize = 20;
/// A dispatch running this long is reported while it still runs.
const STUCK: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct Stall {
    pub source: &'static str,
    pub took: Duration,
    pub at: SystemTime,
}

#[derive(Clone)]
pub struct Heartbeat {
    shared: Arc<Shared>,
}

struct Shared {
    epoch: Instant,
    /// Nanoseconds after `epoch` at which the current dispatch began; 0 while idle.
    began: AtomicU64,
    depth: AtomicU32,
    source: Mutex<&'static str>,
    budget_ns: AtomicU64,
    recent: Mutex<VecDeque<Stall>>,
}

/// Marks the main loop busy with `source` until dropped.
pub struct Beat {
    shared: Arc<Shared>,
    outermost: bool,
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self::new()
    }
}

impl Heartbeat {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                epoch: Instant::now(),
                began: AtomicU64::new(0),
                depth: AtomicU32::new(0),
                source: Mutex::new(""),
                budget_ns: AtomicU64::new(Duration::from_millis(16).as_nanos() as u64),
                recent: Mutex::new(VecDeque::new()),
            }),
        }
    }

    /// The longest a dispatch may take before it is logged: the fastest output's frame.
    pub fn set_budget(&self, budget: Duration) {
        self.shared
            .budget_ns
            .store(budget.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn beat(&self, source: &'static str) -> Beat {
        let shared = self.shared.clone();
        let outermost = shared.depth.fetch_add(1, Ordering::AcqRel) == 0;
        if outermost {
            *shared.source.lock().unwrap() = source;
            shared
                .began
                .store(shared.now_ns().max(1), Ordering::Release);
        }
        Beat { shared, outermost }
    }

    pub fn recent_stalls(&self) -> Vec<Stall> {
        self.shared
            .recent
            .try_lock()
            .map(|r| r.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Starts the watchdog thread. Called on the main thread, which it signals on a hang.
    /// A dispatch stuck for `abort_after` writes a crash record and aborts the main thread
    /// so its stack is in the core dump.
    pub fn spawn_watchdog(&self, abort_after: Option<Duration>) {
        let shared = self.shared.clone();
        let main = unsafe { libc::pthread_self() };
        let heartbeat = self.clone();
        thread::Builder::new()
            .name("watchdog".into())
            .spawn(move || {
                let mut reported = 0;
                loop {
                    thread::sleep(Duration::from_millis(250));
                    let began = shared.began.load(Ordering::Acquire);
                    if began == 0 {
                        continue;
                    }
                    let age = Duration::from_nanos(shared.now_ns().saturating_sub(began));
                    let source = *shared.source.lock().unwrap();
                    if age >= STUCK && reported != began {
                        reported = began;
                        warn!(source, "main loop stuck for {} ms", age.as_millis());
                    }
                    if let Some(limit) = abort_after
                        && age >= limit
                    {
                        error!(source, "main loop hung for {} s, aborting", age.as_secs());
                        let message = format!("main loop hung in {source} for {} s", age.as_secs());
                        let _ = crate::crash::write(
                            "hang",
                            "main",
                            &message,
                            "in the core dump (coredumpctl)",
                            &heartbeat.recent_stalls(),
                        );
                        unsafe { libc::pthread_kill(main, libc::SIGABRT) };
                        thread::sleep(Duration::from_secs(2));
                        std::process::abort();
                    }
                }
            })
            .expect("watchdog thread");
    }
}

impl Shared {
    fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }
}

impl Drop for Beat {
    fn drop(&mut self) {
        let shared = &self.shared;
        shared.depth.fetch_sub(1, Ordering::AcqRel);
        if !self.outermost {
            return;
        }
        let began = shared.began.swap(0, Ordering::AcqRel);
        let took = Duration::from_nanos(shared.now_ns().saturating_sub(began));
        if took.as_nanos() as u64 <= shared.budget_ns.load(Ordering::Relaxed) {
            return;
        }
        let source = *shared.source.lock().unwrap();
        warn!(
            source,
            "main loop stalled for {:.1} ms",
            took.as_secs_f64() * 1000.0
        );
        let mut recent = shared.recent.lock().unwrap();
        if recent.len() == RECENT {
            recent.pop_front();
        }
        recent.push_back(Stall {
            source,
            took,
            at: SystemTime::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dispatch_over_budget_is_recorded_with_its_source() {
        let heartbeat = Heartbeat::new();
        heartbeat.set_budget(Duration::from_millis(5));
        {
            let _quick = heartbeat.beat("quick");
        }
        {
            let _slow = heartbeat.beat("slow");
            let _inner = heartbeat.beat("inner");
            thread::sleep(Duration::from_millis(20));
        }
        let stalls = heartbeat.recent_stalls();
        assert_eq!(stalls.len(), 1);
        assert_eq!(stalls[0].source, "slow");
        assert!(stalls[0].took >= Duration::from_millis(20));
    }
}
