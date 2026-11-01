//! The greeter mode: the lock and nothing else, with no Wayland socket, no names on the
//! session bus and nothing written under the home. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_greeter_is_the_lock_and_nothing_else() {
    let isle = Isle::start_greeter("greeter-only", |_| {}, |_| Vec::new()).await;
    assert_eq!(status(&isle).await["locked"], true);
    wait_until(|| async {
        let kinds = isle.test().await.frame_kinds("HEADLESS-1").await.unwrap();
        kinds.iter().any(|k| k == "shell")
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("greeter").await;

    // No Wayland socket for clients.
    let sockets: Vec<_> = std::fs::read_dir(isle.dir.join("runtime"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("wayland-"))
        .collect();
    assert!(sockets.is_empty(), "{sockets:?}");

    // None of a session's names.
    let dbus = zbus::fdo::DBusProxy::new(&isle.bus).await.unwrap();
    for name in [
        "org.freedesktop.Notifications",
        "org.kde.StatusNotifierWatcher",
        "org.freedesktop.impl.portal.desktop.isle",
        "org.freedesktop.ScreenSaver",
        "com.canonical.AppMenu.Registrar",
    ] {
        let name = zbus::names::BusName::try_from(name).unwrap();
        assert!(!dbus.name_has_owner(name.clone()).await.unwrap(), "{name} is owned");
    }

    // The greeter's lock never ends.
    isle.test().await.unlock().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(status(&isle).await["locked"], true);

    // Nothing written in the home or the user's configuration.
    for dir in ["home", "config"] {
        let left: Vec<_> = std::fs::read_dir(isle.dir.join(dir)).unwrap().collect();
        assert!(left.is_empty(), "{dir}: {left:?}");
    }
}
