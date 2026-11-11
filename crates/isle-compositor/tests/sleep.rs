//! Power: PowerManagement's sleep holds beside ScreenSaver's display holds, apps listed to keep
//! the display on, idle's sleep step and its refusal, Keep Awake, `isle awake`'s hold and the
//! power button (`p20h-power.md` choice 20). Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use futures_util::StreamExt;
use isle_fakes::logind::Logind;
use isle_ipc::{displays::DisplaysProxy, session::SessionProxy, settings::SettingsProxy};

const POWER: u32 = 116;
const NO_SLEEP_PREVENTED: &str =
    "wallpaper {\n    source \"#1c1c1f\"\n}\nenergy {\n    prevent-sleep #false\n}\n";

#[zbus::proxy(
    interface = "org.freedesktop.PowerManagement.Inhibit",
    default_service = "org.freedesktop.PowerManagement",
    default_path = "/org/freedesktop/PowerManagement/Inhibit"
)]
trait PowerManagement {
    fn inhibit(&self, application: &str, reason: &str) -> zbus::Result<u32>;
    fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;
    fn has_inhibit(&self) -> zbus::Result<bool>;
    #[zbus(signal)]
    fn has_inhibit_changed(&self, has_inhibit_changed: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver"
)]
trait ScreenSaver {
    fn inhibit(&self, application_name: &str, reason_for_inhibit: &str) -> zbus::Result<u32>;
}

async fn idle(isle: &Isle) -> serde_json::Value {
    let text = SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .idle()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

async fn names(isle: &Isle, key: &str) -> Vec<String> {
    idle(isle).await[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_str().unwrap().to_owned())
        .collect()
}

async fn locked(isle: &Isle) -> bool {
    let status: serde_json::Value =
        serde_json::from_str(&isle.test().await.lock_status().await.unwrap()).unwrap();
    status["locked"] == true
}

async fn connect(isle: &Isle) -> zbus::Connection {
    zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .build()
        .await
        .unwrap()
}

async fn set(isle: &Isle, key: &str, value: zvariant::Value<'_>) {
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set(key, &value)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn power_management_holds_sleep_and_not_the_display_until_let_go_or_its_caller_leaves() {
    let isle = Isle::start("sleep-pm").await;
    let app = connect(&isle).await;
    let pm = PowerManagementProxy::new(&app).await.unwrap();
    let mut changes = pm.receive_has_inhibit_changed().await.unwrap();
    assert!(!pm.has_inhibit().await.unwrap());

    let cookie = pm.inhibit("Claude", "Working on a task").await.unwrap();
    // The hold is in force before Inhibit replies (choice 21).
    assert_eq!(names(&isle, "sleep_holders").await, ["Claude"]);
    assert!(
        names(&isle, "holders").await.is_empty(),
        "the display is not held"
    );
    assert!(
        changes
            .next()
            .await
            .unwrap()
            .args()
            .unwrap()
            .has_inhibit_changed
    );
    assert!(pm.has_inhibit().await.unwrap());
    let holds = SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .holds()
        .await
        .unwrap();
    let me = std::process::id() as i32;
    assert!(
        holds
            .iter()
            .any(|h| h.app == "Claude" && h.what == "sleep" && h.pid == me),
        "{holds:?}"
    );

    pm.un_inhibit(cookie).await.unwrap();
    assert!(
        !changes
            .next()
            .await
            .unwrap()
            .args()
            .unwrap()
            .has_inhibit_changed
    );
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;

    pm.inhibit("Claude", "Working on a task").await.unwrap();
    wait_until(|| async { !names(&isle, "sleep_holders").await.is_empty() }).await;
    app.close().await.unwrap();
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_listed_apps_sleep_hold_keeps_the_display_on() {
    let isle = Isle::start("sleep-listed").await;
    let app = connect(&isle).await;
    let pm = PowerManagementProxy::new(&app).await.unwrap();
    pm.inhibit("Claude", "Working on a task").await.unwrap();
    assert!(names(&isle, "holders").await.is_empty());

    set(
        &isle,
        "energy.display-apps",
        zvariant::Value::from(vec!["claude"]),
    )
    .await;
    wait_until(|| async { names(&isle, "holders").await == ["Claude"] }).await;
    let holds = SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .holds()
        .await
        .unwrap();
    assert!(
        holds.iter().any(|h| h.app == "Claude" && h.what == "idle"),
        "{holds:?}"
    );
    // Held, idle's steps do not come.
    isle.test().await.idle_advance(20 * 60_000).await.unwrap();
    assert!(!locked(&isle).await);
    assert_eq!(idle(&isle).await["off"], false);

    set(
        &isle,
        "energy.display-apps",
        zvariant::Value::from(Vec::<&str>::new()),
    )
    .await;
    wait_until(|| async { names(&isle, "holders").await.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn idle_sleeps_a_while_after_the_screens_go_off_unless_held_or_prevented() {
    let isle = Isle::start_with("sleep-step", NO_SLEEP_PREVENTED).await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();

    // A sleep hold keeps the step from coming at all.
    let app = connect(&isle).await;
    let pm = PowerManagementProxy::new(&app).await.unwrap();
    let cookie = pm.inhibit("Claude", "Working on a task").await.unwrap();
    displays.sleep().await.unwrap();
    wait_until(|| async { idle(&isle).await["off"] == true }).await;
    assert_eq!(idle(&isle).await["sleep_armed"], false);

    // Let go while the screens are off, the minute starts.
    pm.un_inhibit(cookie).await.unwrap();
    wait_until(|| async { idle(&isle).await["sleep_armed"] == true }).await;
    assert!(isle.test().await.sleep_now().await.unwrap());
    wait_until(|| async { logind.ended().contains(&"Suspend".to_owned()) }).await;

    // Input ends it; the switch back on keeps it from coming.
    isle.chord(&[30]).await;
    wait_until(|| async { idle(&isle).await["off"] == false }).await;
    assert_eq!(idle(&isle).await["sleep_armed"], false);
    set(&isle, "energy.prevent-sleep", zvariant::Value::from(true)).await;
    displays.sleep().await.unwrap();
    wait_until(|| async { idle(&isle).await["off"] == true }).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(idle(&isle).await["sleep_armed"], false);
    set(&isle, "energy.prevent-sleep", zvariant::Value::from(false)).await;
    wait_until(|| async { idle(&isle).await["sleep_armed"] == true }).await;

    // Display off set to Never: idle never sleeps.
    set(
        &isle,
        "lock-screen.display-off-after",
        zvariant::Value::from(0),
    )
    .await;
    wait_until(|| async { idle(&isle).await["sleep_armed"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_sleep_is_not_tried_again_before_input() {
    let isle = Isle::start_with("sleep-refused", NO_SLEEP_PREVENTED).await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    logind.refuse_sleep(true);
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    displays.sleep().await.unwrap();
    wait_until(|| async { idle(&isle).await["sleep_armed"] == true }).await;
    assert!(isle.test().await.sleep_now().await.unwrap());
    wait_until(|| async { logind.ended() == ["Suspend"] }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(idle(&isle).await["sleep_armed"], false, "not armed again");

    logind.refuse_sleep(false);
    isle.chord(&[30]).await;
    wait_until(|| async { idle(&isle).await["off"] == false }).await;
    displays.sleep().await.unwrap();
    wait_until(|| async { idle(&isle).await["sleep_armed"] == true }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn keep_awake_holds_the_display_or_only_the_computer_and_ends_when_turned_off() {
    let isle = Isle::start("sleep-keep-awake").await;
    set(
        &isle,
        "control-centre.modules",
        zvariant::Value::from(vec!["keep-awake:s"]),
    )
    .await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("keep-awake").await["on"] == false }).await;

    isle.panel(serde_json::json!({ "toggle": "keep-awake" }))
        .await;
    wait_until(|| async { names(&isle, "holders").await == ["Keep Awake"] }).await;
    assert!(isle.glyphs().await.contains(&"keep-awake".to_owned()));
    assert!(idle(&isle).await["keep_awake"]["left_ms"].is_null());
    isle.test().await.idle_advance(20 * 60_000).await.unwrap();
    assert!(!locked(&isle).await, "the session stays unlocked");

    // The display let go: the computer only, and the lock comes.
    isle.panel(serde_json::json!({ "switch": ["keep-awake", "display", false] }))
        .await;
    wait_until(|| async { names(&isle, "holders").await.is_empty() }).await;
    assert_eq!(names(&isle, "sleep_holders").await, ["Keep Awake"]);
    isle.test().await.idle_advance(20 * 60_000).await.unwrap();
    wait_until(|| async { locked(&isle).await }).await;
    assert_eq!(
        names(&isle, "sleep_holders").await,
        ["Keep Awake"],
        "locked, sleep is held"
    );

    // For an hour.
    isle.test().await.unlock().await.unwrap();
    wait_until(|| async { !locked(&isle).await }).await;
    // The first key wakes the screens and goes nowhere.
    isle.chord(&[30]).await;
    wait_until(|| async { idle(&isle).await["off"] == false }).await;
    isle.open_panel(None).await;
    isle.panel(serde_json::json!({ "choose": ["keep-awake", "1"] }))
        .await;
    let left = idle(&isle).await["keep_awake"]["left_ms"].as_u64().unwrap();
    assert!((3_590_000..=3_600_000).contains(&left), "{left}");

    isle.panel(serde_json::json!({ "toggle": "keep-awake" }))
        .await;
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;
    assert!(idle(&isle).await["keep_awake"].is_null());
    assert!(!isle.glyphs().await.contains(&"keep-awake".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn isle_awakes_hold_is_sleep_or_display_and_ends_with_its_caller() {
    let isle = Isle::start("sleep-awake").await;
    let cli = connect(&isle).await;
    let session = SessionProxy::new(&cli).await.unwrap();
    let cookie = session.hold("make", "make -j8", false).await.unwrap();
    assert_eq!(names(&isle, "sleep_holders").await, ["make"]);
    assert!(names(&isle, "holders").await.is_empty());
    session.release(cookie).await.unwrap();
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;

    session
        .hold("isle awake", "isle awake", true)
        .await
        .unwrap();
    assert_eq!(names(&isle, "holders").await, ["isle awake"]);
    cli.close().await.unwrap();
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;

    // Firefox's sound through ScreenSaver holds sleep alone.
    let firefox = connect(&isle).await;
    let saver = ScreenSaverProxy::new(&firefox).await.unwrap();
    saver.inhibit("firefox", "Playing audio").await.unwrap();
    assert_eq!(names(&isle, "sleep_holders").await, ["firefox"]);
    assert!(names(&isle, "holders").await.is_empty());
    firefox.close().await.unwrap();
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;

    // ScreenSaver's are display holds otherwise.
    let player = connect(&isle).await;
    ScreenSaverProxy::new(&player)
        .await
        .unwrap()
        .inhibit("Player", "Video")
        .await
        .unwrap();
    wait_until(|| async { names(&isle, "holders").await == ["Player"] }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_power_button_does_what_energy_says() {
    let isle = Isle::start("sleep-power-button").await;
    let logind = Logind::start(&isle.system_address()).await.unwrap();
    let session = || async {
        let text = isle.test().await.session().await.unwrap();
        serde_json::from_str::<serde_json::Value>(&text).unwrap()
    };

    set(&isle, "energy.power-button", zvariant::Value::from("sleep")).await;
    isle.chord(&[POWER]).await;
    wait_until(|| async { logind.ended().contains(&"Suspend(interactive)".to_owned()) }).await;
    assert_eq!(session().await["open"], false);

    set(&isle, "energy.power-button", zvariant::Value::from("lock")).await;
    isle.chord(&[POWER]).await;
    wait_until(|| async { locked(&isle).await }).await;
    // Locked, it wakes the lock's face and nothing else.
    isle.chord(&[POWER]).await;
    assert_eq!(logind.ended(), ["Suspend(interactive)"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn sound_playing_holds_sleep_and_not_the_display() {
    let mut isle = Isle::start("sleep-sound").await;
    if !isle.start_audio().await {
        eprintln!("PipeWire is not installed; skipped");
        return;
    }
    wait_until(|| async { isle.services().await["audio"]["available"] == true }).await;
    let player = isle.audio_process(
        "pacat",
        &[
            "--playback",
            "--client-name=Test Player",
            "--device=isle-test-speakers",
            "/dev/zero",
        ],
    );
    wait_until(|| async { names(&isle, "sleep_holders").await == ["Test Player"] }).await;
    assert!(names(&isle, "holders").await.is_empty());
    player.lock().unwrap().kill().unwrap();
    wait_until(|| async { names(&isle, "sleep_holders").await.is_empty() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn keep_awakes_module_page_and_glyph_are_drawn() {
    let isle = Isle::start("sleep-keep-awake-shots").await;
    set(
        &isle,
        "control-centre.modules",
        zvariant::Value::from(vec!["keep-awake:m", "night-light:s", "energy-mode:s"]),
    )
    .await;
    isle.open_panel(None).await;
    isle.panel_settled().await;
    isle.keep_shot("keep-awake-off").await;
    isle.panel(serde_json::json!({ "toggle": "keep-awake" }))
        .await;
    isle.panel_settled().await;
    isle.keep_shot("keep-awake-on").await;
    isle.open_panel(Some("keep-awake")).await;
    isle.panel_settled().await;
    isle.keep_shot("keep-awake-page").await;
    let rows = isle.rows().await;
    assert!(
        rows.iter()
            .any(|r| r["label"] == "Until Turned Off" && r["checked"] == true),
        "{rows:?}"
    );
    assert!(
        rows.iter()
            .any(|r| r["label"] == "Keep the Display On" && r["on"] == true)
    );
    isle.panel(serde_json::json!({ "switch": ["keep-awake", "display", false] }))
        .await;
    wait_until(|| async {
        isle.rows()
            .await
            .iter()
            .any(|r| r["label"] == "Keep the Display On" && r["on"] == false)
    })
    .await;
    isle.panel_settled().await;
    isle.keep_shot("keep-awake-page-display-off").await;
    // Back to the grid, then closed.
    isle.chord(&[1]).await;
    isle.chord(&[1]).await;
    wait_until(|| async { isle.island_output().await["panel"].is_null() }).await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("keep-awake-glyph-computer").await;
    set(
        &isle,
        "energy.keep-awake-display",
        zvariant::Value::from(true),
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("keep-awake-glyph-display").await;
}
