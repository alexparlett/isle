//! Spotlight: opening, the field, the keys and the surfaces it gives way to. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, keys::*, wait_until};
use isle_ipc::session::SessionProxy;

const SPACE: u32 = 57;
const ESCAPE: u32 = 1;
const TAB: u32 = 15;
const BACKSPACE: u32 = 14;
const I: u32 = 23;
const E: u32 = 18;
const N: u32 = 49;
const ONE: u32 = 2;
const SLASH: u32 = 53;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

/// Cmd+Space, then `keys` at once, with no wait between.
async fn open_and_type(isle: &Isle, keys: &[u32]) {
    let test = isle.test().await;
    test.key(SUPER, true).await.unwrap();
    test.key(SPACE, true).await.unwrap();
    test.key(SPACE, false).await.unwrap();
    test.key(SUPER, false).await.unwrap();
    for key in keys {
        test.key(*key, true).await.unwrap();
        test.key(*key, false).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_typed_with_the_chord_reach_the_field() {
    let mut isle = Isle::start("spotlight-keys").await;
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;
    open_and_type(&isle, &[F, I, R, E]).await;
    wait_until(|| async { status(&isle).await["text"] == "fire" }).await;
    window.wait_for("keyboard-leave").await;
    assert!(!window.printed("key 33"), "no key reached the window");

    press(&isle, BACKSPACE).await;
    wait_until(|| async { status(&isle).await["text"] == "fir" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["text"] == "" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    window.wait_for("keyboard-enter").await;
    assert_eq!(
        status(&isle).await["kept"],
        serde_json::Value::Null,
        "an emptied query is not kept"
    );

    open_and_type(&isle, &[N, E]).await;
    wait_until(|| async { status(&isle).await["text"] == "ne" }).await;
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    assert_eq!(status(&isle).await["kept"], "ne");
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["text"] == "ne" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn scopes_come_from_prefixes_and_quick_keys() {
    let isle = Isle::start("spotlight-scopes").await;
    open_and_type(&isle, &[SLASH]).await;
    wait_until(|| async { status(&isle).await["scope"] == "Files { contents: false }" }).await;
    press(&isle, BACKSPACE).await;
    wait_until(|| async { status(&isle).await["scope"] == "General" }).await;
    isle.chord(&[SUPER, ONE]).await;
    wait_until(|| async { status(&isle).await["scope"] == "Apps" }).await;
    isle.test().await.spotlight_act("quick 1").await.unwrap();
    wait_until(|| async { status(&isle).await["scope"] == "General" }).await;
    press(&isle, TAB).await;
    assert_eq!(status(&isle).await["open"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_switcher_and_the_lock_close_it() {
    let mut isle = Isle::start("spotlight-closes").await;
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;
    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    isle.chord(&[SUPER, TAB]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;

    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prompt_takes_the_keys_and_gives_them_back() {
    let isle = Isle::start("spotlight-prompt").await;
    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["text"] == "f" }).await;
    let test = isle.test().await;
    let prompt = serde_json::json!({
        "kind": "polkit",
        "title": "Test needs your password",
        "message": "Authentication is needed to run a test.",
        "fields": [{"id": "password", "label": "Password for test", "secret": true}],
        "action": "Allow",
    });
    let id = test.prompt_ask(&prompt.to_string()).await.unwrap();
    wait_until(|| async {
        let p: serde_json::Value = serde_json::from_str(&test.prompts().await.unwrap()).unwrap();
        p["showing"] == id
    })
    .await;
    press(&isle, I).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(status(&isle).await["text"], "f", "the prompt has the keys");
    assert_eq!(status(&isle).await["open"], true, "and Spotlight stays");
    test.prompt_done(id).await.unwrap();
    wait_until(|| async {
        press(&isle, I).await;
        status(&isle).await["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("fi"))
    })
    .await;
}

/// Writes Spotlight's look to `$ISLE_SHOT_DIR` for a person to see; run by name.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn look() {
    let Some(dir) = std::env::var_os("ISLE_SHOT_DIR") else {
        return;
    };
    let isle = Isle::start("spotlight-look").await;
    open_and_type(&isle, &[F, I]).await;
    wait_until(|| async { status(&isle).await["text"] == "fi" }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let test = isle.test().await;
    let path = std::path::Path::new(&dir).join("spotlight-field.png");
    test.screenshot("HEADLESS-1", path.to_str().unwrap())
        .await
        .unwrap();
    isle.chord(&[SUPER, ONE]).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    test.screenshot(
        "HEADLESS-1",
        std::path::Path::new(&dir)
            .join("spotlight-apps.png")
            .to_str()
            .unwrap(),
    )
    .await
    .unwrap();
}
