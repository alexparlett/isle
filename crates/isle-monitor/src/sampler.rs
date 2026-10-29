//! Sampling on the worker while a window is shown (`p19a-monitor.md` choices 3 and 14): each
//! sample on a blocking thread, then the window list and the holds from the bus. Not shown,
//! the loop waits on its control and wakes for nothing.

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use isle_sysinfo::monitor::{Full, MonitorSampler};
use tokio::sync::watch;
use tracing::debug;

use crate::model::Extras;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Control {
    pub shown: bool,
    pub every: Duration,
}

/// One sample and what the window needs beside it.
#[derive(Clone)]
pub struct Sample {
    pub at: Instant,
    pub full: Arc<Full>,
    pub extras: Arc<Extras>,
}

/// Starts the loop; the window sets the control and takes the samples.
pub fn start(worker: &isle_app::Worker, bus: Option<zbus::Connection>) -> (watch::Sender<Control>, UnboundedReceiver<Sample>) {
    let (control, mut watched) = watch::channel(Control { shown: false, every: Duration::from_secs(2) });
    let (tx, rx) = unbounded();
    worker.spawn(async move {
        let names = Arc::new(Names::load());
        let users = Arc::new(crate::model::users(std::path::Path::new("/")));
        let mut sampler: Option<MonitorSampler> = None;
        let system = zbus::Connection::system().await.ok();
        loop {
            if watched.wait_for(|c| c.shown).await.is_err() {
                return;
            }
            let taken = sampler.take();
            let Ok((kept, full)) = tokio::task::spawn_blocking(move || {
                let mut s = taken.unwrap_or_else(MonitorSampler::new);
                let full = s.sample();
                (s, full)
            })
            .await
            else {
                return;
            };
            sampler = Some(kept);
            let extras = extras(bus.as_ref(), system.as_ref(), &names, &users).await;
            if send(&tx, Sample { at: Instant::now(), full: Arc::new(full), extras: Arc::new(extras) }).is_err() {
                return;
            }
            let every = watched.borrow().every;
            tokio::select! {
                _ = tokio::time::sleep(every) => {}
                changed = watched.changed() => if changed.is_err() { return },
            }
        }
    });
    (control, rx)
}

fn send(tx: &UnboundedSender<Sample>, sample: Sample) -> Result<(), ()> {
    tx.unbounded_send(sample).map_err(|_| ())
}

/// Apps' names and icons by app id, from the desktop entries, read once.
struct Names {
    index: isle_apps::Index,
    icons: std::sync::Mutex<isle_apps::icons::Icons>,
}

impl Names {
    fn load() -> Self {
        Self {
            index: isle_apps::Index::scan(&isle_apps::Index::data_dirs()),
            icons: std::sync::Mutex::new(isle_apps::icons::Icons::papirus(true)),
        }
    }

    fn of(&self, app_id: &str) -> (String, Option<std::path::PathBuf>) {
        let id = self.index.resolve(app_id);
        let entry = self.index.get(&id);
        let icon = entry.and_then(|e| e.icon.clone()).and_then(|i| self.icons.lock().ok()?.lookup(&i, 16));
        (self.index.name(&id), icon)
    }
}

/// The windowed processes' names and icons, and every hold on idle or sleep by pid: logind's
/// block inhibitors and those the compositor took for its clients.
async fn extras(bus: Option<&zbus::Connection>, system: Option<&zbus::Connection>, names: &Names, users: &Arc<HashMap<u32, String>>) -> Extras {
    let mut out = Extras { users: (**users).clone(), ..Default::default() };
    let Some(bus) = bus else { return out };
    if let Ok(windows) = isle_ipc::windows::WindowsProxy::new(bus).await
        && let Ok(list) = windows.list().await
    {
        for w in list.into_iter().filter(|w| w.pid > 0) {
            out.windows.entry(w.pid).or_insert_with(|| names.of(&w.app_id));
        }
    }
    if let Ok(session) = isle_ipc::session::SessionProxy::new(bus).await
        && let Ok(holds) = session.holds().await
    {
        for h in holds.into_iter().filter(|h| h.pid > 0) {
            let what = out.holds.entry(h.pid).or_insert_with(BTreeSet::new);
            what.extend(words(&h.what));
        }
    }
    match logind(system).await {
        Ok(list) => {
            for (what, _, _, mode, _, pid) in list {
                if mode == "block" {
                    out.holds.entry(pid as i32).or_default().extend(words(&what));
                }
            }
        }
        Err(err) => debug!("logind's inhibitors: {err}"),
    }
    out
}

/// logind's `what` as Monitor's column writes it (`p19a-monitor.md` choice 13).
fn words(what: &str) -> Vec<&'static str> {
    what.split(':')
        .filter_map(|w| match w {
            "sleep" => Some("Sleep"),
            "idle" => Some("Idle"),
            "shutdown" => Some("Shut down"),
            _ => None,
        })
        .collect()
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login {
    fn list_inhibitors(&self) -> zbus::Result<Vec<(String, String, String, String, u32, u32)>>;
}

async fn logind(system: Option<&zbus::Connection>) -> zbus::Result<Vec<(String, String, String, String, u32, u32)>> {
    let system = system.ok_or_else(|| zbus::Error::Failure("no system bus".into()))?;
    LoginProxy::new(system).await?.list_inhibitors().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn logind_words_become_the_columns() {
        assert_eq!(super::words("sleep:idle"), ["Sleep", "Idle"]);
        assert_eq!(super::words("shutdown:handle-power-key"), ["Shut down"]);
    }
}
