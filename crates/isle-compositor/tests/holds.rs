//! A window's pid on the window list, and the holds on idle the compositor takes for clients,
//! each with its client's pid. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use futures_util::StreamExt;
use isle_ipc::session::SessionProxy;

#[zbus::proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/ScreenSaver"
)]
trait ScreenSaver {
    fn inhibit(&self, application_name: &str, reason_for_inhibit: &str) -> zbus::Result<u32>;
    fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_carries_its_clients_pid() {
    let mut isle = Isle::start("holds-pid").await;
    let client = isle.client("isle.test.Pid", "ff0000");
    let info = isle.wait_for_window("isle.test.Pid").await;
    assert_eq!(info.pid, client.pid());
}

#[tokio::test(flavor = "multi_thread")]
async fn holds_name_the_inhibiting_surface_and_the_screensaver_caller() {
    let mut isle = Isle::start("holds-list").await;
    let session = SessionProxy::new(&isle.bus).await.unwrap();
    let mut changed = session.receive_holds_changed().await.unwrap();
    let window = isle.client("isle.test.Video", "ff0000");
    window.wait_for("keyboard-enter").await;
    window.send("idle-inhibit");
    changed.next().await.unwrap();
    let pid = window.pid();
    wait_until(|| async { session.holds().await.unwrap().iter().any(|h| h.pid == pid && h.what == "idle") }).await;

    let player = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let saver = ScreenSaverProxy::new(&player).await.unwrap();
    let cookie = saver.inhibit("Player", "Playing a video").await.unwrap();
    let me = std::process::id() as i32;
    wait_until(|| async {
        session.holds().await.unwrap().iter().any(|h| h.app == "Player" && h.pid == me)
    })
    .await;
    saver.un_inhibit(cookie).await.unwrap();
    wait_until(|| async { !session.holds().await.unwrap().iter().any(|h| h.app == "Player") }).await;
}
