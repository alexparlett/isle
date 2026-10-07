//! Bluetooth from BlueZ: discovery only while the page shows, a connected device's glyph and
//! battery, connecting and pairing, and BlueZ going and returning.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::bluez::{BlueZ, Device};

fn headset(connected: bool) -> Device {
    Device {
        id: "00_11_22_33_44_55".into(),
        name: "WH-1000XM6".into(),
        icon: "audio-headset".into(),
        paired: true,
        connected,
        battery: Some(70),
    }
}

async fn ready(isle: &Isle) {
    wait_until(|| async { isle.services().await["bluetooth"]["available"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn discovery_runs_only_while_the_page_shows_and_pairs_a_nearby_device() {
    let isle = Isle::start("bluetooth-discovery").await;
    let bluez = BlueZ::start(&isle.system_address(), vec![headset(false)])
        .await
        .unwrap();
    ready(&isle).await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("bluetooth").await["state"] == "On" }).await;
    assert!(!bluez.discovering().await);
    isle.open_panel(Some("bluetooth")).await;
    wait_until(|| async { bluez.discovering().await }).await;
    wait_until(|| async {
        isle.rows()
            .await
            .iter()
            .any(|r| r["label"] == "Nearby Speaker")
    })
    .await;

    let speaker = isle
        .rows()
        .await
        .iter()
        .find(|r| r["label"] == "Nearby Speaker")
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .unwrap();
    isle.panel(serde_json::json!({ "choose": ["bluetooth", speaker] }))
        .await;
    wait_until(|| async { bluez.paired("AA_BB_CC_00_11_22").await }).await;
    wait_until(|| async { bluez.connected("AA_BB_CC_00_11_22").await }).await;

    isle.panel(serde_json::json!({ "page": null })).await;
    wait_until(|| async { !bluez.discovering().await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_connected_device_shows_the_glyph_and_its_battery() {
    let isle = Isle::start("bluetooth-glyph").await;
    let bluez = BlueZ::start(&isle.system_address(), vec![headset(false)])
        .await
        .unwrap();
    ready(&isle).await;
    assert!(!isle.glyphs().await.contains(&"bluetooth".to_owned()));
    isle.open_panel(Some("bluetooth")).await;
    let row = isle
        .rows()
        .await
        .iter()
        .find(|r| r["label"] == "WH-1000XM6")
        .cloned()
        .unwrap();
    assert_eq!(row["battery"], 70);
    isle.panel(serde_json::json!({ "choose": ["bluetooth", row["id"]] }))
        .await;
    wait_until(|| async { bluez.connected("00_11_22_33_44_55").await }).await;
    wait_until(|| async { isle.glyphs().await.contains(&"bluetooth".to_owned()) }).await;
    wait_until(|| async { isle.module("bluetooth").await["state"] == "WH-1000XM6" }).await;

    isle.panel(serde_json::json!({ "toggle": "bluetooth" }))
        .await;
    wait_until(|| async { !bluez.powered().await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bluez_restarting_is_followed_again() {
    let isle = Isle::start("bluetooth-restart").await;
    let bluez = BlueZ::start(&isle.system_address(), vec![headset(true)])
        .await
        .unwrap();
    ready(&isle).await;
    wait_until(|| async { isle.glyphs().await.contains(&"bluetooth".to_owned()) }).await;
    let quiet = isle.log().len();
    bluez.vanish().await;
    wait_until(|| async { !isle.glyphs().await.contains(&"bluetooth".to_owned()) }).await;
    let _bluez = BlueZ::start(&isle.system_address(), vec![headset(true)])
        .await
        .unwrap();
    wait_until(|| async { isle.glyphs().await.contains(&"bluetooth".to_owned()) }).await;
    let stalls = isle.job_stalls_since(quiet);
    assert!(stalls.is_empty(), "{stalls:?}");
}
