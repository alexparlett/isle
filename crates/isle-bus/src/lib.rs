//! What Isle's service crates share: the buses, a daemon's name followed, snapshots of a
//! daemon's state, and a count of who is listening.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::{Notify, OnceCell, watch};
use zbus::{Connection, fdo::DBusProxy, names::BusName};

static SYSTEM: OnceCell<Connection> = OnceCell::const_new();
static SESSION: OnceCell<Connection> = OnceCell::const_new();

/// The system bus, one connection for the process. `DBUS_SYSTEM_BUS_ADDRESS` moves it, as
/// the tests do.
pub async fn system() -> zbus::Result<Connection> {
    SYSTEM.get_or_try_init(Connection::system).await.cloned()
}

/// The session bus, one connection for the process.
pub async fn session() -> zbus::Result<Connection> {
    SESSION.get_or_try_init(Connection::session).await.cloned()
}

/// A well-known name followed as its owner comes and goes.
pub struct NameWatch {
    name: String,
    owned: bool,
    changes: zbus::fdo::NameOwnerChangedStream,
    dbus: DBusProxy<'static>,
}

impl NameWatch {
    pub async fn new(conn: &Connection, name: &str) -> zbus::Result<Self> {
        let dbus = DBusProxy::new(conn).await?;
        // Subscribed before asking, so an owner arriving between the two is not missed.
        let changes = dbus
            .receive_name_owner_changed_with_args(&[(0, name)])
            .await?;
        let owned = dbus
            .name_has_owner(BusName::try_from(name)?)
            .await
            .unwrap_or(false);
        Ok(Self {
            name: name.to_owned(),
            owned,
            changes,
            dbus,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owned(&self) -> bool {
        self.owned
    }

    /// Waits until the name has an owner, starting its daemon first where the bus can:
    /// UPower, UDisks2 and the like run only once something asks for them. Errors when
    /// the bus connection ends.
    pub async fn appeared(&mut self) -> zbus::Result<()> {
        if !self.owned
            && let Ok(names) = self.dbus.list_activatable_names().await
            && names.iter().any(|n| n.as_str() == self.name)
            && let Ok(name) = zbus::names::WellKnownName::try_from(self.name.as_str())
        {
            let _ = self.dbus.start_service_by_name(name, 0).await;
        }
        while !self.owned {
            self.next().await?;
        }
        Ok(())
    }

    /// Waits until the name loses its owner, or changes hands, which is a restart.
    pub async fn vanished(&mut self) -> zbus::Result<()> {
        loop {
            let replaced = self.next().await?;
            if !self.owned || replaced {
                return Ok(());
            }
        }
    }

    /// The next change of owner; true when one owner was replaced by another.
    async fn next(&mut self) -> zbus::Result<bool> {
        let Some(change) = self.changes.next().await else {
            return Err(zbus::Error::Failure("the bus connection ended".into()));
        };
        let args = change.args()?;
        let replaced = args.old_owner().is_some() && args.new_owner().is_some();
        self.owned = args.new_owner().is_some();
        Ok(replaced)
    }
}

/// A daemon's state as a service last knew it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Snapshot<T> {
    /// The daemon answers. While it does not, `state` is the last it knew.
    pub available: bool,
    pub state: T,
}

/// The writing end of a service's snapshot, kept across the service's restarts.
#[derive(Clone)]
pub struct Publisher<T>(watch::Sender<Snapshot<T>>);

/// A new snapshot channel, unavailable until the service first publishes.
pub fn snapshot<T: Default>() -> (Publisher<T>, watch::Receiver<Snapshot<T>>) {
    let (tx, rx) = watch::channel(Snapshot::default());
    (Publisher(tx), rx)
}

impl<T: PartialEq + Clone> Publisher<T> {
    /// The daemon answers with `state`. Readers wake only when something changed.
    pub fn publish(&self, state: T) {
        self.0.send_if_modified(|s| {
            let changed = !s.available || s.state != state;
            s.available = true;
            s.state = state;
            changed
        });
    }

    /// Changes the state in place, as a signal's partial update does.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.0.send_if_modified(|s| {
            let before = s.state.clone();
            f(&mut s.state);
            let changed = !s.available || s.state != before;
            s.available = true;
            changed
        });
    }

    /// The daemon went; the last state stays for readers that show it greyed.
    pub fn unavailable(&self) {
        self.0
            .send_if_modified(|s| std::mem::replace(&mut s.available, false));
    }

    pub fn current(&self) -> T {
        self.0.borrow().state.clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Snapshot<T>> {
        self.0.subscribe()
    }
}

/// How many readers want a sampled value, so a service samples only while one does
/// (`platform.md` Service crates 4).
#[derive(Clone, Default)]
pub struct Listeners {
    count: Arc<AtomicUsize>,
    changed: Arc<Notify>,
}

/// One reader's interest, given up when dropped.
pub struct Listening(Listeners);

impl Listeners {
    pub fn listen(&self) -> Listening {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.changed.notify_waiters();
        Listening(self.clone())
    }

    pub fn any(&self) -> bool {
        self.count.load(Ordering::SeqCst) > 0
    }

    /// Waits until someone listens.
    pub async fn wanted(&self) {
        loop {
            let changed = self.changed.notified();
            if self.any() {
                return;
            }
            changed.await;
        }
    }

    /// Waits until no one listens.
    pub async fn unwanted(&self) {
        loop {
            let changed = self.changed.notified();
            if !self.any() {
                return;
            }
            changed.await;
        }
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.0.count.fetch_sub(1, Ordering::SeqCst);
        self.0.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn readers_wake_only_on_change() {
        let (publisher, mut rx) = snapshot::<u32>();
        publisher.publish(3);
        assert!(rx.has_changed().unwrap());
        assert_eq!(
            *rx.borrow_and_update(),
            Snapshot {
                available: true,
                state: 3
            }
        );
        publisher.publish(3);
        assert!(!rx.has_changed().unwrap());
        publisher.unavailable();
        assert_eq!(
            *rx.borrow_and_update(),
            Snapshot {
                available: false,
                state: 3
            }
        );
        publisher.unavailable();
        assert!(!rx.has_changed().unwrap());
        publisher.update(|n| *n += 1);
        assert_eq!(
            *rx.borrow_and_update(),
            Snapshot {
                available: true,
                state: 4
            }
        );
    }

    #[tokio::test]
    async fn listeners_count_their_guards() {
        let listeners = Listeners::default();
        assert!(!listeners.any());
        let first = listeners.listen();
        let second = listeners.listen();
        listeners.wanted().await;
        drop(first);
        assert!(listeners.any());
        let waiting = tokio::spawn({
            let listeners = listeners.clone();
            async move { listeners.unwanted().await }
        });
        drop(second);
        waiting.await.unwrap();
        assert!(!listeners.any());
    }
}

/// One object's properties on one interface, read whole, with lenient getters: daemons
/// differ in the integer widths they send.
#[derive(Clone, Debug, Default)]
pub struct Props(pub std::collections::HashMap<String, zbus::zvariant::OwnedValue>);

impl Props {
    pub async fn read(
        conn: &Connection,
        destination: &str,
        path: &str,
        interface: &str,
    ) -> zbus::Result<Self> {
        let proxy = zbus::fdo::PropertiesProxy::builder(conn)
            .destination(destination.to_owned())?
            .path(path.to_owned())?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        let interface = zbus::names::InterfaceName::try_from(interface.to_owned())?;
        Ok(Self(proxy.get_all(interface).await?))
    }

    fn value(&self, name: &str) -> Option<&zbus::zvariant::Value<'static>> {
        self.0.get(name).map(|v| &**v)
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        match self.value(name)? {
            zbus::zvariant::Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn i64(&self, name: &str) -> Option<i64> {
        use zbus::zvariant::Value as V;
        match self.value(name)? {
            V::U8(n) => Some(i64::from(*n)),
            V::I16(n) => Some(i64::from(*n)),
            V::U16(n) => Some(i64::from(*n)),
            V::I32(n) => Some(i64::from(*n)),
            V::U32(n) => Some(i64::from(*n)),
            V::I64(n) => Some(*n),
            V::U64(n) => i64::try_from(*n).ok(),
            V::F64(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn f64(&self, name: &str) -> Option<f64> {
        match self.value(name)? {
            zbus::zvariant::Value::F64(n) => Some(*n),
            _ => self.i64(name).map(|n| n as f64),
        }
    }

    pub fn str(&self, name: &str) -> Option<String> {
        match self.value(name)? {
            zbus::zvariant::Value::Str(s) => Some(s.to_string()),
            zbus::zvariant::Value::ObjectPath(p) => Some(p.to_string()),
            _ => None,
        }
    }

    /// An `ay` holding text, as NetworkManager's SSIDs are.
    pub fn bytes(&self, name: &str) -> Option<Vec<u8>> {
        let value = self.value(name)?;
        <Vec<u8>>::try_from(value.try_clone().ok()?).ok()
    }

    /// An array of object paths.
    pub fn paths(&self, name: &str) -> Vec<String> {
        let Some(value) = self.value(name) else {
            return Vec::new();
        };
        let Ok(value) = value.try_clone() else {
            return Vec::new();
        };
        <Vec<zbus::zvariant::OwnedObjectPath>>::try_from(value)
            .map(|v| v.into_iter().map(|p| p.to_string()).collect())
            .unwrap_or_default()
    }

    /// An `aa{sv}`, each map as its own `Props`.
    pub fn maps(&self, name: &str) -> Vec<Props> {
        let Some(zbus::zvariant::Value::Array(array)) = self.value(name) else {
            return Vec::new();
        };
        array
            .iter()
            .filter_map(|item| {
                let dict = zbus::zvariant::Dict::try_from(item.try_clone().ok()?).ok()?;
                let map: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
                    dict.try_into().ok()?;
                Some(Props(map))
            })
            .collect()
    }
}

/// Every signal `sender` emits on objects under `path`, for a service that reads its
/// daemon's state again whenever anything there changes.
pub async fn signals_under(
    conn: &Connection,
    sender: &str,
    path: &str,
) -> zbus::Result<zbus::MessageStream> {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(sender.to_owned())?
        .path_namespace(path.to_owned())?
        .build();
    zbus::MessageStream::for_match_rule(rule, conn, Some(64)).await
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Follows a daemon whose state is small enough to read whole: while `name` has an owner,
/// reads it with `read` at once and again after any signal the daemon sends under `path`,
/// a burst read once, and publishes it; while the name has no owner, the snapshot is
/// unavailable. Returns only when the bus connection ends.
pub async fn follow<T, F, Fut>(
    conn: &Connection,
    name: &str,
    path: &str,
    publisher: &Publisher<T>,
    mut read: F,
) -> Result<(), Error>
where
    T: PartialEq + Clone,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    use futures_util::FutureExt;
    let mut owner = NameWatch::new(conn, name).await?;
    loop {
        owner.appeared().await?;
        let mut signals = signals_under(conn, name, path).await?;
        let mut result = read().await.map(|state| publisher.publish(state));
        while result.is_ok() {
            tokio::select! {
                gone = owner.vanished() => {
                    gone?;
                    break;
                }
                signal = signals.next() => {
                    if signal.is_none() {
                        return Err("the bus connection ended".into());
                    }
                    while let Some(Some(_)) = signals.next().now_or_never() {}
                    result = read().await.map(|state| publisher.publish(state));
                }
            }
        }
        publisher.unavailable();
        if result.is_err() {
            // A daemon that answers wrongly is asked again, not hammered.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
