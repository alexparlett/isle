//! P4's X11 window manager, run headless with Xwayland and X11 clients. Needs
//! `--features test-hooks` and Xwayland on the path.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

mod common;

use common::{Isle, wait_until};

const RED: [u8; 3] = [0xff, 0, 0];
const TITLEBAR: [u8; 3] = [0x25, 0x26, 0x2b];

async fn window(isle: &Isle, app: &str) -> isle_ipc::WindowInfo {
    isle.wait_for_window(app).await
}

#[tokio::test(flavor = "multi_thread")]
async fn an_x11_window_maps_wears_isles_bar_and_takes_keys() {
    let mut isle = Isle::start("x11map").await;
    let client = isle
        .x11_client(&["--class", "isle.test.X11", "--colour", "ff0000"])
        .await;
    let w = window(&isle, "isle.test.X11").await;
    client.wait_for("focus-in").await;
    // An X11 window takes focus through its window manager a moment after it maps.
    wait_until(|| async { isle.window(w.id).await.focused }).await;
    assert_eq!((w.width, w.height), (400, 300));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let shot = isle.shot("HEADLESS-1", "x11").await;
    assert_eq!(
        shot.at((w.x + 320) as usize, (w.y - 16) as usize),
        TITLEBAR,
        "the bar"
    );
    assert_eq!(
        shot.at((w.x + 200) as usize, (w.y + 150) as usize),
        RED,
        "the window"
    );
    client
        .wait_for(&format!("configured 400 300 at {} {}", w.x, w.y))
        .await;

    isle.chord(&[30]).await;
    client.wait_for("key 38 pressed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn override_redirect_and_untitled_windows_are_left_where_they_ask() {
    let mut isle = Isle::start("x11or").await;
    let main = isle
        .x11_client(&["--class", "isle.test.Main", "--colour", "0000ff"])
        .await;
    window(&isle, "isle.test.Main").await;
    main.wait_for("focus-in").await;
    let menu = isle
        .x11_client(&[
            "--class",
            "isle.test.Menu",
            "--override-redirect",
            "--pos",
            "100,120",
            "--size",
            "200x150",
            "--colour",
            "ff0000",
        ])
        .await;
    menu.wait_for("mapped").await;
    let untitled = isle
        .x11_client(&[
            "--class",
            "isle.test.Untitled",
            "--untitled",
            "--pos",
            "1500,700",
            "--size",
            "120x100",
            "--colour",
            "00ff00",
        ])
        .await;
    untitled.wait_for("mapped").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let shot = isle.shot("HEADLESS-1", "x11or").await;
    assert_eq!(shot.at(150, 170), RED, "drawn at its own place");
    assert_eq!(
        shot.at(1550, 750),
        [0, 0xff, 0],
        "untitled, at its own place"
    );
    let listed = isle.windows().await;
    assert!(
        listed.iter().all(|w| w.app == "isle.test.Main"),
        "{listed:?}"
    );
    assert!(!menu.printed("focus-in") && !untitled.printed("focus-in"));
}

#[tokio::test(flavor = "multi_thread")]
async fn motif_hints_turn_the_bar_off_and_state_requests_act() {
    let mut isle = Isle::start("x11state").await;
    let client = isle
        .x11_client(&[
            "--class",
            "isle.test.Bare",
            "--no-decorations",
            "--colour",
            "ff0000",
        ])
        .await;
    let w = window(&isle, "isle.test.Bare").await;
    client.wait_for("focus-in").await;
    // Centred with no bar over it.
    assert_eq!((w.x, w.y), (760, 414));

    client.send("maximize");
    wait_until(|| async { isle.window(w.id).await.mode == "filled" }).await;
    client.send("unmaximize");
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;
    client.send("fullscreen");
    wait_until(|| async { isle.window(w.id).await.mode == "fullscreen" }).await;
    client.wait_for("configured 1920 1080 at 0 0").await;
    client.send("unfullscreen");
    wait_until(|| async { isle.window(w.id).await.mode == "floating" }).await;
    client.send("minimize");
    wait_until(|| async { isle.window(w.id).await.minimised }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn net_wm_moveresize_moves_the_window_and_tiles_at_an_edge() {
    let mut isle = Isle::start("x11move").await;
    let client = isle
        .x11_client(&["--class", "isle.test.Mover", "--no-decorations"])
        .await;
    let w = window(&isle, "isle.test.Mover").await;
    client.wait_for("focus-in").await;
    client.send("move-on-press");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let test = isle.test().await;
    let (x, y) = (w.x as f64 + 200.0, w.y as f64 + 20.0);
    test.pointer_to(x, y).await.unwrap();
    test.button(272, true).await.unwrap();
    client.wait_for("button 1 pressed").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    for step in 1..=4 {
        let f = step as f64 / 4.0;
        test.pointer_to(x + (2.0 - x) * f, y + (540.0 - y) * f)
            .await
            .unwrap();
    }
    test.button(272, false).await.unwrap();
    wait_until(|| async { isle.window(w.id).await.mode == "tiled:left-half" }).await;
    client.wait_for("configured 942 1008 at 12 60").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transient_opens_over_its_parent() {
    let mut isle = Isle::start("x11dialog").await;
    let client = isle
        .x11_client(&[
            "--class",
            "isle.test.Parent",
            "--size",
            "800x600",
            "--no-decorations",
        ])
        .await;
    let parent = window(&isle, "isle.test.Parent").await;
    client.send("dialog");
    client.wait_for("w1 mapped").await;
    wait_until(|| async { isle.windows().await.len() == 2 }).await;
    let dialog = isle
        .windows()
        .await
        .into_iter()
        .find(|w| w.id != parent.id)
        .unwrap();
    assert_eq!(dialog.parent, parent.id);
    assert_eq!(
        (dialog.x + dialog.width / 2, dialog.y + dialog.height / 2),
        (parent.x + 400, parent.y + 300)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn xwayland_killed_comes_back_on_the_same_display() {
    let mut isle = Isle::start("x11restart").await;
    let first = isle.x11_client(&["--class", "isle.test.Before"]).await;
    window(&isle, "isle.test.Before").await;
    let test = isle.test().await;
    let display = test.x_display().await.unwrap();
    let _ = std::process::Command::new("pkill")
        .args(["-f", &format!("Xwayland :{display} ")])
        .status();
    wait_until(|| async { isle.windows().await.is_empty() }).await;
    wait_until(|| async { first.exited() }).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while test.x_display().await.unwrap() != display {
        assert!(std::time::Instant::now() < deadline, "{}", isle.log());
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _after = isle.x11_client(&["--class", "isle.test.After"]).await;
    window(&isle, "isle.test.After").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shaped_window_wears_no_bar_and_no_shadow() {
    let mut isle = Isle::start("x11shape").await;
    let client = isle
        .x11_client(&[
            "--class",
            "isle.test.Shaped",
            "--shaped",
            "--argb",
            "--colour",
            "ff0000",
        ])
        .await;
    let w = window(&isle, "isle.test.Shaped").await;
    client.wait_for("focus-in").await;
    // Centred with no bar: 414 is the top of a 300-tall window in the work area.
    assert_eq!((w.x, w.y), (760, 414));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let shot = isle.shot("HEADLESS-1", "shaped").await;
    assert_eq!(
        shot.at((w.x + 100) as usize, (w.y + 150) as usize),
        RED,
        "its left half"
    );
    // What lies outside the shape is whatever Xwayland's buffer holds there; xeyes clears it
    // and shows through in the VM. Isle adds nothing around it.
    assert_eq!(
        shot.at((w.x + 100) as usize, (w.y - 16) as usize),
        [28, 28, 31],
        "no bar"
    );
    assert_eq!(
        shot.at((w.x + 100) as usize, (w.y - 16) as usize),
        [28, 28, 31],
        "no bar"
    );
    assert_eq!(
        shot.at((w.x + 300) as usize, (w.y + 300 + 20) as usize),
        [28, 28, 31],
        "no shadow"
    );
}

/// Whether the window `w` wears Isle's bar, from the pixel at its bar's middle right.
async fn barred(isle: &Isle, w: &isle_ipc::WindowInfo, name: &str) -> bool {
    tokio::time::sleep(Duration::from_millis(300)).await;
    let shot = isle.shot("HEADLESS-1", name).await;
    shot.at((w.x + w.width - 40) as usize, (w.y - 16) as usize) == TITLEBAR
}

#[tokio::test(flavor = "multi_thread")]
async fn x11_windows_are_decorated_by_their_type_motif_hints_and_frame() {
    let mut isle = Isle::start("x11rules").await;
    let cases: &[(&str, &[&str], bool)] = &[
        ("isle.test.Utility", &["--type", "UTILITY"], true),
        ("isle.test.Splash", &["--type", "SPLASH"], false),
        ("isle.test.Dock", &["--type", "DOCK"], false),
        // Title and border asked for, then the border alone.
        ("isle.test.Titled", &["--motif", "10"], true),
        ("isle.test.Border", &["--motif", "2"], false),
        // ALL less TITLE.
        ("isle.test.AllButTitle", &["--motif", "9"], false),
        ("isle.test.Gtk", &["--gtk-frame-extents", "20"], false),
    ];
    for (class, extra, wanted) in cases {
        let mut args = vec!["--class", class, "--colour", "ff0000"];
        args.extend_from_slice(extra);
        let client = isle.x11_client(&args).await;
        let w = window(&isle, class).await;
        assert_eq!(barred(&isle, &w, class).await, *wanted, "{class}");
        client.send("quit");
        wait_until(|| async { isle.windows().await.is_empty() }).await;
    }
}

async fn full_screen_desktops(isle: &Isle) -> usize {
    isle_ipc::desktops::DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .iter()
        .filter(|d| d.kind == "full-screen")
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn big_picture_is_a_full_screen_game_that_minimising_closes() {
    let mut isle = Isle::start("big-picture").await;
    let bp = isle
        .x11_client(&[
            "--class", "steam", "--title", "Steam Big Picture Mode", "--size", "1920x1080",
            "--no-decorations",
        ])
        .await;
    let window = window(&isle, "steam").await;
    wait_until(|| async { isle.window(window.id).await.game }).await;
    wait_until(|| async { full_screen_desktops(&isle).await == 1 }).await;

    bp.send("minimize");
    wait_until(|| async { full_screen_desktops(&isle).await == 0 }).await;
    let apps = isle.windows_proxy().await.apps().await.unwrap();
    let steam = apps.iter().find(|a| a.app == "steam");
    assert!(steam.is_none_or(|a| a.windows == 0), "{steam:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_steam_toast_is_left_where_it_asks_and_takes_nothing() {
    let mut isle = Isle::start("steam-toast").await;
    let _main = isle
        .x11_client(&["--class", "isle.test.Main", "--colour", "0000ff"])
        .await;
    let main = window(&isle, "isle.test.Main").await;
    wait_until(|| async { isle.window(main.id).await.focused }).await;
    let _toast = isle
        .x11_client(&[
            "--class", "steam", "--title", "notificationtoasts_7_desktop", "--size", "300x100",
            "--pos", "1600,950",
        ])
        .await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!isle.windows().await.iter().any(|w| w.app == "steam"));
    assert!(isle.window(main.id).await.focused);
}

#[tokio::test(flavor = "multi_thread")]
async fn quitting_steam_asks_steam_rather_than_closing_its_window() {
    let mut isle = Isle::start("steam-quit").await;
    let client = isle.client("steam", "3366ff");
    isle.wait_for_window("steam").await;
    assert!(isle.windows_proxy().await.quit_app("steam").await.unwrap());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!client.printed("closed"));
    assert!(isle.log().contains("quit through Steam"));
}
