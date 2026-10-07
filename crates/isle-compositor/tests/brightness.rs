//! Screen brightness from the Display module and the keys, and the keyboard backlight's OSD.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::{logind::Logind, upower::UPower};

const KEY_BRIGHTNESS_UP: u32 = 225;

#[tokio::test(flavor = "multi_thread")]
async fn a_screen_with_no_control_has_no_display_module() {
    let isle = Isle::start("brightness-none").await;
    isle.open_panel(None).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(isle.module("display").await.is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_keys_and_the_slider_set_the_screen_with_an_osd_for_the_keys() {
    let isle = Isle::start("brightness-set").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    isle.test()
        .await
        .fake_screen("HEADLESS-1", 50, 100)
        .await
        .unwrap();

    isle.chord(&[KEY_BRIGHTNESS_UP]).await;
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "event" && island["text"] == "Headless"
    })
    .await;
    // 50% and a sixteenth: 56 of 100.
    wait_until(|| async {
        logind.brightness().await.last() == Some(&("backlight".into(), "test".into(), 56))
    })
    .await;

    wait_until(|| async { isle.island_output().await["form"] != "event" }).await;
    isle.open_panel(Some("display")).await;
    let rows = isle.rows().await;
    assert_eq!(rows[0]["id"], "HEADLESS-1", "{rows:?}");
    isle.panel(serde_json::json!({ "slide": ["display", "", 0.25] }))
        .await;
    wait_until(|| async {
        logind.brightness().await.last() == Some(&("backlight".into(), "test".into(), 25))
    })
    .await;
    assert_ne!(isle.island_output().await["form"], "event");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_keyboard_backlight_change_raises_its_osd() {
    let isle = Isle::start("brightness-keyboard").await;
    let mut upower = UPower::start(&isle.system_address(), vec![]).await.unwrap();
    upower.set_keyboard(1).await.unwrap();
    wait_until(|| async {
        isle.services().await["power"]["state"]["keyboard"] == serde_json::json!([1, 3])
    })
    .await;
    assert_ne!(isle.island_output().await["form"], "event");
    upower.set_keyboard(2).await.unwrap();
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "event" && island["text"] == "Keyboard"
    })
    .await;
}
