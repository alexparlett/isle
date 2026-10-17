//! ext-background-effect-v1: a client's blur region drawn as glass behind its window, and
//! none under Reduce Transparency. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

/// What the frame draws right behind the window: glass is a shell element.
async fn behind_window(isle: &Isle) -> String {
    let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
    let last = kinds.iter().rposition(|k| k == "client").unwrap_or(0);
    kinds.get(last + 1).cloned().unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blur_region_is_glass_behind_the_window() {
    let mut isle = Isle::start("background-effect").await;
    let client = isle.client_with(&["--app-id", "isle.test.Blur", "--title", "blur"]);
    client.wait_for("keyboard-enter").await;
    assert_eq!(behind_window(&isle).await, "wallpaper");
    client.send("blur 0 0 120 400");
    wait_until(|| async { behind_window(&isle).await == "shell" }).await;

    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap();
    settings
        .set(
            "appearance.reduce-transparency",
            &zvariant::Value::from(true),
        )
        .await
        .unwrap();
    wait_until(|| async { behind_window(&isle).await == "wallpaper" }).await;
}
