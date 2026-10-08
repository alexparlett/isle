//! logind and the lock: the lock before sleep and the delay it holds until every screen
//! shows it, `loginctl lock-session` and `unlock-session`, the locked hint, and the power
//! key's inhibitor. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::logind::Logind;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn sleep_waits_for_the_lock_on_every_screen_and_takes_its_delay_again_after() {
    let mut isle = Isle::start("lock-sleep").await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    let _window = isle.client("isle.test.Behind", "ff0000");
    isle.wait_for_window("isle.test.Behind").await;
    wait_until(|| async {
        let held = logind.inhibitors();
        held.contains(&"sleep:delay".to_owned())
            && held.contains(&"handle-power-key:block".to_owned())
    })
    .await;

    logind.prepare_for_sleep(true).await.unwrap();
    wait_until(|| async { status(&isle).await["locked"] == true }).await;
    wait_until(|| async { !logind.inhibitors().contains(&"sleep:delay".to_owned()) }).await;
    // Released only once the lock was on screen.
    let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
    assert!(!kinds.iter().any(|k| k == "client"), "{kinds:?}");
    assert!(logind.locked_hint().await, "the locked hint is set");

    logind.prepare_for_sleep(false).await.unwrap();
    wait_until(|| async { logind.inhibitors().contains(&"sleep:delay".to_owned()) }).await;
    assert_eq!(
        status(&isle).await["locked"],
        true,
        "resume leaves the lock"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn loginctl_locks_and_unlocks_the_session() {
    let isle = Isle::start("lock-loginctl").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { logind.inhibitors().contains(&"sleep:delay".to_owned()) }).await;
    logind.lock_session().await.unwrap();
    wait_until(|| async { status(&isle).await["locked"] == true }).await;
    wait_until(|| async { logind.locked_hint().await }).await;
    logind.unlock_session().await.unwrap();
    wait_until(|| async { status(&isle).await["locked"] == false }).await;
    wait_until(|| async { !logind.locked_hint().await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sleep_without_the_lock_setting_goes_on_unlocked() {
    let isle = Isle::start_with(
        "lock-sleep-off",
        "wallpaper {\n    source \"#1c1c1f\"\n}\nlock-screen {\n    lock-on-sleep #false\n}\n",
    )
    .await;
    let mut logind = Logind::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { logind.inhibitors().contains(&"sleep:delay".to_owned()) }).await;
    logind.prepare_for_sleep(true).await.unwrap();
    wait_until(|| async { !logind.inhibitors().contains(&"sleep:delay".to_owned()) }).await;
    assert_eq!(status(&isle).await["locked"], false);
}
