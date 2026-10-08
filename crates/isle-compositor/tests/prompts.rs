//! System prompts: one on screen over a scrim with the keyboard, armed after a moment,
//! waiting while locked and when set aside, the queue in order. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

const RETURN: u32 = 28;
const CMD: u32 = 125;
const TAB: u32 = 15;
const P: u32 = 25;
const W: u32 = 17;

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

async fn ask(isle: &Isle, json: serde_json::Value) -> u64 {
    isle.test()
        .await
        .prompt_ask(&json.to_string())
        .await
        .unwrap()
}

fn password_prompt(title: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "polkit",
        "title": title,
        "message": "Authentication is needed to run a test.",
        "fields": [{"id": "password", "label": "Password for test", "secret": true}],
        "action": "Allow",
        "details": "Action: io.github.alexparlett.Isle.test",
    })
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prompt_takes_the_keyboard_and_answers_only_once_armed() {
    let mut isle = Isle::start("prompt-armed").await;
    let window = isle.client("isle.test.Behind", "3366ff");
    window.wait_for("keyboard-enter").await;
    let id = ask(&isle, password_prompt("Test needs your password")).await;
    window.wait_for("keyboard-leave").await;
    wait_until(|| async { prompts(&isle).await["showing"] == id }).await;
    assert_eq!(prompts(&isle).await["armed"], false);
    press(&isle, P).await;
    press(&isle, W).await;
    press(&isle, RETURN).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        prompts(&isle).await["answers"],
        serde_json::json!([]),
        "Return before the armed moment answers nothing"
    );
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.keep_shot("prompt").await;
    press(&isle, RETURN).await;
    wait_until(|| async {
        prompts(&isle).await["answers"] == serde_json::json!([format!("{id} password=pw")])
    })
    .await;
    window.wait_for("keyboard-enter").await;
    assert!(
        !window.printed("key 25"),
        "a key typed into the prompt reached the window"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prompt_waits_while_locked_and_comes_back_armed_afresh() {
    let isle = Isle::start("prompt-locked").await;
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    let id = ask(&isle, password_prompt("Test needs your password")).await;
    wait_until(|| async {
        prompts(&isle).await["waiting"] == serde_json::json!([{"id": id, "why": "locked"}])
    })
    .await;
    assert!(prompts(&isle).await["showing"].is_null());
    isle.test().await.unlock().await.unwrap();
    wait_until(|| async { prompts(&isle).await["showing"] == id }).await;
    assert_eq!(
        prompts(&isle).await["armed"],
        false,
        "armed afresh after unlock"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_tab_sets_a_prompt_aside_and_show_brings_it_back() {
    let mut isle = Isle::start("prompt-aside").await;
    let window = isle.client("isle.test.Behind", "3366ff");
    window.wait_for("keyboard-enter").await;
    let id = ask(&isle, password_prompt("Test needs your password")).await;
    wait_until(|| async { prompts(&isle).await["showing"] == id }).await;
    let test = isle.test().await;
    test.key(CMD, true).await.unwrap();
    test.key(TAB, true).await.unwrap();
    test.key(TAB, false).await.unwrap();
    test.key(CMD, false).await.unwrap();
    wait_until(|| async {
        prompts(&isle).await["waiting"] == serde_json::json!([{"id": id, "why": "setaside"}])
    })
    .await;
    test.prompt_activity("show").await.unwrap();
    wait_until(|| async { prompts(&isle).await["showing"] == id }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_users_own_request_goes_first_and_a_cancel_moves_the_queue_on() {
    let isle = Isle::start("prompt-queue").await;
    let first = ask(&isle, password_prompt("First")).await;
    let second = ask(&isle, password_prompt("Second")).await;
    let mine = {
        let mut p = password_prompt("Mine");
        p["user_started"] = true.into();
        ask(&isle, p).await
    };
    wait_until(|| async { prompts(&isle).await["showing"] == first }).await;
    isle.test().await.prompt_done(first).await.unwrap();
    wait_until(|| async { prompts(&isle).await["showing"] == mine }).await;
    isle.test().await.prompt_done(second).await.unwrap();
    assert_eq!(prompts(&isle).await["waiting"], serde_json::json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_cannot_take_focus_or_clicks_from_a_prompt() {
    let mut isle = Isle::start("prompt-scrim").await;
    let id = ask(&isle, password_prompt("Test needs your password")).await;
    wait_until(|| async { prompts(&isle).await["showing"] == id }).await;
    let window = isle.client("isle.test.Late", "3366ff");
    let info = isle.wait_for_window("isle.test.Late").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!window.printed("keyboard-enter"));
    let test = isle.test().await;
    test.pointer_to(info.x as f64 + 5.0, (info.y + info.height - 5) as f64)
        .await
        .unwrap();
    test.button(0x110, true).await.unwrap();
    test.button(0x110, false).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !window.printed("button"),
        "a click on the scrim reached a window"
    );
}
