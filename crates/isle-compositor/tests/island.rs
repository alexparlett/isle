#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};
use isle_ipc::{desktops::DesktopsProxy, island::IslandProxy, settings::SettingsProxy};
use serde_json::Value;

const CTRL: u32 = 29;
const SPACE: u32 = 57;

async fn status(isle: &Isle) -> Value {
    let text = IslandProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The island on `output`, once its shape has settled.
async fn settled(isle: &Isle, output: &str) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let s = status(isle).await;
        let island = s["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["output"] == output)
            .cloned()
            .unwrap();
        let target = |i: usize| {
            island["target"]
                .as_array()
                .map(|t| t[i].as_f64().unwrap())
                .unwrap_or(0.0)
        };
        if (island["width"].as_f64().unwrap() - target(0)).abs() < 0.5
            && (island["height"].as_f64().unwrap() - target(1)).abs() < 0.5
        {
            return island;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never settled: {island}"
        );
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

/// Waits for the island's status to satisfy `f`, naming the last one seen if it never does.
async fn until(isle: &Isle, label: &str, f: impl Fn(&Value) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let s = status(isle).await;
        if f(&s) {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "{label}: {s}");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

async fn post(isle: &Isle, json: &str) {
    isle.test().await.island(json).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn at_rest_the_island_is_the_clock_and_draws_once_a_minute() {
    let isle = Isle::start("island-rest").await;
    let island = settled(&isle, "HEADLESS-1").await;
    assert_eq!(island["form"], "pill");
    assert_eq!(island["open"], true);
    assert!(island["glyphs"].as_array().unwrap().is_empty());
    let clock = status(&isle).await["clock"].as_str().unwrap().to_owned();
    assert_eq!(clock.len(), 5, "time only, 24-hour: {clock}");
    assert!(
        island["width"].as_f64().unwrap() < 80.0,
        "a pill of about 64"
    );

    let shot = isle.shot("HEADLESS-1", "rest").await;
    assert_ne!(
        shot.at(960 - 20, 24),
        [0x1c, 0x1c, 0x1f],
        "the pill is drawn at the top centre"
    );
    assert_eq!(
        shot.at(960 - 200, 24),
        [0x1c, 0x1c, 0x1f],
        "and nothing beside it"
    );

    // A still desktop with the island at rest: no frames, but for a minute's turn.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let test = isle.test().await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(test.frames("HEADLESS-1").await.unwrap() - before <= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_osd_pre_empts_an_event_and_what_it_bumped_returns() {
    let isle = Isle::start("island-osd").await;
    post(&isle, r#"{"event": {"kind": "ended", "symbol": "record", "text": "Recording stopped: the window closed"}}"#).await;
    let island = settled(&isle, "HEADLESS-1").await;
    assert_eq!(island["form"], "event");
    assert_eq!(island["text"], "Recording stopped: the window closed");

    post(
        &isle,
        r#"{"event": {"kind": "osd", "symbol": "speaker-high", "name": "Speakers", "level": 0.4}}"#,
    )
    .await;
    let island = settled(&isle, "HEADLESS-1").await;
    assert_eq!(island["text"], "Speakers");
    assert_eq!(
        island["height"].as_f64().unwrap().round(),
        48.0,
        "the level form"
    );

    wait_until(|| async {
        status(&isle).await["outputs"][0]["text"] == "Recording stopped: the window closed"
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn privacy_shows_on_every_screen_and_over_full_screen_alone() {
    let mut isle = Isle::start("island-privacy").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Headless", "2", 1280, 720)
        .await
        .unwrap();
    post(&isle, r#"{"privacy": {"mark": "microphone", "on": true}}"#).await;
    let outputs = |s: &Value| s["outputs"].as_array().unwrap().clone();
    until(&isle, "privacy on both", |s| {
        outputs(s).len() == 2 && outputs(s).iter().all(|o| o["privacy"] == 1)
    })
    .await;

    // A timer's end holds until acknowledged, so a loaded machine cannot miss it.
    post(
        &isle,
        r#"{"event": {"kind": "timer-done", "symbol": "timer", "text": "Tea done"}}"#,
    )
    .await;
    until(&isle, "the event on the main screen alone", |s| {
        outputs(s).iter().any(|o| o["main"] == true && o["form"] == "event")
            && outputs(s).iter().any(|o| o["main"] == false && o["form"] == "pill")
    })
    .await;

    let _client = isle.client("isle.test.Full", "3366ff");
    let window = isle.wait_for_window("isle.test.Full").await;
    DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(window.id, true)
        .await
        .unwrap();
    wait_until(|| async { isle.window(window.id).await.mode == "fullscreen" }).await;
    let output = isle.window(window.id).await.output;
    until(&isle, "the privacy form over full screen", |s| {
        outputs(s).iter().any(|o| o["output"] == output && o["form"] == "privacy")
    })
    .await;

    post(&isle, r#"{"privacy": {"mark": "microphone", "on": false}}"#).await;
    until(&isle, "nothing over full screen", |s| {
        outputs(s)
            .iter()
            .any(|o| o["output"] == output && o["form"] == "nothing" && o["open"] == false)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_island_shows_who_holds_the_keys_and_a_rejected_edit() {
    let mut isle = Isle::start("island-glyphs").await;
    let vm = isle.client("isle.test.Vm", "3366ff");
    vm.wait_for("keyboard-enter").await;
    vm.send("inhibit");
    vm.wait_for("inhibitor active").await;
    // A notice inside the window names the chord that takes the keys back.
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        shell.list().await.unwrap().iter().any(|s| s.name == "inhibit-notice" && s.open)
    })
    .await;
    wait_until(|| async {
        status(&isle).await["outputs"][0]["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "inhibited")
    })
    .await;

    std::fs::write(isle.dir.join("config/settings.kdl"), "keyboard {\n").unwrap();
    wait_until(|| async {
        status(&isle).await["outputs"][0]["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "config-error")
    })
    .await;
    std::fs::write(
        isle.dir.join("config/settings.kdl"),
        "wallpaper {\n    source \"#1c1c1f\"\n}\n",
    )
    .unwrap();
    wait_until(|| async {
        !status(&isle).await["outputs"][0]["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "config-error")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_glyph_set_to_always_shows_and_never_hides() {
    let isle = Isle::start("island-tiers").await;
    post(&isle, r#"{"glyph": {"kind": "wifi", "active": false, "symbol": "wifi-high", "level": "wifi-medium", "alt": "Wi-Fi"}}"#).await;
    let glyphs = |s: Value| s["outputs"][0]["glyphs"].as_array().unwrap().len();
    assert_eq!(glyphs(status(&isle).await), 0, "normal shows nothing");
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("island-glyphs.wifi", &zvariant::Value::from("always"))
        .await
        .unwrap();
    wait_until(|| async { glyphs(status(&isle).await) == 1 }).await;
    let width = settled(&isle, "HEADLESS-1").await["width"]
        .as_f64()
        .unwrap();
    assert!(width > 80.0, "the glyph widens the pill: {width}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_layout_change_shows_its_name() {
    let isle = Isle::start("island-layout").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.layouts", &zvariant::Value::from(vec!["us", "de"]))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.chord(&[CTRL, SPACE]).await;
    wait_until(|| async {
        status(&isle).await["outputs"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("German")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn activities_lead_by_rank() {
    let isle = Isle::start("island-activities").await;
    post(
        &isle,
        r#"{"activity": {"kind": "media", "face": "", "title": "Song"}}"#,
    )
    .await;
    post(
        &isle,
        r#"{"activity": {"kind": "recording", "face": "0:42", "title": "Recording"}}"#,
    )
    .await;
    wait_until(|| async {
        let s = status(&isle).await;
        s["outputs"][0]["form"] == "pill" && s["outputs"][0]["text"] == "0:42"
    })
    .await;
    post(&isle, r#"{"end": "recording"}"#).await;
    wait_until(|| async { status(&isle).await["outputs"][0]["text"] == "" }).await;
}
