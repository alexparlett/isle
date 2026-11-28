//! The tray's items are followed through a burst of signals larger than the stream holds.

use std::time::Duration;

use zbus::object_server::SignalEmitter;

const ITEM_PATH: &str = "/StatusNotifierItem";

/// An item whose first read comes after a burst of `NewIcon` that overfills a follower's
/// stream.
struct Item {
    reads: std::sync::atomic::AtomicU32,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    async fn id(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        if self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            for _ in 0..200 {
                Self::new_icon(&emitter).await.unwrap();
            }
        }
        "item".into()
    }

    #[zbus(signal)]
    async fn new_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

async fn item(address: &str) -> zbus::Connection {
    let conn = zbus::connection::Builder::address(address)
        .unwrap()
        .serve_at(
            ITEM_PATH,
            Item {
                reads: Default::default(),
            },
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    conn.call_method(
        Some("org.kde.StatusNotifierWatcher"),
        "/StatusNotifierWatcher",
        Some("org.kde.StatusNotifierWatcher"),
        "RegisterStatusNotifierItem",
        &(ITEM_PATH,),
    )
    .await
    .unwrap();
    conn
}

#[tokio::test(flavor = "multi_thread")]
async fn a_burst_during_a_read_does_not_wedge_the_host() {
    let bus = isle_fakes::Bus::start();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();
    let Ok(host) = isle_tray::own(&conn).await.unwrap() else {
        panic!("the watcher's name was taken")
    };
    let (publisher, mut rx) = isle_bus::snapshot::<isle_tray::Tray>();
    let follower = conn.clone();
    tokio::spawn(async move { host.follow(&follower, publisher).await });

    let _first = item(&bus.address).await;
    tokio::time::timeout(
        Duration::from_secs(10),
        rx.wait_for(|s| s.state.items.len() == 1),
    )
    .await
    .expect("the first item read")
    .unwrap();

    let _second = item(&bus.address).await;
    tokio::time::timeout(
        Duration::from_secs(10),
        rx.wait_for(|s| s.state.items.len() == 2),
    )
    .await
    .expect("an item registered after the burst is seen")
    .unwrap();
}
