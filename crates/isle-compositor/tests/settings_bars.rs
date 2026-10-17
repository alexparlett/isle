//! System Settings against apps.md's acceptance bars that a headless Isle can check: gone
//! when closed (3), whole with no daemon (4), its own menus (5), Cmd and not Ctrl (6),
//! killed without harm (7), live settings (8). Needs `--features test-hooks` and
//! `isle-settings` built.
#![cfg(feature = "test-hooks")]

use std::{path::PathBuf, time::Duration};

mod common;

use common::{Isle, keys::*, wait_until};

const APP: &str = "io.github.alexparlett.Isle.Settings";
const W: u32 = 17;

fn settings_bin() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle-settings");
    path.exists().then(|| path.to_string_lossy().into_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_its_window_ends_it_and_a_kill_harms_nothing() {
    let Some(bin) = settings_bin() else {
        eprintln!("skipped: build isle-system-settings first");
        return;
    };
    let mut isle = Isle::start("settings-bars-close").await;
    let app = isle.spawn_logged(&bin, &[]);
    isle.wait_for_window(APP).await;
    isle.chord(&[SUPER, W]).await;
    // Bar 3: gone within five seconds of its last window closing.
    let closed = std::time::Instant::now();
    wait_until(|| async { app.exited() }).await;
    assert!(closed.elapsed() < Duration::from_secs(5));

    // Bar 7: killed, the session goes on and it opens again at once.
    let again = isle.spawn_logged(&bin, &[]);
    isle.wait_for_window(APP).await;
    unsafe { libc::kill(again.pid(), libc::SIGKILL) };
    wait_until(|| async { again.exited() }).await;
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    assert!(!shell.list().await.unwrap().is_empty(), "the shell answers");
    wait_until(|| async { isle.windows().await.iter().all(|w| w.app_id != APP) }).await;
    let _third = isle.spawn_logged(&bin, &[]);
    isle.wait_for_window(APP).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_daemon_it_answers_and_its_menus_are_its_own() {
    let Some(bin) = settings_bin() else {
        eprintln!("skipped: build isle-system-settings first");
        return;
    };
    // The private system bus has no NetworkManager, BlueZ or power-profiles-daemon.
    let mut isle = Isle::start("settings-bars-daemons").await;
    let app = isle.spawn_logged(&bin, &["isle-settings://wi-fi"]);
    isle.wait_for_window(APP).await;

    // Bar 5: the bar shows its own menus, every pane in View.
    let menus = isle_ipc::menus::MenusProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async { menus.bar().await.unwrap().contains("Show Changed Settings") }).await;
    let bar = menus.bar().await.unwrap();
    for item in [
        "\"Bluetooth\"",
        "\"Displays\"",
        "\"Privacy & Security\"",
        "\"Mouse\"",
    ] {
        assert!(bar.contains(item), "{item} in {bar}");
    }

    // Bar 4: with the daemons missing it still takes a command at once.
    assert!(menus.activate(&["View", "Sound"]).await.unwrap());
    app.wait_for_part("id=\"pane:sound\"").await;

    // Bar 6: Cmd's chord reaches its command; Ctrl's does not.
    isle.chord(&[CTRL, F]).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!app.printed_part("id=\"find\""), "Ctrl+F is not Cmd+F");
    isle.chord(&[SUPER, F]).await;
    app.wait_for_part("id=\"find\"").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hand_edit_shows_in_the_window_within_a_second() {
    let Some(bin) = settings_bin() else {
        eprintln!("skipped: build isle-system-settings first");
        return;
    };
    let mut isle =
        Isle::start_with("settings-bars-live", "appearance {\n    mode \"dark\"\n}\n").await;
    let _app = isle.spawn_logged(&bin, &[]);
    let w = isle.wait_for_window(APP).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let (x, y) = (w.x as usize + 600, w.y as usize + 400);
    let dark = isle.shot("HEADLESS-1", "dark").await.at(x, y);

    // Bar 8: a hand edit, not through Settings.
    std::fs::write(
        isle.dir.join("config/settings.kdl"),
        "appearance {\n    mode \"light\"\n}\n",
    )
    .unwrap();
    let edited = std::time::Instant::now();
    let mut light = dark;
    while light == dark && edited.elapsed() < Duration::from_secs(1) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        light = isle.shot("HEADLESS-1", "light").await.at(x, y);
    }
    assert_ne!(light, dark, "the window is light within a second");
    assert!(
        light.iter().map(|c| u32::from(*c)).sum::<u32>()
            > dark.iter().map(|c| u32::from(*c)).sum::<u32>()
    );
}

/// Settings' memory idle for 30 s, by renderer, printed for the record; run by name.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn idle_memory() {
    let Some(bin) = settings_bin() else {
        eprintln!("skipped: build isle-system-settings first");
        return;
    };
    for renderer in ["software", "gl"] {
        let mut isle = Isle::start(&format!("settings-memory-{renderer}")).await;
        let app = if renderer == "software" {
            isle.spawn_logged(&bin, &[])
        } else {
            isle.spawn_logged("env", &["FREYA_RENDERER=gl", &bin])
        };
        isle.wait_for_window(APP).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
        let rollup = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", app.pid())).unwrap();
        let field = |name: &str| {
            rollup
                .lines()
                .find(|l| l.starts_with(name))
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("0")
                .to_owned()
        };
        println!(
            "{renderer}: Pss {} kB, Pss_Anon {} kB, Pss_File {} kB, SwapPss {} kB, Rss {} kB",
            field("Pss:"),
            field("Pss_Anon:"),
            field("Pss_File:"),
            field("SwapPss:"),
            field("Rss:")
        );
    }
}
