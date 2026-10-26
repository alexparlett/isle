//! Show Desktop: windows aside at the edges and every way back. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

const CMD: u32 = 125;
const F3: u32 = 61;
const LEFT_BUTTON: u32 = 272;

fn showing(w: &isle_ipc::WindowInfo) -> bool {
    w.x + w.width <= 24 + 1 || w.x >= 1920 - 24 - 1 || w.y >= 1080 - 24 - 1
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_f3_moves_every_window_aside_and_again_puts_them_back() {
    let mut isle = Isle::start("show-desktop").await;
    let _a = isle.client("isle.test.A", "ff0000");
    let a = isle.wait_for_window("isle.test.A").await;
    let _b = isle.client("isle.test.B", "00ff00");
    let b = isle.wait_for_window("isle.test.B").await;

    isle.chord(&[CMD, F3]).await;
    wait_until(|| async { showing(&isle.window(a.id).await) && showing(&isle.window(b.id).await) })
        .await;
    let moved = isle.window(a.id).await;
    assert_eq!(
        (moved.width, moved.height),
        (a.width, a.height),
        "sizes kept"
    );

    isle.chord(&[CMD, F3]).await;
    wait_until(|| async {
        let (x, y) = (isle.window(a.id).await, isle.window(b.id).await);
        (x.x, x.y) == (a.x, a.y) && (y.x, y.y) == (b.x, b.y)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_on_a_sliver_or_a_new_window_puts_them_back() {
    let mut isle = Isle::start("show-desktop-back").await;
    let _a = isle.client("isle.test.A", "ff0000");
    let a = isle.wait_for_window("isle.test.A").await;

    isle.chord(&[CMD, F3]).await;
    wait_until(|| async { showing(&isle.window(a.id).await) }).await;
    let w = isle.window(a.id).await;
    // A point of the sliver still on screen.
    let x = (w.x + w.width / 2).clamp(2, 1918) as f64;
    let y = (w.y + w.height / 2).clamp(2, 1078) as f64;
    let (x, y) = if w.x < 0 {
        (10.0, y)
    } else if w.x >= 1896 {
        (1910.0, y)
    } else {
        (x, 1070.0)
    };
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { !showing(&isle.window(a.id).await) }).await;

    isle.chord(&[CMD, F3]).await;
    wait_until(|| async { showing(&isle.window(a.id).await) }).await;
    let _b = isle.client("isle.test.B", "00ff00");
    isle.wait_for_window("isle.test.B").await;
    wait_until(|| async { !showing(&isle.window(a.id).await) }).await;
}
