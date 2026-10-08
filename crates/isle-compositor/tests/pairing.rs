//! Bluetooth pairing: the agent is BlueZ's default with KeyboardDisplay, and each of its
//! questions is a prompt; a code shown follows the digits typed; BlueZ's cancel closes it.
//! Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::sync::Arc;

use common::{Isle, wait_until};
use isle_fakes::bluez::{BlueZ, Device};

const RETURN: u32 = 28;
const ESC: u32 = 1;

fn speaker() -> Device {
    Device {
        id: "AA_BB_CC_DD_EE_FF".into(),
        name: "Speaker".into(),
        icon: "audio-card".into(),
        paired: false,
        connected: false,
        battery: None,
    }
}

async fn prompts(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.prompts().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

async fn start(name: &str) -> (Isle, Arc<BlueZ>) {
    let isle = Isle::start(name).await;
    let bluez = BlueZ::start(&isle.system_address(), vec![speaker()])
        .await
        .unwrap();
    wait_until(|| async { bluez.agent().is_some_and(|a| a.default) }).await;
    assert_eq!(bluez.agent().unwrap().capability, "KeyboardDisplay");
    (isle, Arc::new(bluez))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_code_to_confirm_pairs_on_pair_and_refuses_on_cancel() {
    let (isle, bluez) = start("pair-confirm").await;
    let device = BlueZ::device_path("AA_BB_CC_DD_EE_FF");
    let asking = bluez.clone();
    let path = device.clone();
    let reply = tokio::spawn(async move {
        asking
            .ask_agent::<_, ()>("RequestConfirmation", (path, 123456u32))
            .await
    });
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    assert_eq!(prompts(&isle).await["title"], "Pair with Speaker?");
    press(&isle, RETURN).await;
    assert_eq!(reply.await.unwrap(), Ok(()));

    let asking = bluez.clone();
    let path = device.clone();
    let reply = tokio::spawn(async move {
        asking
            .ask_agent::<_, ()>("RequestAuthorization", (path,))
            .await
    });
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    assert_eq!(prompts(&isle).await["title"], "Speaker wants to pair");
    press(&isle, ESC).await;
    assert_eq!(
        reply.await.unwrap(),
        Err("org.bluez.Error.Rejected".to_owned())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_passkey_typed_reaches_bluez() {
    let (isle, bluez) = start("pair-passkey").await;
    let path = BlueZ::device_path("AA_BB_CC_DD_EE_FF");
    let asking = bluez.clone();
    let reply =
        tokio::spawn(async move { asking.ask_agent::<_, u32>("RequestPasskey", (path,)).await });
    wait_until(|| async { prompts(&isle).await["armed"] == true }).await;
    // 0, 4, 2, 4, 2, 4
    for key in [11, 5, 3, 5, 3, 5, RETURN] {
        press(&isle, key).await;
    }
    assert_eq!(reply.await.unwrap(), Ok(42424));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shown_code_follows_the_digits_and_closes_on_cancel() {
    let (isle, bluez) = start("pair-display").await;
    let path = BlueZ::device_path("AA_BB_CC_DD_EE_FF");
    bluez
        .ask_agent::<_, ()>("DisplayPasskey", (path.clone(), 123456u32, 0u16))
        .await
        .unwrap();
    wait_until(|| async { prompts(&isle).await["title"] == "Type this code on Speaker" }).await;
    bluez
        .ask_agent::<_, ()>("DisplayPasskey", (path.clone(), 123456u32, 3u16))
        .await
        .unwrap();
    bluez.ask_agent::<_, ()>("Cancel", ()).await.unwrap();
    wait_until(|| async { prompts(&isle).await["showing"].is_null() }).await;
}
