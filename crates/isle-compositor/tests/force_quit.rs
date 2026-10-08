//! Force Quit: the panel lists every running app, a frozen one marked and selected, a
//! background one included; the sheet asks once and the kill ends the app; and the alert a
//! frozen window shows on close waits or force quits. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Client, Isle, wait_until};

const ESC: u32 = 1;
const W: u32 = 17;
const RETURN: u32 = 28;
const SHIFT: u32 = 42;
const OPTION: u32 = 56;
const DOWN: u32 = 108;
const CMD: u32 = 125;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.force_quit().await.unwrap()).unwrap()
}

/// A client that has stopped answering pings, marked once a click has pinged it.
async fn frozen(isle: &mut Isle, app_id: &str) -> (Client, u64) {
    let client = isle.client(app_id, "ff0000");
    let w = isle.wait_for_window(app_id).await;
    client.send("pings ignore");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let test = isle.test().await;
    test.pointer_to(w.x as f64 + 50.0, w.y as f64 + 50.0)
        .await
        .unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    client.wait_for("ping ignored").await;
    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    while !isle.window(w.id).await.not_responding {
        assert!(std::time::Instant::now() < deadline, "never marked");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    (client, w.id)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_frozen_app_is_marked_and_selected_and_force_quit_after_the_sheet() {
    let mut isle = Isle::start("force-quit-panel").await;
    let fine = isle.client("isle.test.Fine", "3366ff");
    fine.wait_for("keyboard-enter").await;
    let (hung, _) = frozen(&mut isle, "isle.test.Hung").await;

    isle.chord(&[CMD, OPTION, ESC]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let s = status(&isle).await;
    assert_eq!(s["selected"], serde_json::json!(["isle.test.Hung"]));
    let rows = s["rows"].as_array().unwrap();
    let row = |app: &str| rows.iter().find(|r| r["app"] == app).unwrap().clone();
    assert_eq!(row("isle.test.Hung")["not_responding"], true);
    assert_eq!(row("isle.test.Fine")["not_responding"], false);
    isle.keep_shot("force-quit").await;

    // Return asks; Return on the sheet is its Cancel.
    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["asking"] == true }).await;
    isle.keep_shot("force-quit-sheet").await;
    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["asking"] == false }).await;
    assert!(!hung.exited());

    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["asking"] == true }).await;
    isle.test().await.force_quit_act("confirm").await.unwrap();
    wait_until(|| async { hung.exited() }).await;
    wait_until(|| async {
        status(&isle).await["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["app"] != "isle.test.Hung")
    })
    .await;

    fine.clear();
    isle.chord(&[ESC]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    fine.wait_for("keyboard-enter").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_background_app_is_listed_and_shift_extends_the_selection() {
    let mut isle = Isle::start("force-quit-background").await;
    let behind = isle.client("isle.test.Behind", "ff6633");
    behind.wait_for("keyboard-enter").await;
    behind.send("linger");
    isle.chord(&[CMD, W]).await;
    behind.wait_for("closed").await;
    let front = isle.client("isle.test.Front", "3366ff");
    front.wait_for("keyboard-enter").await;

    isle.chord(&[CMD, OPTION, ESC]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let s = status(&isle).await;
    assert_eq!(s["selected"], serde_json::json!(["isle.test.Front"]));
    assert!(
        s["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["app"] == "isle.test.Behind"),
        "{s}"
    );
    isle.chord(&[SHIFT, DOWN]).await;
    wait_until(|| async { status(&isle).await["selected"].as_array().unwrap().len() == 2 }).await;

    isle.test()
        .await
        .force_quit_act("select isle.test.Behind")
        .await
        .unwrap();
    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["asking"] == true }).await;
    isle.test().await.force_quit_act("confirm").await.unwrap();
    wait_until(|| async { behind.exited() }).await;
    assert!(!front.exited());
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_a_frozen_window_brings_the_alert_which_waits_or_force_quits() {
    let mut isle = Isle::start("force-quit-alert").await;
    let (hung, id) = frozen(&mut isle, "isle.test.Hung").await;

    isle.chord(&[CMD, W]).await;
    wait_until(|| async { status(&isle).await["alerts"] == serde_json::json!([id]) }).await;
    let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
    assert!(kinds.iter().any(|k| k == "shell"), "{kinds:?}");
    isle.keep_shot("not-responding-alert").await;
    // Return is Wait.
    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["alerts"] == serde_json::json!([]) }).await;
    assert!(!hung.exited());

    // Answering again takes the alert down.
    isle.chord(&[CMD, W]).await;
    wait_until(|| async { status(&isle).await["alerts"] == serde_json::json!([id]) }).await;
    hung.send("pings answer");
    wait_until(|| async { status(&isle).await["alerts"] == serde_json::json!([]) }).await;

    let (hung2, id2) = frozen(&mut isle, "isle.test.Hung2").await;
    isle.chord(&[CMD, W]).await;
    wait_until(|| async { status(&isle).await["alerts"] == serde_json::json!([id2]) }).await;
    isle.test()
        .await
        .force_quit_act(&format!("force-quit {id2}"))
        .await
        .unwrap();
    wait_until(|| async { hung2.exited() }).await;
}
