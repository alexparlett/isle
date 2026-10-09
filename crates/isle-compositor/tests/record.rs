//! Screen recordings through `isle-record` on a private PipeWire: the file, its sound, a
//! recording cut short, and one whose window closes. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::{path::PathBuf, process::Command, time::Duration};

use common::{Isle, wait_until};

/// The recorder, built for this run.
fn recorder() -> PathBuf {
    let status = Command::new(env!("CARGO"))
        .args(["build", "-q", "-p", "isle-record"])
        .status()
        .expect("cargo");
    assert!(status.success(), "isle-record did not build");
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle-record")
}

async fn start(name: &str, settings: &str) -> Isle {
    let helper = recorder();
    Isle::start_env(
        name,
        settings,
        |_| {},
        move |_| vec![("ISLE_RECORD_HELPER".into(), helper.display().to_string())],
    )
    .await
}

async fn recording(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.recording().await.unwrap()).unwrap()
}

/// The streams and format of a media file, as ffprobe reads them, or `None` if it cannot.
fn probe(path: &std::path::Path) -> Option<serde_json::Value> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| serde_json::from_slice(&out.stdout).ok())
        .flatten()
}

fn recordings(isle: &Isle) -> Vec<PathBuf> {
    std::fs::read_dir(isle.dir.join("home/Videos/Recordings"))
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recording_of_the_screen_plays_with_its_sound() {
    let mut isle = start(
        "record-screen",
        "capture {\n    system-audio #true\n    microphone \"isle-test-microphone\"\n}\n",
    )
    .await;
    if !isle.start_audio().await || Command::new("ffprobe").arg("-version").output().is_err() {
        return;
    }
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    wait_until(|| async { isle.services().await["audio"]["available"] == true }).await;
    let test = isle.test().await;
    test.record(&serde_json::json!({"screen": "HEADLESS-1"}).to_string())
        .await
        .unwrap();
    wait_until(|| async { recording(&isle).await["phase"] == "Running" }).await;
    // Something moves, so frames keep coming.
    for colour in ["00ff00", "0000ff", "ff0000", "00ff00"] {
        window.send(&format!("colour {colour}"));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert!(test.stop_recording().await.unwrap());
    wait_until(|| async { recording(&isle).await.is_null() }).await;
    let files = recordings(&isle);
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("Screen Recording ") && name.ends_with(".mp4"),
        "{name}"
    );
    let info = probe(&files[0]).expect("the file plays");
    let streams = info["streams"].as_array().unwrap();
    let video = streams.iter().find(|s| s["codec_type"] == "video").unwrap();
    assert_eq!(video["codec_name"], "h264");
    assert_eq!(
        (video["width"].as_u64(), video["height"].as_u64()),
        (Some(1920), Some(1080))
    );
    let audio = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .expect("a sound track");
    assert_eq!(audio["codec_name"], "aac");
    let duration: f64 = info["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(duration > 0.5, "{duration}");
    // The thumbnail follows with the file.
    wait_until(|| async {
        let t: serde_json::Value =
            serde_json::from_str(&isle.test().await.thumbnail().await.unwrap()).unwrap();
        t["path"].as_str() == Some(files[0].to_str().unwrap())
    })
    .await;
    // No stream is left, and the mark has gone.
    let casts: Vec<serde_json::Value> = serde_json::from_str(&test.casts().await.unwrap()).unwrap();
    assert!(casts.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recorded_window_that_closes_keeps_what_was_recorded() {
    let mut isle = start("record-window", "").await;
    if !isle.start_audio().await || Command::new("ffprobe").arg("-version").output().is_err() {
        return;
    }
    let window = isle.client("isle.test.Blue", "0000ff");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Blue").await;
    let test = isle.test().await;
    test.record(&serde_json::json!({"window": info.id}).to_string())
        .await
        .unwrap();
    wait_until(|| async { recording(&isle).await["phase"] == "Running" }).await;
    for colour in ["00ff00", "0000ff", "ff0000"] {
        window.send(&format!("colour {colour}"));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    isle.windows_proxy().await.close(info.id).await.unwrap();
    wait_until(|| async { recording(&isle).await.is_null() }).await;
    wait_until(|| async {
        isle.island_output().await["text"] == "Recording stopped: the window closed"
    })
    .await;
    let files = recordings(&isle);
    assert_eq!(files.len(), 1);
    let probed = probe(&files[0]).expect("the file plays");
    let video = &probed["streams"][0];
    assert_eq!(video["width"].as_u64(), Some(info.width as u64 & !1));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recorder_killed_midway_leaves_a_file_that_plays() {
    let mut isle = start("record-killed", "").await;
    if !isle.start_audio().await || Command::new("ffprobe").arg("-version").output().is_err() {
        return;
    }
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let test = isle.test().await;
    test.record(&serde_json::json!({"screen": "HEADLESS-1"}).to_string())
        .await
        .unwrap();
    wait_until(|| async { recording(&isle).await["phase"] == "Running" }).await;
    for colour in ["00ff00", "0000ff", "ff0000", "00ff00", "0000ff"] {
        window.send(&format!("colour {colour}"));
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    let pid = recording(&isle).await["pid"].as_u64().unwrap();
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
    wait_until(|| async { recording(&isle).await.is_null() }).await;
    let files = recordings(&isle);
    assert_eq!(files.len(), 1);
    assert!(
        probe(&files[0]).is_some(),
        "a recording cut short does not play"
    );
}
