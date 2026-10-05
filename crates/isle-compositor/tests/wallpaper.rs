#![cfg(feature = "test-hooks")]

mod common;

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use common::{Isle, wait_until};
use isle_ipc::{desktops::DesktopsProxy, settings::SettingsProxy, wallpaper::WallpaperProxy};

fn near(got: [u8; 3], want: [u8; 3], by: u8) -> bool {
    got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= by)
}

async fn set(isle: &Isle, key: &str, value: &str) {
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set(key, &zvariant::Value::from(value))
        .await
        .unwrap();
}

/// A PNG of `width` by `height`, its leftmost `band` columns `edge` and the rest `body`.
fn picture(path: &Path, width: u32, height: u32, band: u32, edge: [u8; 3], body: [u8; 3]) {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for _ in 0..height {
        for x in 0..width {
            rgb.extend_from_slice(if x < band { &edge } else { &body });
        }
    }
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgb)
        .unwrap();
}

/// Shoots the corner until it is no longer `from`, and returns what it became first.
async fn first_change(isle: &Isle, from: [u8; 3]) -> [u8; 3] {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let corner = isle.shot("HEADLESS-1", "change").await.at(10, 10);
        if !near(corner, from, 2) {
            return corner;
        }
        assert!(Instant::now() < deadline, "the wallpaper never changed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn frost_is_drawn_in_the_dark_and_the_light() {
    let isle = Isle::start_with("frost", "").await;
    // The light from above the island, beside it, and the darker corners
    // (`surfaces/wallpaper.md`).
    let dark = isle.shot("HEADLESS-1", "dark").await;
    assert!(
        near(dark.at(850, 2), [0x27, 0x28, 0x2d], 4),
        "top centre {:?}",
        dark.at(850, 2)
    );
    assert!(
        near(dark.at(1, 1), [0x0f, 0x10, 0x12], 4),
        "corner {:?}",
        dark.at(1, 1)
    );

    set(&isle, "appearance.mode", "light").await;
    wait_until(|| async { isle.shot("HEADLESS-1", "light").await.at(850, 2)[0] > 0xf0 }).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let light = isle.shot("HEADLESS-1", "light").await;
    assert!(
        near(light.at(850, 2), [0xff, 0xff, 0xff], 4),
        "top centre {:?}",
        light.at(850, 2)
    );
    assert!(
        near(light.at(1, 1), [0xd9, 0xd9, 0xdb], 4),
        "corner {:?}",
        light.at(1, 1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn frost_at_rest_presents_no_frames() {
    let isle = Isle::start_with("frost-idle", "").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let test = isle.test().await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wide_picture_fills_by_cropping_its_sides_and_fits_with_bars() {
    let isle = Isle::start("picture").await;
    // 21:9 on a 16:9 screen, green down its left edge.
    let file = isle.dir.join("wide.png");
    picture(&file, 2100, 900, 100, [0, 200, 0], [0, 0, 220]);
    set(&isle, "wallpaper.source", file.to_str().unwrap()).await;
    wait_until(|| async {
        near(
            isle.shot("HEADLESS-1", "fill").await.at(10, 540),
            [0, 0, 220],
            3,
        )
    })
    .await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let fill = isle.shot("HEADLESS-1", "fill").await;
    assert!(
        near(fill.at(2, 540), [0, 0, 220], 3),
        "the green edge is cropped away"
    );
    assert!(near(fill.at(10, 2), [0, 0, 220], 3), "no bars");

    set(&isle, "wallpaper.fit", "fit").await;
    wait_until(|| async { isle.shot("HEADLESS-1", "fit").await.at(2, 540)[1] > 150 }).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let fit = isle.shot("HEADLESS-1", "fit").await;
    assert!(
        near(fit.at(2, 540), [0, 200, 0], 3),
        "the whole picture, green edge and all"
    );
    let bar = fit.at(960, 10);
    assert!(
        bar[1] > 0 && bar[2] > 0 && bar != [0, 0, 220],
        "the bars are the picture's mean colour, not its body: {bar:?}"
    );

    let status = WallpaperProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    assert_eq!(status[0].file, file.to_str().unwrap());
    let saved = fs::read_to_string(isle.dir.join("state/isle/wallpapers.json")).unwrap();
    assert!(saved.contains("\"mean\""), "{saved}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_crossfades_and_a_game_in_front_cuts_it() {
    let mut isle = Isle::start("crossfade").await;
    set(&isle, "wallpaper.source", "#000000").await;
    wait_until(|| async {
        near(
            isle.shot("HEADLESS-1", "black").await.at(10, 10),
            [0, 0, 0],
            2,
        )
    })
    .await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    set(&isle, "wallpaper.source", "#ffffff").await;
    let first = first_change(&isle, [0, 0, 0]).await;
    assert!(
        first[0] < 240,
        "the first frame of a change is part way: {first:?}"
    );
    wait_until(|| async { isle.shot("HEADLESS-1", "white").await.at(10, 10)[0] == 255 }).await;

    let _game = isle.client("steam_app_1", "3366ff");
    isle.wait_for_window("steam_app_1").await;
    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .any(|w| w.app_id == "steam_app_1" && w.focused)
    })
    .await;
    set(&isle, "wallpaper.source", "#000000").await;
    let first = first_change(&isle, [255, 255, 255]).await;
    assert!(
        near(first, [0, 0, 0], 2),
        "over a game a change is a cut: {first:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_picture_keeps_the_last_one_and_says_why() {
    let isle = Isle::start("missing").await;
    set(&isle, "wallpaper.source", "#3366ff").await;
    wait_until(|| async {
        near(
            isle.shot("HEADLESS-1", "blue").await.at(10, 10),
            [0x33, 0x66, 0xff],
            2,
        )
    })
    .await;
    set(&isle, "wallpaper.source", "/nonexistent/sunset.jpg").await;
    let wallpaper = WallpaperProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async { !wallpaper.status().await.unwrap()[0].problem.is_empty() }).await;
    assert_eq!(
        wallpaper.status().await.unwrap()[0].problem,
        "“sunset.jpg” couldn't be found."
    );
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(near(
        isle.shot("HEADLESS-1", "kept").await.at(10, 10),
        [0x33, 0x66, 0xff],
        2
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_screen_window_hides_the_wallpaper_entirely() {
    let mut isle = Isle::start("covered").await;
    let _client = isle.client("isle.test.Full", "3366ff");
    let window = isle.wait_for_window("isle.test.Full").await;
    DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(window.id, true)
        .await
        .unwrap();
    wait_until(|| async { isle.window(window.id).await.mode == "fullscreen" }).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // A change under it draws nothing: the wallpaper adds no element while covered.
    let test = isle.test().await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    set(&isle, "wallpaper.source", "#ff0000").await;
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), before);

    // Back from full screen, the new colour is there at once.
    DesktopsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set_full_screen(window.id, false)
        .await
        .unwrap();
    wait_until(|| async { isle.window(window.id).await.mode != "fullscreen" }).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(near(
        isle.shot("HEADLESS-1", "back").await.at(5, 1075),
        [255, 0, 0],
        2
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_screen_s_own_choice_stands_over_the_shared_one() {
    let isle = Isle::start("per-screen").await;
    let test = isle.test().await;
    test.add_output("HEADLESS-2", "Isle", "Headless", "2", 1280, 720)
        .await
        .unwrap();
    set(&isle, "wallpaper-output[Isle Headless 2].source", "#00ff00").await;
    set(&isle, "wallpaper.source", "#0000ff").await;
    wait_until(|| async {
        near(
            isle.shot("HEADLESS-1", "one").await.at(10, 10),
            [0, 0, 255],
            2,
        ) && near(
            isle.shot("HEADLESS-2", "two").await.at(10, 10),
            [0, 255, 0],
            2,
        )
    })
    .await;
}
