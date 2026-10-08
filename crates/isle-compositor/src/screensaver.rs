//! `org.freedesktop.ScreenSaver`, owned on the worker before the socket is announced; the
//! loop keeps what it holds (`p9-lock-and-prompts.md` choice 14). Active means locked.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use futures_util::StreamExt;
use tokio::sync::broadcast;
use tracing::{info, warn};
use zbus::{Connection, fdo, interface, message::Header, object_server::SignalEmitter};

use crate::worker::{Event, ToLoop};

const NAME: &str = "org.freedesktop.ScreenSaver";
/// Firefox calls the short path; the rest the long one.
const PATHS: [&str; 2] = ["/org/freedesktop/ScreenSaver", "/ScreenSaver"];

#[derive(Clone)]
struct ScreenSaver {
    to_loop: ToLoop,
    next: Arc<AtomicU32>,
}

#[interface(name = "org.freedesktop.ScreenSaver")]
impl ScreenSaver {
    /// Holds idle off until `UnInhibit` with the cookie, or the caller leaves the bus.
    async fn inhibit(
        &self,
        application_name: String,
        reason_for_inhibit: String,
        #[zbus(header)] header: Header<'_>,
    ) -> u32 {
        let cookie = self.next.fetch_add(1, Ordering::Relaxed);
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        info!(
            app = application_name,
            reason = reason_for_inhibit,
            cookie,
            "idle inhibited"
        );
        self.to_loop.post(move |state| {
            state
                .isle
                .idle
                .cookies
                .insert(cookie, (sender, application_name));
            state.idle_holds_changed();
        });
        cookie
    }

    async fn un_inhibit(&self, cookie: u32) {
        self.to_loop.post(move |state| {
            if state.isle.idle.cookies.remove(&cookie).is_some() {
                state.idle_holds_changed();
            }
        });
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

/// Owns the name on `conn` and serves both paths; `Ok(false)` when another program owns it.
pub async fn own(conn: &Connection, to_loop: ToLoop) -> zbus::Result<bool> {
    let server = ScreenSaver {
        to_loop,
        next: Arc::new(AtomicU32::new(1)),
    };
    for path in PATHS {
        conn.object_server().at(path, server.clone()).await?;
    }
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    match conn
        .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(true),
        Ok(_) | Err(zbus::Error::NameTaken) => {
            for path in PATHS {
                conn.object_server().remove::<ScreenSaver, _>(path).await?;
            }
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

/// Drops the inhibitors of callers that leave the bus, and says when the lock comes and goes.
pub async fn follow(
    conn: Connection,
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dbus = fdo::DBusProxy::new(&conn).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    let mut changes = events.subscribe();
    loop {
        tokio::select! {
            Some(signal) = owners.next() => {
                let args = signal.args()?;
                if args.new_owner().is_none() && args.name().starts_with(':') {
                    let gone = args.name().to_string();
                    to_loop.post(move |state| {
                        let before = state.isle.idle.cookies.len();
                        state.isle.idle.cookies.retain(|_, (sender, _)| *sender != gone);
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
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
                _ => {}
            },
        }
    }
}
