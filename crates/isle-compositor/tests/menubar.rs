#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::{menus::MenusProxy, shell::ShellProxy};
use serde_json::Value;

const ESC: u32 = 1;
const CTRL: u32 = 29;
const F2: u32 = 60;
const LEFT: u32 = 272;
const RIGHT: u32 = 273;

async fn bar(isle: &Isle) -> Value {
    let text = MenusProxy::new(&isle.bus)
        .await
        .unwrap()
        .bar()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn title<'a>(bar: &'a Value, label: &str) -> Option<&'a Value> {
    bar["titles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["label"] == label)
}

fn item<'a>(menu: &'a Value, label: &str) -> Option<&'a Value> {
    menu["items"]
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
async fn ctrl_f2_opens_the_system_menu_and_escape_gives_the_keys_back() {
    let mut isle = Isle::start("bar-keys").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    isle.chord(&[CTRL, F2]).await;
    wait_until(|| async { bar(&isle).await["open"] == 0 }).await;
    client.wait_for("keyboard-leave").await;
    let b = bar(&isle).await;
    let system = &b["titles"][0];
    assert_eq!(system["kind"], "system");
    assert_eq!(item(system, "Sleep").unwrap()["enabled"], true);
    assert_eq!(item(system, "Restart…").unwrap()["enabled"], true);

    client.clear();
    isle.chord(&[ESC]).await;
    wait_until(|| async { bar(&isle).await["open"].is_null() }).await;
    isle.chord(&[ESC]).await;
    wait_until(|| async { bar(&isle).await["output"].is_null() }).await;
    client.wait_for("keyboard-enter").await;

    // Revealed again after it closed: its tree was rebuilt cleanly, never through a panic.
    isle.chord(&[CTRL, F2]).await;
    wait_until(|| async { bar(&isle).await["open"] == 0 }).await;
    assert!(!isle.log().contains("surface panicked"), "{}", isle.log());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_push_outside_the_island_reveals_the_bar_and_leaving_hides_it() {
    let isle = Isle::start("bar-push").await;
    let test = isle.test().await;
    test.pointer_to(300.0, 0.0).await.unwrap();
    test.pointer_by(0.0, -8.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        bar(&isle).await["output"].is_null(),
        "a touch is not a push"
    );
    test.pointer_by(0.0, -30.0).await.unwrap();
    wait_until(|| async { bar(&isle).await["output"] == "HEADLESS-1" }).await;
    test.pointer_to(300.0, 600.0).await.unwrap();
    wait_until(|| async { bar(&isle).await["output"].is_null() }).await;

    // Not while a button is held, so a drag to the top never brings it.
    test.button(LEFT, true).await.unwrap();
    test.pointer_to(300.0, 0.0).await.unwrap();
    test.pointer_by(0.0, -60.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(bar(&isle).await["output"].is_null());
    test.button(LEFT, false).await.unwrap();

    // Inside the island's zone the edge is the island's.
    test.pointer_to(960.0, 0.0).await.unwrap();
    test.pointer_by(0.0, -60.0).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(bar(&isle).await["output"].is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_browser_s_fallback_menus_send_its_own_chords() {
    let mut isle = Isle::start("bar-fallback").await;
    let client = isle.client("chromium", "3366ff");
    client.wait_for("keyboard-enter").await;
    let b = bar(&isle).await;
    let file = title(&b, "File").expect("a File menu from the tables");
    assert_eq!(item(file, "New Tab").unwrap()["hint"], "⌘T");
    assert!(item(file, "Close Window").is_some());
    assert!(title(&b, "History").is_some());
    let app = &b["titles"][1];
    assert_eq!(app["kind"], "app");
    assert_eq!(item(app, "Hide Others").unwrap()["hint"], "⌥⌘H");

    client.clear();
    assert!(activate(&isle, &["File", "New Tab"]).await);
    client.wait_for("sym t pressed").await;
    let lines = client.lines.lock().unwrap().clone();
    assert!(
        lines.iter().any(|l| l.starts_with("mods Control")),
        "{lines:?}"
    );
    assert!(!activate(&isle, &["File", "No Such Item"]).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_window_menu_keeps_a_window_on_top_and_lists_the_app_s_windows() {
    let mut isle = Isle::start("bar-window").await;
    let _client = isle.client("isle.test.Two", "3366ff");
    isle.wait_for_window("isle.test.Two").await;
    let b = bar(&isle).await;
    let window = title(&b, "Window").unwrap();
    assert_eq!(item(window, "Keep on top").unwrap()["mark"], "none");
    assert!(item(window, "Move and resize").unwrap()["items"].is_array());
    assert!(activate(&isle, &["Window", "Keep on top"]).await);
    wait_until(|| async {
        let b = bar(&isle).await;
        item(title(&b, "Window").unwrap(), "Keep on top").unwrap()["mark"] == "check"
    })
    .await;
    assert!(activate(&isle, &["Window", "Move and resize", "Left half"]).await);
    wait_until(|| async { isle.windows().await[0].mode == "tiled:left-half" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secondary_click_on_a_title_bar_opens_the_window_menu() {
    let mut isle = Isle::start("bar-titlebar").await;
    let _client = isle.client_with(&[
        "--app-id",
        "isle.test.Barred",
        "--colour",
        "ff0000",
        "--do",
        "decoration server",
    ]);
    let w = isle.wait_for_window("isle.test.Barred").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let test = isle.test().await;
    // The window's box starts under its bar.
    test.pointer_to(w.x as f64 + 300.0, w.y as f64 - 16.0)
        .await
        .unwrap();
    test.button(RIGHT, true).await.unwrap();
    test.button(RIGHT, false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        shell
            .list()
            .await
            .unwrap()
            .iter()
            .any(|s| s.name == "window-menu" && s.open)
    })
    .await;
    isle.chord(&[ESC]).await;
    wait_until(|| async {
        !shell
            .list()
            .await
            .unwrap()
            .iter()
            .any(|s| s.name == "window-menu" && s.open)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn on_a_narrow_screen_the_app_s_menus_fold_and_window_and_help_stay() {
    let mut isle = Isle::start("bar-fold").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Headless", "2", 900, 600)
        .await
        .unwrap();
    let client = isle.client("code", "3366ff");
    client.wait_for("keyboard-enter").await;
    // Over the narrow screen, outside the island's zone.
    test.pointer_to(1920.0 + 60.0, 0.0).await.unwrap();
    test.pointer_by(0.0, -40.0).await.unwrap();
    wait_until(|| async { bar(&isle).await["output"] == "HEADLESS-2" }).await;
    let b = bar(&isle).await;
    let kinds: Vec<&str> = b["titles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"overflow"), "{kinds:?}");
    let labels: Vec<&str> = b["titles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["label"].as_str().unwrap())
        .collect();
    assert_eq!(&labels[labels.len() - 2..], ["Window", "Help"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bar_goes_with_the_window_it_was_revealed_for() {
    let mut isle = Isle::start("bar-window-gone").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    isle.chord(&[CTRL, F2]).await;
    wait_until(|| async { !bar(&isle).await["output"].is_null() }).await;
    client.send("destroy");
    wait_until(|| async { bar(&isle).await["output"].is_null() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_with_no_app_id_is_named_by_its_process() {
    let mut isle = Isle::start("bar-no-app-id").await;
    let client = isle.client_with(&["--app-id", "", "--colour", "3366ff"]);
    client.wait_for("keyboard-enter").await;
    // The kernel keeps fifteen characters of a process's name.
    wait_until(|| async { bar(&isle).await["app"] == "isle-test-clien" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dialog_with_its_own_app_id_is_named_as_its_parent_s_app() {
    let mut isle = Isle::start("bar-dialog-name").await;
    let client = isle.client("isle.test.Writer", "3366ff");
    client.wait_for("keyboard-enter").await;
    wait_until(|| async { bar(&isle).await["app"] == "isle.test.Writer" }).await;
    client.send("dialog");
    client.wait_for("w1 configured 300 200").await;
    client.send("w1 app-id isle.test.Tips");
    wait_until(|| async { isle.windows().await.iter().any(|w| w.app == "isle.test.Tips" && w.focused) }).await;
    assert_eq!(bar(&isle).await["app"], "isle.test.Writer");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_submenu_with_nothing_enabled_is_disabled() {
    let mut isle = Isle::start("bar-empty-submenu").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    let b = bar(&isle).await;
    let window = title(&b, "Window").unwrap();
    assert_eq!(item(window, "Full screen tile").unwrap()["enabled"], false);
    assert_eq!(item(window, "Move and resize").unwrap()["enabled"], true);
}
