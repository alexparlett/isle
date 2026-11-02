//! A pretend HDR screen: the headless output drawn through the HDR pass into 10 bits (P22).
//! Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

use isle_ipc::displays::DisplaysProxy;

mod common;

use common::{Isle, wait_until};

fn pq(nits: f64) -> u32 {
    let y = (nits / 10000.0).clamp(0.0, 1.0).powf(2610.0 / 16384.0);
    (((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y)).powf(78.84375) * 1023.0).round() as u32
}

fn near(got: (u32, u32, u32), want: u32) -> bool {
    [got.0, got.1, got.2].iter().all(|v| v.abs_diff(want) <= 2)
}

/// A white window on HEADLESS-1 made HDR, and the pixel at its centre.
async fn white_window(isle: &mut Isle) -> (i32, i32) {
    let _client = isle.client("isle.test.White", "ffffff");
    let window = isle.wait_for_window("isle.test.White").await;
    isle.test().await.make_hdr("HEADLESS-1", 302.0, 203.0).await.unwrap();
    (window.x + window.width / 2, window.y + window.height / 2)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_sdr_window_shows_at_the_reference_white() {
    let mut isle = Isle::start("hdr-white").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    let white = pq(203.0);
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), white) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sdr_brightness_applies_without_relighting() {
    let mut isle = Isle::start("hdr-sdr-white").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(203.0)) }).await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let info = displays.list().await.unwrap().into_iter().find(|d| d.name == "HEADLESS-1").unwrap();
    let identity = format!("{} {} {}", info.make, info.model, info.serial);
    displays
        .configure(&format!(r#"{{"{identity}": {{"sdr_white": 280}}}}"#))
        .await
        .unwrap();
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(280.0)) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_still_hdr_desktop_presents_no_frames() {
    let mut isle = Isle::start("hdr-idle").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(203.0)) }).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    // The island's clock may turn a minute in the window.
    assert!(test.frames("HEADLESS-1").await.unwrap() - before <= 1);
}
