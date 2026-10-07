//! The shell's glass by eye: `ISLE_TEST_KEEP=1 cargo test --features test-hooks --test glass
//! -- --ignored`, then look at the screenshots it leaves.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::Isle;

const CTRL: u32 = 29;
const F8: u32 = 66;
const F2: u32 = 60;
const ESC: u32 = 1;

async fn surfaces(backdrop: &str, light_window: bool, name: &str) {
    let mut isle = Isle::start_with(
        name,
        &format!("wallpaper {{\n    source \"{backdrop}\"\n}}\n"),
    )
    .await;
    if light_window {
        let _ = isle.client_with(&[
            "--app-id",
            "isle.test.Light",
            "--colour",
            "f2f2f2",
            "--size",
            "1900x1000",
        ]);
        isle.wait_for_window("isle.test.Light").await;
    }
    let _app = isle.client_with(&[
        "--app-id",
        "isle.test.App",
        "--colour",
        "3366ff",
        "--size",
        "500x300",
    ]);
    isle.wait_for_window("isle.test.App").await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    isle.shot("HEADLESS-1", "island").await;
    isle.chord(&[CTRL, F8]).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    isle.shot("HEADLESS-1", "control-centre").await;
    isle.chord(&[ESC]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    isle.chord(&[CTRL, F2]).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    isle.shot("HEADLESS-1", "menu").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "screenshots to look at"]
async fn glass_over_frost() {
    surfaces("frost", false, "glassfrost").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "screenshots to look at"]
async fn glass_over_a_light_window() {
    surfaces("frost", true, "glasslight").await;
}
