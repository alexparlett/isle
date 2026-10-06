#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::{
    desktops::DesktopsProxy, island::IslandProxy, menus::MenusProxy, settings::SettingsProxy,
};
use serde_json::Value;

const ESC: u32 = 1;
const CTRL: u32 = 29;
const F8: u32 = 66;
const F2: u32 = 60;
const LEFT: u32 = 272;

async fn island(isle: &Isle) -> Value {
    let text = IslandProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str::<Value>(&text).unwrap()["outputs"][0].clone()
}

async fn click(isle: &Isle, x: f64, y: f64) {
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT, true).await.unwrap();
    test.button(LEFT, false).await.unwrap();
}

/// Control Centre's settled shape, so its tiles are where the layout puts them.
async fn panel_settled(isle: &Isle) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let i = island(isle).await;
        let t = &i["target"];
        if !i["panel"].is_null()
            && (i["height"].as_f64().unwrap() - t[1].as_f64().unwrap()).abs() < 0.5
        {
            return i;
        }
        assert!(std::time::Instant::now() < deadline, "never settled: {i}");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rest_on_the_island_peeks_and_typing_stays_in_the_window() {
    let mut isle = Isle::start("cc-peek").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;
    test.pointer_to(960.0, 20.0).await.unwrap();
    wait_until(|| async { island(&isle).await["panel"]["pinned"] == false }).await;
    assert!(
        !client.printed("keyboard-leave"),
        "a peek takes no keyboard"
    );

    // Escape with the pointer resting there closes the peek, and the window never sees it.
    client.clear();
    isle.chord(&[ESC]).await;
    wait_until(|| async { island(&isle).await["panel"].is_null() }).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!client.printed(&format!("key {ESC} pressed")));

    // Leaving and coming back peeks again; leaving closes it after the grace.
    test.pointer_to(400.0, 500.0).await.unwrap();
    test.pointer_to(960.0, 20.0).await.unwrap();
    wait_until(|| async { !island(&isle).await["panel"].is_null() }).await;
    test.pointer_to(400.0, 500.0).await.unwrap();
    wait_until(|| async { island(&isle).await["panel"].is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_pins_with_the_keyboard_and_escape_gives_it_back() {
    let mut isle = Isle::start("cc-pin").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    click(&isle, 960.0, 24.0).await;
    wait_until(|| async { island(&isle).await["panel"]["pinned"] == true }).await;
    client.wait_for("keyboard-leave").await;
    client.clear();
    isle.chord(&[ESC]).await;
    wait_until(|| async { island(&isle).await["panel"].is_null() }).await;
    client.wait_for("keyboard-enter").await;

    isle.chord(&[CTRL, F8]).await;
    wait_until(|| async { island(&isle).await["panel"]["pinned"] == true }).await;
    isle.chord(&[CTRL, F8]).await;
    wait_until(|| async { island(&isle).await["panel"].is_null() }).await;

    // Pinned, it is the screen's one panel: the bar goes.
    let menus = MenusProxy::new(&isle.bus).await.unwrap();
    let bar = || async { serde_json::from_str::<Value>(&menus.bar().await.unwrap()).unwrap() };
    isle.chord(&[CTRL, F2]).await;
    wait_until(|| async { !bar().await["output"].is_null() }).await;
    isle.chord(&[CTRL, F8]).await;
    wait_until(|| async { island(&isle).await["panel"]["pinned"] == true }).await;
    wait_until(|| async { bar().await["output"].is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dark_mode_switches_the_appearance_and_keyboard_lists_its_layouts() {
    let isle = Isle::start("cc-modules").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    settings
        .set("keyboard.layouts", &zvariant::Value::from(vec!["us", "de"]))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.chord(&[CTRL, F8]).await;
    let panel = panel_settled(&isle).await;
    // The panel's left edge, then its padding and the pill's row: the first grid row.
    let left = 960.0 - panel["width"].as_f64().unwrap() / 2.0 + 8.0;
    let row = 9.0 + 30.0 + 8.0 + 32.0;
    click(&isle, left + 42.0, row).await;
    wait_until(|| async {
        let mode = settings.get("appearance.mode").await.unwrap();
        matches!(&*mode, zvariant::Value::Str(s) if s.as_str() == "light")
    })
    .await;

    click(&isle, left + 92.0 + 88.0, row).await;

    wait_until(|| async { island(&isle).await["panel"]["page"] == "keyboard-layout" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_push_at_the_edge_brings_the_island_back_over_full_screen() {
    let mut isle = Isle::start("cc-push").await;
    let _client = isle.client("isle.test.Full", "3366ff");
    let window = isle.wait_for_window("isle.test.Full").await;
    DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(window.id, true)
        .await
        .unwrap();
    wait_until(|| async { island(&isle).await["form"] == "nothing" }).await;
    let test = isle.test().await;
    test.pointer_to(960.0, 0.0).await.unwrap();
    test.pointer_by(0.0, -10.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        island(&isle).await["form"],
        "nothing",
        "a touch is not a push"
    );
    test.pointer_by(0.0, -20.0).await.unwrap();
    wait_until(|| async { island(&isle).await["form"] == "pill" }).await;
    // The return and a peek never fire from one dwell.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(island(&isle).await["panel"].is_null());
    test.pointer_to(400.0, 500.0).await.unwrap();
    wait_until(|| async { island(&isle).await["form"] == "nothing" }).await;
}
