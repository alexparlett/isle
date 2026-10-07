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
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owned(&self) -> bool {
        self.owned
    }

    /// Waits until the name has an owner. Errors when the bus connection ends.
    pub async fn appeared(&mut self) -> zbus::Result<()> {
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
