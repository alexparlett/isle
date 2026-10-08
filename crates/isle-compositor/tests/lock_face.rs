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
