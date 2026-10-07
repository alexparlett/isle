//! Cmd+Tab's switcher: the hold, its keys and the commit on release. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Client, Isle, wait_until};
use serde_json::Value;

const CMD: u32 = 125;
const SHIFT: u32 = 42;
const TAB: u32 = 15;
const ESCAPE: u32 = 1;
const RETURN: u32 = 28;
const Q: u32 = 16;
const W: u32 = 17;
const H: u32 = 35;
const GRAVE: u32 = 41;
const DOWN: u32 = 108;
const F8: u32 = 66;
const CTRL: u32 = 29;

async fn switcher(isle: &Isle) -> Value {
    serde_json::from_str(&isle.test().await.switcher().await.unwrap()).unwrap()
}

async fn focused_app(isle: &Isle) -> Option<String> {
    isle.windows()
        .await
        .into_iter()
        .find(|w| w.focused)
        .map(|w| w.app)
}

async fn press(isle: &Isle, key: u32) {
    isle.test().await.key(key, true).await.unwrap();
}

async fn release(isle: &Isle, key: u32) {
    isle.test().await.key(key, false).await.unwrap();
}

async fn tap(isle: &Isle, key: u32) {
    press(isle, key).await;
    release(isle, key).await;
}

/// Three apps mapped in order, so the last is in front and the first is last.
async fn three(isle: &mut Isle) -> Vec<Client> {
    let mut clients = Vec::new();
    for (app, colour) in [
        ("isle.test.A", "ff0000"),
        ("isle.test.B", "00ff00"),
        ("isle.test.C", "0000ff"),
    ] {
        let client = isle.client(app, colour);
        let id = isle.wait_for_window(app).await.id;
        wait_until(|| async { isle.window(id).await.focused }).await;
        clients.push(client);
    }
    clients
}

#[tokio::test(flavor = "multi_thread")]
async fn a_quick_cmd_tab_switches_to_the_previous_app_and_draws_nothing() {
    let mut isle = Isle::start("switchquick").await;
    let clients = three(&mut isle).await;
    for c in &clients {
        c.clear();
    }
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.B");
    assert_eq!(
        focused_app(&isle).await.as_deref(),
        Some("isle.test.C"),
        "nothing moves yet"
    );
    release(&isle, CMD).await;
    assert!(switcher(&isle).await.is_null());
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.B"));
    assert_eq!(isle.test().await.switcher_draws().await.unwrap(), 0);

    // And back again.
    isle.chord(&[CMD, TAB]).await;
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.C"));
    for c in &clients {
        assert!(
            !c.printed(&format!("key {TAB} pressed")),
            "Tab never reaches an app"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn held_it_draws_moves_and_escape_leaves_focus_where_it_was() {
    let mut isle = Isle::start("switchheld").await;
    let clients = three(&mut isle).await;
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let s = switcher(&isle).await;
    assert_eq!(s["drawn"], true);
    assert_eq!(s["entries"].as_array().unwrap().len(), 3);
    tap(&isle, TAB).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.A");
    press(&isle, SHIFT).await;
    tap(&isle, TAB).await;
    release(&isle, SHIFT).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.B");
    tap(&isle, ESCAPE).await;
    assert!(switcher(&isle).await.is_null());
    release(&isle, CMD).await;
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.C"));
    assert_eq!(isle.test().await.switcher_draws().await.unwrap(), 1);
    for c in &clients {
        assert!(!c.printed(&format!("key {ESCAPE} pressed")));
    }

    // Shift+Cmd+Tab opens from the end; Return commits with Cmd still down.
    press(&isle, CMD).await;
    press(&isle, SHIFT).await;
    tap(&isle, TAB).await;
    release(&isle, SHIFT).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.A");
    tap(&isle, RETURN).await;
    assert!(switcher(&isle).await.is_null());
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.A"));
    release(&isle, CMD).await;
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.A"));
}

#[tokio::test(flavor = "multi_thread")]
async fn q_h_and_w_act_on_the_selected_app_and_never_reach_the_one_in_front() {
    let mut isle = Isle::start("switchkeys").await;
    let clients = three(&mut isle).await;
    let (a, b, c) = (&clients[0], &clients[1], &clients[2]);
    b.send("window");
    b.wait_for("w1 configured 300 200").await;
    let windows = isle.windows_proxy().await;
    windows.activate_app("isle.test.C").await.unwrap();
    wait_until(|| async { focused_app(&isle).await.as_deref() == Some("isle.test.C") }).await;
    c.clear();

    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.B");
    // W closes B's front window; H hides B; Q on A quits it.
    tap(&isle, W).await;
    wait_until(|| async { b.printed("w1 closed") || b.printed("closed") }).await;
    tap(&isle, H).await;
    wait_until(|| async {
        windows
            .apps()
            .await
            .unwrap()
            .iter()
            .any(|x| x.app == "isle.test.B" && x.hidden)
    })
    .await;
    tap(&isle, TAB).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.A");
    tap(&isle, Q).await;
    a.wait_for("closed").await;
    wait_until(|| async { a.exited() }).await;
    // The quit app went: the selection moved on, and the switcher is still held.
    wait_until(|| async {
        let s = switcher(&isle).await;
        !s.is_null() && s["entries"].as_array().unwrap().len() == 2
    })
    .await;
    release(&isle, CMD).await;
    for key in [Q, W, H] {
        assert!(
            !c.printed(&format!("key {key} pressed")),
            "{key} reached the app in front"
        );
    }
    assert!(c.printed("mods none") || !c.printed("mods Mod4"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_window_row_commits_the_window_chosen_with_grave() {
    let mut isle = Isle::start("switchrow").await;
    let clients = three(&mut isle).await;
    let b = &clients[1];
    b.send("window");
    b.wait_for("w1 configured 300 200").await;
    let b_windows: Vec<u64> = {
        let mut ids: Vec<u64> = isle
            .windows()
            .await
            .into_iter()
            .filter(|w| w.app == "isle.test.B")
            .map(|w| w.id)
            .collect();
        ids.sort();
        ids
    };
    // B's second window is in front now; C is the previous app.
    isle.windows_proxy()
        .await
        .activate_app("isle.test.C")
        .await
        .unwrap();
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    assert_eq!(switcher(&isle).await["selected"], "isle.test.B");
    tap(&isle, DOWN).await;
    assert_eq!(
        switcher(&isle).await["row"],
        1,
        "opens on its most recent window"
    );
    tap(&isle, GRAVE).await;
    assert_eq!(switcher(&isle).await["row"], 0);
    release(&isle, CMD).await;
    let focused = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.focused)
        .unwrap();
    assert_eq!(focused.id, b_windows[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_thousand_switches_commit_on_release_and_leave_no_modifier_held() {
    let mut isle = Isle::start("switchstress").await;
    let clients = three(&mut isle).await;
    let windows = isle.windows_proxy().await;
    let ids: Vec<(String, u64)> = isle
        .windows()
        .await
        .into_iter()
        .map(|w| (w.app, w.id))
        .collect();
    let mut order: Vec<String> = ["isle.test.C", "isle.test.B", "isle.test.A"]
        .map(String::from)
        .to_vec();
    let mut seed: u64 = 0x9e3779b97f4a7c15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..1000 {
        let tabs = 1 + (next() % 3) as usize;
        press(&isle, CMD).await;
        for _ in 0..tabs {
            tap(&isle, TAB).await;
        }
        let target = order[tabs % order.len()].clone();
        if next() % 25 == 0 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        // Now and then a window takes focus mid-chord; the selection stays.
        if next() % 10 == 0 {
            let other = order[(tabs + 1) % order.len()].clone();
            let id = ids.iter().find(|(a, _)| *a == other).unwrap().1;
            windows.focus(id).await.unwrap();
            order.retain(|a| *a != other);
            order.insert(0, other);
        }
        release(&isle, CMD).await;
        order.retain(|a| *a != target);
        order.insert(0, target.clone());
        assert_eq!(
            focused_app(&isle).await.as_deref(),
            Some(target.as_str()),
            "round {round}"
        );
        assert!(switcher(&isle).await.is_null(), "round {round}");
    }
    for c in &clients {
        assert!(!c.printed(&format!("key {TAB} pressed")));
        let last = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|l| l.starts_with("mods "))
            .cloned();
        // Num Lock (Mod2) is a lock, not a key held.
        let held = last
            .iter()
            .flat_map(|l| l["mods ".len()..].split('+'))
            .any(|m| ["Shift", "Control", "Mod1", "Mod4"].contains(&m));
        assert!(!held, "a modifier stuck: {last:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_the_keyboard_mid_hold_cancels() {
    let mut isle = Isle::start("switchunplug").await;
    let _clients = three(&mut isle).await;
    let test = isle.test().await;
    let id = test.add_keyboard("Apple Magic Keyboard").await.unwrap();
    test.key_on(&id, CMD, true).await.unwrap();
    test.key_on(&id, TAB, true).await.unwrap();
    test.key_on(&id, TAB, false).await.unwrap();
    assert!(!switcher(&isle).await.is_null());
    test.remove_keyboard(&id).await.unwrap();
    assert!(switcher(&isle).await.is_null());
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.C"));
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_tab_closes_control_centre_and_the_bar() {
    let mut isle = Isle::start("switchcloses").await;
    let _clients = three(&mut isle).await;
    isle.chord(&[CTRL, F8]).await;
    let island = || async {
        let text = isle_ipc::island::IslandProxy::new(&isle.bus)
            .await
            .unwrap()
            .status()
            .await
            .unwrap();
        serde_json::from_str::<Value>(&text).unwrap()["outputs"][0].clone()
    };
    wait_until(|| async { island().await["panel"]["pinned"] == true }).await;
    isle.chord(&[CMD, TAB]).await;
    wait_until(|| async { island().await["panel"].is_null() }).await;
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.B"));
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_tab_leaves_a_full_screen_game_and_escape_gives_it_back_its_keys() {
    let mut isle = Isle::start("switchgame").await;
    let _other = isle.client("isle.test.Other", "ff0000");
    isle.wait_for_window("isle.test.Other").await;
    let game = isle.client("steam_app_1", "3366ff");
    let id = isle.wait_for_window("steam_app_1").await.id;
    wait_until(|| async { isle.window(id).await.game }).await;
    game.send("fullscreen");
    wait_until(|| async { isle.window(id).await.mode == "fullscreen" }).await;
    wait_until(|| async { isle.window(id).await.focused }).await;

    // Held and cancelled, the game keeps the keyboard and gets the next key.
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(switcher(&isle).await["drawn"], true);
    tap(&isle, ESCAPE).await;
    release(&isle, CMD).await;
    assert!(isle.window(id).await.focused);
    game.clear();
    tap(&isle, Q).await;
    game.wait_for(&format!("key {Q} pressed")).await;

    // A quick tap leaves its full-screen desktop for the other app.
    isle.chord(&[CMD, TAB]).await;
    assert_eq!(focused_app(&isle).await.as_deref(), Some("isle.test.Other"));
    let other = isle.wait_for_window("isle.test.Other").await;
    let shown = isle.windows().await;
    assert_ne!(
        shown.iter().find(|w| w.id == id).unwrap().desktop,
        other.desktop,
        "the game keeps its own desktop"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_panel_shows_each_apps_live_preview_and_goes_on_release() {
    let mut isle = Isle::start("switchpanel").await;
    let clients = three(&mut isle).await;
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    // Three plates of 272 in a panel 840 wide, centred on 1920 by 1080; the cards' centres.
    let (y, xs) = (507, [684, 960, 1236]);
    let shot = isle.shot("HEADLESS-1", "held").await;
    assert_eq!(shot.at(xs[0], y), [0, 0, 255], "C, in front, first");
    assert_eq!(shot.at(xs[1], y), [0, 255, 0], "B selected");
    assert_eq!(shot.at(xs[2], y), [255, 0, 0], "A last");
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    let tree = shell.tree("switcher").await.unwrap();
    assert!(
        tree.contains("App switcher") && tree.contains("isle.test.B"),
        "{tree}"
    );

    // A window that draws while held is drawn again in its card.
    clients[1].send("colour ffff00");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let shot = isle.shot("HEADLESS-1", "recoloured").await;
    assert_eq!(shot.at(xs[1], y), [255, 255, 0]);

    release(&isle, CMD).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let shot = isle.shot("HEADLESS-1", "released").await;
    assert_ne!(shot.at(xs[2], y), [255, 0, 0], "the panel has gone");
    let open = shell.list().await.unwrap();
    assert!(open.iter().all(|s| s.name != "switcher" || !s.open));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "a screenshot to look at: ISLE_TEST_KEEP=1"]
async fn marks_by_eye() {
    let mut isle = Isle::start("switchmarks").await;
    let bg = isle.client_with(&[
        "--app-id",
        "isle.test.Background",
        "--colour",
        "888888",
        "--do",
        "linger",
    ]);
    let bgw = isle.wait_for_window("isle.test.Background").await;
    let windows = isle.windows_proxy().await;
    windows.close(bgw.id).await.unwrap();
    bg.wait_for("closed").await;
    let clients = three(&mut isle).await;
    clients[1].send("window");
    clients[1].wait_for("w1 configured 300 200").await;
    clients[1].send("badge count=4 progress=0.4");
    clients[1].wait_for("badge sent").await;
    windows.set_app_hidden("isle.test.A", true).await.unwrap();
    windows.activate_app("isle.test.C").await.unwrap();
    press(&isle, CMD).await;
    tap(&isle, TAB).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    isle.shot("HEADLESS-1", "marks").await;
    tap(&isle, DOWN).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    isle.shot("HEADLESS-1", "row").await;
    release(&isle, CMD).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drag_carries_through_cmd_tab_into_the_app_brought_forward() {
    let mut isle = Isle::start("switchdrag").await;
    let target = isle.client("isle.test.Target", "00ff00");
    let target_id = isle.wait_for_window("isle.test.Target").await.id;
    let source = isle.client_with(&["--app-id", "isle.test.Source", "--colour", "ff0000"]);
    let source_window = isle.wait_for_window("isle.test.Source").await;
    wait_until(|| async { isle.window(source_window.id).await.focused }).await;
    source.send("drag-on-press");
    let test = isle.test().await;
    let (x, y) = (source_window.x as f64 + 50.0, source_window.y as f64 + 50.0);
    test.pointer_to(x, y).await.unwrap();
    test.button(272, true).await.unwrap();
    source.wait_for("drag started").await;
    test.pointer_to(x + 10.0, y + 10.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let shot = isle.shot("HEADLESS-1", "dragging").await;
    assert_eq!(
        shot.at(x as usize + 20, y as usize + 20),
        [255, 0, 255],
        "the icon is drawn"
    );

    // Cmd+Tab while dragging brings the target forward; the drop lands in it.
    isle.chord(&[CMD, TAB]).await;
    wait_until(|| async { isle.window(target_id).await.focused }).await;
    let t = isle.window(target_id).await;
    test.pointer_to(t.x as f64 + 40.0, t.y as f64 + 40.0)
        .await
        .unwrap();
    target.wait_for("dnd enter").await;
    test.button(272, false).await.unwrap();
    target.wait_for("dnd drop").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let shot = isle.shot("HEADLESS-1", "dropped").await;
    assert_ne!(
        shot.at(t.x as usize + 40, t.y as usize + 40),
        [255, 0, 255],
        "the icon goes"
    );
}
