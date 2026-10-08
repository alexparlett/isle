//! `org.freedesktop.ScreenSaver`: inhibitors hold idle until their caller lets go or leaves,
//! Lock locks, and nothing on the bus ends the lock. Needs `--features test-hooks`.
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
    fn lock(&self) -> zbus::Result<()>;
    fn get_active(&self) -> zbus::Result<bool>;
    fn set_active(&self, active: bool) -> zbus::Result<bool>;
    #[zbus(signal)]
    fn active_changed(&self, active: bool) -> zbus::Result<()>;
}

async fn holders(isle: &Isle) -> Vec<String> {
    let idle: serde_json::Value = serde_json::from_str(
        &SessionProxy::new(&isle.bus)
            .await
            .unwrap()
            .idle()
            .await
            .unwrap(),
    )
    .unwrap();
    idle["holders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_inhibitor_holds_idle_until_let_go_or_its_caller_leaves() {
    let isle = Isle::start("screensaver-inhibit").await;
    let player = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let saver = ScreenSaverProxy::new(&player).await.unwrap();
    let cookie = saver.inhibit("Player", "Playing a video").await.unwrap();
    wait_until(|| async { holders(&isle).await == ["Player"] }).await;
    saver.un_inhibit(cookie).await.unwrap();
    wait_until(|| async { holders(&isle).await.is_empty() }).await;

    saver.inhibit("Player", "Playing a video").await.unwrap();
    wait_until(|| async { holders(&isle).await == ["Player"] }).await;
    player.close().await.unwrap();
    wait_until(|| async { holders(&isle).await.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn lock_locks_and_set_active_false_is_refused() {
    let isle = Isle::start("screensaver-lock").await;
    let saver = ScreenSaverProxy::builder(&isle.bus)
        .path("/org/freedesktop/ScreenSaver")
        .unwrap()
        .build()
        .await
        .unwrap();
    let mut changes = saver.receive_active_changed().await.unwrap();
    assert!(!saver.get_active().await.unwrap());
    saver.lock().await.unwrap();
    let change = changes.next().await.unwrap();
    assert!(change.args().unwrap().active);
    assert!(saver.get_active().await.unwrap());
    assert!(!saver.set_active(false).await.unwrap());
    assert!(
        saver.get_active().await.unwrap(),
        "the bus cannot end the lock"
    );
}
