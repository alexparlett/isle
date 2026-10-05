//! P4's window model, run headless with real clients. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

mod common;

use common::{Isle, wait_until};

const RED: [u8; 3] = [0xff, 0, 0];
const GREEN: [u8; 3] = [0, 0xff, 0];

#[tokio::test(flavor = "multi_thread")]
async fn a_minimised_window_leaves_the_screen_and_comes_back_in_its_place() {
    let mut isle = Isle::start("minimise").await;
    let _under = isle.client("isle.test.Under", "ff0000");
    isle.wait_for_window("isle.test.Under").await;
    let over = isle.client("isle.test.Over", "00ff00");
    let before = isle.wait_for_window("isle.test.Over").await;
    over.wait_for("keyboard-enter").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(isle.shot("HEADLESS-1", "before").await.at(960, 540), GREEN);

    let windows = isle.windows_proxy().await;
    assert!(windows.minimise(before.id).await.unwrap());
    over.wait_for("states suspended").await;
    over.wait_for("keyboard-leave").await;
    let minimised = isle.window(before.id).await;
    assert!(minimised.minimised && !minimised.focused);
    assert_eq!(isle.shot("HEADLESS-1", "minimised").await.at(960, 540), RED);
    assert!(
        isle.windows()
            .await
            .iter()
            .any(|w| w.app_id == "isle.test.Under" && w.focused)
    );

    over.clear();
    assert!(windows.unminimise(before.id).await.unwrap());
    over.wait_for("states activated").await;
    let after = isle.window(before.id).await;
    assert!(!after.minimised && after.focused);
    assert_eq!((after.x, after.y), (before.x, before.y));
    assert_eq!(
        isle.shot("HEADLESS-1", "restored").await.at(960, 540),
        GREEN
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn hiding_an_app_parks_all_its_windows_and_showing_keeps_their_order() {
    let mut isle = Isle::start("hide").await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let app = isle.client("isle.test.Two", "ff0000");
    let first = isle.wait_for_window("isle.test.Two").await;
    app.send("window");
    app.wait_for("w1 configured 300 200").await;
    wait_until(|| async { isle.windows().await.len() == 3 }).await;

    let windows = isle.windows_proxy().await;
    assert!(windows.set_app_hidden("isle.test.Two", true).await.unwrap());
    let listed = isle.windows().await;
    assert!(
        listed
            .iter()
            .filter(|w| w.app == "isle.test.Two")
            .all(|w| w.hidden)
    );
    assert!(
        listed
            .iter()
            .any(|w| w.app == "isle.test.Other" && w.focused)
    );
    assert_eq!(
        isle.shot("HEADLESS-1", "hidden").await.at(960, 540),
        [0, 0, 0xff]
    );

    assert!(
        windows
            .set_app_hidden("isle.test.Two", false)
            .await
            .unwrap()
    );
    let listed = isle.windows().await;
    assert!(listed.iter().all(|w| !w.hidden));
    // The second window was in front before the hide and is again.
    let order: Vec<u64> = listed.iter().map(|w| w.id).collect();
    let second = listed.iter().find(|w| w.title == "window 1").unwrap().id;
    assert!(order.iter().position(|i| *i == second) < order.iter().position(|i| *i == first.id));
    assert!(
        !windows
            .set_app_hidden("isle.test.None", true)
            .await
            .unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_box_set_by_id_moves_and_resizes_the_window() {
    let mut isle = Isle::start("setbox").await;
    let client = isle.client("isle.test.Box", "ff0000");
    let window = isle.wait_for_window("isle.test.Box").await;
    let windows = isle.windows_proxy().await;
    assert!(
        windows
            .set_box(window.id, 100, 120, 640, 480)
            .await
            .unwrap()
    );
    client.wait_for("configured 640 480").await;
    wait_until(|| async {
        let w = isle.window(window.id).await;
        (w.x, w.y, w.width, w.height) == (100, 120, 640, 480)
    })
    .await;
    assert!(!windows.set_box(9999, 0, 0, 10, 10).await.unwrap());
}
