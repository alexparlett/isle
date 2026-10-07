//! Focus: Do Not Disturb by its chord holding banners and the summary after, Game Mode for a
//! game in front with the profile it holds, and what passes.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, keys, wait_until};
use isle_fakes::upower::PowerProfiles;
use isle_ipc::{focus::FocusProxy, notifications::NotificationsProxy};

const D: u32 = 32;

async fn active(isle: &Isle) -> serde_json::Value {
    let text = FocusProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["active"].clone()
}

async fn cards(isle: &Isle) -> Vec<serde_json::Value> {
    let text = isle.test().await.banners().await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["cards"].as_array().cloned().unwrap_or_default()
}

async fn fates(isle: &Isle) -> Vec<String> {
    let text = NotificationsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["fate"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn do_not_disturb_holds_banners_and_the_summary_follows_its_end() {
    let mut isle = Isle::start("focus-dnd").await;
    let a = isle.client("isle.test.Chat", "3366ff");
    a.wait_for("keyboard-enter").await;
    isle.chord(&[keys::CTRL, keys::ALT, keys::SUPER, D]).await;
    wait_until(|| async { active(&isle).await == "Do Not Disturb" }).await;
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "event" && island["text"] == "Do Not Disturb on"
    })
    .await;
    wait_until(|| async { isle.glyphs().await.contains(&"focus".to_owned()) }).await;

    a.send("notify summary=One");
    a.wait_for("notified 1").await;
    let b = isle.client("isle.test.Mail", "ff6633");
    b.wait_for("keyboard-enter").await;
    b.send("notify summary=Two");
    b.wait_for("notified 2").await;
    // Critical ones pass a Focus that allows time-sensitive notifications.
    b.send("notify summary=Alarm urgency=2");
    b.wait_for("notified 3").await;
    wait_until(|| async { fates(&isle).await == ["held", "held", "banner"] }).await;
    assert_eq!(cards(&isle).await.len(), 1);
    isle.test()
        .await
        .banner(&serde_json::json!({ "close": 3 }).to_string())
        .await
        .unwrap();

    isle.chord(&[keys::CTRL, keys::ALT, keys::SUPER, D]).await;
    wait_until(|| async { active(&isle).await.is_null() }).await;
    wait_until(|| async {
        cards(&isle)
            .await
            .iter()
            .any(|c| c["summary"] == "While you were away")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_game_in_front_is_game_mode_holding_the_profile_until_it_leaves() {
    let mut isle = Isle::start("focus-game").await;
    let ppd = PowerProfiles::start(&isle.system_address()).await.unwrap();
    let desk = isle.client("isle.test.Desk", "3366ff");
    desk.wait_for("keyboard-enter").await;
    let game = isle.client_env("isle.test.Game", &[("SteamGameId", "440")]);
    game.wait_for("keyboard-enter").await;
    wait_until(|| async { active(&isle).await == "Game Mode" }).await;
    wait_until(|| async { ppd.holds().await == 1 && ppd.active().await == "performance" }).await;

    desk.send("notify summary=Ping");
    desk.wait_for("notified 1").await;
    wait_until(|| async { fates(&isle).await == ["held"] }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(cards(&isle).await.is_empty());

    // The game leaves the front: the one held comes back as its own banner.
    game.send("quit");
    wait_until(|| async { active(&isle).await.is_null() }).await;
    wait_until(|| async { ppd.holds().await == 0 }).await;
    wait_until(|| async { cards(&isle).await.iter().any(|c| c["summary"] == "Ping") }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn control_centres_focus_turns_the_last_mode_on_without_an_event() {
    let isle = Isle::start("focus-module").await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("focus").await["state"] == "Off" }).await;
    isle.panel(serde_json::json!({ "toggle": "focus" })).await;
    wait_until(|| async { active(&isle).await == "Do Not Disturb" }).await;
    assert_eq!(isle.module("focus").await["name"], "Do Not Disturb");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_ne!(isle.island_output().await["text"], "Do Not Disturb on");
    isle.open_panel(Some("focus")).await;
    isle.panel(serde_json::json!({ "choose": ["focus", "Do Not Disturb"] }))
        .await;
    wait_until(|| async { active(&isle).await.is_null() }).await;
}
