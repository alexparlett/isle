//! Split View: the chooser, the panes, the divider, and a pane leaving. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use serde_json::Value;

const ESCAPE: u32 = 1;
const LEFT_BUTTON: u32 = 272;
/// Each pane of the 1920-wide headless screen beside the 8-point divider.
const HALF: i32 = (1920 - 8) / 2;

async fn mission(isle: &Isle) -> Value {
    serde_json::from_str(&isle.test().await.mission().await.unwrap()).unwrap()
}

async fn desktops(isle: &Isle) -> Vec<isle_ipc::DesktopInfo> {
    isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
}

async fn proxy(isle: &Isle) -> isle_ipc::desktops::DesktopsProxy<'static> {
    isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
}

async fn click(isle: &Isle, x: f64, y: f64) {
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
}

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

/// Red on the left of the screen and green chosen for the right.
async fn paired(isle: &mut Isle) -> (u64, u64, common::Client, common::Client) {
    let red_client = isle.client("isle.test.Red", "ff0000");
    let red = isle.wait_for_window("isle.test.Red").await.id;
    let green_client = isle.client("isle.test.Green", "00ff00");
    let green = isle.wait_for_window("isle.test.Green").await.id;
    assert!(proxy(isle).await.split(red, true).await.unwrap());
    wait_until(|| async { mission(isle).await["mode"]["chooser"] == red }).await;
    let (x, y) = tile_centre(&mission(isle).await, green);
    assert!(x > HALF as f64, "offered in the free half");
    click(isle, x, y).await;
    wait_until(|| async { mission(isle).await.is_null() }).await;
    wait_until(|| async {
        let (r, g) = (isle.window(red).await, isle.window(green).await);
        (r.x, r.width) == (0, HALF) && (g.x, g.width) == (HALF + 8, HALF) && r.height == 1080
    })
    .await;
    (red, green, red_client, green_client)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_chosen_in_the_free_half_shares_the_screen_as_two_tiled_panes() {
    let mut isle = Isle::start("split-pair").await;
    let (red, green, red_client, _green) = paired(&mut isle).await;
    let rows = desktops(&isle).await;
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[1].kind.as_str(), rows[1].current), ("split", true));
    red_client.wait_for_part("tiled-left").await;
    assert!(
        !red_client
            .lines
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .contains("fullscreen"),
        "a pane is not told it is full screen"
    );
    // The divider between them.
    let shot = isle.shot("HEADLESS-1", "split").await;
    assert_eq!(shot.at(100, 500), [255, 0, 0]);
    assert_eq!(shot.at(1800, 500), [0, 255, 0]);
    assert_ne!(shot.at(HALF as usize + 4, 300), [255, 0, 0]);
    let _ = green;
}

#[tokio::test(flavor = "multi_thread")]
async fn escape_in_the_chooser_puts_the_first_window_back() {
    let mut isle = Isle::start("split-cancel").await;
    let _red = isle.client("isle.test.Red", "ff0000");
    let before = isle.wait_for_window("isle.test.Red").await;
    let _green = isle.client("isle.test.Green", "00ff00");
    isle.wait_for_window("isle.test.Green").await;
    assert!(proxy(&isle).await.split(before.id, false).await.unwrap());
    wait_until(|| async { !mission(&isle).await.is_null() }).await;
    wait_until(|| async { isle.window(before.id).await.x == HALF + 8 }).await;
    // The free half on the left, veiled, offering green; red keeps the right.
    wait_until(|| async { isle.shot("HEADLESS-1", "chooser").await.at(1800, 500) == [255, 0, 0] })
        .await;
    isle.chord(&[ESCAPE]).await;
    wait_until(|| async { desktops(&isle).await.len() == 1 }).await;
    wait_until(|| async {
        let w = isle.window(before.id).await;
        (w.x, w.y, w.width, w.height) == (before.x, before.y, before.width, before.height)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_divider_moves_both_panes_and_a_double_click_halves_them() {
    let mut isle = Isle::start("split-divider").await;
    let (red, green, _r, _g) = paired(&mut isle).await;
    let test = isle.test().await;
    let x = HALF as f64 + 4.0;
    test.pointer_to(x, 500.0).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    for step in 1..=4 {
        test.pointer_to(x + 60.0 * step as f64, 500.0)
            .await
            .unwrap();
    }
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async {
        let (r, g) = (isle.window(red).await, isle.window(green).await);
        r.width > HALF + 200 && g.x == r.width + 8 && r.width + g.width == 1912
    })
    .await;
    let at = isle.window(red).await.width as f64 + 4.0;
    for _ in 0..2 {
        test.pointer_to(at, 500.0).await.unwrap();
        test.button(LEFT_BUTTON, true).await.unwrap();
        test.button(LEFT_BUTTON, false).await.unwrap();
    }
    wait_until(|| async { isle.window(red).await.width == HALF }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pane_leaving_or_closing_leaves_the_other_full_screen_alone() {
    let mut isle = Isle::start("split-leave").await;
    let (red, green, _r, green_client) = paired(&mut isle).await;
    assert!(
        proxy(&isle)
            .await
            .set_full_screen(red, false)
            .await
            .unwrap()
    );
    wait_until(|| async {
        let rows = desktops(&isle).await;
        rows.len() == 2 && rows[1].kind == "full-screen" && rows[1].window == green
    })
    .await;
    wait_until(|| async { isle.window(green).await.width == 1920 }).await;
    green_client.wait_for_part("fullscreen").await;

    // A pair again, then a pane closes.
    let mut isle2 = Isle::start("split-close").await;
    let (red2, _green2, _r2, green_client2) = paired(&mut isle2).await;
    green_client2.send("quit");
    wait_until(|| async {
        let rows = desktops(&isle2).await;
        rows.len() == 2 && rows[1].kind == "full-screen" && rows[1].window == red2
    })
    .await;
    wait_until(|| async { isle2.window(red2).await.width == 1920 }).await;
}
