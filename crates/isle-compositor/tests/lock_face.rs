//! The lock's face: shown on the pointer's screen, typing into it, following the pointer,
//! and its panics, which never show the session and bring in the minimal lock. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

const TAB: u32 = 15;
const A: u32 = 30;
const B: u32 = 48;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

async fn lock(isle: &Isle) {
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { status(isle).await["locked"] == true }).await;
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_face_takes_typing_and_moves_with_the_pointer() {
    let mut isle = Isle::start("lock-face").await;
    let _window = isle.client("isle.test.Behind", "ff0000");
    isle.wait_for_window("isle.test.Behind").await;
    lock(&isle).await;
    wait_until(|| async { status(&isle).await["field_focused"] == true }).await;
    assert_eq!(status(&isle).await["face"], "HEADLESS-1");
    wait_until(|| async {
        let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
        kinds.iter().any(|k| k == "shell") && !kinds.iter().any(|k| k == "client")
    })
    .await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    isle.keep_shot("lock-face").await;

    press(&isle, A).await;
    press(&isle, B).await;
    assert_eq!(status(&isle).await["dots"], 2);

    // The face goes to the screen the pointer moves to, with what was typed.
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Test", "2", 1280, 720)
        .await
        .unwrap();
    wait_until(|| async {
        isle.test()
            .await
            .frame_kinds("HEADLESS-2")
            .await
            .unwrap()
            .iter()
            .any(|k| k == "shell")
    })
    .await;
    test.pointer_to(1920.0 + 640.0, 360.0).await.unwrap();
    wait_until(|| async { status(&isle).await["face"] == "HEADLESS-2" }).await;
    assert_eq!(status(&isle).await["dots"], 2);
    press(&isle, A).await;
    assert_eq!(status(&isle).await["dots"], 3);

    // Tab moves on from the field, and typing no longer fills it.
    press(&isle, TAB).await;
    wait_until(|| async { status(&isle).await["field_focused"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn panics_in_the_face_never_show_the_session_and_bring_the_minimal_lock() {
    let mut isle = Isle::start("lock-panics").await;
    let _window = isle.client("isle.test.Behind", "ff0000");
    isle.wait_for_window("isle.test.Behind").await;
    lock(&isle).await;
    let test = isle.test().await;
    test.panic_lock().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        status(&isle).await["minimal"],
        false,
        "one panic rebuilds the face"
    );
    test.panic_lock().await.unwrap();
    wait_until(|| async { status(&isle).await["minimal"] == true }).await;
    for _ in 0..100 {
        test.panic_lock().await.unwrap();
        let kinds = test.frame_kinds("HEADLESS-1").await.unwrap();
        assert!(!kinds.iter().any(|k| k == "client"), "{kinds:?}");
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.keep_shot("lock-minimal").await;
    // The minimal lock still takes the password.
    press(&isle, A).await;
    assert_eq!(status(&isle).await["dots"], 1);
    assert!(status(&isle).await["locked"] == true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_quiet_face_dozes_and_a_key_wakes_it_and_types() {
    let isle = Isle::start("lock-doze").await;
    lock(&isle).await;
    tokio::time::sleep(Duration::from_millis(12_500)).await;
    assert_eq!(
        status(&isle).await["awake"],
        false,
        "the face dozes with the field empty"
    );
    press(&isle, A).await;
    let now = status(&isle).await;
    assert_eq!(now["awake"], true, "{now}");
    assert_eq!(now["dots"], 1, "the waking key is typed");
}

async fn tap_on(isle: &Isle, id: &str, code: u32) {
    let test = isle.test().await;
    test.key_on(id, code, true).await.unwrap();
    test.key_on(id, code, false).await.unwrap();
}

/// `held` down, `code` tapped, `held` up, on keyboard `id`.
async fn chord_on(isle: &Isle, id: &str, held: u32, code: u32) {
    let test = isle.test().await;
    test.key_on(id, held, true).await.unwrap();
    tap_on(isle, id, code).await;
    test.key_on(id, held, false).await.unwrap();
}

const KEY_3: u32 = 4;
const C: u32 = 46;
const V: u32 = 47;
const X: u32 = 45;
const BACKSPACE: u32 = 14;
const LEFTALT: u32 = 56;
const LEFTMETA: u32 = 125;

#[tokio::test(flavor = "multi_thread")]
async fn the_field_takes_a_password_fields_chords() {
    let isle = Isle::start("lock-chords").await;
    isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.layouts", &zvariant::Value::from(vec!["gb"]))
        .await
        .unwrap();
    let test = isle.test().await;
    let mac = test.add_keyboard("Keychron Q6 Max").await.unwrap();
    let pc = test
        .add_keyboard("AT Translated Set 2 keyboard")
        .await
        .unwrap();
    lock(&isle).await;
    wait_until(|| async { status(&isle).await["field_focused"] == true }).await;
    let dots = || async { status(&isle).await["dots"].as_u64().unwrap() };

    // Cmd+A selects what is typed; the next character replaces it.
    tap_on(&isle, &mac, A).await;
    tap_on(&isle, &mac, B).await;
    chord_on(&isle, &mac, LEFTMETA, A).await;
    assert_eq!(status(&isle).await["selected"], true);
    assert_eq!(dots().await, 2);
    tap_on(&isle, &mac, A).await;
    assert_eq!(dots().await, 1);
    assert_eq!(status(&isle).await["selected"], false);

    // Selected, Backspace clears it all.
    tap_on(&isle, &mac, B).await;
    chord_on(&isle, &mac, LEFTMETA, A).await;
    tap_on(&isle, &mac, BACKSPACE).await;
    assert_eq!(dots().await, 0);

    // Cmd+C and Cmd+X leave the field as it is and copy nothing.
    for code in [A, B, A] {
        tap_on(&isle, &mac, code).await;
    }
    chord_on(&isle, &mac, LEFTMETA, C).await;
    chord_on(&isle, &mac, LEFTMETA, X).await;
    assert_eq!(dots().await, 3);
    assert_eq!(test.clipboard().await.unwrap(), "");

    // Cmd+Backspace and Option+Backspace clear the field.
    chord_on(&isle, &mac, LEFTMETA, BACKSPACE).await;
    assert_eq!(dots().await, 0);
    tap_on(&isle, &mac, A).await;
    chord_on(&isle, &mac, LEFTALT, BACKSPACE).await;
    assert_eq!(dots().await, 0);

    // A character typed with Option is a character: # on gb(mac).
    chord_on(&isle, &mac, LEFTALT, KEY_3).await;
    assert_eq!(dots().await, 1);

    // On a PC keyboard Cmd is the left Alt.
    chord_on(&isle, &pc, LEFTALT, A).await;
    assert_eq!(status(&isle).await["selected"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_v_pastes_the_clipboard_without_its_line_breaks() {
    let mut isle = Isle::start("lock-paste").await;
    let test = isle.test().await;
    let mac = test.add_keyboard("Keychron Q6 Max").await.unwrap();
    let app = isle.client("isle.test.Copier", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.send("copy-lines pa ss\\nword");
    app.wait_for("copied").await;
    lock(&isle).await;
    wait_until(|| async { status(&isle).await["field_focused"] == true }).await;
    tap_on(&isle, &mac, A).await;
    chord_on(&isle, &mac, LEFTMETA, A).await;
    chord_on(&isle, &mac, LEFTMETA, V).await;
    wait_until(|| async { status(&isle).await["dots"] == 9 }).await;
    assert_eq!(status(&isle).await["selected"], false);
}

const LEFT: u32 = 105;
const LEFTSHIFT: u32 = 42;
const LEFTCTRL: u32 = 29;
const E: u32 = 18;

#[tokio::test(flavor = "multi_thread")]
async fn the_field_moves_selects_and_repeats_as_a_text_field() {
    let isle = Isle::start("lock-editing").await;
    isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.layouts", &zvariant::Value::from(vec!["gb"]))
        .await
        .unwrap();
    let test = isle.test().await;
    let mac = test.add_keyboard("Keychron Q6 Max").await.unwrap();
    lock(&isle).await;
    wait_until(|| async { status(&isle).await["field_focused"] == true }).await;
    let at = || async {
        let s = status(&isle).await;
        (s["dots"].as_u64().unwrap(), s["caret"].as_u64().unwrap())
    };

    // Left moves the caret back and typing goes in there.
    for code in [A, B, A] {
        tap_on(&isle, &mac, code).await;
    }
    tap_on(&isle, &mac, LEFT).await;
    tap_on(&isle, &mac, B).await;
    assert_eq!(at().await, (4, 3));

    // Shift+Left selects; Backspace takes the selection.
    chord_on(&isle, &mac, LEFTSHIFT, LEFT).await;
    chord_on(&isle, &mac, LEFTSHIFT, LEFT).await;
    assert_eq!(status(&isle).await["selected"], true);
    tap_on(&isle, &mac, BACKSPACE).await;
    assert_eq!(at().await, (2, 1));

    // Ctrl+A and Ctrl+E go to the ends; Option+Left in a secure field goes to the start.
    chord_on(&isle, &mac, LEFTCTRL, E).await;
    assert_eq!(at().await, (2, 2));
    chord_on(&isle, &mac, LEFTCTRL, A).await;
    assert_eq!(at().await, (2, 0));
    chord_on(&isle, &mac, LEFTCTRL, E).await;
    chord_on(&isle, &mac, LEFTALT, LEFT).await;
    assert_eq!(at().await, (2, 0));

    // Backspace held repeats at the keyboard's rate.
    chord_on(&isle, &mac, LEFTCTRL, E).await;
    for _ in 0..8 {
        tap_on(&isle, &mac, A).await;
    }
    test.key_on(&mac, BACKSPACE, true).await.unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    test.key_on(&mac, BACKSPACE, false).await.unwrap();
    let (dots, _) = at().await;
    assert!(dots < 9, "held Backspace deleted only one: {dots} left");
    let after = at().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(at().await, after, "the repeat outlived the key");
}
