//! Stage Manager: one set on the stage, the rest parked, activation swapping them, the
//! column and Mission Control's sets. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_ipc::shell::ShellProxy;
use serde_json::Value;

async fn mission(isle: &Isle) -> Value {
    serde_json::from_str(&isle.test().await.mission().await.unwrap()).unwrap()
}

async fn column_open(isle: &Isle) -> bool {
    ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .iter()
        .any(|s| s.name == "stage" && s.open)
}

async fn centre(isle: &Isle) -> [u8; 3] {
    isle.shot("HEADLESS-1", "stage").await.at(960, 600)
}

#[tokio::test(flavor = "multi_thread")]
async fn one_set_holds_the_stage_and_activating_another_swaps_it_in() {
    let mut isle = Isle::start_with(
        "stage",
        "stage-manager {\n    recent-apps #true\n}\n",
    )
    .await;
    let _red = isle.client("isle.test.Red", "ff0000");
    let red = isle.wait_for_window("isle.test.Red").await.id;
    let _green = isle.client("isle.test.Green", "00ff00");
    let green = isle.wait_for_window("isle.test.Green").await.id;
    let proxy = isle_ipc::desktops::DesktopsProxy::new(&isle.bus).await.unwrap();
    assert!(proxy.set_stage("", 1, true).await.unwrap());

    // Green was used last: its set is on the stage, centred in the area beside the column.
    wait_until(|| async { centre(&isle).await == [0, 255, 0] }).await;
    wait_until(|| async { column_open(&isle).await }).await;
    isle.shot("HEADLESS-1", "column").await;
    let g = isle.window(green).await;
    assert!(g.x >= 192, "clear of the column: {}", g.x);

    isle.windows_proxy().await.focus(red).await.unwrap();
    wait_until(|| async { centre(&isle).await == [255, 0, 0] }).await;

    // Mission Control spreads both, clustered by set.
    proxy.mission_control().await.unwrap();
    wait_until(|| async {
        mission(&isle).await["screens"][0]["clusters"]
            .as_array()
            .is_some_and(|c| c.len() == 2)
    })
    .await;
    proxy.mission_control().await.unwrap();
    wait_until(|| async { mission(&isle).await.is_null() }).await;

    // Off again: both shown where they are.
    assert!(proxy.set_stage("", 1, false).await.unwrap());
    wait_until(|| async { !column_open(&isle).await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_window_joins_its_apps_set_on_the_stage() {
    let mut isle = Isle::start("stage-map").await;
    let _red = isle.client("isle.test.Red", "ff0000");
    isle.wait_for_window("isle.test.Red").await;
    let proxy = isle_ipc::desktops::DesktopsProxy::new(&isle.bus).await.unwrap();
    assert!(proxy.set_stage("", 1, true).await.unwrap());
    let _green = isle.client("isle.test.Green", "00ff00");
    isle.wait_for_window("isle.test.Green").await;
    wait_until(|| async { centre(&isle).await == [0, 255, 0] }).await;
    // The column stays hidden with recent apps off.
    assert!(!column_open(&isle).await);
}
