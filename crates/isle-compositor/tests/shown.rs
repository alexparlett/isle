//! Whether a window is shown, as suspended and `isle_visibility_v1` tell it: covered by opaque
//! windows, minimised and locked windows are not. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::Isle;
use isle_ipc::session::SessionProxy;

/// A window at `rect` that has said whether it is shown.
async fn placed(isle: &mut Isle, app: &str, colour: &str, rect: (i32, i32, i32, i32)) -> (common::Client, u64) {
    let client = isle.client(app, colour);
    let info = isle.wait_for_window(app).await;
    let windows = isle.windows_proxy().await;
    windows.set_box(info.id, rect.0, rect.1, rect.2, rect.3).await.unwrap();
    client.wait_for(&format!("configured {} {}", rect.2, rect.3)).await;
    client.send("visibility");
    client.wait_for("shown 1").await;
    (client, info.id)
}

/// Waits a little and checks `client` has not been told it is hidden.
async fn still_shown(client: &common::Client) {
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!client.printed("shown 0"), "{:?}", client.lines_with("shown"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_opaque_window_over_another_hides_it_and_moving_away_shows_it() {
    let mut isle = Isle::start("shown-cover").await;
    let (under, _) = placed(&mut isle, "isle.test.Under", "ff0000", (200, 200, 400, 300)).await;
    let (over, over_id) = placed(&mut isle, "isle.test.Over", "00ff00", (100, 100, 800, 600)).await;

    // Without an opaque region the window above may be translucent, so it covers nothing.
    still_shown(&under).await;

    over.send("opaque");
    under.wait_for("shown 0").await;
    under.wait_for("states suspended").await;

    under.clear();
    let windows = isle.windows_proxy().await;
    windows.set_box(over_id, 500, 100, 800, 600).await.unwrap();
    under.wait_for("shown 1").await;

    // Partly covered is shown.
    under.clear();
    still_shown(&under).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_off_every_screen_but_for_a_covered_part_is_hidden() {
    let mut isle = Isle::start("shown-edge").await;
    let (under, _) = placed(&mut isle, "isle.test.Under", "ff0000", (1700, 400, 400, 300)).await;
    let (over, _) = placed(&mut isle, "isle.test.Over", "00ff00", (1600, 300, 320, 500)).await;
    over.send("opaque");
    under.wait_for("shown 0").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn minimised_and_locked_windows_are_not_shown_until_they_return() {
    let mut isle = Isle::start("shown-park").await;
    let (window, id) = placed(&mut isle, "isle.test.Alone", "ff0000", (200, 200, 400, 300)).await;
    let windows = isle.windows_proxy().await;

    assert!(windows.minimise(id).await.unwrap());
    window.wait_for("shown 0").await;
    window.clear();
    assert!(windows.unminimise(id).await.unwrap());
    window.wait_for("shown 1").await;

    window.clear();
    SessionProxy::new(&isle.bus).await.unwrap().lock().await.unwrap();
    window.wait_for("shown 0").await;
    window.clear();
    isle.test().await.unlock().await.unwrap();
    window.wait_for("shown 1").await;
}
