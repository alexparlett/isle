//! The widgets dashboard: opening, its keys, its pass over the screen and the surfaces it
//! gives way to. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, keys::*, wait_until};
use isle_ipc::shell::ShellProxy;

const ESCAPE: u32 = 1;
const E: u32 = 18;
const W: u32 = 17;
const SPACE: u32 = 57;
const DOWN: u32 = 108;
const END: u32 = 107;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.dashboard().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

async fn kinds(isle: &Isle) -> Vec<String> {
    isle.test().await.frame_kinds("HEADLESS-1").await.unwrap()
}

/// Ctrl+Option+Cmd+W.
async fn chord(isle: &Isle) {
    isle.chord(&[CTRL, ALT, SUPER, W]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_chord_opens_it_over_everything_and_gives_focus_back() {
    let mut isle = Isle::start("dashboard-open").await;
    isle.test().await.transitions(false).await.unwrap();
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let s = status(&isle).await;
    assert_eq!(s["grid"], serde_json::json!([21, 10]), "the headless output is 1920×1080");
    assert_eq!(s["layout"].as_array().unwrap().len(), 8, "the default block");
    assert_eq!(s["layout"][0], "clock#1 2 4 2 2");
    window.wait_for("keyboard-leave").await;
    wait_until(|| async {
        let k = kinds(&isle).await;
        k.contains(&"wallpaper".into()) && !k.contains(&"client".into())
    })
    .await;

    // Keys move the selection, with the ring, in reading order.
    press(&isle, END).await;
    wait_until(|| async { status(&isle).await["selected"] == "notifications#1" }).await;
    assert_eq!(status(&isle).await["ring"], true);

    // E enters edit mode; Escape leaves it, then closes.
    press(&isle, E).await;
    wait_until(|| async { status(&isle).await["edit"] == true }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["edit"] == false }).await;
    assert_eq!(status(&isle).await["open"], true);
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    window.wait_for("keyboard-enter").await;
    wait_until(|| async { kinds(&isle).await.contains(&"client".into()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_chord_again_closes_it_and_other_overlays_take_its_place() {
    let isle = Isle::start("dashboard-overlays").await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    // Cmd+Space: Spotlight closes the dashboard.
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let spotlight: serde_json::Value =
        serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap();
    assert_eq!(spotlight["open"], true);

    // And the dashboard closes Spotlight.
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let spotlight: serde_json::Value =
        serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap();
    assert_eq!(spotlight["open"], false);
    assert_eq!(status(&isle).await["edit"], true);
    assert!(!shell.widgets("close").await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn locking_closes_it_and_the_island_is_covered_over_it() {
    let isle = Isle::start("dashboard-lock").await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let island = isle.island_output().await;
    assert!(
        island["form"] == "nothing" || island["form"] == "privacy",
        "the island's covered form over the dashboard: {island}"
    );
    isle.test().await.dashboard_act("session lock").await.unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let lock: serde_json::Value =
        serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap();
    assert_eq!(lock["locked"], true);
    // Locked, the chord opens nothing.
    chord(&isle).await;
    press(&isle, DOWN).await;
    assert_eq!(status(&isle).await["open"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wallpapers_menu_opens_it_in_edit_mode() {
    const RIGHT_BUTTON: u32 = 0x111;
    const RETURN: u32 = 28;
    let isle = Isle::start("dashboard-wallpaper").await;
    isle.test().await.transitions(false).await.unwrap();
    let test = isle.test().await;
    test.pointer_to(900.0, 700.0).await.unwrap();
    test.button(RIGHT_BUTTON, true).await.unwrap();
    test.button(RIGHT_BUTTON, false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        shell.list().await.unwrap().iter().any(|s| s.name == "window-menu" && s.open)
    })
    .await;
    press(&isle, DOWN).await;
    press(&isle, DOWN).await;
    press(&isle, RETURN).await;
    wait_until(|| async { status(&isle).await["edit"] == true }).await;
    assert_eq!(status(&isle).await["output"], "HEADLESS-1");
}
