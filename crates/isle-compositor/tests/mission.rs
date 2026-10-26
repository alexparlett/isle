//! Mission Control and App Exposé: opening, the tiles drawn by the compositor, choosing,
//! the keyboard kept from windows, and drags. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use serde_json::Value;

const CTRL: u32 = 29;
const UP: u32 = 103;
const DOWN: u32 = 108;
const ESCAPE: u32 = 1;
const A: u32 = 30;
const LEFT_BUTTON: u32 = 272;

async fn mission(isle: &Isle) -> Value {
    serde_json::from_str(&isle.test().await.mission().await.unwrap()).unwrap()
}

/// A tile's centre in the global space, on the first output.
fn tile_centre(m: &Value, window: u64) -> (f64, f64) {
    let tile = m["screens"][0]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["window"] == window)
        .unwrap_or_else(|| panic!("no tile for {window}: {m}"));
    let r = &tile["rect"];
    (
        r[0].as_f64().unwrap() + r[2].as_f64().unwrap() / 2.0,
        r[1].as_f64().unwrap() + r[3].as_f64().unwrap() / 2.0,
    )
}

async fn open(isle: &Isle) -> Value {
    isle.chord(&[CTRL, UP]).await;
    wait_until(|| async { mission(isle).await["progress"].as_f64() == Some(1.0) }).await;
    mission(isle).await
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_up_spreads_every_window_drawn_by_the_compositor_and_escape_closes() {
    let mut isle = Isle::start("mission-open").await;
    let _red = isle.client("isle.test.Red", "ff0000");
    let red = isle.wait_for_window("isle.test.Red").await.id;
    let _green = isle.client("isle.test.Green", "00ff00");
    let green = isle.wait_for_window("isle.test.Green").await.id;

    let m = open(&isle).await;
    let tiles = m["screens"][0]["tiles"].as_array().unwrap();
    assert_eq!(tiles.len(), 2, "{m}");
    assert!(
        m["screens"][0]["bar"]["items"].as_array().unwrap().len() >= 2,
        "a desktop and +"
    );
    wait_until(|| async { mission(&isle).await["screens"][0]["miniatures"] == 1 }).await;

    // Each tile shows its window where the model put it.
    let shot = isle.shot("HEADLESS-1", "mission").await;
    for (id, colour) in [(red, [255, 0, 0]), (green, [0, 255, 0])] {
        let (x, y) = tile_centre(&m, id);
        let px = shot.at(x as usize, y as usize);
        assert!(
            px.iter()
                .zip(colour)
                .all(|(a, b)| (*a as i32 - b).abs() < 12),
            "tile {id} at ({x}, {y}) shows {px:?}"
        );
    }

    isle.chord(&[ESCAPE]).await;
    wait_until(|| async { mission(&isle).await.is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_on_a_tile_chooses_its_window() {
    let mut isle = Isle::start("mission-click").await;
    let _red = isle.client("isle.test.Red", "ff0000");
    let red = isle.wait_for_window("isle.test.Red").await.id;
    let _green = isle.client("isle.test.Green", "00ff00");
    isle.wait_for_window("isle.test.Green").await;

    let m = open(&isle).await;
    let (x, y) = tile_centre(&m, red);
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { mission(&isle).await.is_null() }).await;
    wait_until(|| async { isle.window(red).await.focused }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn typing_reaches_no_window_while_it_is_open() {
    let mut isle = Isle::start("mission-keys").await;
    let client = isle.client("isle.test.Keys", "0000ff");
    let id = isle.wait_for_window("isle.test.Keys").await.id;
    wait_until(|| async { isle.window(id).await.focused }).await;

    open(&isle).await;
    isle.chord(&[A]).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !client
            .lines
            .lock()
            .unwrap()
            .iter()
            .any(|l| l == "key 30 pressed"),
        "a key went through"
    );
    isle.chord(&[ESCAPE]).await;
    wait_until(|| async { mission(&isle).await.is_null() }).await;
    isle.chord(&[A]).await;
    client.wait_for("key 30 pressed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_down_opens_app_expose_of_the_front_app() {
    let mut isle = Isle::start("mission-expose").await;
    let _a = isle.client("isle.test.One", "ff0000");
    isle.wait_for_window("isle.test.One").await;
    let _b = isle.client("isle.test.Two", "00ff00");
    let two = isle.wait_for_window("isle.test.Two").await.id;
    wait_until(|| async { isle.window(two).await.focused }).await;

    isle.chord(&[CTRL, DOWN]).await;
    wait_until(|| async { !mission(&isle).await.is_null() }).await;
    let m = mission(&isle).await;
    assert_eq!(m["mode"], "isle.test.Two", "{m}");
    let tiles = m["screens"][0]["tiles"].as_array().unwrap();
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0]["window"], two);
    assert!(m["screens"][0]["bar"].is_null());
    // ⌃↑ goes from App Exposé to Mission Control.
    isle.chord(&[CTRL, UP]).await;
    wait_until(|| async { mission(&isle).await["mode"].is_null() }).await;
    assert_eq!(
        mission(&isle).await["screens"][0]["tiles"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tile_dropped_on_the_plus_tile_gets_a_desktop_of_its_own() {
    let mut isle = Isle::start("mission-drag").await;
    let _red = isle.client("isle.test.Red", "ff0000");
    let red = isle.wait_for_window("isle.test.Red").await.id;
    let _green = isle.client("isle.test.Green", "00ff00");
    isle.wait_for_window("isle.test.Green").await;

    let m = open(&isle).await;
    let (x, y) = tile_centre(&m, red);
    let plus = m["screens"][0]["bar"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["desk"].is_null())
        .unwrap()["miniature"]
        .clone();
    let (px, py) = (
        plus[0].as_f64().unwrap() + plus[2].as_f64().unwrap() / 2.0,
        plus[1].as_f64().unwrap() + plus[3].as_f64().unwrap() / 2.0,
    );
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    for step in 1..=6 {
        let f = step as f64 / 6.0;
        test.pointer_to(x + (px - x) * f, y + (py - y) * f)
            .await
            .unwrap();
    }
    wait_until(|| async { !mission(&isle).await["drag"].is_null() }).await;
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async {
        let m = mission(&isle).await;
        m["screens"][0]["desks"]
            .as_array()
            .is_some_and(|d| d.len() == 2)
            && m["screens"][0]["desks"][1]["windows"]
                .as_array()
                .is_some_and(|w| w.iter().any(|w| *w == red))
    })
    .await;
    // Still open, the red window off the desktop in view.
    let m = mission(&isle).await;
    assert_eq!(m["screens"][0]["current"], 0);
    assert_eq!(m["screens"][0]["tiles"].as_array().unwrap().len(), 1);
}
