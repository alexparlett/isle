//! Presentation: what clients are told about their frames, and the plane trace. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::displays::DisplaysProxy;

mod common;

use common::{Client, Isle, wait_until};

/// The `presented` lines a client printed: seconds, nanoseconds, refresh, sequence, flags.
fn presented(client: &Client) -> Vec<[u64; 5]> {
    client
        .lines
        .lock()
        .unwrap()
        .iter()
        .filter_map(|l| l.strip_prefix("presented "))
        .map(|l| {
            let n: Vec<u64> = l.split(' ').map(|n| n.parse().unwrap()).collect();
            [n[0], n[1], n[2], n[3], n[4]]
        })
        .collect()
}

const VSYNC: u64 = 0x1;
const HW_CLOCK: u64 = 0x2;

#[tokio::test(flavor = "multi_thread")]
async fn presentation_times_rise_by_the_refresh() {
    let mut isle = Isle::start("presented").await;
    let client = isle.client("isle.test.Presented", "3366ff");
    isle.wait_for_window("isle.test.Presented").await;
    client.send("timing");
    client.send("animate");
    wait_until(|| async { presented(&client).len() >= 10 }).await;

    let frames = presented(&client);
    let refresh = frames[0][2];
    assert!((16_000_000..17_000_000).contains(&refresh), "{refresh}");
    let times: Vec<u128> = frames
        .iter()
        .map(|f| f[0] as u128 * 1_000_000_000 + f[1] as u128)
        .collect();
    assert!(times.windows(2).all(|w| w[1] > w[0]), "{times:?}");
    for f in &frames {
        assert_eq!(f[2], refresh);
        assert_ne!(f[4] & VSYNC, 0);
        // Headless frames carry Isle's own time.
        assert_eq!(f[4] & HW_CLOCK, 0);
    }

    let trace = DisplaysProxy::new(&isle.bus)
        .await
        .unwrap()
        .trace("HEADLESS-1", 0)
        .await
        .unwrap();
    assert!(trace.frames >= 10);
    assert_eq!(trace.scanned_out, 0);
    let last = trace.recent.last().unwrap();
    assert_eq!(last.primary, "composited");
    assert!(!last.hardware && !last.asynchronous && !last.vrr);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_layer_surface_is_told_when_its_frame_was_shown() {
    let mut isle = Isle::start("presented-layer").await;
    let layer = isle.client_with(&["--layer", "top", "--size", "400x40"]);
    layer.wait_for("configured 1920 40").await;
    layer.send("timing");
    layer.send("animate");
    wait_until(|| async { presented(&layer).len() >= 3 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_trace_resets_and_names_no_display_it_lacks() {
    let mut isle = Isle::start("trace").await;
    let client = isle.client("isle.test.Trace", "33ff66");
    isle.wait_for_window("isle.test.Trace").await;
    client.send("animate");
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async { displays.trace("HEADLESS-1", 0).await.unwrap().frames > 5 }).await;
    displays.reset_trace("HEADLESS-1").await.unwrap();
    let after = displays.trace("HEADLESS-1", 0).await.unwrap();
    assert!(after.frames < 5, "{}", after.frames);
    assert!(displays.trace("DP-9", 0).await.is_err());
}
