//! Ending the session: the chooser from the power key and its chord with its letters and
//! countdown, the confirmation, apps quitting and those that stop it, logind's Reboot and
//! PowerOff, log out ending the compositor, and the lock's power row. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_fakes::logind::Logind;

const ESC: u32 = 1;
const BACKSPACE: u32 = 14;
const Q: u32 = 16;
const R: u32 = 19;
const RETURN: u32 = 28;
const CTRL: u32 = 29;
const L: u32 = 38;
const SHIFT: u32 = 42;
const OPTION: u32 = 56;
const LEFT: u32 = 105;
const POWER: u32 = 116;
const CMD: u32 = 125;

async fn session(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.session().await.unwrap()).unwrap()
}

async fn lock_status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    isle.chord(&[key]).await;
}

async fn armed(isle: &Isle) {
    wait_until(|| async { session(isle).await["armed"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_chooser_opens_from_the_power_key_and_the_chord_and_takes_its_letters() {
    let mut isle = Isle::start("session-chooser").await;
    let client = isle.client("isle.test.Open", "3366ff");
    client.wait_for("keyboard-enter").await;

    press(&isle, POWER).await;
    wait_until(|| async { session(&isle).await["open"] == true }).await;
    let s = session(&isle).await;
    assert_eq!(s["form"], "chooser Shut Down");
    assert_eq!(s["countdown"], 60);
    client.wait_for("keyboard-leave").await;
    let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
    assert!(kinds.iter().any(|k| k == "solid"), "the scrim: {kinds:?}");

    armed(&isle).await;
    isle.keep_shot("session-chooser").await;
    press(&isle, LEFT).await;
    wait_until(|| async { session(&isle).await["form"] == "chooser Restart" }).await;
    assert!(
        session(&isle).await["countdown"].is_null(),
        "a key stops the countdown"
    );
    press(&isle, ESC).await;
    wait_until(|| async { session(&isle).await["open"] == false }).await;
    client.wait_for("keyboard-enter").await;

    isle.chord(&[CTRL, CMD, BACKSPACE]).await;
    wait_until(|| async { session(&isle).await["open"] == true }).await;
    armed(&isle).await;
    press(&isle, L).await;
    wait_until(|| async { lock_status(&isle).await["locked"] == true }).await;
    assert_eq!(session(&isle).await["open"], false);

    // Over the lock the power key wakes the face and opens nothing.
    press(&isle, POWER).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(session(&isle).await["open"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_countdown_counts_and_stops_on_a_key() {
    let isle = Isle::start("session-countdown").await;
    press(&isle, POWER).await;
    wait_until(|| async { session(&isle).await["countdown"] == 59 }).await;
    press(&isle, SHIFT).await;
    wait_until(|| async { session(&isle).await["countdown"].is_null() }).await;
    assert_eq!(session(&isle).await["open"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_and_shut_down_quit_every_app_then_call_logind() {
    let mut isle = Isle::start("session-restart").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let one = isle.client("isle.test.One", "3366ff");
    one.wait_for("keyboard-enter").await;
    let two = isle.client("isle.test.Two", "ff6633");
    two.wait_for("keyboard-enter").await;

    press(&isle, POWER).await;
    armed(&isle).await;
    press(&isle, R).await;
    wait_until(|| async { logind.ended() == ["Reboot"] }).await;
    assert!(isle.windows().await.is_empty(), "every app quit first");
}

#[tokio::test(flavor = "multi_thread")]
async fn return_in_the_chooser_shuts_down() {
    let mut isle = Isle::start("session-shut-down").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let client = isle.client("isle.test.Open", "3366ff");
    client.wait_for("keyboard-enter").await;
    press(&isle, POWER).await;
    armed(&isle).await;
    press(&isle, RETURN).await;
    wait_until(|| async { logind.ended() == ["PowerOff"] }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_keeps_its_window_stops_the_restart_and_show_brings_it_forward() {
    let mut isle = Isle::start("session-stopped").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let keeper = isle.client("isle.test.Keeper", "3366ff");
    keeper.wait_for("keyboard-enter").await;
    keeper.send("keep-open");

    press(&isle, POWER).await;
    armed(&isle).await;
    press(&isle, R).await;
    wait_until(|| async { session(&isle).await["stop"] == "open isle.test.Keeper" }).await;
    assert!(logind.ended().is_empty());
    isle.keep_shot("session-stopped").await;

    keeper.clear();
    press(&isle, RETURN).await;
    wait_until(|| async { session(&isle).await["open"] == false }).await;
    keeper.wait_for("keyboard-enter").await;
    assert!(session(&isle).await["ending"].is_null(), "called off");
    assert!(logind.ended().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_does_not_answer_is_force_quit_and_the_restart_goes_on() {
    let mut isle = Isle::start("session-frozen").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let frozen = isle.client("isle.test.Frozen", "3366ff");
    frozen.wait_for("keyboard-enter").await;
    frozen.send("keep-open");
    frozen.send("pings ignore");
    tokio::time::sleep(Duration::from_millis(100)).await;

    press(&isle, POWER).await;
    armed(&isle).await;
    press(&isle, R).await;
    wait_until(|| async { session(&isle).await["stop"] == "not responding isle.test.Frozen" })
        .await;
    isle.keep_shot("session-not-responding").await;
    // Force Quit is never the default: Return leaves it.
    press(&isle, RETURN).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!frozen.exited());

    isle.test()
        .await
        .session_act("force-quit isle.test.Frozen")
        .await
        .unwrap();
    wait_until(|| async { frozen.exited() }).await;
    wait_until(|| async { logind.ended() == ["Reboot"] }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_holding_shutdown_is_named() {
    let isle = Isle::start("session-inhibited").await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    logind
        .block("shutdown", "Updater", "Installing updates")
        .await
        .unwrap();
    press(&isle, POWER).await;
    armed(&isle).await;
    press(&isle, R).await;
    wait_until(|| async {
        session(&isle).await["stop"]
            == "refused Couldn't restart: Updater is preventing it (Installing updates)."
    })
    .await;
    assert!(logind.ended().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn log_out_quits_the_apps_and_ends_the_compositor_cleanly() {
    let mut isle = Isle::start("session-log-out").await;
    let client = isle.client("isle.test.Open", "3366ff");
    client.wait_for("keyboard-enter").await;

    // Shift+Cmd+Q asks; Escape cancels.
    isle.chord(&[SHIFT, CMD, Q]).await;
    wait_until(|| async { session(&isle).await["form"] == "confirm Log Out" }).await;
    armed(&isle).await;
    isle.keep_shot("session-confirm").await;
    press(&isle, ESC).await;
    wait_until(|| async { session(&isle).await["open"] == false }).await;
    client.wait_for("keyboard-enter").await;

    // With Option, it does not ask.
    isle.chord(&[OPTION, SHIFT, CMD, Q]).await;
    wait_until(|| async { client.exited() }).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = isle.compositor_exit() {
            break status;
        }
        assert!(std::time::Instant::now() < deadline, "never exited");
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(status.success(), "{status:?}\n{}", isle.log());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_lock_confirms_in_place_and_says_what_stopped_it_without_naming_it() {
    let mut isle = Isle::start("session-lock").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let keeper = isle.client("isle.test.Keeper", "3366ff");
    keeper.wait_for("keyboard-enter").await;
    keeper.send("keep-open");
    isle.chord(&[CTRL, CMD, Q]).await;
    wait_until(|| async { lock_status(&isle).await["locked"] == true }).await;

    isle.test().await.session_act("restart").await.unwrap();
    wait_until(|| async { session(&isle).await["lock_power"] == "Confirm { restart: true }" })
        .await;
    isle.keep_shot("session-lock-confirm").await;
    press(&isle, ESC).await;
    wait_until(|| async { session(&isle).await["lock_power"] == "Row" }).await;

    isle.test().await.session_act("restart").await.unwrap();
    press(&isle, RETURN).await;
    wait_until(|| async { session(&isle).await["lock_power"] == "Quitting" }).await;
    wait_until(|| async {
        lock_status(&isle).await["session_line"]
            == "Couldn't restart because an app didn't quit. Unlock to see it."
    })
    .await;
    assert!(logind.ended().is_empty());
    isle.keep_shot("session-lock-stopped").await;

    keeper.clear();
    isle.test().await.unlock().await.unwrap();
    keeper.wait_for("keyboard-enter").await;
    assert!(session(&isle).await["ending"].is_null());
}
