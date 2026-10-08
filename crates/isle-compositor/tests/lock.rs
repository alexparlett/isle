//! The lock's core: no client reaches a screen or takes input while locked, and a compositor
//! that finds the marker starts locked. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

const CTRL: u32 = 29;
const CMD: u32 = 125;
const Q: u32 = 16;
const TAB: u32 = 15;
const A: u32 = 30;

async fn locked(isle: &Isle) -> bool {
    SessionProxy::builder(&isle.bus)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .unwrap()
        .locked()
        .await
        .unwrap()
}

async fn kinds(isle: &Isle) -> Vec<String> {
    isle.test().await.frame_kinds("HEADLESS-1").await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn locking_draws_no_client_and_takes_no_input_until_unlocked() {
    let mut isle = Isle::start("lock-core").await;
    let window = isle.client("isle.test.Behind", "ff0000");
    window.wait_for("keyboard-enter").await;
    window.send("inhibit");
    window.wait_for("inhibitor active").await;
    let overlay = isle.client_with(&[
        "--layer",
        "overlay",
        "--anchor",
        "top",
        "--size",
        "10x100",
        "--colour",
        "00ff00",
        "--app-id",
        "isle.test.Overlay",
    ]);
    overlay.wait_for("configured 1920 100").await;
    let info = isle.wait_for_window("isle.test.Behind").await;
    let centre = (
        (info.x + info.width / 2) as f64,
        (info.y + info.height / 2) as f64,
    );
    wait_until(|| async { kinds(&isle).await.iter().any(|k| k == "client") }).await;

    // The window holds the keys, so the bus locks rather than the chord.
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { locked(&isle).await }).await;
    window.wait_for("keyboard-leave").await;
    // The frame queued by the lock replaces the last; every frame from then is the lock's alone.
    wait_until(|| async { !kinds(&isle).await.iter().any(|k| k == "client") }).await;
    for _ in 0..5 {
        let frame = kinds(&isle).await;
        assert!(!frame.iter().any(|k| k == "client"), "{frame:?}");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let shot = isle.shot("HEADLESS-1", "locked").await;
    let [r, g, b] = shot.at(centre.0 as usize, centre.1 as usize);
    assert!(
        !(r > 200 && g < 60 && b < 60),
        "the window shows: {:?}",
        [r, g, b]
    );
    let [r, g, b] = shot.at(960, 40);
    assert!(!(g > 200 && r < 60), "the overlay shows: {:?}", [r, g, b]);

    // Keys, clicks and a desktop chord reach nothing.
    window.clear();
    let test = isle.test().await;
    test.key(A, true).await.unwrap();
    test.key(A, false).await.unwrap();
    test.pointer_to(centre.0, centre.1).await.unwrap();
    test.button(0x110, true).await.unwrap();
    test.button(0x110, false).await.unwrap();
    isle.chord(&[CMD, TAB]).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!window.printed("key 30"), "a key reached the window");
    assert!(!window.printed("button"), "a click reached the window");
    assert!(!window.printed("keyboard-enter"), "the window took focus");
    assert!(isle.test().await.switcher().await.unwrap() == "null");

    test.unlock().await.unwrap();
    window.wait_for("keyboard-enter").await;
    assert!(!locked(&isle).await);
    wait_until(|| async { kinds(&isle).await.iter().any(|k| k == "client") }).await;
    assert!(
        !isle.dir.join("runtime/isle/locked").exists(),
        "the marker outlived the lock"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_mapped_while_locked_is_not_given_focus() {
    let mut isle = Isle::start("lock-map").await;
    isle.chord(&[CTRL, CMD, Q]).await;
    wait_until(|| async { locked(&isle).await }).await;
    let window = isle.client("isle.test.Late", "0000ff");
    isle.wait_for_window("isle.test.Late").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !window.printed("keyboard-enter"),
        "a window took focus while locked"
    );
    let frame = kinds(&isle).await;
    assert!(!frame.iter().any(|k| k == "client"), "{frame:?}");

    isle.test().await.unlock().await.unwrap();
    window.wait_for("keyboard-enter").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_compositor_that_finds_the_marker_starts_locked() {
    let mut isle = Isle::start_prepared("lock-marker", "", |dir| {
        std::fs::create_dir_all(dir.join("runtime/isle")).unwrap();
        std::fs::write(dir.join("runtime/isle/locked"), "locked\n").unwrap();
    })
    .await;
    assert!(locked(&isle).await);
    let window = isle.client("isle.test.First", "ff0000");
    isle.wait_for_window("isle.test.First").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let frame = kinds(&isle).await;
    assert!(
        !frame.is_empty() && !frame.iter().any(|k| k == "client"),
        "{frame:?}"
    );
    assert!(!window.printed("keyboard-enter"));
}
