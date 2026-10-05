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

/// The places file once its writer has had time to run.
async fn places(isle: &Isle) -> Option<serde_json::Value> {
    tokio::time::sleep(Duration::from_millis(900)).await;
    let text = std::fs::read_to_string(isle.dir.join("state/isle/places.json")).ok()?;
    Some(serde_json::from_str(&text).unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_main_window_reopens_at_its_place_from_its_first_frame() {
    let mut isle = Isle::start("place").await;
    let client = isle.client("isle.test.Place", "ff0000");
    let window = isle.wait_for_window("isle.test.Place").await;
    let windows = isle.windows_proxy().await;
    assert!(
        windows
            .set_box(window.id, 100, 200, 640, 480)
            .await
            .unwrap()
    );
    client.wait_for("configured 640 480").await;
    let saved = places(&isle).await.expect("places.json");
    let place = &saved["data"]["apps"]["isle.test.Place"]["Isle Headless 1"];
    assert_eq!(
        (place["x"].as_i64(), place["y"].as_i64()),
        (Some(100), Some(152))
    );

    client.send("quit");
    wait_until(|| async { isle.windows().await.is_empty() }).await;
    let again = isle.client("isle.test.Place", "00ff00");
    let reopened = isle.wait_for_window("isle.test.Place").await;
    assert_eq!(
        (reopened.x, reopened.y, reopened.width, reopened.height),
        (100, 200, 640, 480)
    );
    again.wait_for("configured 640 480").await;
    let first = again
        .lines
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.starts_with("configured"))
        .cloned();
    assert_eq!(first.as_deref(), Some("configured 640 480"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_window_of_an_app_cascades_and_is_not_recorded() {
    let mut isle = Isle::start("cascade").await;
    let _one = isle.client("isle.test.Cascade", "ff0000");
    let first = isle.wait_for_window("isle.test.Cascade").await;
    let _two = isle.client("isle.test.Cascade", "00ff00");
    wait_until(|| async { isle.windows().await.len() == 2 }).await;
    let second = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.id != first.id)
        .unwrap();
    assert_eq!((second.x, second.y), (first.x + 32, first.y + 32));
    let windows = isle.windows_proxy().await;
    windows.set_box(second.id, 10, 60, 400, 300).await.unwrap();
    assert!(places(&isle).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn dialogs_open_over_their_parent_and_modal_ones_hold_its_focus() {
    let mut isle = Isle::start("dialog").await;
    let client = isle.client_with(&["--app-id", "isle.test.Parent", "--size", "800x600"]);
    let parent = isle.wait_for_window("isle.test.Parent").await;
    client.send("dialog");
    client.wait_for("w1 configured 300 200").await;
    wait_until(|| async { isle.windows().await.len() == 2 }).await;
    let dialog = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.parent == parent.id)
        .unwrap();
    assert_eq!(
        (dialog.x, dialog.y),
        (parent.x + 250, parent.y + 200),
        "centred over the parent"
    );
    let windows = isle.windows_proxy().await;
    windows.set_box(dialog.id, 0, 60, 300, 200).await.unwrap();
    assert!(places(&isle).await.is_none(), "a dialog is never recorded");

    client.send("modal");
    client.wait_for("w2 configured 300 200").await;
    wait_until(|| async { isle.windows().await.len() == 3 }).await;
    let modal = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.parent == parent.id && w.id != dialog.id)
        .unwrap();
    assert_eq!(
        (modal.x, modal.y),
        (parent.x + 250, parent.y),
        "under the parent's top"
    );
    assert!(modal.focused);

    // A click on the parent leaves the keys with its modal dialog.
    let test = isle.test().await;
    test.pointer_to(parent.x as f64 + 20.0, parent.y as f64 + 580.0)
        .await
        .unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    assert!(isle.window(modal.id).await.focused);

    // The modal dialog moves with its parent.
    windows
        .set_box(parent.id, parent.x - 100, parent.y + 50, 800, 600)
        .await
        .unwrap();
    wait_until(|| async {
        let m = isle.window(modal.id).await;
        (m.x, m.y) == (modal.x - 100, modal.y + 50)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_place_on_an_absent_output_is_clamped_onto_one_present() {
    let mut isle = Isle::start("absent").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "AOC", "U34G2G4R3", "7", 2560, 1440)
        .await
        .unwrap();
    test.pointer_to(1920.0 + 1280.0, 720.0).await.unwrap();
    let client = isle.client("isle.test.Away", "ff0000");
    let window = isle.wait_for_window("isle.test.Away").await;
    assert_eq!(window.output, "HEADLESS-2");
    let windows = isle.windows_proxy().await;
    windows
        .set_box(window.id, 1920 + 2000, 300, 400, 300)
        .await
        .unwrap();
    client.wait_for("configured 400 300").await;
    assert!(places(&isle).await.is_some());

    client.send("quit");
    wait_until(|| async { isle.windows().await.is_empty() }).await;
    assert!(test.remove_output("HEADLESS-2").await.unwrap());
    test.pointer_to(960.0, 540.0).await.unwrap();
    let _again = isle.client("isle.test.Away", "00ff00");
    let reopened = isle.wait_for_window("isle.test.Away").await;
    assert_eq!(reopened.output, "HEADLESS-1");
    assert_eq!((reopened.x, reopened.y), (1920 - 400, 300));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_box_partly_off_its_output_is_never_recorded() {
    let mut isle = Isle::start("offscreen").await;
    let client = isle.client("isle.test.Off", "ff0000");
    let window = isle.wait_for_window("isle.test.Off").await;
    let windows = isle.windows_proxy().await;
    windows
        .set_box(window.id, 1800, 300, 400, 300)
        .await
        .unwrap();
    client.wait_for("configured 400 300").await;
    assert!(places(&isle).await.is_none());
}
