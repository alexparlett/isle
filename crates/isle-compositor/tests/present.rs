//! Presentation: what clients are told about their frames, and the plane trace. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::displays::DisplaysProxy;

mod common;

use common::{Client, Isle, wait_until};

/// The `presented` lines a client printed: seconds, nanoseconds, refresh, sequence, flags.
fn presented(client: &Client) -> Vec<[u64; 5]> {
    client
        .lines
        .lock()
        .unwrap()
        .iter()
        .filter_map(|l| l.strip_prefix("presented "))
        .map(|l| {
            let n: Vec<u64> = l.split(' ').map(|n| n.parse().unwrap()).collect();
            [n[0], n[1], n[2], n[3], n[4]]
        })
        .collect()
}

const VSYNC: u64 = 0x1;
const HW_CLOCK: u64 = 0x2;

#[tokio::test(flavor = "multi_thread")]
async fn presentation_times_rise_by_the_refresh() {
    let mut isle = Isle::start("presented").await;
    let client = isle.client("isle.test.Presented", "3366ff");
    isle.wait_for_window("isle.test.Presented").await;
    client.send("timing");
    client.send("animate");
    wait_until(|| async { presented(&client).len() >= 10 }).await;

    let frames = presented(&client);
    let refresh = frames[0][2];
    assert!((16_000_000..17_000_000).contains(&refresh), "{refresh}");
    let times: Vec<u128> = frames
        .iter()
        .map(|f| f[0] as u128 * 1_000_000_000 + f[1] as u128)
        .collect();
    assert!(times.windows(2).all(|w| w[1] > w[0]), "{times:?}");
    for f in &frames {
        assert_eq!(f[2], refresh);
        assert_ne!(f[4] & VSYNC, 0);
        // Headless frames carry Isle's own time.
        assert_eq!(f[4] & HW_CLOCK, 0);
    }

    let trace = DisplaysProxy::new(&isle.bus)
        .await
        .unwrap()
        .trace("HEADLESS-1", 0)
        .await
        .unwrap();
    assert!(trace.frames >= 10);
    assert_eq!(trace.scanned_out, 0);
    let last = trace.recent.last().unwrap();
    assert_eq!(last.primary, "composited");
    assert!(!last.hardware && !last.asynchronous && !last.vrr);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_layer_surface_is_told_when_its_frame_was_shown() {
    let mut isle = Isle::start("presented-layer").await;
    let layer = isle.client_with(&["--layer", "top", "--size", "400x40"]);
    layer.wait_for("configured 1920 40").await;
    layer.send("timing");
    layer.send("animate");
    wait_until(|| async { presented(&layer).len() >= 3 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_trace_resets_and_names_no_display_it_lacks() {
    let mut isle = Isle::start("trace").await;
    let client = isle.client("isle.test.Trace", "33ff66");
    isle.wait_for_window("isle.test.Trace").await;
    client.send("animate");
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async { displays.trace("HEADLESS-1", 0).await.unwrap().frames > 5 }).await;
    displays.reset_trace("HEADLESS-1").await.unwrap();
    let after = displays.trace("HEADLESS-1", 0).await.unwrap();
    assert!(after.frames < 5, "{}", after.frames);
    assert!(displays.trace("DP-9", 0).await.is_err());
}

fn nanos(f: &[u64; 5]) -> u128 {
    f[0] as u128 * 1_000_000_000 + f[1] as u128
}

#[tokio::test(flavor = "multi_thread")]
async fn fifo_frames_are_each_shown_a_refresh_apart() {
    let mut isle = Isle::start("fifo").await;
    let client = isle.client("isle.test.Fifo", "3366ff");
    isle.wait_for_window("isle.test.Fifo").await;
    client.send("timing");
    client.send("fifo 6");
    wait_until(|| async { presented(&client).len() >= 6 }).await;
    // Each fifo frame is shown; only the frame before the first may be replaced.
    let lines = client.lines.lock().unwrap().clone();
    let first = lines.iter().position(|l| l.starts_with("presented ")).unwrap();
    assert!(!lines[first..].iter().any(|l| l == "discarded"), "{lines:?}");
    let frames = presented(&client);
    let refresh = frames[0][2] as u128;
    for pair in frames.windows(2) {
        let gap = nanos(&pair[1]) - nanos(&pair[0]);
        assert!(gap * 10 >= refresh * 9, "{gap} under {refresh}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timed_commit_is_shown_no_sooner_than_asked_on_a_still_desktop() {
    let mut isle = Isle::start("commit-timing").await;
    let client = isle.client("isle.test.Timed", "3366ff");
    isle.wait_for_window("isle.test.Timed").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    client.send("timing");
    client.send("commit-at 400");
    wait_until(|| async { !presented(&client).is_empty() }).await;
    let target = client
        .lines
        .lock()
        .unwrap()
        .iter()
        .find_map(|l| l.strip_prefix("target ").map(str::to_owned))
        .unwrap();
    let (sec, nsec) = target.split_once(' ').unwrap();
    let target = sec.parse::<u128>().unwrap() * 1_000_000_000 + nsec.parse::<u128>().unwrap();
    let shown = nanos(&presented(&client)[0]);
    assert!(shown >= target, "shown {} ms early", (target - shown) / 1_000_000);
    assert!(shown < target + 100_000_000, "shown {} ms late", (shown - target) / 1_000_000);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_game_protocols_are_announced() {
    let mut isle = Isle::start("game-globals").await;
    let client = isle.client("isle.test.Globals", "3366ff");
    isle.wait_for_window("isle.test.Globals").await;
    client.send("globals");
    client.wait_for("globals done").await;
    let lines = client.lines.lock().unwrap().clone();
    let has = |name: &str| lines.iter().any(|l| l.starts_with(&format!("global {name} ")));
    // Every global gamescope's Wayland backend binds, and the game protocols.
    for name in [
        "wl_compositor",
        "wl_subcompositor",
        "wl_shm",
        "xdg_wm_base",
        "zwp_linux_dmabuf_v1",
        "wp_viewporter",
        "wp_single_pixel_buffer_manager_v1",
        "wp_fractional_scale_manager_v1",
        "wl_data_device_manager",
        "zwp_primary_selection_device_manager_v1",
        "wp_presentation",
        "wp_tearing_control_manager_v1",
        "wp_fifo_manager_v1",
        "wp_commit_timing_manager_v1",
        "wp_content_type_manager_v1",
        "zwp_pointer_constraints_v1",
        "zwp_relative_pointer_manager_v1",
    ] {
        assert!(has(name), "{name} missing");
    }
    // Explicit sync is served when the render node can wait on sync points.
    assert!(
        has("wp_linux_drm_syncobj_manager_v1") || isle.log().contains("no explicit sync"),
        "neither syncobj nor the reason it is missing"
    );
    client.send("content-type game");
    client.send("globals");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!client.exited());
}

async fn vrr_on(isle: &Isle) -> bool {
    DisplaysProxy::new(&isle.bus).await.unwrap().list().await.unwrap()[0].vrr_on
}

#[tokio::test(flavor = "multi_thread")]
async fn vrr_follows_a_full_screen_window_in_front_and_the_setting() {
    let mut isle = Isle::start("vrr").await;
    let game = isle.client("isle.test.FullScreen", "3366ff");
    isle.wait_for_window("isle.test.FullScreen").await;
    let other = isle.client("isle.test.Beside", "ff3366");
    isle.wait_for_window("isle.test.Beside").await;
    other.send("animate");
    assert!(!vrr_on(&isle).await);

    let windows = isle.windows_proxy().await;
    let full = isle.wait_for_window("isle.test.FullScreen").await;
    windows.focus(full.id).await.unwrap();
    game.send("fullscreen");
    wait_until(|| async { vrr_on(&isle).await }).await;
    let display = &DisplaysProxy::new(&isle.bus).await.unwrap().list().await.unwrap()[0];
    assert_eq!((display.vrr_capability.as_str(), display.vrr_setting.as_str()), ("on-demand", "fullscreen"));

    // Its window loses the keyboard: VRR goes; it comes back: VRR returns.
    let beside = isle.wait_for_window("isle.test.Beside").await;
    windows.focus(beside.id).await.unwrap();
    wait_until(|| async { !vrr_on(&isle).await }).await;
    windows.focus(full.id).await.unwrap();
    wait_until(|| async { vrr_on(&isle).await }).await;

    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus).await.unwrap();
    let identity = format!("{} {} {}", display.make, display.model, display.serial);
    let key = format!("output[{identity}].vrr");
    settings.set(&key, &zvariant::Value::from("off")).await.unwrap();
    wait_until(|| async { !vrr_on(&isle).await }).await;
    settings.set(&key, &zvariant::Value::from("always")).await.unwrap();
    game.send("unfullscreen");
    wait_until(|| async { vrr_on(&isle).await }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_screen_window_that_asks_tears_only_where_allowed() {
    let mut isle = Isle::start("tearing").await;
    let game = isle.client("isle.test.Tearing", "3366ff");
    let full = isle.wait_for_window("isle.test.Tearing").await;
    game.wait_for("keyboard-enter").await;
    game.send("animate");
    game.send("tearing async");
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let asked = || async {
        let trace = displays.trace("HEADLESS-1", 0).await.unwrap();
        trace.recent.last().is_some_and(|f| f.asked_async)
    };
    let display = &displays.list().await.unwrap()[0];
    let key = format!("output[{} {} {}].tearing", display.make, display.model, display.serial);
    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus).await.unwrap();

    // Allowed, but the window is not full screen.
    settings.set(&key, &zvariant::Value::from(true)).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(!asked().await);

    game.send("fullscreen");
    wait_until(|| async { asked().await }).await;

    game.send("tearing vsync");
    wait_until(|| async { !asked().await }).await;
    game.send("tearing async");
    wait_until(|| async { asked().await }).await;

    settings.set(&key, &zvariant::Value::from(false)).await.unwrap();
    wait_until(|| async { !asked().await }).await;
    let _ = full;
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_gamescope_starts_and_shows_its_game() {
    if std::process::Command::new("gamescope").arg("--help").output().is_err() {
        eprintln!("skipped: no gamescope");
        return;
    }
    let mut isle = Isle::start("gamescope").await;
    let inner = env!("CARGO_BIN_EXE_isle-test-x11-client");
    let gamescope = isle.spawn_logged(
        "gamescope",
        &["-W", "800", "-H", "600", "--backend", "wayland", "--", inner, "--class", "isle.test.Inner"],
    );
    let window = isle.wait_for_window("gamescope").await;
    wait_until(|| async { isle.window(window.id).await.game }).await;
    wait_until(|| async {
        gamescope
            .lines
            .lock()
            .unwrap()
            .iter()
            .any(|l| l.contains("Post-Initted Wayland backend"))
    })
    .await;
    let log = gamescope.lines.lock().unwrap().join("\n");
    assert!(!log.contains("Fatal") && !log.contains("not supported by the compositor"), "{log}");
}
