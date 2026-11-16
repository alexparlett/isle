//! Sticky Keys and Slow Keys. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::{desktops::DesktopsProxy, island::IslandProxy, settings::SettingsProxy};

mod common;

use common::{Isle, wait_until};

const CMD: u32 = 125;
const SHIFT: u32 = 42;
const CTRL: u32 = 29;
const TAB: u32 = 15;
const A: u32 = 30;

async fn tap(isle: &Isle, code: u32) {
    let test = isle.test().await;
    test.key(code, true).await.unwrap();
    test.key(code, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn sticky_cmd_holds_the_switcher_until_cmd_is_tapped_again() {
    let mut isle = Isle::start("sticky").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    settings
        .set("keyboard.sticky-keys-shortcut", &zvariant::Value::from(true))
        .await
        .unwrap();
    let client = isle.client("isle.test.Sticky", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;

    for _ in 0..5 {
        tap(&isle, SHIFT).await;
    }
    wait_until(|| async {
        settings.get("keyboard.sticky-keys").await.unwrap()
            == zvariant::Value::from(true).try_into().unwrap()
    })
    .await;

    tap(&isle, CMD).await;
    tap(&isle, TAB).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        0,
        "Cmd stays latched for the switcher"
    );
    tap(&isle, TAB).await;
    assert_eq!(test.held_releases().await.unwrap(), 0);
    tap(&isle, CMD).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        1,
        "the next Cmd tap commits"
    );

    client.clear();
    tap(&isle, A).await;
    client.wait_for("sym a released").await;
    assert!(!client.printed("key 29 pressed"), "nothing is latched now");
}

async fn sticky(settings: &SettingsProxy<'_>) -> bool {
    settings.get("keyboard.sticky-keys").await.unwrap()
        == zvariant::Value::from(true).try_into().unwrap()
}

async fn island(isle: &Isle) -> serde_json::Value {
    let text = IslandProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["outputs"][0].clone()
}

async fn island_text(isle: &Isle) -> serde_json::Value {
    island(isle).await["text"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_island_shows_each_latched_and_locked_modifier_over_full_screen_too() {
    let mut isle = Isle::start("sticky-shown").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.sticky-keys", &zvariant::Value::from(true))
        .await
        .unwrap();
    let _client = isle.client("isle.test.StickyShown", "3366ff");
    let window = isle.wait_for_window("isle.test.StickyShown").await;
    let latched = |s: serde_json::Value| s["latched"].clone();

    tap(&isle, CTRL).await;
    wait_until(|| async { latched(island(&isle).await) == serde_json::json!(["⌃"]) }).await;
    assert_eq!(island(&isle).await["form"], "pill");
    tap(&isle, CTRL).await;
    wait_until(|| async { latched(island(&isle).await) == serde_json::json!(["⌃ locked"]) })
        .await;

    DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(window.id, true)
        .await
        .unwrap();
    wait_until(|| async { isle.window(window.id).await.mode == "fullscreen" }).await;
    wait_until(|| async { island(&isle).await["form"] == "latched" }).await;

    tap(&isle, CTRL).await;
    wait_until(|| async { island(&isle).await["form"] == "nothing" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn shift_five_times_toggles_sticky_keys_only_with_the_shortcut_on_and_says_so() {
    let mut isle = Isle::start("sticky-shortcut").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let client = isle.client("isle.test.StickyShortcut", "3366ff");
    client.wait_for("keyboard-enter").await;

    for _ in 0..5 {
        tap(&isle, SHIFT).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!sticky(&settings).await, "the shortcut is off by default");

    settings
        .set("keyboard.sticky-keys-shortcut", &zvariant::Value::from(true))
        .await
        .unwrap();
    for _ in 0..5 {
        tap(&isle, SHIFT).await;
    }
    wait_until(|| async { sticky(&settings).await }).await;
    wait_until(|| async { island_text(&isle).await == "Sticky Keys On" }).await;

    for _ in 0..5 {
        tap(&isle, SHIFT).await;
    }
    wait_until(|| async { !sticky(&settings).await }).await;
    wait_until(|| async { island_text(&isle).await == "Sticky Keys Off" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn with_slow_keys_a_short_press_sends_nothing() {
    let mut isle = Isle::start("slow").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.slow-keys", &zvariant::Value::from(300i64))
        .await
        .unwrap();
    let client = isle.client("isle.test.Slow", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;
    client.clear();
    tap(&isle, A).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(
        !client.printed("key 30 pressed"),
        "a short press is dropped"
    );

    test.key(A, true).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    client.wait_for("key 30 pressed").await;
    test.key(A, false).await.unwrap();
    client.wait_for("key 30 released").await;
}
