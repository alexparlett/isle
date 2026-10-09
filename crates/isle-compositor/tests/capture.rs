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
    // The chord is the desktop's, never the game's.
    assert!(!game.printed(&format!("key {THREE}")));
}
