//! What the compositor spends at rest and under a drawing client, read from `Perf` (P19p).
//! Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

use isle_ipc::displays::DisplaysProxy;
use serde_json::Value;

mod common;

use common::Isle;

async fn perf(isle: &Isle) -> Value {
    let json = DisplaysProxy::new(&isle.bus).await.unwrap().perf().await.unwrap();
    serde_json::from_str(&json).unwrap()
}

/// How far `key`'s count moved between two samples; `key` is a JSON pointer.
fn moved(before: &Value, after: &Value, key: &str) -> u64 {
    let at = |v: &Value| v.pointer(key).and_then(Value::as_u64).unwrap_or(0);
    at(after).saturating_sub(at(before))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_still_desktop_does_no_work() {
    let mut isle = Isle::start("perf-idle").await;
    let _client = isle.client("isle.test.Still", "888888");
    isle.wait_for_window("isle.test.Still").await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let before = perf(&isle).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = perf(&isle).await;
    // The island's clock may turn a minute in the window: one frame at most.
    assert!(moved(&before, &after, "/redraws") <= 1, "{before}\n{after}");
    assert!(moved(&before, &after, "/outputs/0/frames") <= 1, "{after}");
    assert_eq!(moved(&before, &after, "/glass_runs"), 0, "{after}");
    assert_eq!(moved(&before, &after, "/index/syncs"), 0, "{after}");
    assert_eq!(moved(&before, &after, "/commits/isle.test.Still"), 0, "{after}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_service_post_that_changes_nothing_draws_nothing() {
    let mut isle = Isle::start("perf-island").await;
    let _client = isle.client("isle.test.Still", "888888");
    isle.wait_for_window("isle.test.Still").await;
    let wifi = r#"{"glyph": {"kind": "wifi", "active": false, "symbol": "wifi-high", "level": "wifi-medium", "alt": "Wi-Fi"}}"#;
    let test = isle.test().await;
    test.island(wifi).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let before = perf(&isle).await;
    for _ in 0..10 {
        test.island(wifi).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let after = perf(&isle).await;
    assert!(moved(&before, &after, "/redraws") <= 1, "{before}\n{after}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drawing_window_costs_one_pass_and_one_element_list_a_frame() {
    let mut isle = Isle::start("perf-drawing").await;
    let client = isle.client("isle.test.Drawing", "3366ff");
    isle.wait_for_window("isle.test.Drawing").await;
    client.send("animate");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let before = perf(&isle).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = perf(&isle).await;
    let frames = moved(&before, &after, "/outputs/0/frames");
    let redraws = moved(&before, &after, "/redraws");
    let builds = moved(&before, &after, "/element_builds");
    assert!(frames >= 60, "the window drew: {after}");
    assert!(
        moved(&before, &after, "/commits/isle.test.Drawing") >= frames - 2,
        "its commits are its own: {after}"
    );
    assert!(redraws <= frames + 2, "no pass beyond the frames: {redraws} for {frames}");
    assert!(builds <= redraws + 2, "one element list a pass: {builds} for {redraws}");
}

#[tokio::test(flavor = "multi_thread")]
async fn glass_blurs_again_only_for_what_changes_behind_it() {
    let mut isle = Isle::start("perf-glass").await;
    let behind = isle.client_with(&["--app-id", "isle.test.Behind", "--colour", "3366ff", "--size", "1900x1000"]);
    isle.wait_for_window("isle.test.Behind").await;
    let small = isle.client_with(&["--app-id", "isle.test.Small", "--colour", "ff3366", "--size", "300x200"]);
    isle.wait_for_window("isle.test.Small").await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The small window opens centred, clear of the island's pill at the top.
    small.send("animate");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let before = perf(&isle).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after = perf(&isle).await;
    let frames = moved(&before, &after, "/outputs/0/frames");
    assert!(frames >= 30, "{after}");
    assert_eq!(moved(&before, &after, "/glass_runs"), 0, "nothing behind the pill changed: {after}");
    assert!(moved(&before, &after, "/glass_skips") >= frames - 2, "{after}");
    small.send("animate");

    behind.send("animate");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let before = perf(&isle).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after = perf(&isle).await;
    let frames = moved(&before, &after, "/outputs/0/frames");
    assert!(
        moved(&before, &after, "/glass_runs") >= frames - 2,
        "the window behind the pill changed each frame: {frames} frames, {after}"
    );
}
