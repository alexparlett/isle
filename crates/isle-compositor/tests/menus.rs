#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_ipc::menus::MenusProxy;
use serde_json::Value;

async fn show(isle: &Isle, id: u64) -> Value {
    let text = MenusProxy::new(&isle.bus)
        .await
        .unwrap()
        .show(id)
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

async fn bar(isle: &Isle) -> Value {
    let text = MenusProxy::new(&isle.bus)
        .await
        .unwrap()
        .bar()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn titles(bar: &Value) -> Vec<String> {
    bar["titles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["label"].as_str().unwrap().to_owned())
        .collect()
}

fn item<'a>(bar: &'a Value, title: &str, label: &str) -> Option<&'a Value> {
    bar["titles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["label"] == title)?["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == label)
}

async fn activate(isle: &Isle, path: &[&str]) -> bool {
    MenusProxy::new(&isle.bus)
        .await
        .unwrap()
        .activate(path)
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn menus_set_through_the_kde_protocol_fill_the_bar_with_mac_hints() {
    let mut isle = Isle::start("menus-appmenu").await;
    let client = isle.client_with(&[
        "--app-id",
        "isle.test.Qt",
        "--colour",
        "3366ff",
        "--appmenu",
    ]);
    client.wait_for("menus exported").await;
    let w = isle.wait_for_window("isle.test.Qt").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "dbusmenu" }).await;
    wait_until(|| async { titles(&bar(&isle).await).contains(&"Edit".to_owned()) }).await;
    let b = bar(&isle).await;
    let labels = titles(&b);
    assert_eq!(
        &labels[2..],
        ["File", "Edit", "Window", "Help"],
        "{labels:?}"
    );
    assert_eq!(
        item(&b, "File", "New").unwrap()["hint"],
        "⌘N",
        "Ctrl+N shown as the chord pressed"
    );
    assert_eq!(item(&b, "Edit", "Word Wrap").unwrap()["mark"], "check");
    assert_eq!(item(&b, "Edit", "Paste").unwrap()["enabled"], false);
    assert!(
        item(&b, "Help", "About Test").is_some(),
        "the app's Help items join Isle's"
    );
    let about = &b["titles"][1]["items"][0];
    assert_eq!(about["label"], "About isle.test.Qt");

    client.clear();
    assert!(activate(&isle, &["File", "New"]).await);
    client.wait_for("menu activated New").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn gtk_s_menus_arrive_through_gtk_shell_and_their_actions_run() {
    let mut isle = Isle::start("menus-gtk").await;
    let client = isle.client_with(&[
        "--app-id",
        "isle.test.Gtk",
        "--colour",
        "3366ff",
        "--gtk-menu",
    ]);
    client.wait_for("gtk capabilities 2").await;
    client.wait_for("menus exported").await;
    let w = isle.wait_for_window("isle.test.Gtk").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "gmenu" }).await;
    wait_until(|| async { item(&bar(&isle).await, "Edit", "Copy").is_some() }).await;
    let b = bar(&isle).await;
    assert_eq!(item(&b, "Edit", "Copy").unwrap()["hint"], "⌘C");
    assert_eq!(item(&b, "Edit", "Word Wrap").unwrap()["mark"], "check");
    client.clear();
    assert!(activate(&isle, &["Edit", "Copy"]).await);
    client.wait_for("menu activated copy").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_registration_no_window_matches_is_found_by_the_app_s_process() {
    let mut isle = Isle::start("menus-pid").await;
    let client = isle.client_with(&[
        "--app-id",
        "isle.test.Electron",
        "--colour",
        "3366ff",
        "--register",
        "4242",
    ]);
    client.wait_for("menus exported").await;
    let w = isle.wait_for_window("isle.test.Electron").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "dbusmenu" }).await;
    assert_eq!(show(&isle, w.id).await["registrar"], "owned");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_registers_after_its_window_maps_gets_its_menus() {
    let mut isle = Isle::start("menus-late").await;
    let _client = isle.client_with(&[
        "--app-id",
        "isle.test.Late",
        "--colour",
        "3366ff",
        "--register",
        "4243",
        "--register-late",
    ]);
    let w = isle.wait_for_window("isle.test.Late").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "none" }).await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "dbusmenu" }).await;
    assert!(titles(&bar(&isle).await).contains(&"Edit".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_exports_nothing_keeps_the_fallback() {
    let mut isle = Isle::start("menus-none").await;
    let _client = isle.client("chromium", "3366ff");
    let w = isle.wait_for_window("chromium").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "none" }).await;
    assert!(item(&bar(&isle).await, "File", "New Tab").is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn search_finds_items_in_submenus_the_app_fills_once_help_opens() {
    const SHIFT: u32 = 42;
    const CMD: u32 = 125;
    const SLASH: u32 = 53;
    let mut isle = Isle::start("menus-search").await;
    let client = isle.client_with(&["--app-id", "isle.test.Qt", "--colour", "3366ff", "--appmenu"]);
    client.wait_for("menus exported").await;
    let w = isle.wait_for_window("isle.test.Qt").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "dbusmenu" }).await;
    let menus = MenusProxy::new(&isle.bus).await.unwrap();
    let found: Value = serde_json::from_str(&menus.search("wrap").await.unwrap()).unwrap();
    assert_eq!(found[0]["label"], "Word Wrap");
    assert_eq!(found[0]["path"], serde_json::json!(["Edit"]));
    let empty: Value = serde_json::from_str(&menus.search("recent one").await.unwrap()).unwrap();
    assert!(empty.as_array().unwrap().is_empty(), "Open Recent is not filled yet");

    // Shift+Cmd+/ opens Help with its search, and asks the app to fill its submenus.
    isle.chord(&[SHIFT, CMD, SLASH]).await;
    wait_until(|| async {
        let found: Value = serde_json::from_str(&menus.search("recent one").await.unwrap()).unwrap();
        found[0]["path"] == serde_json::json!(["File", "Open Recent"])
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dialog_with_no_menus_of_its_own_keeps_its_app_s() {
    let mut isle = Isle::start("menus-dialog").await;
    let client = isle.client_with(&["--app-id", "isle.test.Qt", "--colour", "3366ff", "--appmenu"]);
    client.wait_for("menus exported").await;
    let w = isle.wait_for_window("isle.test.Qt").await;
    wait_until(|| async { show(&isle, w.id).await["source"] == "dbusmenu" }).await;
    client.send("dialog");
    wait_until(|| async { isle.windows().await.iter().any(|d| d.parent == w.id && d.focused) }).await;
    wait_until(|| async { titles(&bar(&isle).await).contains(&"Edit".to_owned()) }).await;
    assert!(activate(&isle, &["Edit", "Copy"]).await);
    client.wait_for("menu activated Copy").await;
}
