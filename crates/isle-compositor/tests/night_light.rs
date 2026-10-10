//! Night light: the schedule, Control Centre's tap and Game Mode's hold. Headless has no
//! gamma ramp, so the warmth is read from the state. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::displays::DisplaysProxy;

mod common;

use common::{Isle, wait_until};

async fn status(isle: &Isle) -> serde_json::Value {
    let text = DisplaysProxy::new(&isle.bus).await.unwrap().night_light().await.unwrap();
    serde_json::from_str(&text).unwrap()
}

const MANUAL: &str = "night-light {\n    schedule \"off\"\n    warmth 3000\n}\nwindows {\n    games \"isle.test.NightGame\"\n}\n";

#[tokio::test(flavor = "multi_thread")]
async fn a_tap_warms_the_screens_and_another_cools_them() {
    let isle = Isle::start_with("night-tap", MANUAL).await;
    let before = status(&isle).await;
    assert_eq!((before["on"].as_bool(), before["kelvin"].as_f64()), (Some(false), Some(6500.0)));
    assert_eq!(before["state"], "Off");

    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    displays.toggle_night_light().await.unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(3000.0) }).await;
    assert_eq!(status(&isle).await["state"], "On");
    isle.open_panel(None).await;
    assert_eq!(isle.module("night-light").await["on"], true);

    displays.toggle_night_light().await.unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(6500.0) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn game_mode_turns_it_off_at_once_and_gives_it_back() {
    let mut isle = Isle::start_with("night-game", MANUAL).await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    displays.toggle_night_light().await.unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(3000.0) }).await;

    let game = isle.client("isle.test.NightGame", "3366ff");
    isle.wait_for_window("isle.test.NightGame").await;
    wait_until(|| async { status(&isle).await["held-by-game"] == true }).await;
    let held = status(&isle).await;
    assert_eq!(held["kelvin"].as_f64(), Some(6500.0), "off at once, no fade");
    assert_eq!(held["state"], "Off during Game Mode");

    game.send("quit");
    wait_until(|| async { status(&isle).await["held-by-game"] == false }).await;
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(3000.0) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn turned_on_during_game_mode_it_stays_on() {
    let mut isle = Isle::start_with("night-game-user", MANUAL).await;
    let _game = isle.client("isle.test.NightGame", "3366ff");
    isle.wait_for_window("isle.test.NightGame").await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    displays.toggle_night_light().await.unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(3000.0) }).await;
    assert_eq!(status(&isle).await["held-by-game"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_warmth_setting_applies_live() {
    let isle = Isle::start_with("night-warmth", MANUAL).await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    displays.toggle_night_light().await.unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(3000.0) }).await;
    isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("night-light.warmth", &zvariant::Value::from(4200i64))
        .await
        .unwrap();
    wait_until(|| async { status(&isle).await["kelvin"].as_f64() == Some(4200.0) }).await;
}
