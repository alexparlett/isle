//! Batteries and energy modes from UPower and power-profiles-daemon.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::upower::{Device, PowerProfiles, UPower};

fn laptop() -> Device {
    Device {
        id: "BAT0".into(),
        kind: 2,
        power_supply: true,
        model: "Battery".into(),
        percentage: 80.0,
        state: 2,
    }
}

fn mouse(percentage: f64) -> Device {
    Device {
        id: "mouse_hidpp".into(),
        kind: 5,
        power_supply: false,
        model: "MX Master 3S".into(),
        percentage,
        state: 2,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_desktop_with_no_battery_shows_neither_glyph_nor_module() {
    let isle = Isle::start("power-desktop").await;
    let _upower = UPower::start(&isle.system_address(), vec![]).await.unwrap();
    isle.open_panel(None).await;
    wait_until(|| async { isle.island_output().await["panel"]["modules"].is_array() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(isle.module("batteries").await.is_null());
    assert!(!isle.glyphs().await.contains(&"battery".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_laptop_on_its_battery_shows_the_glyph_and_a_low_mouse_does_too() {
    let isle = Isle::start("power-glyph").await;
    let mut upower = UPower::start(&isle.system_address(), vec![laptop(), mouse(60.0)])
        .await
        .unwrap();
    wait_until(|| async { isle.module_state_ready().await }).await;
    // On mains the laptop's battery is normal: no glyph.
    assert!(!isle.glyphs().await.contains(&"battery".to_owned()));
    upower.set_on_battery(true).await.unwrap();
    wait_until(|| async { isle.glyphs().await.contains(&"battery".to_owned()) }).await;
    upower.set_on_battery(false).await.unwrap();
    wait_until(|| async { !isle.glyphs().await.contains(&"battery".to_owned()) }).await;
    upower.set_percentage("mouse_hidpp", 12.0).await.unwrap();
    wait_until(|| async { isle.glyphs().await.contains(&"battery".to_owned()) }).await;

    isle.open_panel(Some("batteries")).await;
    let rows = isle.rows().await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["label"], "Battery");
    assert_eq!(rows[1]["label"], "MX Master 3S");
    assert_eq!(rows[1]["battery"], 12);
}

#[tokio::test(flavor = "multi_thread")]
async fn energy_mode_sets_the_profile_and_a_users_choice_ends_a_hold() {
    let isle = Isle::start("power-profiles").await;
    let ppd = PowerProfiles::start(&isle.system_address()).await.unwrap();
    isle.open_panel(Some("energy-mode")).await;
    wait_until(|| async { isle.module("energy-mode").await["state"] == "Automatic" }).await;
    isle.panel(serde_json::json!({ "choose": ["energy-mode", "power-saver"] }))
        .await;
    wait_until(|| async { ppd.active().await == "power-saver" }).await;
    wait_until(|| async { isle.module("energy-mode").await["state"] == "Low Power" }).await;

    // A hold of Isle's, as Game Mode makes, reads as Game Mode's.
    let conn = isle_fakes::connect(&isle.system_address()).await.unwrap();
    conn.call_method(
        Some("org.freedesktop.UPower.PowerProfiles"),
        "/org/freedesktop/UPower/PowerProfiles",
        Some("org.freedesktop.UPower.PowerProfiles"),
        "HoldProfile",
        &("performance", "Game Mode", "io.github.alexparlett.Isle"),
    )
    .await
    .unwrap();
    wait_until(|| async {
        isle.module("energy-mode").await["state"] == "High Power while Game Mode is on"
    })
    .await;
    isle.panel(serde_json::json!({ "choose": ["energy-mode", "balanced"] }))
        .await;
    wait_until(|| async { ppd.holds().await == 0 }).await;
    wait_until(|| async { isle.module("energy-mode").await["state"] == "Automatic" }).await;
}
