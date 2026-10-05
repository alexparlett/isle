//! Keyboards, their profiles and keymaps, and what clients receive from them. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::{keyboard::KeyboardProxy, settings::SettingsProxy};

mod common;

use common::{Isle, wait_until};

const KEY_A: u32 = 30;
const KEY_C: u32 = 46;
const KEY_UP: u32 = 103;
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

    // The PC keyboard's left Alt is Cmd, beside the space bar: with C it copies.
    test.key_on(&pc, KEY_LEFTALT, true).await.unwrap();
    tap(&isle, &pc, KEY_C).await;
    test.key_on(&pc, KEY_LEFTALT, false).await.unwrap();
    client.wait_for("mods Control+Mod2").await;
    client.wait_for("sym c released").await;
    for code in [KEY_LEFTALT, KEY_LEFTMETA] {
        assert!(!client.printed(&format!("key {code} pressed")));
    }

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
    // Unswapped, left Alt is Option: Alt with a key that has no third level.
    client.clear();
    test.key_on(&pc, KEY_LEFTALT, true).await.unwrap();
    tap(&isle, &pc, KEY_UP).await;
    test.key_on(&pc, KEY_LEFTALT, false).await.unwrap();
    client.wait_for("keymap English (UK, Macintosh)").await;
    client.wait_for(&format!("key {KEY_LEFTALT} pressed")).await;
    client.wait_for("sym Up pressed").await;
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

const KEY_LEFTSHIFT: u32 = 42;
const KEY_LEFTCTRL: u32 = 29;
const KEY_3: u32 = 4;
const KEY_LEFT: u32 = 105;
const KEY_HOME: u32 = 102;

/// The lines a client printed, from the first that starts with `from`.
fn lines_from(client: &common::Client, from: &str) -> Vec<String> {
    let lines = client.lines.lock().unwrap().clone();
    let start = lines.iter().position(|l| l.starts_with(from)).unwrap_or(0);
    lines[start..].to_vec()
}

async fn mac_keyboard(isle: &Isle) -> String {
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.layouts", &zvariant::Value::from(vec!["gb"]))
        .await
        .unwrap();
    isle.test().await.add_keyboard("Keychron K2").await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_and_option_become_what_a_mac_user_means() {
    let mut isle = Isle::start("translate").await;
    let mac = mac_keyboard(&isle).await;
    let test = isle.test().await;
    let client = isle.client("isle.test.Translate", "3366ff");
    client.wait_for("keyboard-enter").await;
    tap(&isle, &mac, KEY_A).await;
    client.wait_for("keymap English (UK, Macintosh)").await;

    // A tap of Cmd sends nothing; Cmd+C is Ctrl+C and Super never reaches the client.
    client.clear();
    tap(&isle, &mac, KEY_LEFTMETA).await;
    test.key_on(&mac, KEY_LEFTMETA, true).await.unwrap();
    tap(&isle, &mac, KEY_C).await;
    test.key_on(&mac, KEY_LEFTMETA, false).await.unwrap();
    client.wait_for("sym c released").await;
    client
        .wait_for(&format!("key {KEY_LEFTCTRL} released"))
        .await;
    let lines = client.lines.lock().unwrap().clone();
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("Super") || l.contains("Mod4")),
        "{lines:?}"
    );
    assert_eq!(lines[0], format!("key {KEY_LEFTCTRL} pressed"), "{lines:?}");
    assert!(lines.contains(&"mods Control+Mod2".to_owned()));

    // Option with 3 types # on a UK Mac keyboard, from the left Option.
    client.clear();
    test.key_on(&mac, KEY_LEFTALT, true).await.unwrap();
    tap(&isle, &mac, KEY_3).await;
    test.key_on(&mac, KEY_LEFTALT, false).await.unwrap();
    client.wait_for("sym numbersign pressed").await;
    assert!(!client.printed(&format!("key {KEY_LEFTALT} pressed")));

    // Cmd+Left is Home, sent with no modifier at all.
    client.clear();
    test.key_on(&mac, KEY_LEFTMETA, true).await.unwrap();
    test.key_on(&mac, KEY_LEFT, true).await.unwrap();
    test.key_on(&mac, KEY_LEFT, false).await.unwrap();
    test.key_on(&mac, KEY_LEFTMETA, false).await.unwrap();
    client.wait_for(&format!("key {KEY_HOME} released")).await;
    let lines = client.lines.lock().unwrap().clone();
    assert_eq!(
        lines,
        vec![
            format!("key {KEY_HOME} pressed"),
            "sym Home pressed".into(),
            format!("key {KEY_HOME} released"),
            "sym Home released".into(),
        ],
        "only Home, once"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_focus_change_mid_chord_leaves_no_key_held_in_either_window() {
    let mut isle = Isle::start("unwind").await;
    let mac = mac_keyboard(&isle).await;
    let test = isle.test().await;
    let first = isle.client("isle.test.First", "3366ff");
    first.wait_for("keyboard-enter").await;
    let second = isle.client("isle.test.Second", "ff6633");
    second.wait_for("keyboard-enter").await;
    let first_id = isle.wait_for_window("isle.test.First").await.id;
    let windows = isle_ipc::windows::WindowsProxy::new(&isle.bus)
        .await
        .unwrap();
    windows.focus(first_id).await.unwrap();
    first.wait_for("keyboard-enter").await;
    first.clear();
    second.clear();

    for key in [KEY_LEFTSHIFT, KEY_LEFTMETA, KEY_C] {
        test.key_on(&mac, key, true).await.unwrap();
    }
    first.wait_for("sym C pressed").await;
    windows
        .focus(isle.wait_for_window("isle.test.Second").await.id)
        .await
        .unwrap();
    second.wait_for("keyboard-enter").await;
    for key in [KEY_C, KEY_LEFTMETA, KEY_LEFTSHIFT] {
        test.key_on(&mac, key, false).await.unwrap();
    }
    second
        .wait_for(&format!("key {KEY_LEFTSHIFT} released"))
        .await;

    let left = lines_from(&first, &format!("key {KEY_C} released"));
    assert!(
        left.contains(&format!("key {KEY_LEFTCTRL} released")),
        "{left:?}"
    );
    assert!(
        left.contains(&format!("key {KEY_LEFTSHIFT} released")),
        "{left:?}"
    );
    assert_eq!(left.last().unwrap(), "keyboard-leave", "{left:?}");
    let entered = second.lines.lock().unwrap().clone();
    let keys: Vec<&String> = entered.iter().filter(|l| l.starts_with("key ")).collect();
    assert_eq!(
        keys,
        vec![
            &format!("key {KEY_LEFTSHIFT} pressed"),
            &format!("key {KEY_LEFTSHIFT} released")
        ],
        "the new window holds Shift only: {entered:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_with_cmd_held_arrives_with_ctrl() {
    let mut isle = Isle::start("cmd-click").await;
    let mac = mac_keyboard(&isle).await;
    let test = isle.test().await;
    let client = isle.client("isle.test.Click", "3366ff");
    client.wait_for("keyboard-enter").await;
    let window = isle.wait_for_window("isle.test.Click").await;
    test.pointer_to((window.x + 40) as f64, (window.y + 60) as f64)
        .await
        .unwrap();
    client.clear();
    test.key_on(&mac, KEY_LEFTMETA, true).await.unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    test.key_on(&mac, KEY_LEFTMETA, false).await.unwrap();
    client
        .wait_for(&format!("key {KEY_LEFTCTRL} released"))
        .await;
    let lines = client.lines.lock().unwrap().clone();
    let ctrl = lines
        .iter()
        .position(|l| *l == format!("key {KEY_LEFTCTRL} pressed"));
    let button = lines
        .iter()
        .position(|l| l.starts_with("button 272 pressed"));
    assert!(
        ctrl.is_some() && button.is_some() && ctrl < button,
        "{lines:?}"
    );
}
