//! Sound against a private PipeWire: OSDs from changes, the glyph, per-app streams, the
//! microphone and camera marks, and the server going and returning.
#![cfg(feature = "test-hooks")]

mod common;

use std::time::Duration;

use common::{Isle, wait_until};

const KEY_MUTE: u32 = 113;

async fn started(name: &str) -> Option<Isle> {
    let mut isle = Isle::start(name).await;
    if !isle.start_audio().await {
        eprintln!("PipeWire is not installed; skipped");
        return None;
    }
    wait_until(|| async { isle.services().await["audio"]["available"] == true }).await;
    Some(isle)
}

async fn speakers(isle: &Isle) -> serde_json::Value {
    isle.services().await["audio"]["state"]["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "isle-test-speakers")
        .cloned()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_from_anywhere_raises_one_osd_and_control_centres_own_raises_none() {
    let Some(isle) = started("audio-osd").await else {
        return;
    };
    assert_eq!(speakers(&isle).await["default"], true);
    isle.audio_tool("pactl", &["set-sink-volume", "isle-test-speakers", "40%"]);
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "event" && island["text"] == "Test Speakers"
    })
    .await;
    assert!((speakers(&isle).await["volume"].as_f64().unwrap() - 0.4).abs() < 0.01);

    // Once the OSD has gone, the module's slider moves the volume without one.
    wait_until(|| async { isle.island_output().await["form"] != "event" }).await;
    isle.open_panel(None).await;
    isle.panel(serde_json::json!({ "slide": ["sound", "", 0.7] }))
        .await;
    wait_until(|| async { (speakers(&isle).await["volume"].as_f64().unwrap() - 0.7).abs() < 0.01 })
        .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_ne!(isle.island_output().await["form"], "event");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_mute_key_mutes_and_shows_the_glyph() {
    let Some(isle) = started("audio-mute").await else {
        return;
    };
    isle.chord(&[KEY_MUTE]).await;
    wait_until(|| async { speakers(&isle).await["muted"] == true }).await;
    wait_until(|| async { isle.glyphs().await.contains(&"sound".to_owned()) }).await;
    isle.chord(&[KEY_MUTE]).await;
    wait_until(|| async { !isle.glyphs().await.contains(&"sound".to_owned()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stream_has_its_own_slider_and_a_recording_marks_the_microphone() {
    let Some(mut isle) = started("audio-streams").await else {
        return;
    };
    isle.audio_process(
        "pacat",
        &[
            "--playback",
            "--client-name=Test Player",
            "--device=isle-test-speakers",
            "/dev/zero",
        ],
    );
    isle.open_panel(Some("sound")).await;
    wait_until(|| async {
        isle.rows()
            .await
            .iter()
            .any(|r| r["label"] == "Test Player" && r["value"].is_number())
    })
    .await;
    assert!(
        !isle
            .rows()
            .await
            .iter()
            .any(|r| r["status"] == "No apps playing sound")
    );
    // Its slider moves its own volume.
    let index = isle.services().await["audio"]["state"]["playing"][0]["index"].clone();
    isle.panel(serde_json::json!({ "slide": ["sound", format!("app:{index}"), 0.5] }))
        .await;
    wait_until(|| async {
        let out = isle.audio_tool("pactl", &["list", "sink-inputs"]);
        String::from_utf8_lossy(&out.stdout).contains("/  50% /")
    })
    .await;

    // The page's input meter is a meter, not a use of the microphone.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(isle.island_output().await["privacy"], 0);
    isle.audio_process(
        "parec",
        &[
            "--client-name=Test Recorder",
            "--device=isle-test-microphone",
            "/dev/null",
        ],
    );
    wait_until(|| async { isle.island_output().await["privacy"] == 1 }).await;
    let rows = isle.island_output().await["panel"]["privacy"].clone();
    assert_eq!(rows[0]["app"], "Test Recorder", "{rows}");
    isle.panel_settled().await;
    isle.keep_shot("sound-page").await;
    isle.panel(serde_json::json!({ "page": null })).await;
    isle.panel_settled().await;
    isle.keep_shot("control-centre-sound").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_camera_read_by_an_app_marks_the_camera() {
    let Some(isle) = started("audio-camera").await else {
        return;
    };
    wait_until(|| async { isle.services().await["cameras"]["available"] == true }).await;
    isle.audio_tool(
        "pw-link",
        &[
            "isle-test-camera:capture_MONO",
            "isle-test-call:playback_MONO",
        ],
    );
    wait_until(|| async {
        isle.services().await["cameras"]["state"]["in_use"][0]["app"] == "Test Call"
    })
    .await;
    assert_eq!(isle.island_output().await["privacy"], 1);
    isle.audio_tool(
        "pw-link",
        &[
            "-d",
            "isle-test-camera:capture_MONO",
            "isle-test-call:playback_MONO",
        ],
    );
    wait_until(|| async { isle.island_output().await["privacy"] == 0 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sound_server_going_and_returning_is_followed_without_a_stall() {
    let Some(mut isle) = started("audio-restart").await else {
        return;
    };
    let quiet = isle.log().len();
    isle.stop_audio();
    wait_until(|| async { isle.services().await["audio"]["available"] == false }).await;
    assert!(isle.start_audio().await);
    wait_until(|| async { isle.services().await["audio"]["available"] == true }).await;
    wait_until(|| async { isle.services().await["cameras"]["available"] == true }).await;
    let stalls = isle.job_stalls_since(quiet);
    assert!(stalls.is_empty(), "{stalls:?}");

    // Nothing of the audio path loads a second D-Bus stack. (The NVIDIA EGL driver dlopens
    // libdbus itself, outside Isle.)
    let pid = isle_ipc::version::VersionProxy::new(&isle.bus)
        .await
        .unwrap()
        .pid()
        .await
        .unwrap();
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).unwrap();
    for module in ["libpipewire-module-rt", "libspa-dbus", "libpulse"] {
        assert!(!maps.contains(module), "{module} is loaded");
    }
}
