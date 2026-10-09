//! Stills: what a shot of a screen, a region or a window holds, and where it goes. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

struct Png {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Png {
    fn read(path: &str) -> Self {
        let decoder =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
        let mut reader = decoder.read_info().unwrap();
        let mut rgba = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba).unwrap();
        Self {
            width: info.width as usize,
            height: info.height as usize,
            rgba,
        }
    }

    fn at(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

async fn capture(isle: &Isle, spec: serde_json::Value) -> zbus::Result<Vec<String>> {
    isle.test().await.capture(&spec.to_string()).await
}

fn red([r, g, b, _]: [u8; 4]) -> bool {
    r > 200 && g < 60 && b < 60
}

fn green([r, g, b, _]: [u8; 4]) -> bool {
    g > 200 && r < 60 && b < 60
}

#[tokio::test(flavor = "multi_thread")]
async fn a_screen_shot_is_the_screen_named_as_macos_names_it() {
    let mut isle = Isle::start("capture-screen").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let folder = isle.dir.join("Shots");
    let written = capture(
        &isle,
        serde_json::json!({"target": "screen", "output": "HEADLESS-1",
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    assert_eq!(written.len(), 1);
    let name = std::path::Path::new(&written[0])
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        name.starts_with("Screenshot ") && name.ends_with(".png"),
        "{name}"
    );
    assert!(name.contains(" at "), "{name}");
    let png = Png::read(&written[0]);
    assert_eq!((png.width, png.height), (1920, 1080));
    let centre = (
        (info.x + info.width / 2) as usize,
        (info.y + info.height / 2) as usize,
    );
    assert!(
        red(png.at(centre.0, centre.1)),
        "{:?}",
        png.at(centre.0, centre.1)
    );

    // A second shot in the same second does not replace the first.
    let again = capture(
        &isle,
        serde_json::json!({"target": "screen", "output": "HEADLESS-1",
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    assert_ne!(again, written);
    assert!(std::path::Path::new(&written[0]).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_region_is_cut_from_the_screen() {
    let mut isle = Isle::start("capture-region").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let folder = isle.dir.join("Shots");
    let written = capture(
        &isle,
        serde_json::json!({"target": "region", "output": "HEADLESS-1",
                           "x": info.x + 10, "y": info.y + 40, "w": 120, "h": 80,
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    let png = Png::read(&written[0]);
    assert_eq!((png.width, png.height), (120, 80));
    assert!(red(png.at(60, 40)), "{:?}", png.at(60, 40));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_covered_window_comes_out_whole() {
    let mut isle = Isle::start("capture-window").await;
    let below = isle.client("isle.test.Below", "ff0000");
    below.wait_for("keyboard-enter").await;
    let below_info = isle.wait_for_window("isle.test.Below").await;
    let above = isle.client("isle.test.Above", "00ff00");
    above.wait_for("keyboard-enter").await;
    let above_info = isle.wait_for_window("isle.test.Above").await;
    let windows = isle.windows_proxy().await;
    windows
        .set_box(below_info.id, 100, 100, 400, 300)
        .await
        .unwrap();
    windows
        .set_box(above_info.id, 200, 150, 400, 300)
        .await
        .unwrap();
    wait_until(|| async { isle.window(above_info.id).await.x == 200 }).await;

    let folder = isle.dir.join("Shots");
    let bare = capture(
        &isle,
        serde_json::json!({"target": "window", "id": below_info.id, "shadow": false,
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    let png = Png::read(&bare[0]);
    let frame = isle.window(below_info.id).await;
    assert_eq!(png.width, frame.width as usize);
    // Where the green window covers it on screen, the shot is still red.
    let covered = png.at(png.width - 20, png.height - 20);
    assert!(red(covered), "{covered:?}");
    assert!(!green(covered));

    // With its shadow the picture is larger, and its edge is see-through.
    let shadowed = capture(
        &isle,
        serde_json::json!({"target": "window", "id": below_info.id, "shadow": true,
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    let png = Png::read(&shadowed[0]);
    assert!(png.width > frame.width as usize, "{} wide", png.width);
    assert!(png.at(0, 0)[3] < 255);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_locked_screen_shows_the_lock_and_no_window_is_given() {
    let mut isle = Isle::start("capture-locked").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    window.wait_for("keyboard-leave").await;
    let folder = isle.dir.join("Shots");
    let written = capture(
        &isle,
        serde_json::json!({"target": "screen", "output": "HEADLESS-1",
                           "to": folder.to_str().unwrap()}),
    )
    .await
    .unwrap();
    let png = Png::read(&written[0]);
    let centre = png.at(
        (info.x + info.width / 2) as usize,
        (info.y + info.height / 2) as usize,
    );
    assert!(
        !red(centre),
        "the window shows through the lock: {centre:?}"
    );

    let refused = capture(
        &isle,
        serde_json::json!({"target": "window", "id": info.id,
                           "to": folder.to_str().unwrap()}),
    )
    .await;
    assert!(refused.is_err(), "a window was given while locked");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shot_that_cannot_be_written_is_copied_instead() {
    let mut isle = Isle::start("capture-unwritable").await;
    // A file where the folder should be.
    let folder = isle.dir.join("NotAFolder");
    std::fs::write(&folder, b"").unwrap();
    let written = capture(
        &isle,
        serde_json::json!({"target": "screen", "output": "HEADLESS-1",
                           "to": folder.join("Shots").to_str().unwrap()}),
    )
    .await
    .unwrap();
    assert!(written.is_empty());
    let test = isle.test().await;
    wait_until(|| async {
        test.banners()
            .await
            .unwrap()
            .contains("Couldn't save the screenshot")
    })
    .await;
    let _ = &mut isle;
}

const SHIFT: u32 = 42;
const CTRL: u32 = 29;
const CMD: u32 = 125;
const THREE: u32 = 4;

fn screenshots(isle: &Isle) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(isle.dir.join("home/Pictures/Screenshots"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[tokio::test(flavor = "multi_thread")]
async fn shift_cmd_3_writes_a_file_per_screen() {
    let isle = Isle::start("capture-chord").await;
    isle.test()
        .await
        .add_output("HEADLESS-2", "Test", "Second", "2", 1280, 720)
        .await
        .unwrap();
    isle.chord(&[SHIFT, CMD, THREE]).await;
    wait_until(|| async { screenshots(&isle).len() == 2 }).await;
    let mut names = screenshots(&isle);
    names.sort_by_key(|n| n.contains(" (2)"));
    assert!(!names[0].contains(" (2)"), "{names:?}");
    assert!(names[1].ends_with(" (2).png"), "{names:?}");
    let first = Png::read(
        isle.dir
            .join("home/Pictures/Screenshots")
            .join(&names[0])
            .to_str()
            .unwrap(),
    );
    let second = Png::read(
        isle.dir
            .join("home/Pictures/Screenshots")
            .join(&names[1])
            .to_str()
            .unwrap(),
    );
    let mut sizes = [(first.width, first.height), (second.width, second.height)];
    sizes.sort();
    assert_eq!(sizes, [(1280, 720), (1920, 1080)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_puts_the_shot_on_the_clipboard_instead() {
    let isle = Isle::start("capture-copy").await;
    isle.chord(&[CTRL, SHIFT, CMD, THREE]).await;
    wait_until(|| async { isle.island_output().await["text"] == "Screenshot copied" }).await;
    assert!(screenshots(&isle).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shot_over_a_full_screen_game_says_so_in_the_island() {
    let mut isle = Isle::start("capture-game").await;
    let game = isle.client("steam_app_1", "3366ff");
    let id = isle.wait_for_window("steam_app_1").await.id;
    wait_until(|| async { isle.window(id).await.game }).await;
    game.send("fullscreen");
    wait_until(|| async { isle.window(id).await.mode == "fullscreen" }).await;
    wait_until(|| async { isle.window(id).await.focused }).await;
    game.clear();
    isle.chord(&[SHIFT, CMD, THREE]).await;
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    wait_until(|| async { isle.island_output().await["text"] == "Screenshot saved" }).await;
    assert!(thumbnail(&isle).await.is_null(), "a thumbnail over a game");
    // The chord is the desktop's, never the game's.
    assert!(!game.printed(&format!("key {THREE}")));
}

const FOUR: u32 = 5;
const ESC: u32 = 1;
const SPACE: u32 = 57;
const ENTER: u32 = 28;
const OPTION: u32 = 56;
const RIGHT: u32 = 106;
const DOWN: u32 = 108;
const BTN_LEFT: u32 = 272;

async fn pick(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.pick().await.unwrap()).unwrap()
}

async fn tap(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

fn newest_screenshot(isle: &Isle) -> Png {
    let dir = isle.dir.join("home/Pictures/Screenshots");
    let newest = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
        .unwrap();
    Png::read(newest.path().to_str().unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_region_is_cut_from_the_frame_frozen_at_the_chord() {
    let mut isle = Isle::start("pick-region").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_until(|| async { pick(&isle).await["mode"] == "region" }).await;
    // Frozen: the window turning green changes nothing in the shot.
    window.send("colour 00ff00");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let test = isle.test().await;
    let (x, y) = ((info.x + 20) as f64, (info.y + 40) as f64);
    test.pointer_to(x, y).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    test.pointer_to(x + 100.0, y + 50.0).await.unwrap();
    wait_until(|| async { pick(&isle).await["readout"] == true }).await;
    assert_eq!(
        pick(&isle).await["selection"],
        serde_json::json!([x, y, 100.0, 50.0])
    );
    isle.keep_shot("pick-region").await;
    test.button(BTN_LEFT, false).await.unwrap();
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    assert_eq!(pick(&isle).await, serde_json::Value::Null);
    let png = newest_screenshot(&isle);
    assert_eq!((png.width, png.height), (100, 50));
    assert!(red(png.at(50, 25)), "{:?}", png.at(50, 25));
    // The window kept the keyboard and saw neither the click nor its release.
    assert!(!window.printed("keyboard-leave"));
    assert!(!window.printed("button 272"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_region_can_be_taken_with_the_keys_alone() {
    let isle = Isle::start("pick-keys").await;
    let test = isle.test().await;
    test.pointer_to(300.0, 200.0).await.unwrap();
    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_until(|| async { !pick(&isle).await.is_null() }).await;
    tap(&isle, ENTER).await;
    test.key(SHIFT, true).await.unwrap();
    for _ in 0..5 {
        tap(&isle, RIGHT).await;
    }
    for _ in 0..3 {
        tap(&isle, DOWN).await;
    }
    test.key(SHIFT, false).await.unwrap();
    tap(&isle, RIGHT).await;
    wait_until(|| async {
        pick(&isle).await["selection"] == serde_json::json!([300.0, 200.0, 51.0, 30.0])
    })
    .await;
    tap(&isle, ENTER).await;
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    let png = newest_screenshot(&isle);
    assert_eq!((png.width, png.height), (51, 30));
}

#[tokio::test(flavor = "multi_thread")]
async fn space_picks_a_window_whole_and_option_leaves_its_shadow_out() {
    let mut isle = Isle::start("pick-window").await;
    let below = isle.client("isle.test.Below", "ff0000");
    below.wait_for("keyboard-enter").await;
    let below_info = isle.wait_for_window("isle.test.Below").await;
    let above = isle.client("isle.test.Above", "00ff00");
    above.wait_for("keyboard-enter").await;
    let above_info = isle.wait_for_window("isle.test.Above").await;
    let windows = isle.windows_proxy().await;
    windows
        .set_box(below_info.id, 100, 100, 400, 300)
        .await
        .unwrap();
    windows
        .set_box(above_info.id, 200, 150, 400, 300)
        .await
        .unwrap();
    wait_until(|| async { isle.window(above_info.id).await.x == 200 }).await;
    let frame = isle.window(below_info.id).await;

    let test = isle.test().await;
    test.pointer_to(120.0, frame.y as f64 + 100.0)
        .await
        .unwrap();
    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_until(|| async { !pick(&isle).await.is_null() }).await;
    tap(&isle, SPACE).await;
    wait_until(|| async { pick(&isle).await["lifted"] == below_info.id }).await;
    assert_eq!(pick(&isle).await["camera"], true);
    isle.keep_shot("pick-window").await;
    test.key(OPTION, true).await.unwrap();
    tap(&isle, ENTER).await;
    test.key(OPTION, false).await.unwrap();
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    let png = newest_screenshot(&isle);
    assert_eq!(png.width, frame.width as usize);
    let covered = png.at(png.width - 20, png.height - 20);
    assert!(red(covered), "{covered:?}");

    // A click takes it with its shadow.
    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_until(|| async { !pick(&isle).await.is_null() }).await;
    tap(&isle, SPACE).await;
    test.pointer_to(120.0, frame.y as f64 + 100.0)
        .await
        .unwrap();
    wait_until(|| async { pick(&isle).await["lifted"] == below_info.id }).await;
    test.button(BTN_LEFT, true).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    wait_until(|| async { screenshots(&isle).len() == 2 }).await;
    assert!(newest_screenshot(&isle).width > frame.width as usize);
    assert!(!below.printed("button 272"));
}

#[tokio::test(flavor = "multi_thread")]
async fn escape_leaves_the_pick_and_the_frames_live_again() {
    let isle = Isle::start("pick-escape").await;
    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_until(|| async { !pick(&isle).await.is_null() }).await;
    wait_until(|| async {
        isle.test()
            .await
            .frame_kinds("HEADLESS-1")
            .await
            .unwrap()
            .iter()
            .any(|k| k == "frozen")
    })
    .await;
    tap(&isle, ESC).await;
    wait_until(|| async { pick(&isle).await.is_null() }).await;
    wait_until(|| async {
        !isle
            .test()
            .await
            .frame_kinds("HEADLESS-1")
            .await
            .unwrap()
            .iter()
            .any(|k| k == "frozen")
    })
    .await;
    assert!(screenshots(&isle).is_empty());
}

async fn thumbnail(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.thumbnail().await.unwrap()).unwrap()
}

/// Shift+Cmd+3 and its thumbnail, and the file it holds.
async fn shot_with_thumbnail(isle: &Isle) -> (serde_json::Value, std::path::PathBuf) {
    isle.chord(&[SHIFT, CMD, THREE]).await;
    wait_until(|| async { thumbnail(isle).await["drawn"] == true }).await;
    let shown = thumbnail(isle).await;
    let path = std::path::PathBuf::from(shown["path"].as_str().unwrap());
    (shown, path)
}

fn centre_of(rect: &serde_json::Value) -> (f64, f64) {
    let r: Vec<f64> = rect
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_thumbnail_sits_in_the_corner_and_goes_on_its_own() {
    let isle = Isle::start("thumbnail-corner").await;
    let test = isle.test().await;
    test.pointer_to(100.0, 100.0).await.unwrap();
    let (shown, path) = shot_with_thumbnail(&isle).await;
    assert!(path.exists());
    isle.keep_shot("thumbnail").await;
    let r: Vec<i64> = shown["rect"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert!(r[0] + r[2] > 1800 && r[0] + r[2] < 1920, "{r:?}");
    assert!(r[1] + r[3] > 960 && r[1] + r[3] < 1080, "{r:?}");
    // A 16:9 shot fills the thumbnail's width, in a mat on each side.
    assert!(r[2] > 240 && r[2] <= 256, "{r:?}");

    // Held while the pointer is over it, gone after its time once it leaves.
    let (cx, cy) = centre_of(&shown["rect"]);
    test.pointer_to(cx, cy).await.unwrap();
    wait_until(|| async { thumbnail(&isle).await["hovered"] == true }).await;
    tokio::time::sleep(std::time::Duration::from_millis(5500)).await;
    assert!(!thumbnail(&isle).await.is_null(), "it went while held");
    test.pointer_to(100.0, 100.0).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5500)).await;
    assert!(thumbnail(&isle).await.is_null(), "it outstayed its time");
    assert!(path.exists(), "the shot went with its thumbnail");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flick_dismisses_and_a_drag_carries_the_file() {
    let mut isle = Isle::start("thumbnail-drag").await;
    let target = isle.client("isle.test.Target", "3366ff");
    target.wait_for("keyboard-enter").await;
    let w = isle.wait_for_window("isle.test.Target").await;
    let test = isle.test().await;

    let (shown, path) = shot_with_thumbnail(&isle).await;
    let (cx, cy) = centre_of(&shown["rect"]);
    test.pointer_to(cx, cy).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    test.pointer_to(cx + 40.0, cy + 2.0).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    wait_until(|| async { thumbnail(&isle).await.is_null() }).await;
    assert!(path.exists());
    assert!(!target.printed("button 272"));

    let (shown, path) = shot_with_thumbnail(&isle).await;
    let (cx, cy) = centre_of(&shown["rect"]);
    test.pointer_to(cx, cy).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    for f in [0.1, 0.4, 0.7] {
        test.pointer_to(cx + (w.x as f64 - cx) * f, cy + (w.y as f64 - cy) * f)
            .await
            .unwrap();
    }
    wait_until(|| async { thumbnail(&isle).await.is_null() }).await;
    let (tx, ty) = ((w.x + w.width / 2) as f64, (w.y + w.height / 2) as f64);
    test.pointer_to(tx - 10.0, ty).await.unwrap();
    test.pointer_to(tx, ty).await.unwrap();
    target.wait_for("dnd enter").await;
    test.button(BTN_LEFT, false).await.unwrap();
    target
        .wait_for(&format!("dropped file://{}", path.display()))
        .await;
    assert!(path.exists(), "a drop into an app keeps the file");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_menu_moves_and_trashes_the_shot() {
    let isle = Isle::start("thumbnail-menu").await;
    let test = isle.test().await;
    let (_, path) = shot_with_thumbnail(&isle).await;
    test.thumbnail_act("save-documents").await.unwrap();
    let moved = isle
        .dir
        .join("home/Documents")
        .join(path.file_name().unwrap());
    assert!(moved.exists() && !path.exists());
    assert_eq!(
        thumbnail(&isle).await["path"].as_str().unwrap(),
        moved.to_str().unwrap()
    );
    test.thumbnail_act("trash").await.unwrap();
    assert!(!moved.exists());
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(isle.dir.join("data/Trash/files").join(&name).exists());
    let info = std::fs::read_to_string(isle.dir.join(format!("data/Trash/info/{name}.trashinfo")))
        .unwrap();
    assert!(info.starts_with("[Trash Info]\nPath=/"), "{info}");
    assert!(thumbnail(&isle).await.is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn no_thumbnail_follows_a_copy() {
    let isle = Isle::start("thumbnail-copy").await;
    isle.chord(&[CTRL, SHIFT, CMD, THREE]).await;
    wait_until(|| async { isle.island_output().await["text"] == "Screenshot copied" }).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(thumbnail(&isle).await.is_null());
}

const FIVE: u32 = 6;
const TAB: u32 = 15;
const SLASH_SPACE: u32 = 57;

async fn toolbar(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.toolbar().await.unwrap()).unwrap()
}

/// Tab onto the modes and step to `mode`.
async fn toolbar_mode(isle: &Isle, mode: &str) {
    tap(isle, TAB).await;
    tap(isle, TAB).await;
    for _ in 0..3 {
        if toolbar(isle).await["mode"] == mode {
            return;
        }
        tap(isle, RIGHT).await;
    }
    assert_eq!(toolbar(isle).await["mode"], mode);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shot_from_the_toolbar_holds_none_of_it() {
    let isle = Isle::start("toolbar-screen").await;
    let test = isle.test().await;
    test.pointer_to(400.0, 300.0).await.unwrap();
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    let bar = toolbar(&isle).await["rect"].clone();
    let (bx, by) = centre_of(&bar);
    let live = isle.keep_shot("toolbar-live").await;
    let shown = live.at(bx as usize, by as usize);
    tap(&isle, ENTER).await;
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    assert_eq!(toolbar(&isle).await["open"], false);
    let png = newest_screenshot(&isle);
    let [r, g, b, _] = png.at(bx as usize, by as usize);
    assert_ne!([r, g, b], shown, "the bar is in the shot");
    let [r2, g2, b2, _] = png.at(bx as usize, 1060);
    assert!(
        (r as i32 - r2 as i32).abs() < 8 && (g as i32 - g2 as i32).abs() < 8,
        "the bar's place differs from the desktop beside it: {:?} {:?}",
        [r, g, b],
        [r2, g2, b2]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_portion_moves_by_keys_and_is_what_the_shot_holds() {
    let isle = Isle::start("toolbar-portion").await;
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    toolbar_mode(&isle, "shot-portion").await;
    let portion = toolbar(&isle).await["portion"].clone();
    assert_eq!(portion, serde_json::json!([480.0, 270.0, 960.0, 540.0]));
    isle.keep_shot("toolbar-portion").await;
    // Up and Down move the portion while the modes have the keys; Left and Right change mode.
    tap(&isle, DOWN).await;
    assert_eq!(
        toolbar(&isle).await["portion"],
        serde_json::json!([480.0, 271.0, 960.0, 540.0])
    );
    let test = isle.test().await;
    test.pointer_to(600.0, 400.0).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    test.pointer_to(640.0, 420.0).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    assert_eq!(
        toolbar(&isle).await["portion"],
        serde_json::json!([520.0, 291.0, 960.0, 540.0])
    );
    // A drag from a corner sizes it.
    test.pointer_to(1480.0, 831.0).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    test.pointer_to(1400.0, 801.0).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    assert_eq!(
        toolbar(&isle).await["portion"],
        serde_json::json!([520.0, 291.0, 880.0, 510.0])
    );
    tap(&isle, TAB).await;
    tap(&isle, TAB).await;
    tap(&isle, TAB).await;
    assert_eq!(toolbar(&isle).await["focus"], "Act");
    tap(&isle, ENTER).await;
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    let png = newest_screenshot(&isle);
    assert_eq!((png.width, png.height), (880, 510));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_toolbar_and_spotlight_close_each_other() {
    let isle = Isle::start("toolbar-centre").await;
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    isle.chord(&[CMD, SLASH_SPACE]).await;
    wait_until(|| async { toolbar(&isle).await["open"] == false }).await;
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    let spotlight: serde_json::Value =
        serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap();
    assert_eq!(spotlight["open"], false);
    tap(&isle, ESC).await;
    wait_until(|| async { toolbar(&isle).await["open"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_clicked_in_window_mode_is_taken_and_never_clicked() {
    let mut isle = Isle::start("toolbar-window").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    toolbar_mode(&isle, "shot-window").await;
    let test = isle.test().await;
    let (cx, cy) = (
        (info.x + info.width / 2) as f64,
        (info.y + info.height / 2) as f64,
    );
    test.pointer_to(cx, cy).await.unwrap();
    wait_until(|| async { toolbar(&isle).await["lifted"] == info.id }).await;
    test.button(BTN_LEFT, true).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    assert!(newest_screenshot(&isle).width > info.width as usize);
    assert!(!window.printed("button 272"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_countdown_shows_in_the_island_and_ctrl_cmd_escape_or_a_click_stops_it() {
    let isle = Isle::start_with("toolbar-countdown", "capture { timer 5; }").await;
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    tap(&isle, ENTER).await;
    wait_until(|| async { isle.island_output().await["text"] == "5" }).await;
    wait_until(|| async { isle.island_output().await["text"] == "4" }).await;
    isle.chord(&[CTRL, CMD, ESC]).await;
    wait_until(|| async { toolbar(&isle).await["countdown"].is_null() }).await;

    // A click on it stops it too.
    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    tap(&isle, ENTER).await;
    wait_until(|| async { isle.island_output().await["text"] == "5" }).await;
    let test = isle.test().await;
    test.pointer_to(960.0, 24.0).await.unwrap();
    test.button(BTN_LEFT, true).await.unwrap();
    test.button(BTN_LEFT, false).await.unwrap();
    wait_until(|| async { toolbar(&isle).await["countdown"].is_null() }).await;
    test.pointer_to(400.0, 500.0).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    assert!(screenshots(&isle).is_empty(), "a cancelled countdown shot");

    isle.chord(&[SHIFT, CMD, FIVE]).await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    tap(&isle, ENTER).await;
    tokio::time::sleep(std::time::Duration::from_millis(5800)).await;
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    assert_ne!(isle.island_output().await["text"], "1");
}

const SIX: u32 = 7;

#[tokio::test(flavor = "multi_thread")]
async fn the_colour_under_the_loupe_is_copied() {
    let mut isle = Isle::start("colour-pick").await;
    let window = isle.client("isle.test.Teal", "1e1f23");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Teal").await;
    let test = isle.test().await;
    // On the window's edge: one step left is the desktop.
    let (x, y) = (info.x as f64 + 0.5, (info.y + info.height / 2) as f64);
    test.pointer_to(x, y).await.unwrap();
    isle.chord(&[SHIFT, CMD, SIX]).await;
    wait_until(|| async { pick(&isle).await["mode"] == "colour" }).await;
    assert_eq!(pick(&isle).await["value"], "#1E1F23");
    assert_eq!(pick(&isle).await["readout"], true);
    isle.keep_shot("colour-pick").await;
    tap(&isle, RIGHT).await;
    tap(&isle, ENTER).await;
    wait_until(|| async { isle.island_output().await["text"] == "Colour copied" }).await;
    assert!(pick(&isle).await.is_null());
    assert_eq!(test.clipboard().await.unwrap(), "#1E1F23");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_capture_interface_and_the_control_do_what_the_chords_do() {
    let isle = Isle::start("capture-interface").await;
    let capture = isle_ipc::capture::CaptureProxy::new(&isle.bus)
        .await
        .unwrap();
    capture.shoot("screen", false).await.unwrap();
    wait_until(|| async { screenshots(&isle).len() == 1 }).await;
    assert!(capture.shoot("everything", false).await.is_err());
    let status: serde_json::Value = serde_json::from_str(&capture.status().await.unwrap()).unwrap();
    assert_eq!(status["portal"], true);
    assert_eq!(status["streams"], serde_json::json!([]));

    // The `isle` command speaks it too.
    let built = std::process::Command::new(env!("CARGO"))
        .args(["build", "-q", "-p", "isle-cli"])
        .status()
        .unwrap();
    assert!(built.success());
    let isle_cli =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle");
    let out = std::process::Command::new(&isle_cli)
        .args(["--json", "portal", "status"])
        .env("DBUS_SESSION_BUS_ADDRESS", isle.bus_address())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(printed["backend"], true);

    // Control Centre's screenshot control opens the toolbar and the panel goes.
    isle.open_panel(None).await;
    isle.panel(serde_json::json!({"toggle": "screenshot"}))
        .await;
    wait_until(|| async { toolbar(&isle).await["drawn"] == true }).await;
    wait_until(|| async { isle.island_output().await["panel"].is_null() }).await;
}
