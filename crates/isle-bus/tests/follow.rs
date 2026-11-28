//! `follow` keeps reading through a burst of signals larger than its stream holds.

use std::time::Duration;

use zbus::object_server::SignalEmitter;

const NAME: &str = "org.isle.Test";
const PATH: &str = "/org/isle/Test";

/// Answers `Count` with how many times it has been asked; the first answer comes after
/// a burst of signals that overfills a follower's stream.
struct Counter {
    calls: u32,
}

#[zbus::interface(name = "org.isle.Test")]
impl Counter {
    async fn count(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> u32 {
        self.calls += 1;
        if self.calls == 1 {
            for _ in 0..200 {
                Self::ping(&emitter).await.unwrap();
            }
        }
        self.calls
    }

    #[zbus(signal)]
    async fn ping(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_burst_during_a_read_does_not_wedge_the_connection() {
    let bus = isle_fakes::Bus::start();
    let _daemon = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(NAME)
        .unwrap()
        .serve_at(PATH, Counter { calls: 0 })
        .unwrap()
        .build()
        .await
        .unwrap();
    let conn = isle_fakes::connect(&bus.address).await.unwrap();

    let (publisher, mut rx) = isle_bus::snapshot::<u32>();
    let follower = conn.clone();
    tokio::spawn(async move {
        isle_bus::follow(&follower, NAME, PATH, &publisher, || async {
            let reply = follower
                .call_method(Some(NAME), PATH, Some(NAME), "Count", &())
                .await?;
            Ok(reply.body().deserialize::<u32>()?)
        })
        .await
    });

    // The burst asks for a second read, which only happens if the first one's reply arrived.
    tokio::time::timeout(
        Duration::from_secs(10),
        rx.wait_for(|s| s.available && s.state >= 2),
    )
    .await
    .expect("the follower read again after the burst")
    .unwrap();
    // The connection still answers.
    tokio::time::timeout(
        Duration::from_secs(5),
        conn.call_method(Some(NAME), PATH, Some(NAME), "Count", &()),
    )
    .await
    .expect("the connection answers")
    .unwrap();
}
