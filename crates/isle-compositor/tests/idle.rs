//! Idle: the dim, the lock and the screens off in turn on idle's clock, what holds them, the
//! input that ends each, and ext-idle-notify for clients. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::{displays::DisplaysProxy, session::SessionProxy};

const MIN: u64 = 60_000;
const LEFT: u32 = 0x110;
const A: u32 = 30;

async fn idle(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(
        &SessionProxy::new(&isle.bus)
            .await
            .unwrap()
            .idle()
            .await
            .unwrap(),
    )
    .unwrap()
}

async fn locked(isle: &Isle) -> bool {
    let status: serde_json::Value =
        serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap();
    status["locked"] == true
}

async fn advance(isle: &Isle, millis: u64) {
    isle.test().await.idle_advance(millis).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dim_the_lock_and_the_screens_off_come_in_turn() {
    let isle = Isle::start("idle-steps").await;
    advance(&isle, 4 * MIN).await;
    assert_eq!(idle(&isle).await["dimmed"], false);
    advance(&isle, MIN).await;
    wait_until(|| async { idle(&isle).await["dimmed"] == true }).await;
    // The dim is drawn over everything.
    wait_until(|| async {
        let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
        kinds.first().map(String::as_str) == Some("solid")
    })
    .await;
    advance(&isle, 5 * MIN).await;
    wait_until(|| async { locked(&isle).await }).await;
    // Once locked the lock's own shorter times apply: off a minute after the last input.
    advance(&isle, MIN).await;
    wait_until(|| async { idle(&isle).await["off"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_in_the_dim_lands_and_one_in_the_dark_does_not() {
    let mut isle = Isle::start("idle-input").await;
    let window = isle.client("isle.test.Behind", "3366ff");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Behind").await;
    let test = isle.test().await;
    test.pointer_to(
        (info.x + info.width / 2) as f64,
        (info.y + info.height / 2) as f64,
    )
    .await
    .unwrap();

    advance(&isle, 5 * MIN).await;
    wait_until(|| async { idle(&isle).await["dimmed"] == true }).await;
    window.clear();
    test.button(LEFT, true).await.unwrap();
    test.button(LEFT, false).await.unwrap();
    window.wait_for(&format!("button {LEFT} pressed")).await;
    assert_eq!(idle(&isle).await["dimmed"], false, "input ends the dim");

    DisplaysProxy::new(&isle.bus)
        .await
        .unwrap()
        .sleep()
        .await
        .unwrap();
    wait_until(|| async { idle(&isle).await["off"] == true }).await;
    window.clear();
    test.button(LEFT, true).await.unwrap();
    test.button(LEFT, false).await.unwrap();
    test.key(A, true).await.unwrap();
    test.key(A, false).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(idle(&isle).await["off"], false, "input lights the screens");
    assert!(
        !window.printed(&format!("button {LEFT} pressed")),
        "the waking click landed"
    );
    assert!(
        window.printed(&format!("key {A} pressed")),
        "a key after waking types"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_inhibitor_holds_every_step_while_its_window_shows() {
    let mut isle = Isle::start("idle-inhibit").await;
    let video = isle.client("isle.test.Video", "3366ff");
    video.wait_for("keyboard-enter").await;
    video.send("idle-inhibit");
    wait_until(|| async {
        idle(&isle).await["holders"]
            .as_array()
            .is_some_and(|h| h.iter().any(|a| a == "isle.test.Video"))
    })
    .await;
    advance(&isle, 30 * MIN).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let now = idle(&isle).await;
    assert_eq!(now["dimmed"], false, "{now}");
    assert!(!locked(&isle).await);

    // The inhibitor goes; the hold's end counts as input, so the dim is a full timeout on.
    video.send("idle-uninhibit");
    wait_until(|| async {
        idle(&isle).await["holders"]
            .as_array()
            .is_some_and(|h| h.is_empty())
    })
    .await;
    assert_eq!(idle(&isle).await["dimmed"], false);
    advance(&isle, 5 * MIN).await;
    wait_until(|| async { idle(&isle).await["dimmed"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_game_in_front_holds_idle() {
    let mut isle = Isle::start("idle-game").await;
    let game = isle.client("steam_app_1", "3366ff");
    game.wait_for("keyboard-enter").await;
    let id = isle.wait_for_window("steam_app_1").await.id;
    wait_until(|| async { isle.window(id).await.game }).await;
    wait_until(|| async {
        idle(&isle).await["holders"]
            .as_array()
            .is_some_and(|h| !h.is_empty())
    })
    .await;
    advance(&isle, 30 * MIN).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(idle(&isle).await["dimmed"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn clients_are_told_of_idle_and_its_end() {
    let mut isle = Isle::start("idle-notify").await;
    let watcher = isle.client("isle.test.Watcher", "3366ff");
    watcher.wait_for("keyboard-enter").await;
    watcher.send("idle-notify 300");
    watcher.wait_for("idled").await;
    let test = isle.test().await;
    test.key(A, true).await.unwrap();
    test.key(A, false).await.unwrap();
    watcher.wait_for("resumed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lower_timeout_never_fires_at_once() {
    let isle = Isle::start_with(
        "idle-change",
        "wallpaper {\n    source \"#1c1c1f\"\n}\nlock-screen {\n    dim-after 30\n    lock-after 60\n}\n",
    )
    .await;
    advance(&isle, 7 * MIN).await;
    isle_ipc::settings::SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("lock-screen.dim-after", &zvariant::Value::from(5i64))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        idle(&isle).await["dimmed"],
        false,
        "a dim already past waits for the next idle period"
    );
}
