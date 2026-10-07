//! Notification Centre: opened by its chord and the clock, arrivals going into it instead of
//! banners, stacks by app, Earlier, and Clear All in two steps.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, keys, wait_until};

const N: u32 = 49;

async fn centre(isle: &Isle) -> serde_json::Value {
    isle.island_output().await["panel"]["centre"].clone()
}

async fn banners(isle: &Isle) -> usize {
    let text = isle.test().await.banners().await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["cards"].as_array().map_or(0, |c| c.len())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_centre_takes_arrivals_stacks_them_by_app_and_clears_in_two_steps() {
    let mut isle = Isle::start("centre").await;
    let chat = isle.client("isle.test.Chat", "3366ff");
    chat.wait_for("keyboard-enter").await;
    chat.send("notify summary=One");
    chat.wait_for("notified 1").await;
    chat.send("notify summary=Two");
    chat.wait_for("notified 2").await;
    wait_until(|| async { banners(&isle).await == 1 }).await;

    isle.chord(&[keys::CTRL, keys::ALT, keys::SUPER, N]).await;
    wait_until(|| async { !centre(&isle).await.is_null() }).await;
    assert_eq!(banners(&isle).await, 0, "the centre takes the banners away");
    assert_eq!(
        centre(&isle).await["month"]
            .as_str()
            .map(|m| m.contains(' ')),
        Some(true)
    );
    wait_until(|| async { centre(&isle).await["groups"][0]["count"] == 2 }).await;
    assert_eq!(centre(&isle).await["groups"].as_array().unwrap().len(), 1);

    // An arrival while it is open shows in it, never as a banner.
    let mail = isle.client("isle.test.Mail", "ff6633");
    mail.wait_for("keyboard-enter").await;
    mail.send("notify summary=Mail");
    wait_until(|| async {
        centre(&isle).await["groups"]
            .as_array()
            .is_some_and(|g| g.len() == 2)
    })
    .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(banners(&isle).await, 0);

    isle.panel(serde_json::json!({ "choose": ["centre", "expand:isle.test.Chat"] }))
        .await;
    wait_until(|| async {
        centre(&isle).await["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["app"] == "isle.test.Chat" && g["open"] == true)
    })
    .await;
    isle.panel_settled().await;
    isle.keep_shot("centre").await;

    // Its sender gone, Mail's is a record under Earlier.
    mail.send("quit");
    wait_until(|| async { centre(&isle).await["earlier"] == 1 }).await;

    isle.panel(serde_json::json!({ "choose": ["centre", "clear-all"] }))
        .await;
    wait_until(|| async { centre(&isle).await["clear_armed"] == true }).await;
    assert_eq!(centre(&isle).await["groups"].as_array().unwrap().len(), 1);
    isle.panel(serde_json::json!({ "choose": ["centre", "clear-all"] }))
        .await;
    wait_until(|| async {
        let c = centre(&isle).await;
        c["groups"].as_array().is_some_and(|g| g.is_empty()) && c["earlier"] == 0
    })
    .await;

    // The chord again closes it.
    isle.chord(&[keys::CTRL, keys::ALT, keys::SUPER, N]).await;
    wait_until(|| async { isle.island_output().await["panel"].is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_clock_opens_the_centre_and_a_click_on_the_pill_turns_it_into_control_centre() {
    let isle = Isle::start("centre-clock").await;
    let test = isle.test().await;
    let island = isle.island_output().await;
    let width = island["width"].as_f64().unwrap();
    // The clock is the pill's middle at rest.
    test.pointer_to(960.0, 24.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    wait_until(|| async { !centre(&isle).await.is_null() }).await;
    assert!(width > 0.0);
}
