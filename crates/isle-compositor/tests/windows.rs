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
async fn a_tiled_main_window_reopens_in_its_tile_with_its_box_behind_it() {
    let mut isle = Isle::start("placetile").await;
    let client = isle.client("isle.test.PlaceTile", "ff0000");
    let window = isle.wait_for_window("isle.test.PlaceTile").await;
    client.wait_for("keyboard-enter").await;
    let windows = isle.windows_proxy().await;
    windows.set_box(window.id, 100, 200, 640, 480).await.unwrap();
    settles(&isle, window.id, "floating", (100, 200, 640, 480)).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
    settles(&isle, window.id, "tiled:left-half", LEFT_HALF).await;
    let saved = places(&isle).await.expect("places.json");
    let place = &saved["data"]["apps"]["isle.test.PlaceTile"]["Isle Headless 1"];
    assert_eq!(place["zone"].as_str(), Some("left-half"), "{place}");
    assert_eq!((place["x"].as_i64(), place["width"].as_i64()), (Some(100), Some(640)));

    client.send("quit");
    wait_until(|| async { isle.windows().await.is_empty() }).await;
    let again = isle.client("isle.test.PlaceTile", "00ff00");
    let reopened = isle.wait_for_window("isle.test.PlaceTile").await;
    settles(&isle, reopened.id, "tiled:left-half", LEFT_HALF).await;
    again.wait_for("configured 942 1008").await;
    windows.restore(reopened.id).await.unwrap();
    settles(&isle, reopened.id, "floating", (100, 200, 640, 480)).await;
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
    let below = (parent.x as usize + 20, parent.y as usize + 560);
    let dimmed = isle.shot("HEADLESS-1", "scrim").await.at(below.0, below.1);
    assert!(
        darker(dimmed, [0xff, 0x33, 0x66]),
        "the parent is dimmed: {dimmed:?}"
    );

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

use common::keys;

/// The window `id` once its mode and box are as given.
async fn settles(isle: &Isle, id: u64, mode: &str, rect: (i32, i32, i32, i32)) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let w = isle.window(id).await;
        if w.mode == mode && (w.x, w.y, w.width, w.height) == rect {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "wanted {mode} {rect:?}, have {} {:?}",
            w.mode,
            (w.x, w.y, w.width, w.height)
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// With the default margins of 12 on the 1920×1080 output, whose work area starts at 48.
const LEFT_HALF: (i32, i32, i32, i32) = (12, 60, 942, 1008);
const RIGHT_HALF: (i32, i32, i32, i32) = (966, 60, 942, 1008);
const TOP_LEFT: (i32, i32, i32, i32) = (12, 60, 942, 498);
const FILL: (i32, i32, i32, i32) = (12, 60, 1896, 1008);

#[tokio::test(flavor = "multi_thread")]
async fn with_edge_tiling_off_a_drag_to_an_edge_only_moves() {
    let mut isle = Isle::start_with("edges-off", "windows {\n    edge-tiling #false\n}\n").await;
    let client = isle.client("isle.test.EdgesOff", "ff0000");
    let w = isle.wait_for_window("isle.test.EdgesOff").await;
    client.send("move-on-press");
    isle.drag(&client, (w.x as f64 + w.width as f64 / 2.0, w.y as f64 + 20.0), (2.0, 540.0)).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(isle.window(w.id).await.mode, "floating");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drag_to_an_edge_tiles_and_the_bottom_edge_does_not() {
    let mut isle = Isle::start("edges").await;
    let client = isle.client("isle.test.Edges", "ff0000");
    let w = isle.wait_for_window("isle.test.Edges").await;
    let centre = |w: &isle_ipc::WindowInfo| (w.x as f64 + w.width as f64 / 2.0, w.y as f64 + 20.0);

    client.send("move-on-press");
    isle.drag(&client, centre(&w), (2.0, 540.0)).await;
    settles(&isle, w.id, "tiled:left-half", LEFT_HALF).await;
    client
        .wait_for("states activated,tiled-left,tiled-right,tiled-top,tiled-bottom")
        .await;

    // Dragged away, it takes its old size back under the pointer.
    client.send("move-on-press");
    isle.drag(&client, (400.0, 80.0), (900.0, 400.0)).await;
    wait_until(|| async {
        let now = isle.window(w.id).await;
        now.mode == "floating" && (now.width, now.height) == (400, 300)
    })
    .await;

    let w = isle.window(w.id).await;
    client.send("move-on-press");
    isle.drag(&client, centre(&w), (30.0, 1.0)).await;
    settles(&isle, w.id, "tiled:top-left", TOP_LEFT).await;

    let w = isle.window(w.id).await;
    client.send("move-on-press");
    isle.drag(&client, centre(&w), (960.0, 1.0)).await;
    settles(&isle, w.id, "filled", FILL).await;
    client
        .wait_for("states activated,maximized,tiled-left,tiled-right,tiled-top,tiled-bottom")
        .await;

    let w = isle.window(w.id).await;
    client.send("move-on-press");
    isle.drag(&client, centre(&w), (960.0, 1079.0)).await;
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn tiling_chords_give_their_zones_and_cycle() {
    let mut isle = Isle::start("chords").await;
    let client = isle.client("isle.test.Chords", "ff0000");
    let w = isle.wait_for_window("isle.test.Chords").await;
    client.wait_for("keyboard-enter").await;
    let before = (w.x, w.y, w.width, w.height);

    isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
    settles(&isle, w.id, "tiled:left-half", LEFT_HALF).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
    settles(&isle, w.id, "tiled:left-two-thirds", (12, 60, 1260, 1008)).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
    settles(&isle, w.id, "tiled:left-third", (12, 60, 624, 1008)).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::RIGHT]).await;
    settles(&isle, w.id, "tiled:right-half", RIGHT_HALF).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::C]).await;
    settles(&isle, w.id, "tiled:centre", (391, 60, 1138, 1008)).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::C]).await;
    settles(&isle, w.id, "tiled:centre-third", (648, 60, 624, 1008)).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::U]).await;
    settles(&isle, w.id, "tiled:top-left", TOP_LEFT).await;

    isle.chord(&[keys::CTRL, keys::ALT, keys::R]).await;
    settles(&isle, w.id, "floating", before).await;
    client.wait_for("states activated").await;

    isle.chord(&[keys::CTRL, keys::ALT, keys::F]).await;
    settles(&isle, w.id, "filled", FILL).await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::F]).await;
    settles(&isle, w.id, "floating", before).await;
    // The bound keys never reach the window; their modifiers do.
    for key in [keys::LEFT, keys::RIGHT, keys::C, keys::F, keys::R, keys::U] {
        assert!(!client.printed(&format!("key {key} pressed")), "key {key}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_margin_is_the_size_chosen() {
    for (size, half) in [("none", (0, 48, 960, 1032)), ("large", (16, 64, 936, 1000))] {
        let mut isle = Isle::start_with(
            &format!("margin-{size}"),
            &format!("windows {{\n    tile-margin \"{size}\"\n}}\n"),
        )
        .await;
        let client = isle.client("isle.test.Margin", "ff0000");
        let w = isle.wait_for_window("isle.test.Margin").await;
        client.wait_for("keyboard-enter").await;
        isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
        settles(&isle, w.id, "tiled:left-half", half).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_maximises_and_resizes_from_its_left_edge() {
    let mut isle = Isle::start("resize").await;
    let client = isle.client("isle.test.Resize", "ff0000");
    let w = isle.wait_for_window("isle.test.Resize").await;
    let before = (w.x, w.y, w.width, w.height);

    client.send("maximize");
    settles(&isle, w.id, "filled", FILL).await;
    client.send("unmaximize");
    settles(&isle, w.id, "floating", before).await;

    client.send("resize-on-press left");
    let at = (w.x as f64 + 5.0, w.y as f64 + 150.0);
    isle.drag(&client, at, (at.0 - 100.0, at.1)).await;
    settles(
        &isle,
        w.id,
        "floating",
        (w.x - 100, w.y, w.width + 100, w.height),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_asking_to_be_maximised_opens_filled() {
    let mut isle = Isle::start("maxopen").await;
    let client = isle.client_with(&["--app-id", "isle.test.MaxOpen", "--do", "maximize"]);
    let w = isle.wait_for_window("isle.test.MaxOpen").await;
    assert_eq!(
        (w.mode.as_str(), w.x, w.y, w.width, w.height),
        ("filled", 12, 60, 1896, 1008)
    );
    let first = client
        .lines
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.starts_with("configured"))
        .cloned();
    assert_eq!(first.as_deref(), Some("configured 1896 1008"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_cmd_drag_moves_a_window_without_reaching_it() {
    let mut isle = Isle::start("ctrlcmd").await;
    let client = isle.client("isle.test.CtrlCmd", "ff0000");
    let w = isle.wait_for_window("isle.test.CtrlCmd").await;
    let test = isle.test().await;
    test.key(keys::CTRL, true).await.unwrap();
    test.key(keys::SUPER, true).await.unwrap();
    test.pointer_to(w.x as f64 + 200.0, w.y as f64 + 150.0)
        .await
        .unwrap();
    test.button(272, true).await.unwrap();
    test.pointer_to(w.x as f64 + 150.0, w.y as f64 + 250.0)
        .await
        .unwrap();
    test.button(272, false).await.unwrap();
    test.key(keys::SUPER, false).await.unwrap();
    test.key(keys::CTRL, false).await.unwrap();
    settles(
        &isle,
        w.id,
        "floating",
        (w.x - 50, w.y + 100, w.width, w.height),
    )
    .await;
    assert!(!client.printed("button 272 pressed"));
}

async fn keep_running(isle: &Isle, apps: &[&str]) {
    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap();
    let list: Vec<String> = apps.iter().map(|a| a.to_string()).collect();
    settings
        .set("windows.keep-running", &zvariant::Value::from(list))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_a_keep_running_apps_last_window_hides_it_and_quit_ends_it() {
    let mut isle = Isle::start("keep").await;
    keep_running(&isle, &["isle.test.Keep"]).await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let keep = isle.client_with(&["--app-id", "isle.test.Keep", "--do", "linger"]);
    let main = isle.wait_for_window("isle.test.Keep").await;
    keep.send("window");
    keep.wait_for("w1 configured 300 200").await;
    wait_until(|| async { isle.windows().await.len() == 3 }).await;
    let second = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.app == "isle.test.Keep" && w.id != main.id)
        .unwrap();

    let windows = isle.windows_proxy().await;
    // Not the last: it closes.
    assert!(windows.close(second.id).await.unwrap());
    keep.wait_for("w1 closed").await;
    wait_until(|| async { isle.windows().await.len() == 2 }).await;
    // The last: the app hides instead.
    assert!(windows.close(main.id).await.unwrap());
    wait_until(|| async { isle.window(main.id).await.hidden }).await;
    assert!(!keep.printed("closed"));

    // Quit closes it even so, and ends the process its windows left running.
    assert!(windows.quit_app("isle.test.Keep").await.unwrap());
    keep.wait_for("closed").await;
    wait_until(|| async { keep.exited() }).await;
    wait_until(|| async {
        windows
            .apps()
            .await
            .unwrap()
            .iter()
            .all(|a| a.app != "isle.test.Keep")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn hide_then_activate_brings_an_app_back_in_its_order() {
    let mut isle = Isle::start("hideapp").await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let app = isle.client("isle.test.Front", "ff0000");
    let main = isle.wait_for_window("isle.test.Front").await;
    app.send("window");
    app.wait_for("w1 keyboard-enter").await;

    isle.chord(&[keys::SUPER, 35]).await; // Cmd+H
    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .filter(|w| w.app == "isle.test.Front")
            .all(|w| w.hidden)
    })
    .await;
    assert!(
        isle.windows()
            .await
            .iter()
            .any(|w| w.app == "isle.test.Other" && w.focused)
    );
    let apps = isle.windows_proxy().await.apps().await.unwrap();
    assert_eq!(apps[0].app, "isle.test.Other", "most recent first");
    assert!(
        apps.iter()
            .any(|a| a.app == "isle.test.Front" && a.hidden && a.windows == 2)
    );

    assert!(
        isle.windows_proxy()
            .await
            .activate_app("isle.test.Front")
            .await
            .unwrap()
    );
    let listed = isle.windows().await;
    let front: Vec<&isle_ipc::WindowInfo> = listed.iter().take(2).collect();
    assert!(
        front
            .iter()
            .all(|w| w.app == "isle.test.Front" && !w.hidden)
    );
    assert!(
        front[0].focused && front[0].id != main.id,
        "the most recent in front, focused"
    );

    isle.chord(&[keys::SUPER, keys::ALT, 35]).await; // Cmd+Option+H
    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .any(|w| w.app == "isle.test.Other" && w.hidden)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_backtick_walks_the_front_apps_windows_in_a_stable_order() {
    let mut isle = Isle::start("cycle").await;
    let app = isle.client("isle.test.Cycle", "ff0000");
    let main = isle.wait_for_window("isle.test.Cycle").await;
    app.send("window");
    app.wait_for("w1 keyboard-enter").await;
    app.send("window");
    app.wait_for("w2 keyboard-enter").await;
    let ids: Vec<u64> = {
        let mut ids: Vec<u64> = isle.windows().await.iter().map(|w| w.id).collect();
        ids.sort();
        ids
    };
    assert_eq!(ids[0], main.id);
    let focused = || async {
        isle.windows()
            .await
            .into_iter()
            .find(|w| w.focused)
            .unwrap()
            .id
    };
    assert_eq!(focused().await, ids[2]);
    for want in [ids[0], ids[1], ids[2], ids[0]] {
        isle.chord(&[keys::SUPER, 41]).await; // Cmd+`
        wait_until(|| async { focused().await == want }).await;
    }
    isle.chord(&[keys::SUPER, keys::SHIFT, 41]).await;
    wait_until(|| async { focused().await == ids[2] }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_that_misses_a_ping_is_marked_until_it_answers() {
    let mut isle = Isle::start("ping").await;
    let client = isle.client("isle.test.Hung", "ff0000");
    let w = isle.wait_for_window("isle.test.Hung").await;
    client.send("pings ignore");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let test = isle.test().await;
    test.pointer_to(w.x as f64 + 50.0, w.y as f64 + 50.0)
        .await
        .unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    client.wait_for("ping ignored").await;
    assert!(!isle.window(w.id).await.not_responding);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !isle.window(w.id).await.not_responding {
        assert!(std::time::Instant::now() < deadline, "never marked");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    client.send("pings answer");
    wait_until(|| async { !isle.window(w.id).await.not_responding }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_belongs_to_the_app_whose_entry_claims_it() {
    let mut isle = Isle::start("entries").await;
    let _client = isle.client("isle.test.claimed", "ff0000");
    isle.wait_for_window("isle.test.claimed").await;
    std::fs::write(
        isle.dir.join("data/applications/isle-fixture.desktop"),
        "[Desktop Entry]\nType=Application\nName=Fixture\nStartupWMClass=isle.test.Claimed\n",
    )
    .unwrap();
    wait_until(|| async {
        let apps = isle.windows_proxy().await.apps().await.unwrap();
        apps.iter()
            .any(|a| a.app == "isle-fixture" && a.name == "Fixture")
    })
    .await;
}

async fn desktops(isle: &Isle) -> Vec<isle_ipc::DesktopInfo> {
    isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_arrows_move_between_desktops_and_ctrl_numbers_add_them() {
    let mut isle = Isle::start("desktops").await;
    let first = isle.client("isle.test.First", "ff0000");
    let a = isle.wait_for_window("isle.test.First").await;
    assert_eq!(a.desktop, 1);
    assert_eq!(desktops(&isle).await.len(), 1);

    isle.chord(&[keys::CTRL, 3]).await; // Ctrl+2
    wait_until(|| async { desktops(&isle).await.len() == 2 }).await;
    first.wait_for("states suspended").await;
    let second = isle.client("isle.test.Second", "00ff00");
    let b = isle.wait_for_window("isle.test.Second").await;
    assert_eq!(b.desktop, 2);
    assert_eq!(isle.shot("HEADLESS-1", "two").await.at(960, 540), GREEN);

    isle.chord(&[keys::CTRL, keys::LEFT]).await;
    second.wait_for("states suspended").await;
    wait_until(|| async { isle.window(a.id).await.focused }).await;
    assert_eq!(isle.shot("HEADLESS-1", "one").await.at(960, 540), RED);
    // No wrap past the first.
    isle.chord(&[keys::CTRL, keys::LEFT]).await;
    assert!(desktops(&isle).await[0].current);

    isle.chord(&[keys::CTRL, keys::RIGHT]).await;
    wait_until(|| async { isle.window(b.id).await.focused }).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    let saved = std::fs::read_to_string(isle.dir.join("state/isle/desktops.json")).unwrap();
    assert!(saved.contains("\"Isle Headless 1\": 2"), "{saved}");

    // Moved to the first desktop, the window and the view go there.
    let proxy = isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap();
    assert!(proxy.move_window(b.id, 1).await.unwrap());
    wait_until(|| async { isle.window(b.id).await.desktop == 1 }).await;
    assert!(desktops(&isle).await[0].current);
    assert!(proxy.remove("", 2).await.unwrap());
    assert!(
        !proxy.remove("", 1).await.unwrap(),
        "the last desktop stays"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn full_screen_gets_a_desktop_of_its_own_and_gives_it_back() {
    let mut isle = Isle::start("fullscreen").await;
    let client = isle.client("isle.test.Full", "ff0000");
    let w = isle.wait_for_window("isle.test.Full").await;
    let before = (w.x, w.y, w.width, w.height);

    client.send("fullscreen");
    settles(&isle, w.id, "fullscreen", (0, 0, 1920, 1080)).await;
    client.wait_for("states activated,fullscreen").await;
    let rows = desktops(&isle).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (rows[1].kind.as_str(), rows[1].window, rows[1].current),
        ("full-screen", w.id, true)
    );
    assert!(
        !isle.windows_proxy().await.minimise(w.id).await.unwrap()
            || !isle.window(w.id).await.minimised
    );

    client.send("unfullscreen");
    settles(&isle, w.id, "floating", before).await;
    let rows = desktops(&isle).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].current);

    // Closing it in full screen takes its desktop away too.
    client.send("fullscreen");
    wait_until(|| async { desktops(&isle).await.len() == 2 }).await;
    client.send("quit");
    wait_until(|| async { desktops(&isle).await.len() == 1 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_borderless_window_the_size_of_its_output_is_given_a_desktop_but_not_told() {
    let mut isle = Isle::start("borderless").await;
    let client = isle.client_with(&["--app-id", "isle.test.Game", "--size", "1920x1080"]);
    let w = isle.wait_for_window("isle.test.Game").await;
    wait_until(|| async { isle.window(w.id).await.mode == "fullscreen" }).await;
    assert_eq!(desktops(&isle).await.len(), 2);
    assert!(
        !client
            .lines
            .lock()
            .unwrap()
            .iter()
            .any(|l| l.contains("fullscreen"))
    );

    client.send("size 800 600");
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;
    assert_eq!(desktops(&isle).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn windows_keep_their_desktop_when_their_output_goes_and_comes_back() {
    let mut isle = Isle::start("deskout").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "AOC", "U34G2G4R3", "7", 1280, 720)
        .await
        .unwrap();
    test.pointer_to(1920.0 + 640.0, 360.0).await.unwrap();
    let proxy = isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap();
    assert_eq!(proxy.add("HEADLESS-2").await.unwrap(), 2);
    assert!(proxy.switch("HEADLESS-2", 2).await.unwrap());
    let _client = isle.client("isle.test.Away", "ff0000");
    let w = isle.wait_for_window("isle.test.Away").await;
    assert_eq!((w.output.as_str(), w.desktop), ("HEADLESS-2", 2));

    assert!(test.remove_output("HEADLESS-2").await.unwrap());
    wait_until(|| async { isle.window(w.id).await.output == "HEADLESS-1" }).await;
    assert_eq!(isle.window(w.id).await.desktop, 1);

    test.add_output("HEADLESS-3", "AOC", "U34G2G4R3", "7", 1280, 720)
        .await
        .unwrap();
    wait_until(|| async {
        let now = isle.window(w.id).await;
        now.output == "HEADLESS-3" && now.desktop == 2
    })
    .await;
}

const BACKDROP: [u8; 3] = [28, 28, 31];

fn darker(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().map(|c| *c as u32).sum::<u32>() < b.iter().map(|c| *c as u32).sum::<u32>()
}

#[tokio::test(flavor = "multi_thread")]
async fn windows_have_round_corners_and_a_shadow_deeper_while_active() {
    let mut isle = Isle::start("chrome").await;
    let _red = isle.client("isle.test.Corners", "ff0000");
    let w = isle.wait_for_window("isle.test.Corners").await;
    assert_eq!((w.x, w.y), (760, 414));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let shot = isle.shot("HEADLESS-1", "corners").await;
    assert_ne!(shot.at(760, 414), RED, "the corner is cut");
    assert!(
        shot.at(765, 419)[0] > 200,
        "inside the curve: {:?}",
        shot.at(765, 419)
    );
    assert_eq!(shot.at(771, 414), RED, "past the curve");
    let active = shot.at(960, 730);
    assert!(darker(active, BACKDROP), "a shadow under it: {active:?}");

    // Another window takes focus: the first one's shadow lightens.
    let _blue = isle.client_with(&[
        "--app-id",
        "isle.test.Other",
        "--colour",
        "0000ff",
        "--size",
        "100x100",
    ]);
    isle.wait_for_window("isle.test.Other").await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let inactive = isle.shot("HEADLESS-1", "inactive").await.at(960, 730);
    assert!(
        darker(active, inactive) && darker(inactive, BACKDROP),
        "{active:?} {inactive:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn going_full_screen_grows_in_place_while_its_desktop_slides_away() {
    let mut isle = Isle::start("fullscreen-switch").await;
    let test = isle.test().await;
    let _other = isle.client("isle.test.Other", "00ff00");
    let other = isle.wait_for_window("isle.test.Other").await.id;
    let client = isle.client("isle.test.Full", "ff0000");
    let full = isle.wait_for_window("isle.test.Full").await.id;
    tokio::time::sleep(Duration::from_millis(300)).await;
    test.window_motion(true).await.unwrap();
    let motions = || async {
        serde_json::from_str::<serde_json::Value>(&test.motions().await.unwrap()).unwrap()
    };
    let has = |v: &serde_json::Value, key: &str, id: u64| {
        v[key].as_array().unwrap().iter().any(|x| x.as_u64() == Some(id))
    };

    let until = |key: &'static str, id: u64| async move {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let seen = motions().await;
            if has(&seen, key, id) {
                return seen;
            }
            assert!(std::time::Instant::now() < deadline, "never {key} {id}: {seen}");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };

    client.send("fullscreen");
    let seen = until("morphing", full).await;
    assert!(!has(&seen, "moving", full), "the window grows, not slides: {seen}");
    assert!(seen["leaving"].as_u64().unwrap() >= 1, "its desktop slides away: {seen}");

    tokio::time::sleep(Duration::from_millis(800)).await;
    client.send("unfullscreen");
    let seen = until("moving", other).await;
    assert!(!has(&seen, "moving", full), "the window shrinks, not slides: {seen}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_closing_window_fades_from_its_last_frame_then_costs_nothing() {
    let mut isle = Isle::start("closing").await;
    let test = isle.test().await;
    test.window_motion(true).await.unwrap();
    let client = isle.client("isle.test.Closing", "ff0000");
    isle.wait_for_window("isle.test.Closing").await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let before = test.frames("HEADLESS-1").await.unwrap();

    client.send("destroy");
    client.wait_for("destroyed").await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let during = isle.shot("HEADLESS-1", "closing").await.at(960, 564);
    assert!(
        during[0] > during[1] + 40,
        "still drawn, fading: {during:?}"
    );
    tokio::time::sleep(Duration::from_millis(800)).await;
    let after = test.frames("HEADLESS-1").await.unwrap();
    assert!(after > before + 3, "the exit drew frames: {before} {after}");
    assert_eq!(
        isle.shot("HEADLESS-1", "closed").await.at(960, 564),
        BACKDROP
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        test.frames("HEADLESS-1").await.unwrap(),
        after,
        "nothing after it ends"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_null_buffer_unmaps_and_a_new_commit_maps_again() {
    let mut isle = Isle::start("unmap").await;
    let client = isle.client("isle.test.Unmap", "ff0000");
    let w = isle.wait_for_window("isle.test.Unmap").await;
    client.send("unmap");
    wait_until(|| async { isle.windows().await.is_empty() }).await;
    client.clear();
    client.send("remap");
    client.wait_for("configured 400 300").await;
    let again = isle.wait_for_window("isle.test.Unmap").await;
    assert_eq!(again.id, w.id);
}

/// A 400×300 client that asks for Isle's bar; its frame is 400×332, centred under the strip.
const FRAME: (i32, i32) = (760, 398);
const TITLEBAR: [u8; 3] = [0x25, 0x26, 0x2b];

async fn click(isle: &Isle, x: f64, y: f64) {
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    test.button(272, true).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    test.button(272, false).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
}

async fn barred(isle: &mut Isle, app: &str) -> (common::Client, isle_ipc::WindowInfo) {
    let client = isle.client_with(&[
        "--app-id",
        app,
        "--colour",
        "ff0000",
        "--do",
        "decoration server",
    ]);
    let w = isle.wait_for_window(app).await;
    client.wait_for("decoration server").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    (client, w)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_side_window_wears_isles_bar_and_a_client_side_one_does_not() {
    let mut isle = Isle::start("bar").await;
    let (client, w) = barred(&mut isle, "isle.test.Bar").await;
    assert_eq!(
        (w.x, w.y, w.width, w.height),
        (FRAME.0, FRAME.1 + 32, 400, 300)
    );
    let shot = isle.shot("HEADLESS-1", "bar").await;
    assert_eq!(shot.at(960, FRAME.1 as usize + 16), TITLEBAR, "the bar");
    assert_eq!(
        shot.at(960, FRAME.1 as usize + 100),
        RED,
        "the client under it"
    );
    assert_ne!(
        shot.at(FRAME.0 as usize + 22, FRAME.1 as usize + 16),
        TITLEBAR,
        "the close light"
    );

    // Hovering the group shows the glyphs: the close cross darkens the light's centre.
    let test = isle.test().await;
    test.pointer_to(FRAME.0 as f64 + 42.0, FRAME.1 as f64 + 16.0)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let hover = isle.shot("HEADLESS-1", "hover").await;
    let cross = hover.at(FRAME.0 as usize + 22, FRAME.1 as usize + 16);
    assert!(cross[0] < 0xc0, "the cross: {cross:?}");
    test.pointer_to(960.0, 700.0).await.unwrap();

    let tree = isle_ipc::shell::ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .tree("titlebar")
        .await
        .unwrap();
    for name in ["Close", "Minimise", "Enter Full Screen"] {
        assert!(tree.contains(name), "{name} in {tree}");
    }

    client.send("decoration client");
    client.wait_for("decoration client").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let shot = isle.shot("HEADLESS-1", "nobar").await;
    // The backdrop, darkened a little by the window's shadow.
    let above = shot.at(960, FRAME.1 as usize + 16);
    assert!(above[0] <= BACKDROP[0], "no bar: {above:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transparent_bar_draws_the_lights_alone_over_the_window() {
    let mut isle = Isle::start("bar-overlay").await;
    let (client, w) = barred(&mut isle, "isle.test.Overlay").await;
    client.send("titlebar-overlay 0 10");
    client.wait_for("controls 68 32").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let w = isle.window(w.id).await;
    assert_eq!((w.width, w.height), (400, 300), "no bar height added");

    let (x, y) = (w.x as usize, w.y as usize);
    let shot = isle.shot("HEADLESS-1", "overlay").await;
    assert_eq!(shot.at(x + 200, y + 16), RED, "the window's own toolbar shows through");
    assert_ne!(shot.at(x + 22, y + 26), RED, "the close light at the inset");
    assert_eq!(shot.at(x + 22, y + 4), RED, "nothing above the inset");

    click(&isle, w.x as f64 + 22.0, w.y as f64 + 26.0).await;
    client.wait_for("closed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_lights_close_minimise_and_zoom() {
    let mut isle = Isle::start("lights").await;
    let (_client, w) = barred(&mut isle, "isle.test.Lights").await;
    let (fx, fy) = (FRAME.0 as f64, FRAME.1 as f64);

    click(&isle, fx + 62.0, fy + 16.0).await;
    wait_until(|| async { isle.window(w.id).await.mode == "fullscreen" }).await;
    isle.windows_proxy().await.focus(w.id).await.unwrap();
    isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(w.id, false)
        .await
        .unwrap();
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;

    // Option-click on zoom fills.
    let test = isle.test().await;
    test.key(keys::ALT, true).await.unwrap();
    click(&isle, fx + 62.0, fy + 16.0).await;
    test.key(keys::ALT, false).await.unwrap();
    wait_until(|| async { isle.window(w.id).await.mode == "filled" }).await;
    isle.windows_proxy().await.restore(w.id).await.unwrap();
    wait_until(|| async { isle.window(w.id).await.y == FRAME.1 + 32 }).await;

    click(&isle, fx + 42.0, fy + 16.0).await;
    wait_until(|| async { isle.window(w.id).await.minimised }).await;
    isle.windows_proxy().await.unminimise(w.id).await.unwrap();

    click(&isle, fx + 22.0, fy + 16.0).await;
    _client.wait_for("closed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bar_moves_tiles_and_double_clicks_to_fill() {
    let mut isle = Isle::start("bardrag").await;
    let (_client, w) = barred(&mut isle, "isle.test.BarDrag").await;
    let test = isle.test().await;
    let (fx, fy) = (FRAME.0 as f64, FRAME.1 as f64);

    test.pointer_to(fx + 200.0, fy + 16.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    test.button(272, true).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    for step in 1..=4 {
        let f = step as f64 / 4.0;
        test.pointer_to(
            fx + 200.0 + (2.0 - fx - 200.0) * f,
            fy + 16.0 + (540.0 - fy - 16.0) * f,
        )
        .await
        .unwrap();
    }
    test.button(272, false).await.unwrap();
    // The left half's frame, with the client under its bar.
    settles(&isle, w.id, "tiled:left-half", (12, 92, 942, 976)).await;

    isle.windows_proxy().await.restore(w.id).await.unwrap();
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;
    let now = isle.window(w.id).await;
    let (bx, by) = (now.x as f64 + 200.0, now.y as f64 - 16.0);
    click(&isle, bx, by).await;
    click(&isle, bx, by).await;
    settles(&isle, w.id, "filled", (12, 92, 1896, 976)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tile_filled_from_its_bar_goes_back_to_the_tile() {
    let mut isle = Isle::start("filltile").await;
    let (client, w) = barred(&mut isle, "isle.test.FillTile").await;
    client.wait_for("keyboard-enter").await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::LEFT]).await;
    settles(&isle, w.id, "tiled:left-half", (12, 92, 942, 976)).await;
    // The bar sits over the frame's top, which a tile puts at the work area's.
    let (bx, by) = (212.0, 76.0);
    click(&isle, bx, by).await;
    click(&isle, bx, by).await;
    settles(&isle, w.id, "filled", (12, 92, 1896, 976)).await;
    click(&isle, bx, by).await;
    click(&isle, bx, by).await;
    settles(&isle, w.id, "tiled:left-half", (12, 92, 942, 976)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_barred_window_resizes_from_the_band_around_its_frame() {
    let mut isle = Isle::start("band").await;
    let (_client, w) = barred(&mut isle, "isle.test.Band").await;
    let test = isle.test().await;
    let x = (FRAME.0 + 400) as f64 + 2.0;
    let y = (FRAME.1 + 200) as f64;
    test.pointer_to(x, y).await.unwrap();
    test.button(272, true).await.unwrap();
    test.pointer_to(x + 60.0, y).await.unwrap();
    test.pointer_to(x + 100.0, y).await.unwrap();
    test.button(272, false).await.unwrap();
    settles(&isle, w.id, "floating", (w.x, w.y, 500, 300)).await;
}

async fn surface_open(isle: &Isle, name: &str) -> bool {
    isle_ipc::shell::ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .iter()
        .any(|s| s.name == name && s.open)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dwell_on_zoom_opens_its_menu_and_a_slot_tiles_there() {
    let mut isle = Isle::start("zoommenu").await;
    let (_client, w) = barred(&mut isle, "isle.test.Zoom").await;
    let test = isle.test().await;
    let (fx, fy) = (FRAME.0 as f64, FRAME.1 as f64);

    // Passing over the light opens nothing.
    test.pointer_to(fx + 62.0, fy + 16.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    test.pointer_to(fx + 200.0, fy + 16.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert!(!surface_open(&isle, "zoom-menu").await);

    test.pointer_to(fx + 62.0, fy + 16.0).await.unwrap();
    wait_until(|| async { surface_open(&isle, "zoom-menu").await }).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    isle.shot("HEADLESS-1", "menu").await;

    // The menu opens under the zoom light; the right half is the first thumbnail's second slot.
    let (mx, my) = (fx + 56.0, fy + 36.0);
    let (sx, sy) = (mx + 12.0 + 58.0, my + 12.0 + 14.0 + 4.0 + 22.0);
    test.pointer_to(sx - 4.0, sy).await.unwrap();
    test.pointer_to(sx, sy).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    settles(&isle, w.id, "tiled:right-half", (966, 92, 942, 976)).await;
    wait_until(|| async { !surface_open(&isle, "zoom-menu").await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_zoom_menu_by_its_chord_is_worked_by_keys_and_gives_focus_back() {
    let mut isle = Isle::start("zoomkeys").await;
    let (client, w) = barred(&mut isle, "isle.test.ZoomKeys").await;
    client.wait_for("keyboard-enter").await;
    isle.chord(&[keys::CTRL, keys::ALT, 44]).await; // Ctrl+Option+Z
    wait_until(|| async { surface_open(&isle, "zoom-menu").await }).await;
    client.wait_for("keyboard-leave").await;
    client.clear();
    tokio::time::sleep(Duration::from_millis(100)).await;
    isle.chord(&[keys::RIGHT]).await;
    isle.chord(&[28]).await; // Return
    settles(&isle, w.id, "tiled:left-half", (12, 92, 942, 976)).await;
    client.wait_for("keyboard-enter").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn option_in_a_drag_brings_the_zones_to_the_pointer() {
    let mut isle = Isle::start("panel").await;
    let (_client, w) = barred(&mut isle, "isle.test.Panel").await;
    let test = isle.test().await;
    let (fx, fy) = (FRAME.0 as f64, FRAME.1 as f64);
    test.pointer_to(fx + 200.0, fy + 16.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    test.button(272, true).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    test.pointer_to(fx + 220.0, fy + 30.0).await.unwrap();
    test.key(keys::ALT, true).await.unwrap();
    test.pointer_to(960.0, 540.0).await.unwrap();
    wait_until(|| async { surface_open(&isle, "zone-panel").await }).await;
    // The panel's grid starts 12 in from its corner at (984, 480); the left half's slot.
    test.pointer_to(1015.0, 514.0).await.unwrap();
    test.button(272, false).await.unwrap();
    test.key(keys::ALT, false).await.unwrap();
    settles(&isle, w.id, "tiled:left-half", (12, 92, 942, 976)).await;
    wait_until(|| async { !surface_open(&isle, "zone-panel").await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn with_title_bars_off_clients_are_told_to_draw_their_own() {
    let mut isle = Isle::start("barsoff").await;
    let (client, w) = barred(&mut isle, "isle.test.BarsOff").await;
    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap();
    settings
        .set("windows.title-bars", &zvariant::Value::from(false))
        .await
        .unwrap();
    client.wait_for("decoration client").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let w = isle.window(w.id).await;
    let above = isle
        .shot("HEADLESS-1", "off")
        .await
        .at((w.x + 300) as usize, (w.y - 16) as usize);
    assert_ne!(above, TITLEBAR, "no bar");

    client.clear();
    settings
        .set("windows.title-bars", &zvariant::Value::from(true))
        .await
        .unwrap();
    client.wait_for("decoration server").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let w = isle.window(w.id).await;
    let above = isle
        .shot("HEADLESS-1", "on")
        .await
        .at((w.x + 300) as usize, (w.y - 16) as usize);
    assert_eq!(above, TITLEBAR, "the bar again");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_that_cannot_grow_fills_to_its_largest_centred() {
    let mut isle = Isle::start("maxfill").await;
    let client = isle.client("isle.test.MaxFill", "3366ff");
    let w = isle.wait_for_window("isle.test.MaxFill").await;
    client.send("max 600 0");
    client.send("colour 3366ff");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let windows = isle.windows_proxy().await;
    assert!(windows.tile(w.id, "fill").await.unwrap());
    // The work area is 12 to 1908 across; 600 wide, centred in it.
    settles(&isle, w.id, "filled", (660, 60, 600, 1008)).await;
}
