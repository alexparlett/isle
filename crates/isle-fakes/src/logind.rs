//! `org.freedesktop.login1`: the manager's sleep and inhibitors, and the caller's session with
//! its hints and its Lock and Unlock.

use std::{
    io::Read,
    sync::{Arc, Mutex},
};

use zbus::{Connection, object_server::SignalEmitter};

const MANAGER: &str = "/org/freedesktop/login1";
const SESSION: &str = "/org/freedesktop/login1/session/auto";

#[derive(Clone, Default)]
struct Values {
    preparing_for_sleep: bool,
    locked_hint: bool,
    idle_hint: bool,
}

/// An inhibitor a client took: what, its mode, and logind's end of its descriptor.
struct Inhibitor {
    what: String,
    mode: String,
    end: std::io::PipeReader,
}

type Inhibitors = Arc<Mutex<Vec<Inhibitor>>>;

struct Manager {
    preparing_for_sleep: bool,
    inhibitors: Inhibitors,
}

#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> bool {
        self.preparing_for_sleep
    }

    /// The caller holds the inhibitor until it closes the descriptor.
    fn inhibit(
        &self,
        what: String,
        _who: String,
        _why: String,
        mode: String,
    ) -> zbus::fdo::Result<zvariant::OwnedFd> {
        let (end, held) =
            std::io::pipe().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        self.inhibitors
            .lock()
            .unwrap()
            .push(Inhibitor { what, mode, end });
        Ok(zvariant::OwnedFd::from(std::os::fd::OwnedFd::from(held)))
    }

    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> zvariant::OwnedObjectPath {
        zvariant::OwnedObjectPath::try_from(SESSION).unwrap()
    }

    #[zbus(signal)]
    async fn prepare_for_sleep(emitter: &SignalEmitter<'_>, start: bool) -> zbus::Result<()>;
}

struct Session {
    locked_hint: bool,
    idle_hint: bool,
    brightness: Vec<(String, String, u32)>,
}

#[zbus::interface(name = "org.freedesktop.login1.Session")]
impl Session {
    #[zbus(property)]
    fn locked_hint(&self) -> bool {
        self.locked_hint
    }

    #[zbus(property)]
    fn idle_hint(&self) -> bool {
        self.idle_hint
    }

    #[zbus(property)]
    fn active(&self) -> bool {
        true
    }

    fn set_brightness(&mut self, subsystem: String, name: String, value: u32) {
        self.brightness.push((subsystem, name, value));
    }

    async fn set_locked_hint(
        &mut self,
        locked: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.locked_hint = locked;
        let _ = self.locked_hint_changed(&emitter).await;
    }

    async fn set_idle_hint(
        &mut self,
        idle: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.idle_hint = idle;
        let _ = self.idle_hint_changed(&emitter).await;
    }

    #[zbus(signal)]
    async fn lock(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn unlock(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// A logind on the bus at an address.
pub struct Logind {
    address: String,
    values: Values,
    conn: Option<Connection>,
    inhibitors: Inhibitors,
}

impl Logind {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let mut fake = Self {
            address: address.to_owned(),
            values: Values::default(),
            conn: None,
            inhibitors: Default::default(),
        };
        fake.restore().await?;
        Ok(fake)
    }

    /// Leaves the bus, as a stopped daemon does.
    pub async fn vanish(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = conn.close().await;
        }
    }

    /// Comes back with the state it had.
    pub async fn restore(&mut self) -> zbus::Result<()> {
        let v = self.values.clone();
        let conn = zbus::connection::Builder::address(self.address.as_str())?
            .serve_at(
                MANAGER,
                Manager {
                    preparing_for_sleep: v.preparing_for_sleep,
                    inhibitors: self.inhibitors.clone(),
                },
            )?
            .serve_at(
                SESSION,
                Session {
                    locked_hint: v.locked_hint,
                    idle_hint: v.idle_hint,
                    brightness: Vec::new(),
                },
            )?
            .name("org.freedesktop.login1")?
            .build()
            .await?;
        self.conn = Some(conn);
        Ok(())
    }

    fn conn(&self) -> &Connection {
        self.conn.as_ref().expect("the fake logind is on the bus")
    }

    pub async fn set_locked_hint(&mut self, on: bool) -> zbus::Result<()> {
        self.values.locked_hint = on;
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Session>(SESSION)
            .await?;
        iface.get_mut().await.locked_hint = on;
        iface
            .get()
            .await
            .locked_hint_changed(iface.signal_emitter())
            .await
    }

    /// The inhibitors still held, as `what:mode`; one whose holder closed it is gone.
    pub fn inhibitors(&self) -> Vec<String> {
        let mut held = self.inhibitors.lock().unwrap();
        held.retain_mut(|i| {
            // The holder's end closed: the pipe reads to its end. Nothing is ever written.
            use std::os::fd::AsRawFd;
            let mut poll = libc::pollfd {
                fd: i.end.as_raw_fd(),
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut poll, 1, 0) };
            !(ready > 0 && {
                let mut byte = [0u8; 1];
                matches!(i.end.read(&mut byte), Ok(0))
            })
        });
        held.iter().map(|i| format!("{}:{}", i.what, i.mode)).collect()
    }

    /// The session's locked hint as Isle set it.
    pub async fn locked_hint(&self) -> bool {
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Session>(SESSION)
            .await
            .unwrap();
        iface.get().await.locked_hint
    }

    /// `loginctl lock-session`.
    pub async fn lock_session(&self) -> zbus::Result<()> {
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Session>(SESSION)
            .await?;
        Session::lock(iface.signal_emitter()).await
    }

    /// `loginctl unlock-session`.
    pub async fn unlock_session(&self) -> zbus::Result<()> {
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Session>(SESSION)
            .await?;
        Session::unlock(iface.signal_emitter()).await
    }

    /// Every SetBrightness asked of the session: subsystem, device, value.
    pub async fn brightness(&self) -> Vec<(String, String, u32)> {
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Session>(SESSION)
            .await
            .unwrap();
        iface.get().await.brightness.clone()
    }

    /// The manager's announcement before sleep (`true`) and after resume (`false`).
    pub async fn prepare_for_sleep(&mut self, start: bool) -> zbus::Result<()> {
        self.values.preparing_for_sleep = start;
        let iface = self
            .conn()
            .object_server()
            .interface::<_, Manager>(MANAGER)
            .await?;
        iface.get_mut().await.preparing_for_sleep = start;
        iface
            .get()
            .await
            .preparing_for_sleep_changed(iface.signal_emitter())
            .await?;
        Manager::prepare_for_sleep(iface.signal_emitter(), start).await
    }
}

