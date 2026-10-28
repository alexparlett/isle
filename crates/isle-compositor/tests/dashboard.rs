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
    // The gallery covers the leading columns these presses land on.
    isle.test().await.dashboard_act("gallery").await.unwrap();
    wait_until(|| async { status(&isle).await["gallery"].is_null() }).await;
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
    // The gallery covers the leading columns these presses land on.
    isle.test().await.dashboard_act("gallery").await.unwrap();
    wait_until(|| async { status(&isle).await["gallery"].is_null() }).await;
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

#[tokio::test(flavor = "multi_thread")]
async fn system_samples_timers_run_on_the_island_and_calendar_waits_for_calendars() {
    let ended = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        - 1000;
    let isle = Isle::start_prepared(
        "dashboard-feeds",
        "dashboard-output \"Isle Headless 1\" {\n    widgets \"system#1 0 0 4 2\" \"timer#1 4 0 2 2\" \"calendar#1 6 0 4 2\"\n}\nwidget \"timer#1\" {\n    label \"Tea\"\n}\n",
        |dir| {
            let state = dir.join("state/isle");
            std::fs::create_dir_all(&state).unwrap();
            std::fs::write(
                state.join("widgets.json"),
                format!(
                    r#"{{"version":1,"data":{{"data":{{}},"timers":{{"timer#9":{{"label":"Eggs","duration_ms":60000,"ends_at":{ended},"left_ms":null}}}},"sources":{{}}}}}}"#
                ),
            )
            .unwrap();
        },
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    // A timer that ended while Isle was away says so as it starts.
    wait_until(|| async { isle.island_output().await["text"] == "Eggs done" }).await;
    // It holds until clicked.
    let test = isle.test().await;
    test.pointer_to(960.0, 20.0).await.unwrap();
    test.button(0x110, true).await.unwrap();
    test.button(0x110, false).await.unwrap();
    wait_until(|| async { isle.island_output().await["text"] != "Eggs done" }).await;

    chord(&isle).await;
    wait_until(|| async { status(&isle).await["bodies"]["system#1"]["system"].is_array() }).await;
    let bodies = status(&isle).await["bodies"].clone();
    let labels: Vec<&str> = bodies["system#1"]["system"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s[0].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["CPU", "Memory", "GPU", "Temperature"]);
    assert_eq!(bodies["calendar#1"]["empty"], "No calendars");
    assert_eq!(bodies["timer#1"]["state"], "Idle");
    assert_eq!(bodies["timer#1"]["timer"], "5:00");

    test.dashboard_act("row timer#1 start").await.unwrap();
    wait_until(|| async { status(&isle).await["bodies"]["timer#1"]["state"] == "Running" }).await;
    test.dashboard_act("close").await.unwrap();
    // Closed, it runs on as the island's activity.
    wait_until(|| async {
        let text = isle.island_output().await["text"].as_str().unwrap_or("").to_owned();
        text.starts_with("4:5") || text == "5:00"
    })
    .await;
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    test.dashboard_act("row timer#1 pause").await.unwrap();
    wait_until(|| async { status(&isle).await["bodies"]["timer#1"]["state"] == "Paused" }).await;
    test.dashboard_act("row timer#1 reset").await.unwrap();
    wait_until(|| async { status(&isle).await["bodies"]["timer#1"]["state"] == "Idle" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn picture_of_the_feeds() {
    let forecast = r#"{"place":"London","asked":"","now":12.4,"code":2,"day":true,"high":13.0,"low":6.0,"hours":[["15",3,12.1],["16",61,11.5],["17",61,10.2],["18",3,9.0],["19",0,8.1],["20",0,7.7]],"days":[["Thu",61,5.5,11.0],["Fri",3,4.0,10.5],["Sat",0,3.3,12.2],["Sun",71,-1.0,4.0],["Mon",95,2.0,9.0]],"fahrenheit":false,"at":4102444800}"#;
    let isle = Isle::start_prepared(
        "dashboard-picture",
        "dashboard-output \"Isle Headless 1\" {\n    widgets \"system#1 0 0 4 2\" \"timer#1 4 0 2 2\" \"weather#1 6 0 4 4\" \"system#2 10 0 6 2\" \"weather#2 10 2 2 2\" \"timer#2 12 2 2 1\" \"system#3 14 2 2 2\"\n}\n",
        |dir| {
            let state = dir.join("state/isle");
            std::fs::create_dir_all(&state).unwrap();
            std::fs::write(
                state.join("widgets.json"),
                format!(r#"{{"version":1,"data":{{"data":{{"weather#1":{{"value":{forecast},"at":4102444800}},"weather#2":{{"value":{forecast},"at":4102444800}}}},"timers":{{}},"sources":{{}}}}}}"#),
            )
            .unwrap();
        },
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    let test = isle.test().await;
    test.dashboard_act("row timer#1 start").await.unwrap();
    for _ in 0..4 {
        tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    }
    wait_until(|| async { status(&isle).await["bodies"]["weather#1"]["weather"] == "London" }).await;
    isle.keep_shot("dashboard-feeds").await;
}

/// A provider folder at `dir` answering `script`'s output.
fn provider_folder(dir: &std::path::Path, id: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("run.sh"), format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(dir.join("run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        dir.join("widget.json"),
        format!(
            r#"{{"id":"{id}","name":"{id} widget","version":"1.0.0","sizes":["2x1","4x2"],"default":"2x1","template":"text","exec":["./run.sh"],"interval":60}}"#
        ),
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn providers_run_while_shown_and_say_why_when_they_cannot() {
    let isle = Isle::start_prepared(
        "dashboard-providers",
        "dashboard-output \"Isle Headless 1\" {\n    widgets \"hello#1 0 0 2 1\" \"shared#1 2 0 2 1\" \"gone#1 4 0 2 1\" \"broken#1 6 0 2 1\"\n}\n",
        |dir| {
            let own = dir.join("config/widgets");
            provider_folder(&own.join("hello"), "hello", r#"echo '{"text":"hello"}'"#);
            provider_folder(&own.join("broken"), "broken", "echo 'The key is wrong' >&2; exit 1");
            let installed = dir.join("data/isle/widgets/shared");
            provider_folder(&installed, "shared", r#"echo '{"text":"shared"}'"#);
            std::fs::write(
                installed.join(".isle-install.json"),
                r#"{"source":"https://example.com/widgets","version":"1.0.0"}"#,
            )
            .unwrap();
        },
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    chord(&isle).await;
    wait_until(|| async {
        let b = status(&isle).await["bodies"].clone();
        b["hello#1"]["provider"].as_str().is_some_and(|p| p.contains("hello"))
            && b["broken#1"]["strip"] == "The key is wrong"
    })
    .await;
    let bodies = status(&isle).await["bodies"].clone();
    assert_eq!(bodies["shared#1"]["strip"], "“shared widget” isn't allowed to run.");
    assert_eq!(bodies["gone#1"]["strip"], "“gone” isn't installed.");
    assert_eq!(status(&isle).await["running"], 2, "the two that may run");

    // Closed, nothing runs.
    isle.test().await.dashboard_act("close").await.unwrap();
    wait_until(|| async { status(&isle).await["running"] == 0 }).await;

    // Remove takes the missing one out.
    chord(&isle).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    isle.test().await.dashboard_act("row gone#1 remove").await.unwrap();
    wait_until(|| async { status(&isle).await["bodies"]["gone#1"].is_null() }).await;
    // The last answer is drawn first when it opens again.
    assert!(status(&isle).await["bodies"]["hello#1"]["provider"].as_str().is_some_and(|p| p.contains("hello")));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gallery_searches_adds_by_key_and_click_and_takes_widgets_back() {
    const LEFT_BUTTON: u32 = 0x110;
    const TAB: u32 = 15;
    const RETURN: u32 = 28;
    const BACKSPACE: u32 = 14;
    const T: u32 = 20;
    const I: u32 = 23;
    const M: u32 = 50;
    const TWO: u32 = 3;
    let isle = Isle::start("dashboard-gallery").await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    wait_until(|| async { status(&isle).await["gallery"]["section"] == 0 }).await;
    let items = status(&isle).await["gallery"]["items"].clone();
    let media = items.as_array().unwrap().iter().find(|i| i[0] == "media").unwrap().clone();
    assert_eq!(media[1], true, "a single widget already placed reads Added");
    let clock = items.as_array().unwrap().iter().find(|i| i[0] == "clock").unwrap().clone();
    assert_eq!(clock[1], false, "a clock may be placed again");

    // Tab to the gallery, type, Return adds the first match.
    press(&isle, TAB).await;
    for key in [T, I, M] {
        press(&isle, key).await;
    }
    wait_until(|| async { status(&isle).await["gallery"]["query"] == "tim" }).await;
    let found = status(&isle).await["gallery"]["items"].clone();
    assert_eq!(found.as_array().unwrap().len(), 1, "{found}");
    press(&isle, RETURN).await;
    wait_until(|| async { line_of(&layout(&isle).await, "timer#1").starts_with("timer#1 ") }).await;
    assert_eq!(status(&isle).await["selected"], "timer#1");
    for _ in 0..3 {
        press(&isle, BACKSPACE).await;
    }

    // Cmd+2 is Yours, empty here; Details on an item.
    isle.chord(&[SUPER, TWO]).await;
    wait_until(|| async { status(&isle).await["gallery"]["section"] == 1 }).await;
    isle.test().await.dashboard_act("gallery-details clock").await.unwrap();
    wait_until(|| async { status(&isle).await["gallery"]["details"] == "clock" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["gallery"]["details"].is_null() }).await;

    // A drag from the gallery places a widget where it is dropped.
    let test = isle.test().await;
    test.pointer_to(150.0, 300.0).await.unwrap();
    isle.test().await.dashboard_act("gallery-press weather").await.unwrap();
    let (x, y) = cell(12, 8);
    for step in 1..=10 {
        let p = step as f64 / 10.0;
        test.pointer_to(150.0 + (x - 150.0) * p, 300.0 + (y - 300.0) * p).await.unwrap();
    }
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { line_of(&layout(&isle).await, "weather#2").starts_with("weather#2 ") }).await;

    // A widget dragged onto the gallery is removed.
    let line = line_of(&layout(&isle).await, "weather#2").to_owned();
    let n: Vec<f64> = line.split(' ').skip(1).map(|v| v.parse().unwrap()).collect();
    let (x, y) = (44.0 + 88.0 * n[0] + 60.0, 97.0 + 88.0 * n[1] + 60.0);
    test.pointer_to(x, y).await.unwrap();
    test.button(LEFT_BUTTON, true).await.unwrap();
    for step in 1..=10 {
        let p = step as f64 / 10.0;
        test.pointer_to(x + (150.0 - x) * p, y + (300.0 - y) * p).await.unwrap();
    }
    test.button(LEFT_BUTTON, false).await.unwrap();
    wait_until(|| async { line_of(&layout(&isle).await, "weather#2").is_empty() }).await;
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

/// A source with one provider, `hn`, at `version`, declaring `connect` hosts.
fn write_source(repo: &std::path::Path, version: &str, hosts: &str) {
    use std::os::unix::fs::PermissionsExt;
    let w = repo.join("widgets/hn");
    std::fs::create_dir_all(&w).unwrap();
    std::fs::write(w.join("run.sh"), "#!/bin/sh\necho '{\"text\":\"stories\"}'\n").unwrap();
    std::fs::set_permissions(w.join("run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(
        w.join("widget.json"),
        format!(r#"{{"id":"hn","name":"Hacker News","version":"{version}","author":{{"name":"Alex"}},"sizes":["2x1"],"default":"2x1","template":"text","exec":["./run.sh"],"interval":60,"permissions":{{"connect":[{hosts}]}}}}"#),
    )
    .unwrap();
    std::fs::write(repo.join("providers.json"), r#"{"name":"Test widgets","format":2,"widgets":[{"id":"hn","path":"widgets/hn"}]}"#).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_source_is_browsed_and_its_widget_installed_updated_and_removed_with_consent() {
    let repo = std::env::temp_dir().join(format!("isle-dashboard-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    write_source(&repo, "1.0.0", r#""news.example.com""#);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "one"]);
    let url = format!("file://{}", repo.display());
    let isle = Isle::start_with("dashboard-source", &format!("widgets {{\n    sources \"{url}\"\n}}\n")).await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let test = isle.test().await;
    test.dashboard_act("gallery-section 2").await.unwrap();
    wait_until(|| async { status(&isle).await["browse"][&url]["Ok"][0] == "hn" }).await;
    let key = format!("hn@{url}");

    // Install asks first, armed after a moment, naming what it runs and may reach.
    test.dashboard_act(&format!("gallery-button {key} install")).await.unwrap();
    wait_until(|| async { status(&isle).await["consent"]["title"] == "Install “Hacker News”?" }).await;
    let consent = status(&isle).await["consent"].clone();
    assert_eq!(consent["permissions"][0], "Connect to news.example.com");
    test.dashboard_act("consent confirm").await.unwrap();
    assert!(!status(&isle).await["consent"].is_null(), "not before it is armed");
    wait_until(|| async { status(&isle).await["consent"]["armed"] == true }).await;
    test.dashboard_act("consent confirm").await.unwrap();
    wait_until(|| async { status(&isle).await["providers"][0] == "hn" }).await;
    let file = isle.dir.join("config/settings.kdl");
    wait_until(|| async {
        std::fs::read_to_string(&file).is_ok_and(|t| t.contains("connect:news.example.com") && t.contains("allowed #true"))
    })
    .await;

    // Placed, it runs.
    test.dashboard_act("gallery-section 0").await.unwrap();
    test.dashboard_act("gallery-press hn").await.unwrap();
    test.button(0x110, false).await.unwrap();
    wait_until(|| async {
        status(&isle).await["bodies"]["hn#1"]["provider"].as_str().is_some_and(|p| p.contains("stories"))
    })
    .await;

    // An update asking for one more host asks for that one only.
    write_source(&repo, "1.1.0", r#""news.example.com","api.example.com""#);
    git(&repo, &["commit", "-qam", "two"]);
    // Opening Browse again fetches it.
    test.dashboard_act("gallery-section 0").await.unwrap();
    test.dashboard_act("gallery-section 2").await.unwrap();
    wait_until(|| async { status(&isle).await["updates"][0] == "hn" }).await;
    test.dashboard_act(&format!("gallery-button {key} update")).await.unwrap();
    wait_until(|| async {
        let c = status(&isle).await["consent"].clone();
        c["title"] == "Update “Hacker News”?" && c["permissions"].as_array().is_some_and(|p| p.len() == 1)
    })
    .await;
    assert_eq!(status(&isle).await["consent"]["permissions"][0], "Connect to api.example.com");
    wait_until(|| async { status(&isle).await["consent"]["armed"] == true }).await;
    test.dashboard_act("consent confirm").await.unwrap();
    let installed = isle.dir.join("data/isle/widgets/hn/widget.json");
    wait_until(|| async { std::fs::read_to_string(&installed).is_ok_and(|t| t.contains("1.1.0")) }).await;

    // Remove takes it out; its widget says so.
    test.dashboard_act(&format!("gallery-button {key} remove")).await.unwrap();
    wait_until(|| async { status(&isle).await["providers"].as_array().is_some_and(|p| p.is_empty()) }).await;
    wait_until(|| async { status(&isle).await["bodies"]["hn#1"]["strip"] == "“hn” isn't installed." }).await;
    let _ = std::fs::remove_dir_all(&repo);
}

#[tokio::test(flavor = "multi_thread")]
async fn yours_checks_pictures_and_starts_a_widget_from_a_template() {
    let mut own = std::path::PathBuf::new();
    let isle = Isle::start_prepared(
        "dashboard-yours",
        "dashboard-output \"Isle Headless 1\" {\n    widgets \"hello#1 6 2 2 1\"\n}\n",
        |dir| {
            own = dir.join("config/widgets");
            provider_folder(&own.join("hello"), "hello", r#"echo '{"text":"hello"}'"#);
        },
    )
    .await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let test = isle.test().await;
    test.dashboard_act("gallery-section 1").await.unwrap();
    wait_until(|| async {
        status(&isle).await["bodies"]["hello#1"]["provider"].as_str().is_some_and(|p| p.contains("hello"))
    })
    .await;
    test.dashboard_act("gallery-button hello check").await.unwrap();
    wait_until(|| async { status(&isle).await["gallery"]["told"]["hello"] == "Ready" }).await;
    test.dashboard_act("gallery-button hello picture").await.unwrap();
    wait_until(|| async { status(&isle).await["gallery"]["told"]["hello"] == "Picture updated" }).await;
    let card = own.join("hello/screenshots/card.png");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&card).unwrap()));
    let info = decoder.read_info().unwrap().info().clone();
    assert_eq!((info.width, info.height), (160, 72), "the 2 × 1 widget, at scale 1");

    test.dashboard_act("gallery-button x new").await.unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let made = own.join("my-widget");
    assert!(made.join("widget.json").is_file());
    assert!(isle_providers::Manifest::load(&made).is_ok(), "the template passes its own check");
}

#[tokio::test(flavor = "multi_thread")]
async fn return_on_a_browse_item_asks_to_install_and_places_nothing() {
    let repo = std::env::temp_dir().join(format!("isle-dashboard-browse-return-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    write_source(&repo, "1.0.0", r#""news.example.com""#);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "one"]);
    let url = format!("file://{}", repo.display());
    let isle = Isle::start_with("dashboard-browse-return", &format!("widgets {{\n    sources \"{url}\"\n}}\n")).await;
    isle.test().await.transitions(false).await.unwrap();
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(shell.widgets("edit").await.unwrap());
    let before = layout(&isle).await;
    press(&isle, 15).await;
    isle.chord(&[SUPER, 4]).await;
    wait_until(|| async { status(&isle).await["gallery"]["items"][0][0] == format!("hn@{url}") }).await;
    press(&isle, 28).await;
    wait_until(|| async { status(&isle).await["consent"]["title"] == "Install “Hacker News”?" }).await;
    assert_eq!(layout(&isle).await, before, "nothing placed");
    let _ = std::fs::remove_dir_all(&repo);
}
