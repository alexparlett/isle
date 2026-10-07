//! The tray: Isle's own watcher, an item shown with its icon, its presses, its menu as a
//! page, its order, and the watcher coming back.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

async fn tray(isle: &Isle) -> Vec<serde_json::Value> {
    isle.island_output().await["panel"]["tray"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn key(isle: &Isle, title: &str) -> String {
    wait_until(|| async { tray(isle).await.iter().any(|t| t["title"] == title) }).await;
    tray(isle)
        .await
        .iter()
        .find(|t| t["title"] == title)
        .unwrap()["key"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_item_shows_activates_opens_its_menu_and_goes_with_its_owner() {
    let mut isle = Isle::start("tray").await;
    let app = isle.client("isle.test.Tray", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.send("tray");
    app.wait_for("tray registered").await;
    isle.open_panel(None).await;
    let key = key(&isle, "Test Tray").await;
    let item = tray(&isle).await[0].clone();
    let icon = item["icon"].as_str().expect("its pixmap as a file");
    assert!(std::path::Path::new(icon).exists());
    assert_eq!(item["attention"], false);
    isle.panel_settled().await;
    isle.keep_shot("tray").await;

    isle.panel(serde_json::json!({ "tray": [key, "primary"] }))
        .await;
    app.wait_for("tray activated").await;
    wait_until(|| async { isle.island_output().await["panel"].is_null() }).await;

    isle.open_panel(None).await;
    isle.panel(serde_json::json!({ "tray": [key, "middle"] }))
        .await;
    app.wait_for("tray secondary").await;
    isle.panel(serde_json::json!({ "tray": [key, "scroll", 2] }))
        .await;
    app.wait_for("tray scroll 2").await;

    // Secondary opens its menu as a page; a submenu is a page of its own.
    isle.panel(serde_json::json!({ "tray": [key, "secondary"] }))
        .await;
    wait_until(|| async { isle.rows().await.iter().any(|r| r["label"] == "Edit") }).await;
    isle.panel(serde_json::json!({ "page": format!("tray/{key}/2") }))
        .await;
    wait_until(|| async { isle.rows().await.iter().any(|r| r["label"] == "Copy") }).await;
    let rows = isle.rows().await;
    let paste = rows.iter().find(|r| r["label"] == "Paste").unwrap();
    assert_eq!(paste["enabled"], false);
    let wrap = rows.iter().find(|r| r["label"] == "Word Wrap").unwrap();
    assert_eq!(wrap["checked"], true);
    isle.panel_settled().await;
    isle.keep_shot("tray-menu").await;
    isle.panel(serde_json::json!({ "choose": ["tray", "20"] }))
        .await;
    app.wait_for("menu activated Copy").await;
    wait_until(|| async { isle.island_output().await["panel"].is_null() }).await;

    // A submenu the app fills only when asked.
    isle.open_panel(Some(&format!("tray/{key}/1/11"))).await;
    wait_until(|| async { isle.rows().await.iter().any(|r| r["label"] == "Recent One") }).await;
    isle.panel(serde_json::json!({ "page": null })).await;

    app.send("tray-attention");
    wait_until(|| async {
        tray(&isle)
            .await
            .first()
            .is_some_and(|t| t["attention"] == true)
    })
    .await;

    app.send("quit");
    wait_until(|| async { tray(&isle).await.is_empty() }).await;
    assert!(
        isle.island_output().await["panel"]["page"].is_null(),
        "its menu went with it"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn items_by_name_keep_their_order_and_register_again_when_the_watcher_returns() {
    let mut isle = Isle::start("tray-watcher").await;
    let first = isle.client("isle.test.One", "3366ff");
    first.wait_for("keyboard-enter").await;
    first.send("tray --name --id one");
    first.wait_for("tray registered").await;
    let second = isle.client("isle.test.Two", "ff6633");
    second.wait_for("keyboard-enter").await;
    second.send("tray --item-is-menu --id two");
    second.wait_for("tray registered").await;
    isle.open_panel(None).await;
    wait_until(|| async { tray(&isle).await.len() == 2 }).await;

    // An item that is its menu opens it on a click.
    let keys: Vec<String> = tray(&isle)
        .await
        .iter()
        .map(|t| t["key"].as_str().unwrap().to_owned())
        .collect();
    isle.panel(serde_json::json!({ "tray": [keys[1], "primary"] }))
        .await;
    wait_until(|| async { isle.rows().await.iter().any(|r| r["label"] == "File") }).await;
    isle.panel(serde_json::json!({ "page": null })).await;

    isle.panel(serde_json::json!({ "tray-move": [keys[1], 0] }))
        .await;
    wait_until(|| async { tray(&isle).await[0]["key"] == keys[1].as_str() }).await;

    isle.test().await.tray_kill().await.unwrap();
    wait_until(|| async { tray(&isle).await.is_empty() }).await;
    let started = std::time::Instant::now();
    first.wait_for("tray registered").await;
    second.wait_for("tray registered").await;
    wait_until(|| async { tray(&isle).await.len() == 2 }).await;
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "back within a second or so: {:?}",
        started.elapsed()
    );
    let services = isle.services().await;
    assert_eq!(services["tray"]["available"], true);
}
