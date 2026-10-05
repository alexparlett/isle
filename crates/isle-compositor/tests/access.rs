//! Sticky Keys and Slow Keys. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::settings::SettingsProxy;

mod common;

use common::{Isle, wait_until};

const CMD: u32 = 125;
const SHIFT: u32 = 42;
const TAB: u32 = 15;
const A: u32 = 30;

async fn tap(isle: &Isle, code: u32) {
    let test = isle.test().await;
    test.key(code, true).await.unwrap();
    test.key(code, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn sticky_cmd_holds_the_switcher_until_cmd_is_tapped_again() {
    let mut isle = Isle::start("sticky").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let client = isle.client("isle.test.Sticky", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;

    for _ in 0..5 {
        tap(&isle, SHIFT).await;
    }
    wait_until(|| async {
        settings.get("keyboard.sticky-keys").await.unwrap()
            == zvariant::Value::from(true).try_into().unwrap()
    })
    .await;

    tap(&isle, CMD).await;
    tap(&isle, TAB).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        0,
        "Cmd stays latched for the switcher"
    );
    tap(&isle, TAB).await;
    assert_eq!(test.held_releases().await.unwrap(), 0);
    tap(&isle, CMD).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        1,
        "the next Cmd tap commits"
    );

    client.clear();
    tap(&isle, A).await;
    client.wait_for("sym a released").await;
    assert!(!client.printed("key 29 pressed"), "nothing is latched now");
}

#[tokio::test(flavor = "multi_thread")]
async fn with_slow_keys_a_short_press_sends_nothing() {
    let mut isle = Isle::start("slow").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.slow-keys", &zvariant::Value::from(300i64))
        .await
        .unwrap();
    let client = isle.client("isle.test.Slow", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;
    client.clear();
    tap(&isle, A).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(
        !client.printed("key 30 pressed"),
        "a short press is dropped"
    );

    test.key(A, true).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
    client.wait_for("key 30 pressed").await;
    test.key(A, false).await.unwrap();
    client.wait_for("key 30 released").await;
}
