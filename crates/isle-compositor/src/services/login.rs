//! logind: sleep and resume, the session's hints, its Lock and Unlock, and the inhibitors
//! Isle holds: the lock before sleep, and the power key (`p9-lock-and-prompts.md` choices 9
//! and 10).

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

    #[zbus(property)]
    fn block_inhibited(&self) -> zbus::Result<String>;

    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;

    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
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

    fn set_locked_hint(&self, locked: bool) -> zbus::Result<()>;

    fn set_idle_hint(&self, idle: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;
}

/// Isle's own session: the one its process is in, else logind's `auto`.
async fn session(bus: &zbus::Connection) -> zbus::Result<SessionProxy<'static>> {
    let manager = ManagerProxy::new(bus).await?;
    match manager.get_session_by_pid(std::process::id()).await {
        Ok(path) => SessionProxy::builder(bus).path(path)?.build().await,
        Err(_) => SessionProxy::new(bus).await,
    }
}

/// Sets the session's locked or idle hint.
pub async fn set_hint(locked: Option<bool>, idle: Option<bool>) -> Result<(), isle_bus::Error> {
    let bus = isle_bus::system().await?;
    let session = session(&bus).await?;
    if let Some(on) = locked {
        session.set_locked_hint(on).await?;
    }
    if let Some(on) = idle {
        session.set_idle_hint(on).await?;
    }
    Ok(())
}

/// Holds Isle's inhibitors and passes on the session's Lock and Unlock. The sleep delay is
/// given back when `may_sleep` turns true, once every screen shows the lock, and taken again at
/// resume; the power key is held throughout, so it opens the session chooser.
pub async fn hold(
    to_loop: crate::worker::ToLoop,
    mut may_sleep: tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = isle_bus::system().await?;
    let mut name = NameWatch::new(&bus, NAME).await?;
    loop {
        name.appeared().await?;
        let result = tokio::select! {
            result = hold_while_there(&bus, &to_loop, &mut may_sleep) => result,
            result = name.vanished() => result.map_err(Into::into),
        };
        if let Err(err) = result {
            debug!("logind's inhibitors: {err}");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn hold_while_there(
    bus: &zbus::Connection,
    to_loop: &crate::worker::ToLoop,
    may_sleep: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ManagerProxy::new(bus).await?;
    let session = session(bus).await?;
    let mut locks = session.receive_lock().await?;
    let mut unlocks = session.receive_unlock().await?;
    let mut sleeps = manager.receive_prepare_for_sleep().await?;
    let _power = manager
        .inhibit(
            "handle-power-key",
            "Isle",
            "The power key opens the session chooser",
            "block",
        )
        .await
        .map_err(|err| debug!("no power-key inhibitor: {err}"))
        .ok();
    let take_sleep = || {
        manager.inhibit("sleep", "Isle", "Locking the screen before sleep", "delay")
    };
    let mut sleep = Some(take_sleep().await?);
    loop {
        tokio::select! {
            Some(_) = locks.next() => to_loop.post(|state| state.lock("logind")),
            Some(_) = unlocks.next() => to_loop.post(|state| state.unlock("logind")),
            Some(signal) = sleeps.next() => {
                if !signal.args()?.start && sleep.is_none() {
                    sleep = Some(take_sleep().await?);
                    debug!("the sleep inhibitor is held again");
                }
            }
            changed = may_sleep.changed() => {
                changed?;
                if *may_sleep.borrow_and_update() && sleep.take().is_some() {
                    info!("every screen shows the lock; sleep may go on");
                }
            }
            else => return Err("logind's signals ended".into()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Login {
    pub preparing_for_sleep: bool,
    /// Resumes from sleep seen, so the loop notices one inside a burst it saw only the end of.
    pub resumes: u64,
    pub locked_hint: bool,
    pub idle_hint: bool,
    pub active: bool,
    /// What logind's block inhibitors hold, colon-separated: `idle` among them holds idle.
    pub block_inhibited: String,
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
    let session = session(bus).await?;
    let mut sleeps = manager.receive_prepare_for_sleep().await?;
    let mut locked = session.receive_locked_hint_changed().await;
    let mut idle = session.receive_idle_hint_changed().await;
    let mut active = session.receive_active_changed().await;
    let mut blocks = manager.receive_block_inhibited_changed().await;
    let resumes = publisher.current().resumes;
    publisher.publish(Login {
        preparing_for_sleep: manager.preparing_for_sleep().await?,
        resumes,
        locked_hint: session.locked_hint().await.unwrap_or(false),
        idle_hint: session.idle_hint().await.unwrap_or(false),
        active: session.active().await.unwrap_or(true),
        block_inhibited: manager.block_inhibited().await.unwrap_or_default(),
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
            Some(change) = blocks.next() => {
                let what = change.get().await.unwrap_or_default();
                publisher.update(|l| l.block_inhibited = what);
            }
            else => return Err("logind's signals ended".into()),
        }
    }
}

impl State {
    pub fn login_changed(&mut self, snapshot: Snapshot<Login>) {
        let resumed = snapshot.state.resumes > self.isle.services.login.state.resumes;
        let sleeping = snapshot.state.preparing_for_sleep
            && !self.isle.services.login.state.preparing_for_sleep;
        self.isle.services.login = snapshot;
        if sleeping {
            self.before_sleep();
        }
        self.idle_holds_changed();
        if resumed {
            self.after_sleep();
        }
        // Sleep is one of the holds on banners.
        self.silence_check();
        if resumed {
            info!("resumed from sleep");
            // The clock's timer runs on the monotonic clock, which stood still in sleep.
            self.isle.island_resumed();
        }
    }
}
