//! `org.freedesktop.ScreenSaver` and `org.freedesktop.PowerManagement`, owned on the worker
//! before the socket is announced; the loop keeps what they hold (`p9-lock-and-prompts.md`
//! choice 14, `p20h-power.md` choice 5). Active means locked.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use futures_util::StreamExt;
use tokio::sync::broadcast;
use tracing::{info, warn};
use zbus::{Connection, fdo, interface, message::Header, object_server::SignalEmitter};

use crate::{
    idle::{Cookie, Kind},
    worker::{Event, ToLoop},
};

const NAME: &str = "org.freedesktop.ScreenSaver";
/// Firefox calls the short path; the rest the long one.
const PATHS: [&str; 2] = ["/org/freedesktop/ScreenSaver", "/ScreenSaver"];
const PM_NAME: &str = "org.freedesktop.PowerManagement";
const PM_PATH: &str = "/org/freedesktop/PowerManagement/Inhibit";

/// Takes a cookie for the caller of `header`: its pid read from the bus, the hold kept on the
/// loop until `release` or the caller leaves.
pub async fn hold(
    to_loop: &ToLoop,
    next: &AtomicU32,
    conn: &Connection,
    header: &Header<'_>,
    app: String,
    why: String,
    kind: Kind,
) -> u32 {
    let cookie = next.fetch_add(1, Ordering::Relaxed);
    let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
    let pid = match (fdo::DBusProxy::new(conn).await, header.sender()) {
        (Ok(dbus), Some(name)) => dbus
            .get_connection_unix_process_id(name.clone().into())
            .await
            .map_or(0, |p| p as i32),
        _ => 0,
    };
    info!(app, reason = why, cookie, kind = kind.name(), "idle inhibited");
    to_loop.post(move |state| {
        let app_id = state.isle.processes.app_of_tree(pid);
        state.isle.idle.cookies.insert(
            cookie,
            Cookie {
                sender,
                pid,
                app,
                app_id,
                why,
                kind,
            },
        );
        state.idle_holds_changed();
    });
    cookie
}

/// Firefox's wake lock for sound alone comes through ScreenSaver; music must not keep the
/// screen lit, so it holds only sleep (`p20h-power.md` choice 4).
fn audio_only(reason: &str) -> bool {
    ["Playing audio", "audio-playing"]
        .iter()
        .any(|r| reason.eq_ignore_ascii_case(r))
}

/// Ends a cookie of `kind`.
pub fn release(to_loop: &ToLoop, cookie: u32, kind: Kind) {
    to_loop.post(move |state| {
        if state.isle.idle.cookies.get(&cookie).is_some_and(|c| c.kind == kind) {
            state.isle.idle.cookies.remove(&cookie);
            state.idle_holds_changed();
        }
    });
}

#[derive(Clone)]
struct ScreenSaver {
    to_loop: ToLoop,
    next: Arc<AtomicU32>,
}

#[interface(name = "org.freedesktop.ScreenSaver")]
impl ScreenSaver {
    /// Holds the display on until `UnInhibit` with the cookie, or the caller leaves the bus.
    async fn inhibit(
        &self,
        application_name: String,
        reason_for_inhibit: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &Connection,
    ) -> u32 {
        let (app, why) = (application_name, reason_for_inhibit);
        let kind = if audio_only(&why) {
            Kind::Sleep
        } else {
            Kind::Display
        };
        hold(&self.to_loop, &self.next, conn, &header, app, why, kind).await
    }

    async fn un_inhibit(&self, cookie: u32) {
        release(&self.to_loop, cookie, Kind::Display);
        release(&self.to_loop, cookie, Kind::Sleep);
    }

    async fn lock(&self) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(|state| state.lock("the screensaver"))
            .await?)
    }

    async fn get_active(&self) -> fdo::Result<bool> {
        Ok(self.to_loop.call(|state| state.isle.lock.locked).await?)
    }

    /// True locks; false is refused, since only the password ends the lock.
    async fn set_active(&self, active: bool) -> fdo::Result<bool> {
        if !active {
            return Ok(false);
        }
        Ok(self
            .to_loop
            .call(|state| {
                state.lock("the screensaver");
                true
            })
            .await?)
    }

    /// Seconds the session has been locked.
    async fn get_active_time(&self) -> fdo::Result<u32> {
        Ok(self
            .to_loop
            .call(|state| {
                state
                    .isle
                    .lock
                    .since
                    .map_or(0, |since| since.elapsed().as_secs() as u32)
            })
            .await?)
    }

    /// Seconds since the last input.
    async fn get_session_idle_time(&self) -> fdo::Result<u32> {
        Ok(self
            .to_loop
            .call(|state| state.isle.idle.last_activity.elapsed().as_secs() as u32)
            .await?)
    }

    async fn simulate_user_activity(&self) {
        self.to_loop.post(|state| {
            state.idle_activity();
        });
    }

    #[zbus(signal)]
    async fn active_changed(emitter: &SignalEmitter<'_>, active: bool) -> zbus::Result<()>;
}

struct PowerManagement {
    to_loop: ToLoop,
    next: Arc<AtomicU32>,
}

/// Chromium's and Electron's "keep the computer running", and Firefox's: a sleep hold.
#[interface(name = "org.freedesktop.PowerManagement.Inhibit")]
impl PowerManagement {
    async fn inhibit(
        &self,
        application: String,
        reason: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &Connection,
    ) -> u32 {
        hold(&self.to_loop, &self.next, conn, &header, application, reason, Kind::Sleep).await
    }

    async fn un_inhibit(&self, cookie: u32) {
        release(&self.to_loop, cookie, Kind::Sleep);
    }

    /// Whether anything keeps the computer from sleeping by idle.
    async fn has_inhibit(&self) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(|state| !state.isle.sleep_holders().is_empty())
            .await?)
    }

    #[zbus(signal)]
    async fn has_inhibit_changed(
        emitter: &SignalEmitter<'_>,
        has_inhibit_changed: bool,
    ) -> zbus::Result<()>;
}

/// Asks for `name` without queueing; whether it is now `conn`'s.
async fn request(conn: &Connection, name: &'static str) -> zbus::Result<bool> {
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    match conn
        .request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(true),
        Ok(_) | Err(zbus::Error::NameTaken) => Ok(false),
        Err(err) => Err(err),
    }
}

/// What `own` served: each name, or false where another program owns it.
pub struct Owned {
    pub screensaver: bool,
    pub power_management: bool,
}

/// Owns both names on `conn` and serves their paths, one cookie count between them; a name
/// another program owns is left to it.
pub async fn own(conn: &Connection, to_loop: ToLoop, next: Arc<AtomicU32>) -> zbus::Result<Owned> {
    let server = ScreenSaver {
        to_loop: to_loop.clone(),
        next: next.clone(),
    };
    for path in PATHS {
        conn.object_server().at(path, server.clone()).await?;
    }
    let screensaver = request(conn, NAME).await?;
    if !screensaver {
        for path in PATHS {
            conn.object_server().remove::<ScreenSaver, _>(path).await?;
        }
    }
    conn.object_server()
        .at(PM_PATH, PowerManagement { to_loop, next })
        .await?;
    let power_management = request(conn, PM_NAME).await?;
    if !power_management {
        conn.object_server()
            .remove::<PowerManagement, _>(PM_PATH)
            .await?;
    }
    Ok(Owned {
        screensaver,
        power_management,
    })
}

/// Drops the inhibitors of callers that leave the bus, says when the lock comes and goes,
/// and when the computer comes to be held from sleep or let go.
pub async fn follow(
    conn: Connection,
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
    power_management: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dbus = fdo::DBusProxy::new(&conn).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    let mut changes = events.subscribe();
    let mut held = false;
    loop {
        tokio::select! {
            Some(signal) = owners.next() => {
                let args = signal.args()?;
                if args.new_owner().is_none() && args.name().starts_with(':') {
                    let gone = args.name().to_string();
                    to_loop.post(move |state| {
                        let before = state.isle.idle.cookies.len();
                        state.isle.idle.cookies.retain(|_, c| c.sender != gone);
                        if state.isle.idle.cookies.len() != before {
                            state.idle_holds_changed();
                        }
                    });
                }
            }
            event = changes.recv() => match event {
                Ok(Event::LockChanged) => {
                    let locked = to_loop.call(|state| state.isle.lock.locked).await.unwrap_or(false);
                    for path in PATHS {
                        let emitter = SignalEmitter::new(&conn, path)?;
                        if let Err(err) = ScreenSaver::active_changed(&emitter, locked).await {
                            warn!("ScreenSaver's ActiveChanged: {err}");
                        }
                    }
                }
                Ok(Event::HoldsChanged) if power_management => {
                    let now = to_loop
                        .call(|state| !state.isle.sleep_holders().is_empty())
                        .await
                        .unwrap_or(false);
                    if now != held {
                        held = now;
                        let emitter = SignalEmitter::new(&conn, PM_PATH)?;
                        if let Err(err) = PowerManagement::has_inhibit_changed(&emitter, now).await {
                            warn!("PowerManagement's HasInhibitChanged: {err}");
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
                _ => {}
            },
        }
    }
}
