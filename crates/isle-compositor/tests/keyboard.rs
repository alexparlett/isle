//! Keyboards, their profiles and keymaps, and what clients receive from them. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::{keyboard::KeyboardProxy, settings::SettingsProxy};

mod common;

use common::{Isle, wait_until};

const KEY_A: u32 = 30;
const KEY_LEFTALT: u32 = 56;
const KEY_LEFTMETA: u32 = 125;

async fn tap(isle: &Isle, id: &str, code: u32) {
    let test = isle.test().await;
    test.key_on(id, code, true).await.unwrap();
    test.key_on(id, code, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn each_keyboard_types_through_its_own_profile() {
    let mut isle = Isle::start("keyboards").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    settings
        .set("keyboard.layouts", &zvariant::Value::from(vec!["gb"]))
        .await
        .unwrap();
    let test = isle.test().await;
    let mac = test.add_keyboard("Keychron K2").await.unwrap();
    let pc = test
        .add_keyboard("AT Translated Set 2 keyboard")
        .await
        .unwrap();
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    let list = keyboard.list().await.unwrap();
    let by_name = |name: &str| list.iter().find(|k| k.name == name).unwrap().clone();
    let k = by_name("Keychron K2");
    assert_eq!(
        (k.profile.as_str(), k.guessed, k.swap),
        ("mac", true, false)
    );
    assert_eq!(k.layouts, "gb(mac)");
    let p = by_name("AT Translated Set 2 keyboard");
    assert_eq!((p.profile.as_str(), p.swap), ("pc", true));
    assert_eq!((p.layouts.as_str(), p.model.as_str()), ("gb", "pc105"));

    let client = isle.client("isle.test.Keys", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.clear();

    // The PC keyboard's left Alt is Cmd, beside the space bar.
    tap(&isle, &pc, KEY_LEFTALT).await;
    client
        .wait_for(&format!("key {KEY_LEFTMETA} pressed"))
        .await;
    assert!(!client.printed(&format!("key {KEY_LEFTALT} pressed")));

    // The seat takes the Mac keyboard's keymap at its first key, and only then.
    client.clear();
    tap(&isle, &mac, KEY_A).await;
    tap(&isle, &mac, KEY_A).await;
    client.wait_for("keymap English (UK, Macintosh)").await;
    let lines = client.lines.lock().unwrap().clone();
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("keymap")).count(),
        1,
        "{lines:?}"
    );
    let keymap = lines.iter().position(|l| l.starts_with("keymap")).unwrap();
    let key = lines.iter().position(|l| l == "key 30 pressed").unwrap();
    assert!(keymap < key, "the keymap came after the key: {lines:?}");

    client.clear();
    tap(&isle, &pc, KEY_A).await;
    client.wait_for("keymap English (UK)").await;
    client.wait_for("sym a released").await;

    // A profile set over the bus is written to the file and applies to the next key.
    keyboard
        .set_profile("AT Translated Set 2 keyboard", "mac")
        .await
        .unwrap();
    wait_until(|| async {
        keyboard.list().await.unwrap().iter().any(|k| {
            k.name == "AT Translated Set 2 keyboard" && k.profile == "mac" && !k.guessed && !k.swap
        })
    })
    .await;
    let file = std::fs::read_to_string(isle.dir.join("config/settings.kdl")).unwrap();
    assert!(
        file.contains("keyboard-device \"AT Translated Set 2 keyboard\""),
        "{file}"
    );
    client.clear();
    tap(&isle, &pc, KEY_LEFTALT).await;
    client.wait_for(&format!("key {KEY_LEFTALT} pressed")).await;
    client.wait_for("keymap English (UK, Macintosh)").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn num_lock_starts_from_the_settings_and_survives_a_keymap_switch() {
    let mut isle = Isle::start("numlock").await;
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    assert!(keyboard.status().await.unwrap().num_lock);
    let test = isle.test().await;
    let mac = test.add_keyboard("Apple Magic Keyboard").await.unwrap();
    let client = isle.client("isle.test.NumLock", "3366ff");
    client.wait_for("keyboard-enter").await;
    tap(&isle, &mac, KEY_A).await;
    client
        .wait_for("keymap English (Macintosh, ABC, ANSI)")
        .await;
    assert!(keyboard.status().await.unwrap().num_lock);
    assert!(!keyboard.status().await.unwrap().caps_lock);
}
