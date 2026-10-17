//! An Isle app on the compositor: its window under a transparent bar, one instance, the
//! desktop's theme. Runs `isle-settings` from the workspace's build. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::{path::PathBuf, time::Duration};

mod common;

use common::Isle;

const APP: &str = "io.github.alexparlett.Isle.Settings";

fn settings_bin() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle-settings");
    path.exists().then(|| path.to_string_lossy().into_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_opens_under_isles_lights_and_stays_one_instance() {
    let Some(bin) = settings_bin() else {
        eprintln!("skipped: build isle-system-settings first");
        return;
    };
    let mut isle = Isle::start("isle-app").await;
    let app = isle.spawn_logged(&bin, &[]);
    let w = isle.wait_for_window(APP).await;
    assert_eq!((w.width, w.height), (900, 760), "no bar added above it");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let shot = isle.shot("HEADLESS-1", "settings").await;
    if let Some(keep) = std::env::var_os("ISLE_TEST_KEEP") {
        std::fs::copy(isle.dir.join("settings.png"), PathBuf::from(keep).join("settings.png")).unwrap();
    }
    let (x, y) = (w.x as usize, w.y as usize);
    let sidebar = shot.at(x + 110, y + 300);
    assert_ne!(shot.at(x + 22, y + 26), sidebar, "the close light over the sidebar");
    assert_eq!(shot.at(x + 22, y + 4), sidebar, "nothing above the lights");

    let second = isle.spawn_logged(&bin, &["isle-settings://displays"]);
    common::wait_until(|| async { second.exited() }).await;
    app.wait_for_part("Open { uris: [\"isle-settings://displays\"]").await;
    assert_eq!(isle.windows().await.iter().filter(|w| w.app_id == APP).count(), 1);
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.shot("HEADLESS-1", "displays").await;
    if let Some(keep) = std::env::var_os("ISLE_TEST_KEEP") {
        std::fs::copy(isle.dir.join("displays.png"), PathBuf::from(keep).join("displays.png")).unwrap();
    }

    // Its own menus, exported over DBusMenu, in Isle's bar; Quit ends it.
    let menus = isle_ipc::menus::MenusProxy::new(&isle.bus).await.unwrap();
    common::wait_until(|| async { menus.bar().await.unwrap().contains("Quit System Settings") }).await;
    let bar = menus.bar().await.unwrap();
    for title in ["\"System Settings\"", "\"Edit\"", "\"Window\""] {
        assert!(bar.contains(title), "{title} in {bar}");
    }
    assert!(bar.contains("⌘Q"), "Quit's chord as the Mac's: {bar}");
    assert!(menus.activate(&["System Settings", "Quit System Settings"]).await.unwrap());
    common::wait_until(|| async { app.exited() }).await;
}
