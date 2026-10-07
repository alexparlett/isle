//! The desktop's own notifications: a stick plugged in, a broken settings file said once,
//! and the last crash at start.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::udisks::UDisks;
use isle_ipc::notifications::NotificationsProxy;

async fn entries(isle: &Isle) -> Vec<serde_json::Value> {
    let text = NotificationsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["entries"].as_array().cloned().unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stick_plugged_in_says_so_and_mounts_from_its_notification() {
    let isle = Isle::start("desktop-stick").await;
    let udisks = UDisks::start(&isle.system_address()).await.unwrap();
    wait_until(|| async { isle.services().await["disks"]["available"] == true }).await;
    udisks.plug("STICK").await.unwrap();
    wait_until(|| async {
        entries(&isle)
            .await
            .iter()
            .any(|e| e["summary"] == "STICK connected")
    })
    .await;
    let entry = entries(&isle)
        .await
        .into_iter()
        .find(|e| e["summary"] == "STICK connected")
        .unwrap();
    let keys: Vec<&str> = entry["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a[0].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["default", "mount", "eject"]);
    let id = entry["id"].as_u64().unwrap() as u32;
    let proxy = NotificationsProxy::new(&isle.bus).await.unwrap();
    assert!(proxy.invoke(id, "mount").await.unwrap());
    wait_until(|| async { udisks.mounted("STICK").await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_settings_file_is_said_once_until_it_is_fixed() {
    let isle = Isle::start("desktop-settings").await;
    let file = isle.dir.join("config/settings.kdl");
    let said = || async {
        entries(&isle)
            .await
            .iter()
            .filter(|e| {
                e["summary"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("settings.kdl has an error"))
            })
            .count()
    };
    std::fs::write(&file, "keyboard {\n    repeat-rate \n").unwrap();
    wait_until(|| async { said().await == 1 }).await;
    std::fs::write(&file, "keyboard {\n    repeat-rate \n\n").unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    assert_eq!(said().await, 1, "the same broken place is said once");
    std::fs::write(&file, "keyboard {\n    repeat-rate 30\n}\n").unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    std::fs::write(&file, "keyboard {\n    repeat-rate \n").unwrap();
    wait_until(|| async { said().await == 2 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_last_crash_is_said_at_the_next_start() {
    let isle = Isle::start_prepared("desktop-crash", "", |dir| {
        let crashes = dir.join("state/isle/crashes");
        std::fs::create_dir_all(&crashes).unwrap();
        std::fs::write(
            crashes.join("2026-10-07T10-00-00.txt"),
            "isle crash record 1\n",
        )
        .unwrap();
    })
    .await;
    wait_until(|| async {
        entries(&isle)
            .await
            .iter()
            .any(|e| e["summary"] == "Isle quit unexpectedly.")
    })
    .await;
}
