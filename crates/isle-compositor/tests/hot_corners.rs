//! The screens' corners headless: an action when the pointer rests in a corner, once a
//! visit, and only with the modifiers the corner needs. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

use isle_ipc::shell::ShellProxy;

mod common;

use common::{Isle, keys, wait_until};

async fn spotlight_open(isle: &Isle) -> bool {
    ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .iter()
        .any(|s| s.name == "spotlight" && s.open)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bottom_right_corner_opens_the_apps_once_a_visit() {
    let isle = Isle::start("corners").await;
    let test = isle.test().await;
    test.pointer_to(1919.0, 1079.0).await.unwrap();
    wait_until(|| async { spotlight_open(&isle).await }).await;
    // Resting on in the corner fires nothing more.
    test.pointer_to(1918.0, 1078.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(spotlight_open(&isle).await);
    // Leaving and coming back closes what it opened.
    test.pointer_to(900.0, 500.0).await.unwrap();
    test.pointer_to(1919.0, 1079.0).await.unwrap();
    wait_until(|| async { !spotlight_open(&isle).await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_corner_that_needs_a_modifier_waits_for_it() {
    let isle = Isle::start_with(
        "corners-modifier",
        "wallpaper {\n    source \"#1c1c1f\"\n}\nhot-corners {\n    bottom-right-modifiers \"Cmd\"\n}\n",
    )
    .await;
    let test = isle.test().await;
    test.pointer_to(1919.0, 1079.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!spotlight_open(&isle).await, "not without Cmd");
    test.pointer_to(900.0, 500.0).await.unwrap();
    test.key(keys::SUPER, true).await.unwrap();
    test.pointer_to(1919.0, 1079.0).await.unwrap();
    wait_until(|| async { spotlight_open(&isle).await }).await;
    test.key(keys::SUPER, false).await.unwrap();
}
