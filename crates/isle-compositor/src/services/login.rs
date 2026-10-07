//! logind: sleep and resume, and the session's hints.

use std::{error::Error, time::Duration};

use futures_util::StreamExt;
use isle_bus::{NameWatch, Publisher, Snapshot};
use serde::Serialize;
use tracing::{debug, info};

use crate::state::State;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait Session {
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn idle_hint(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn active(&self) -> zbus::Result<bool>;
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Login {
    pub preparing_for_sleep: bool,
    /// Resumes from sleep seen, so the loop notices one inside a burst it saw only the end of.
    pub resumes: u64,
    pub locked_hint: bool,
    pub idle_hint: bool,
    pub active: bool,
}

const NAME: &str = "org.freedesktop.login1";

pub async fn watch(publisher: Publisher<Login>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = isle_bus::system().await?;
    let mut name = NameWatch::new(&bus, NAME).await?;
    loop {
        name.appeared().await?;
        let result = tokio::select! {
            result = follow(&bus, &publisher) => result,
            result = name.vanished() => result.map_err(Into::into),
        };
        publisher.unavailable();
        if let Err(err) = result {
            debug!("logind: {err}");
            // A daemon that answers wrongly is asked again, not hammered.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn follow(
    bus: &zbus::Connection,
    publisher: &Publisher<Login>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ManagerProxy::new(bus).await?;
    let session = SessionProxy::new(bus).await?;
    let mut sleeps = manager.receive_prepare_for_sleep().await?;
    let mut locked = session.receive_locked_hint_changed().await;
    let mut idle = session.receive_idle_hint_changed().await;
    let mut active = session.receive_active_changed().await;
    let resumes = publisher.current().resumes;
    publisher.publish(Login {
        preparing_for_sleep: manager.preparing_for_sleep().await?,
        resumes,
        locked_hint: session.locked_hint().await.unwrap_or(false),
        idle_hint: session.idle_hint().await.unwrap_or(false),
        active: session.active().await.unwrap_or(true),
    });
    loop {
        tokio::select! {
            Some(signal) = sleeps.next() => {
                let start = signal.args()?.start;
                publisher.update(|l| {
                    l.preparing_for_sleep = start;
                    if !start {
                        l.resumes += 1;
                    }
                });
            }
            Some(change) = locked.next() => {
                let on = change.get().await?;
                publisher.update(|l| l.locked_hint = on);
            }
            Some(change) = idle.next() => {
                let on = change.get().await?;
                publisher.update(|l| l.idle_hint = on);
            }
            Some(change) = active.next() => {
                let on = change.get().await?;
                publisher.update(|l| l.active = on);
            }
            else => return Err("logind's signals ended".into()),
        }
    }
}

impl State {
    pub fn login_changed(&mut self, snapshot: Snapshot<Login>) {
        let resumed = snapshot.state.resumes > self.isle.services.login.state.resumes;
        self.isle.services.login = snapshot;
        if resumed {
            info!("resumed from sleep");
            // The clock's timer runs on the monotonic clock, which stood still in sleep.
            self.isle.island_resumed();
        }
    }
}
