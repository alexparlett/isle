//! Wi-Fi and VPN from NetworkManager: the glyph's rule, scanning only while the page shows,
//! joining, a password NetworkManager does not have, VPNs, and the daemon going.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::network::{Ap, NetworkManager, Saved};

fn aps() -> Vec<Ap> {
    vec![
        Ap {
            ssid: "Home".into(),
            strength: 80,
            secured: true,
        },
        Ap {
            ssid: "Cafe".into(),
            strength: 50,
            secured: false,
        },
        Ap {
            ssid: "Office".into(),
            strength: 30,
            secured: true,
        },
    ]
}

/// A network's id in the folded "Other networks" group.
async fn other_network(isle: &Isle, ssid: &str) -> String {
    isle.rows()
        .await
        .iter()
        .find(|r| r["label"] == "Other networks")
        .and_then(|g| {
            g["rows"]
                .as_array()?
                .iter()
                .find(|r| r["label"] == ssid)
                .map(|r| r["id"].as_str().unwrap().to_owned())
        })
        .unwrap()
}

async fn ready(isle: &Isle) {
    wait_until(|| async { isle.services().await["network"]["available"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_glyph_shows_on_wifi_or_offline_and_not_on_a_wire() {
    let isle = Isle::start("network-glyph").await;
    let nm = NetworkManager::start(&isle.system_address(), aps(), vec![])
        .await
        .unwrap();
    ready(&isle).await;
    // Neither wire nor Wi-Fi: no connection at all.
    wait_until(|| async { isle.glyphs().await.contains(&"wifi".to_owned()) }).await;
    nm.plug_cable().await.unwrap();
    wait_until(|| async { !isle.glyphs().await.contains(&"wifi".to_owned()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_scans_while_open_and_joins_what_it_can() {
    let isle = Isle::start("network-join").await;
    let nm = NetworkManager::start(
        &isle.system_address(),
        aps(),
        vec![Saved::Wifi("Home".into())],
    )
    .await
    .unwrap();
    ready(&isle).await;
    wait_until(|| async { isle.glyphs().await.contains(&"wifi".to_owned()) }).await;
    assert_eq!(nm.scans(), 0);
    isle.open_panel(Some("wifi")).await;
    wait_until(|| async { nm.scans() >= 1 }).await;
    assert_eq!(isle.module("wifi").await["state"], "Home");

    let rows = isle.rows().await;
    let other = rows
        .iter()
        .find(|r| r["label"] == "Other networks")
        .unwrap();
    assert_eq!(other["open"], false);
    isle.panel(serde_json::json!({ "expand": ["wifi", "other"] }))
        .await;
    wait_until(|| async {
        isle.rows()
            .await
            .iter()
            .any(|r| r["label"] == "Other networks" && r["open"] == true)
    })
    .await;

    let cafe = other_network(&isle, "Cafe").await;
    isle.panel(serde_json::json!({ "choose": ["wifi", cafe] }))
        .await;
    wait_until(|| async { nm.current().as_deref() == Some("Cafe") }).await;
    wait_until(|| async { isle.module("wifi").await["state"] == "Cafe" }).await;

    // A secured network asks for its password; cancelled, its row says it needs one.
    let office = other_network(&isle, "Office").await;
    isle.panel(serde_json::json!({ "choose": ["wifi", office] }))
        .await;
    wait_until(|| async {
        let prompts: serde_json::Value =
            serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap();
        prompts["showing"].is_u64()
    })
    .await;
    let test = isle.test().await;
    test.key(1, true).await.unwrap();
    test.key(1, false).await.unwrap();
    wait_until(|| async {
        let rows = isle.rows().await;
        let group = rows
            .iter()
            .find(|r| r["label"] == "Other networks")
            .cloned()
            .unwrap();
        group["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["label"] == "Office" && r["error"] == "“Office” needs a password")
    })
    .await;
    assert_eq!(nm.current().as_deref(), Some("Cafe"));

    isle.panel(serde_json::json!({ "toggle": "wifi" })).await;
    wait_until(|| async { !nm.wifi_enabled() }).await;
    nm.changed().await.unwrap();
    wait_until(|| async { isle.module("wifi").await["state"] == "Off" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn vpns_connect_from_the_module_and_show_the_glyph() {
    let isle = Isle::start("network-vpn").await;
    let nm = NetworkManager::start(
        &isle.system_address(),
        aps(),
        vec![Saved::Wifi("Home".into()), Saved::Vpn("Work".into())],
    )
    .await
    .unwrap();
    ready(&isle).await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("vpn").await["state"] == "Not connected" }).await;
    isle.panel(serde_json::json!({ "toggle": "vpn" })).await;
    wait_until(|| async { nm.vpn_active() }).await;
    wait_until(|| async { isle.module("vpn").await["state"] == "Work" }).await;
    assert!(isle.glyphs().await.contains(&"vpn".to_owned()));
    isle.panel(serde_json::json!({ "toggle": "vpn" })).await;
    wait_until(|| async { !nm.vpn_active() }).await;
    wait_until(|| async { !isle.glyphs().await.contains(&"vpn".to_owned()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn network_manager_going_leaves_the_module_not_available_without_a_stall() {
    let isle = Isle::start("network-vanish").await;
    let nm = NetworkManager::start(
        &isle.system_address(),
        aps(),
        vec![Saved::Wifi("Home".into())],
    )
    .await
    .unwrap();
    ready(&isle).await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("wifi").await["state"] == "Home" }).await;
    let quiet = isle.log().len();
    nm.vanish().await;
    wait_until(|| async { isle.module("wifi").await["state"] == "Not available" }).await;
    let _nm = NetworkManager::start(
        &isle.system_address(),
        aps(),
        vec![Saved::Wifi("Home".into())],
    )
    .await
    .unwrap();
    wait_until(|| async { isle.module("wifi").await["state"] == "Home" }).await;
    let stalls = isle.job_stalls_since(quiet);
    assert!(stalls.is_empty(), "{stalls:?}");
}
