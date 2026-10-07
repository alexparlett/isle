//! `org.freedesktop.login1`: the manager's sleep and the caller's session.

use zbus::{Connection, object_server::SignalEmitter};

const MANAGER: &str = "/org/freedesktop/login1";
const SESSION: &str = "/org/freedesktop/login1/session/auto";

#[derive(Clone, Default)]
struct Values {
    preparing_for_sleep: bool,
    locked_hint: bool,
    idle_hint: bool,
}

struct Manager {
    preparing_for_sleep: bool,
}

#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> bool {
        self.preparing_for_sleep
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
}

/// A logind on the bus at an address.
pub struct Logind {
    address: String,
    values: Values,
    conn: Option<Connection>,
}

impl Logind {
    pub async fn start(address: &str) -> zbus::Result<Self> {
        let mut fake = Self {
            address: address.to_owned(),
            values: Values::default(),
            conn: None,
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
