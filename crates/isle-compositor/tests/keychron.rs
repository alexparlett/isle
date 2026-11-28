//! Keychron peripherals from their receivers: batteries beside UPower's, and OSDs for a
//! mouse's DPI and report rate. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::upower::UPower;
use serde_json::{Value, json};

fn mouse(stage: u8, rate_hz: u32) -> Value {
    json!({
        "product": 0xD056, "kind": "mouse", "name": "Keychron M7 8K", "usb": [],
        "connected": true, "battery": 30, "charging": false, "mac": null,
        "dpi": [400, 800, 1600, 3200, 5000], "stage": stage, "rate_hz": rate_hz, "seen": 1,
    })
}

fn keyboard() -> Value {
    json!({
        "product": 0x0861, "kind": "keyboard", "name": "Keychron Q6 Max", "usb": [],
        "connected": true, "battery": 88, "charging": true, "mac": true,
        "dpi": [], "stage": null, "rate_hz": null, "seen": 1,
    })
}

async fn report(isle: &Isle, peripherals: Value) {
    isle.test()
        .await
        .keychron(&peripherals.to_string())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn their_batteries_join_upowers() {
    let isle = Isle::start("keychron-batteries").await;
    let _upower = UPower::start(&isle.system_address(), vec![]).await.unwrap();
    report(&isle, json!([keyboard(), mouse(2, 1000)])).await;
    let names = || async {
        isle.services().await["power"]["state"]["batteries"]
            .as_array()
            .map(|b| {
                b.iter()
                    .map(|b| b["name"].as_str().unwrap_or_default().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    wait_until(|| async { names().await.len() == 2 }).await;
    assert_eq!(names().await, ["Keychron M7 8K", "Keychron Q6 Max"]);
    isle.open_panel(None).await;
    wait_until(|| async { !isle.module("batteries").await.is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dpi_button_and_report_rate_raise_osds() {
    let isle = Isle::start("keychron-osd").await;
    report(&isle, json!([mouse(2, 1000)])).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    // The first reading is no change.
    assert_ne!(isle.island_output().await["form"], "event");

    report(&isle, json!([mouse(3, 1000)])).await;
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "event" && island["text"] == "Keychron M7 8K 3200 DPI"
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("keychron-dpi-osd").await;

    report(&isle, json!([mouse(3, 2000)])).await;
    wait_until(|| async {
        isle.island_output().await["text"] == "Keychron M7 8K Report rate 2000 Hz"
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("keychron-rate-osd").await;
}
