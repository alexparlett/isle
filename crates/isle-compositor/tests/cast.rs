//! Screen streams on a private PipeWire: what each source carries, when a window's stream
//! holds its frame, the lock in a screen's, and the ends. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Client, Isle, wait_until};
use isle_ipc::session::SessionProxy;

async fn casts(isle: &Isle) -> Vec<serde_json::Value> {
    serde_json::from_str(&isle.test().await.casts().await.unwrap()).unwrap()
}

/// Starts a stream by `spec` and a consumer linked to it.
async fn stream(isle: &mut Isle, spec: serde_json::Value) -> (u64, Client) {
    let id = isle
        .test()
        .await
        .cast_start(&spec.to_string())
        .await
        .unwrap();
    wait_until(|| async {
        casts(isle)
            .await
            .iter()
            .any(|c| c["id"] == id && c["node"].is_u64())
    })
    .await;
    let node = casts(isle)
        .await
        .into_iter()
        .find(|c| c["id"] == id)
        .unwrap()["node"]
        .as_u64()
        .unwrap() as u32;
    let consumer = isle.consumer(node);
    (id, consumer)
}

/// The consumer's last frame line: its count and centre colour.
fn last_frame(consumer: &Client) -> Option<(u64, String)> {
    let line = consumer
        .lines
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .rev()
        .find(|l| l.starts_with("frame "))?;
    let parts: Vec<&str> = line.split(' ').collect();
    Some((parts[1].parse().ok()?, parts[3].to_owned()))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_screen_stream_carries_the_screen_and_shows_the_lock() {
    let mut isle = Isle::start("cast-screen").await;
    if !isle.start_audio().await {
        return;
    }
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    isle.wait_for_window("isle.test.Red").await;
    let (id, consumer) = stream(&mut isle, serde_json::json!({"screen": "HEADLESS-1"})).await;
    consumer.wait_for("format 1920x1080").await;
    wait_until(|| async { last_frame(&consumer).is_some_and(|(_, c)| c == "ff0000") }).await;
    let cast = casts(&isle)
        .await
        .into_iter()
        .find(|c| c["id"] == id)
        .unwrap();
    assert_eq!(cast["kind"], "shm");
    assert!(cast["serial"].is_u64(), "{cast}");
    wait_until(|| async { isle.island_output().await["privacy"] == 1 }).await;

    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { last_frame(&consumer).is_some_and(|(_, c)| c != "ff0000") }).await;

    // No D-Bus stack of PipeWire's in the compositor.
    let pid = isle.compositor_pid();
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).unwrap();
    for module in ["libpipewire-module-rt", "libspa-dbus"] {
        assert!(!maps.contains(module), "{module} is loaded");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_stream_holds_while_hidden_and_ends_when_it_closes() {
    let mut isle = Isle::start("cast-window").await;
    if !isle.start_audio().await {
        return;
    }
    let window = isle.client("isle.test.Blue", "0000ff");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Blue").await;
    let (id, consumer) = stream(&mut isle, serde_json::json!({"window": info.id})).await;
    let frame = isle.window(info.id).await;
    consumer
        .wait_for(&format!(
            "format {}x{}",
            frame.width & !1,
            frame.height & !1
        ))
        .await;
    wait_until(|| async { last_frame(&consumer).is_some_and(|(_, c)| c == "0000ff") }).await;

    let windows = isle.windows_proxy().await;
    windows.minimise(info.id).await.unwrap();
    wait_until(|| async { !isle.window(info.id).await.focused }).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let held = last_frame(&consumer).unwrap().0;
    window.send("colour 00ff00");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        last_frame(&consumer).unwrap().0,
        held,
        "a hidden window streamed"
    );

    windows.unminimise(info.id).await.unwrap();
    wait_until(|| async { last_frame(&consumer).is_some_and(|(_, c)| c == "00ff00") }).await;

    windows.close(info.id).await.unwrap();
    wait_until(|| async { !casts(&isle).await.iter().any(|c| c["id"] == id) }).await;
    wait_until(|| async { isle.island_output().await["privacy"] == 0 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_portion_is_its_rectangle_and_a_screen_going_ends_its_stream() {
    let mut isle = Isle::start("cast-portion").await;
    if !isle.start_audio().await {
        return;
    }
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Test", "Second", "2", 1280, 720)
        .await
        .unwrap();
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let (_, consumer) = stream(
        &mut isle,
        serde_json::json!({"portion": "HEADLESS-1", "x": info.x + 10, "y": info.y + 40, "w": 100, "h": 60}),
    )
    .await;
    consumer.wait_for("format 100x60").await;
    wait_until(|| async { last_frame(&consumer).is_some_and(|(_, c)| c == "ff0000") }).await;

    let (second, _consumer) = stream(&mut isle, serde_json::json!({"screen": "HEADLESS-2"})).await;
    test.remove_output("HEADLESS-2").await.unwrap();
    wait_until(|| async { !casts(&isle).await.iter().any(|c| c["id"] == second) }).await;
    assert_eq!(casts(&isle).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_metadata_cursor_moves_over_a_still_frame_without_new_pictures() {
    let mut isle = Isle::start("cast-cursor").await;
    if !isle.start_audio().await {
        return;
    }
    let test = isle.test().await;
    test.pointer_to(100.0, 100.0).await.unwrap();
    let (_, consumer) = stream(
        &mut isle,
        serde_json::json!({"portion": "HEADLESS-1", "x": 50, "y": 50, "w": 400, "h": 300,
                           "cursor": "metadata"}),
    )
    .await;
    consumer.wait_for("format 400x300").await;
    consumer.wait_for("cursor 50 50").await;
    wait_until(|| async { last_frame(&consumer).is_some() }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (frames, _) = last_frame(&consumer).unwrap();

    // The pointer moves over a still screen: the cursor follows with no new picture.
    test.pointer_to(250.0, 150.0).await.unwrap();
    consumer.wait_for("cursor 200 100").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(last_frame(&consumer).unwrap().0, frames);

    // Off the portion, the cursor goes.
    test.pointer_to(900.0, 700.0).await.unwrap();
    consumer.wait_for("cursor gone").await;

    // Over a window with a cursor surface of its own, its image goes with it.
    let window = isle.client("isle.test.Cursor", "3366ff");
    window.wait_for("keyboard-enter").await;
    window.send("cursor-surface");
    let info = isle.wait_for_window("isle.test.Cursor").await;
    isle.windows_proxy()
        .await
        .set_box(info.id, 60, 60, 300, 200)
        .await
        .unwrap();
    wait_until(|| async { isle.window(info.id).await.x == 60 }).await;
    test.pointer_to(150.0, 150.0).await.unwrap();
    window.wait_for("pointer-enter 90 90").await;
    consumer.wait_for("bitmap 128x128").await;
}
