//! The widgets dashboard: opening, its keys, its pass over the screen and the surfaces it
//! gives way to. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, keys::*, wait_until};
use isle_ipc::shell::ShellProxy;

const ESCAPE: u32 = 1;
const E: u32 = 18;
const W: u32 = 17;
const SPACE: u32 = 57;
const DOWN: u32 = 108;
const END: u32 = 107;

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.dashboard().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

async fn kinds(isle: &Isle) -> Vec<String> {
    isle.test().await.frame_kinds("HEADLESS-1").await.unwrap()
}

/// Ctrl+Option+Cmd+W.
async fn chord(isle: &Isle) {
    isle.chord(&[CTRL, ALT, SUPER, W]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_chord_opens_it_over_everything_and_gives_focus_back() {
    let mut isle = Isle::start("dashboard-open").await;
    isle.test().await.transitions(false).await.unwrap();
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let s = status(&isle).await;
    assert_eq!(s["grid"], serde_json::json!([21, 10]), "the headless output is 1920×1080");
    assert_eq!(s["layout"].as_array().unwrap().len(), 8, "the default block");
    assert_eq!(s["layout"][0], "clock#1 2 4 2 2");
    window.wait_for("keyboard-leave").await;
    wait_until(|| async {
        let k = kinds(&isle).await;
        k.contains(&"wallpaper".into()) && !k.contains(&"client".into())
    })
    .await;

    // Keys move the selection, with the ring, in reading order.
    press(&isle, END).await;
    wait_until(|| async { status(&isle).await["selected"] == "notifications#1" }).await;
    assert_eq!(status(&isle).await["ring"], true);

    // E enters edit mode; Escape leaves it, then closes.
    press(&isle, E).await;
    wait_until(|| async { status(&isle).await["edit"] == true }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["edit"] == false }).await;
    assert_eq!(status(&isle).await["open"], true);
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    window.wait_for("keyboard-enter").await;
    wait_until(|| async { kinds(&isle).await.contains(&"client".into()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_chord_again_closes_it_and_other_overlays_take_its_place() {
    let isle = Isle::start("dashboard-overlays").await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    // Cmd+Space: Spotlight closes the dashboard.
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let spotlight: serde_json::Value =
        serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap();
    assert_eq!(spotlight["open"], true);

    // And the dashboard closes Spotlight.
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let spotlight: serde_json::Value =
        serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap();
    assert_eq!(spotlight["open"], false);
    assert_eq!(status(&isle).await["edit"], true);
    assert!(!shell.widgets("close").await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn locking_closes_it_and_the_island_is_covered_over_it() {
    let isle = Isle::start("dashboard-lock").await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    let island = isle.island_output().await;
    assert!(
        island["form"] == "nothing" || island["form"] == "privacy",
        "the island's covered form over the dashboard: {island}"
    );
    isle.test().await.dashboard_act("session lock").await.unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let lock: serde_json::Value =
        serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap();
    assert_eq!(lock["locked"], true);
    // Locked, the chord opens nothing.
    chord(&isle).await;
    press(&isle, DOWN).await;
    assert_eq!(status(&isle).await["open"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wallpapers_menu_opens_it_in_edit_mode() {
    const RIGHT_BUTTON: u32 = 0x111;
    const RETURN: u32 = 28;
    let isle = Isle::start("dashboard-wallpaper").await;
    isle.test().await.transitions(false).await.unwrap();
    let test = isle.test().await;
    test.pointer_to(900.0, 700.0).await.unwrap();
    test.button(RIGHT_BUTTON, true).await.unwrap();
    test.button(RIGHT_BUTTON, false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        shell.list().await.unwrap().iter().any(|s| s.name == "window-menu" && s.open)
    })
    .await;
    press(&isle, DOWN).await;
    press(&isle, DOWN).await;
    press(&isle, RETURN).await;
    wait_until(|| async { status(&isle).await["edit"] == true }).await;
    assert_eq!(status(&isle).await["output"], "HEADLESS-1");
}

/// The point at the middle of cell `(x, y)` on the 1920×1080 headless output, whose grid is
/// 21 × 10 from (44, 97).
fn cell(x: i32, y: i32) -> (f64, f64) {
    (44.0 + 88.0 * x as f64 + 36.0, 97.0 + 88.0 * y as f64 + 36.0)
}

async fn layout(isle: &Isle) -> Vec<String> {
    status(isle).await["layout"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_owned())
        .collect()
}

fn line_of<'a>(layout: &'a [String], id: &str) -> &'a str {
    layout.iter().find(|l| l.starts_with(&format!("{id} "))).map(String::as_str).unwrap_or("")
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_mode_moves_pushes_resizes_removes_and_undoes() {
    const LEFT_BUTTON: u32 = 0x110;
    const RIGHT_BUTTON: u32 = 0x111;
    const Z: u32 = 44;
    const DELETE: u32 = 111;
    let isle = Isle::start("dashboard-edit").await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let test = isle.test().await;
    assert_eq!(line_of(&layout(&isle).await, "clock#1"), "clock#1 2 4 2 2");

    // Dragged two cells right by its body, the clock pushes the calendar along.
    let (x, y) = cell(2, 4);
    test.pointer_to(x + 20.0, y + 20.0).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    for step in 1..=8 {
        test.pointer_to(x + 20.0 + 22.0 * step as f64, y + 20.0).await.unwrap();
    }
    wait_until(|| async { status(&isle).await["dragging"] == true }).await;
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1") == "clock#1 4 4 2 2" }).await;
    let after = layout(&isle).await;
    assert_eq!(line_of(&after, "calendar#1"), "calendar#1 6 4 4 2", "{after:?}");
    let file = isle.dir.join("config/settings.kdl");
    wait_until(|| async {
        std::fs::read_to_string(&file).is_ok_and(|t| t.contains("\"clock#1 4 4 2 2\""))
    })
    .await;

    // Cmd+Z puts every widget back.
    isle.chord(&[SUPER, Z]).await;
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1") == "clock#1 2 4 2 2" }).await;
    assert_eq!(line_of(&layout(&isle).await, "calendar#1"), "calendar#1 4 4 4 2");
    isle.chord(&[SUPER, SHIFT, Z]).await;
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1") == "clock#1 4 4 2 2" }).await;

    // Moved there, the clock cannot grow: the push would leave the grid.
    isle.chord(&[ALT, SUPER, RIGHT]).await;
    wait_until(|| async { status(&isle).await["ring"] == true }).await;
    assert_eq!(line_of(&layout(&isle).await, "clock#1"), "clock#1 4 4 2 2");
    // Back where it was, Option+Cmd+Right grows it to its next size, pushing.
    isle.chord(&[SUPER, Z]).await;
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1") == "clock#1 2 4 2 2" }).await;
    isle.chord(&[ALT, SUPER, RIGHT]).await;
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1") == "clock#1 2 4 4 2" }).await;
    assert_eq!(line_of(&layout(&isle).await, "calendar#1"), "calendar#1 6 4 4 2");

    // A click on the selected widget opens its settings; Escape closes them.
    let (x, y) = cell(3, 4);
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { status(&isle).await["form"] == "clock#1" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["form"].is_null() }).await;
    assert_eq!(status(&isle).await["edit"], true, "Escape left the form, not edit mode");

    // Its menu by a secondary click; Escape closes it.
    test.button(RIGHT_BUTTON, true).await.unwrap();
    test.button(RIGHT_BUTTON, false).await.unwrap();
    wait_until(|| async { status(&isle).await["menu"] == "clock#1" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["menu"].is_null() }).await;

    // Delete removes the selection; the badge removes another.
    press(&isle, DELETE).await;
    wait_until(|| async { line_of(&layout(&isle).await, "clock#1").is_empty() }).await;
    let weather = line_of(&layout(&isle).await, "weather#1").to_owned();
    let parts: Vec<i32> = weather.split(' ').skip(1).map(|n| n.parse().unwrap()).collect();
    test.pointer_to(44.0 + 88.0 * parts[0] as f64 + 15.0, 97.0 + 88.0 * parts[1] as f64 + 15.0)
        .await
        .unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { line_of(&layout(&isle).await, "weather#1").is_empty() }).await;
    assert_eq!(layout(&isle).await.len(), 6);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_drag_past_the_edge_is_refused_and_returns() {
    const LEFT_BUTTON: u32 = 0x110;
    // One free column at the row's end: a push of two cells cannot fit.
    let isle = Isle::start_with(
        "dashboard-refused",
        "dashboard-output \"Isle Headless 1\" {\n    widgets \"media#1 0 0 4 1\" \"storage#1 4 0 4 1\" \"desktops#1 8 0 4 1\" \"session#1 12 0 4 1\" \"clock#1 16 0 2 1\" \"notifications#1 18 0 2 1\"\n}\n",
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let before = layout(&isle).await;
    assert_eq!(before.len(), 6);
    let test = isle.test().await;
    let (x, y) = cell(1, 0);
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    for step in 1..=8 {
        test.pointer_to(x + 22.0 * step as f64, y).await.unwrap();
    }
    wait_until(|| async { status(&isle).await["dragging"] == true }).await;
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { status(&isle).await["dragging"] == false }).await;
    assert_eq!(before, layout(&isle).await, "a refused place leaves everything where it was");
    // One cell has room.
    isle.chord(&[SUPER, RIGHT]).await;
    wait_until(|| async { line_of(&layout(&isle).await, "media#1") == "media#1 1 0 4 1" }).await;
    assert_eq!(line_of(&layout(&isle).await, "notifications#1"), "notifications#1 19 0 2 1");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_built_ins_show_what_the_desktop_has_or_their_empty_line() {
    let mut isle = Isle::start_with(
        "dashboard-builtins",
        "widget \"clock#1\" {\n    zone \"Asia/Tokyo\"\n    face \"analogue\"\n}\n",
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    let client = isle.client("isle.test.Notifier", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("notify summary=One");
    client.send("notify summary=Two");

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["bodies"]["storage#1"] != "title" }).await;
    let bodies = status(&isle).await["bodies"].clone();
    assert_eq!(bodies["clock#1"]["clock"], "Tokyo");
    assert_eq!(bodies["clock#1"]["dial"], true);
    assert_eq!(bodies["media#1"]["empty"], "Not playing");
    assert_eq!(bodies["devices#1"]["empty"], "No devices");
    assert_eq!(bodies["storage#1"]["storage"][0][0], "System", "{bodies}");
    wait_until(|| async {
        status(&isle).await["bodies"]["notifications#1"]["notifications"][0][1] == 2
    })
    .await;

    isle.keep_shot("dashboard-default").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.widgets("edit").await.unwrap();
    wait_until(|| async { status(&isle).await["edit"] == true }).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    isle.keep_shot("dashboard-edit").await;

    // A notification's row opens Notification Centre with that app's stack.
    let app = status(&isle).await["bodies"]["notifications#1"]["notifications"][0][0].clone();
    assert!(app.is_string());
    isle.test()
        .await
        .dashboard_act("close")
        .await
        .unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
}
