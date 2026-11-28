//! The players are followed through a burst of signals larger than the stream holds.

use std::time::Duration;

use zbus::object_server::SignalEmitter;

const PATH: &str = "/org/mpris/MediaPlayer2";

/// A player whose first read comes after a burst of `Seeked` that overfills a follower's
/// stream.
struct Player {
    reads: std::sync::atomic::AtomicU32,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    #[zbus(property)]
    async fn playback_status(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        if self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            for _ in 0..200 {
                Self::seeked(&emitter, 0).await.unwrap();
            }
        }
        "Paused".into()
    }

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

async fn player(address: &str, name: &str) -> zbus::Connection {
    zbus::connection::Builder::address(address)
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(
            PATH,
            Player {
                reads: Default::default(),
            },
        )
        .unwrap()
        .build()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_burst_during_a_read_does_not_wedge_the_session_bus() {
    let bus = isle_fakes::Bus::start();
    // One test in this binary, before any connection: the variable is set once.
    unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &bus.address) };
    let _first = player(&bus.address, "org.mpris.MediaPlayer2.first").await;

    let (publisher, mut rx) = isle_bus::snapshot::<isle_media::Media>();
    tokio::spawn(isle_media::watch(publisher));
    tokio::time::timeout(
        Duration::from_secs(10),
        rx.wait_for(|s| s.state.players.len() == 1),
    )
    .await
    .expect("the first player read")
    .unwrap();

    let _second = player(&bus.address, "org.mpris.MediaPlayer2.second").await;
    tokio::time::timeout(
        Duration::from_secs(10),
        rx.wait_for(|s| s.state.players.len() == 2),
    )
    .await
    .expect("a player arriving after the burst is seen")
    .unwrap();
}
