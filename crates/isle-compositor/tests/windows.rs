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
    assert!(
        windows
            .apps()
            .await
            .unwrap()
            .iter()
            .all(|a| a.app != "isle.test.Keep")
    );
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
