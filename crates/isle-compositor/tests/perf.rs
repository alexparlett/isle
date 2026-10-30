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
