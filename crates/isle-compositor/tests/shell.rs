//! Shell surfaces hosted by the compositor, run headless. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::{fs, time::Duration};

use isle_ipc::{SurfaceInfo, settings::SettingsProxy, shell::ShellProxy};

mod common;

use common::{Isle, wait_until};

struct Shot {
    width: usize,
    rgba: Vec<u8>,
}

impl Shot {
    fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.width + x) * 4;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }
}

async fn shot(isle: &Isle, name: &str) -> Shot {
    let path = isle.dir.join(format!("{name}.png"));
    isle.test()
        .await
        .screenshot("HEADLESS-1", path.to_str().unwrap())
        .await
        .unwrap();
    if let Some(keep) = std::env::var_os("ISLE_TEST_SHOTS") {
        let _ = fs::copy(&path, std::path::Path::new(&keep).join(format!("{name}.png")));
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(&path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgba).unwrap();
    Shot {
        width: info.width as usize,
        rgba,
    }
}

async fn probe(isle: &Isle) -> SurfaceInfo {
    ShellProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name == "probe")
        .expect("the probe is registered in test builds")
}

/// Waits until the output stops presenting frames, and returns the count.
async fn settled(isle: &Isle) -> u64 {
    let test = isle.test().await;
    let mut last = test.frames("HEADLESS-1").await.unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "the output never stopped presenting"
        );
        // Longer than the scrollbar's fade after its timeout.
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let now = test.frames("HEADLESS-1").await.unwrap();
        if now == last {
            return now;
        }
        last = now;
    }
}

async fn tap(isle: &Isle, code: u32) {
    let test = isle.test().await;
    test.key(code, true).await.unwrap();
    test.key(code, false).await.unwrap();
}

async fn click(isle: &Isle, x: f64, y: f64) {
    let test = isle.test().await;
    test.pointer_to(x, y).await.unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
}

async fn model(isle: &Isle) -> (String, u32, bool) {
    isle.test().await.probe().await.unwrap()
}

/// The probe's first button, "Pressed n", and its second, "Animate".
const PRESS: (f64, f64) = (770.0, 426.0);
const ANIMATE: (f64, f64) = (870.0, 426.0);

const RED: [u8; 3] = [255, 0, 0];
/// The probe's fill: `colour-window-raised` in the dark appearance.
const FILL: [u8; 3] = [0x28, 0x29, 0x2e];

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_draws_at_its_place_and_scale() {
    let isle = Isle::start("shell-draw").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(!shell.open("nothing").await.unwrap());
    assert!(shell.open("probe").await.unwrap());
    tap(&isle, 15).await;
    let info = probe(&isle).await;
    assert!(info.open);
    assert_eq!(
        (info.x, info.y, info.width, info.height),
        (720, 360, 480, 360)
    );
    settled(&isle).await;

    let image = shot(&isle, "scale1").await;
    assert_eq!(image.at(1175, 391), RED, "the marker's last pixel");
    assert_eq!(image.at(1176, 392), FILL, "the panel past the marker");
    assert_eq!(image.at(1100, 364), FILL, "the panel's top padding");
    assert_eq!(image.at(720, 360), [28, 28, 31], "the corner is clipped round");
    assert_eq!(image.at(10, 10), [28, 28, 31], "the backdrop");

    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("output[Isle Headless 1].scale", &zvariant::Value::from(2.0f64))
        .await
        .unwrap();
    wait_until(|| async { probe(&isle).await.x == 240 }).await;
    settled(&isle).await;
    let image = shot(&isle, "scale2").await;
    // Freya anti-aliases the marker's far edge at scale 2, so look one pixel in.
    assert_eq!(image.at(1390, 242), RED, "the marker is 16 pixels at scale 2");
    assert_eq!(image.at(1394, 246), FILL);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_open_still_probe_presents_nothing_and_a_closed_one_wakes_nothing() {
    let isle = Isle::start("shell-idle").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;

    shell.open("probe").await.unwrap();
    // The field takes focus as the probe opens and its caret blinks; Tab moves focus on.
    tap(&isle, 15).await;
    let frames = settled(&isle).await;
    let wakes = probe(&isle).await.wakes;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), frames);
    assert_eq!(probe(&isle).await.wakes, wakes, "a still surface is never woken");

    shell.close("probe").await.unwrap();
    settled(&isle).await;
    let image = shot(&isle, "closed").await;
    assert_eq!(image.at(1175, 391), [28, 28, 31], "nothing where the probe was");
    let wakes = probe(&isle).await.wakes;
    let frames = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(probe(&isle).await.wakes, wakes);
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), frames);
}

#[tokio::test(flavor = "multi_thread")]
async fn clicks_keys_and_the_wheel_reach_the_probe() {
    let isle = Isle::start("shell-input").await;
    ShellProxy::new(&isle.bus).await.unwrap().open("probe").await.unwrap();
    // Typed straight after the open: the field takes them once it is mounted.
    for code in [30, 48, 46] {
        tap(&isle, code).await;
    }
    wait_until(|| async { model(&isle).await.0 == "abc" }).await;

    click(&isle, PRESS.0, PRESS.1).await;
    wait_until(|| async { model(&isle).await.1 == 1 }).await;

    settled(&isle).await;
    let before = shot(&isle, "wheel-before").await;
    let test = isle.test().await;
    test.pointer_to(960.0, 580.0).await.unwrap();
    test.axis(0, 360).await.unwrap();
    settled(&isle).await;
    let after = shot(&isle, "wheel-after").await;
    let moved = (520..620).any(|y| (740..1180).any(|x| before.at(x, y) != after.at(x, y)));
    assert!(moved, "the list scrolled");
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_the_probe_gives_the_keyboard_back_before_its_key_arrives() {
    let mut isle = Isle::start("shell-focus").await;
    let client = isle.client("isle.test.Focus", "336699");
    isle.wait_for_window("isle.test.Focus").await;
    client.wait_for("keyboard-enter").await;

    ShellProxy::new(&isle.bus).await.unwrap().open("probe").await.unwrap();
    client.wait_for("keyboard-leave").await;
    tap(&isle, 45).await;
    wait_until(|| async { model(&isle).await.0 == "x" }).await;

    // Enter submits the field: the probe closes and sends an "a".
    tap(&isle, 28).await;
    wait_until(|| async { !probe(&isle).await.open }).await;
    client.wait_for("key 30 pressed").await;
    let lines = client.lines.lock().unwrap().clone();
    assert!(!lines.iter().any(|l| l == "key 45 pressed"), "the x went to the probe");
    let entered = lines.iter().rposition(|l| l == "keyboard-enter").unwrap();
    let key = lines.iter().position(|l| l == "key 30 pressed").unwrap();
    assert!(entered < key, "focus came back before the key: {lines:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_animating_probe_draws_every_frame_and_stops_with_it() {
    let isle = Isle::start("shell-animate").await;
    ShellProxy::new(&isle.bus).await.unwrap().open("probe").await.unwrap();
    tap(&isle, 15).await;
    settled(&isle).await;
    click(&isle, ANIMATE.0, ANIMATE.1).await;
    wait_until(|| async { model(&isle).await.2 }).await;

    let test = isle.test().await;
    let start = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let drawn = test.frames("HEADLESS-1").await.unwrap() - start;
    assert!(drawn >= 50, "only {drawn} frames in a second while animating");

    click(&isle, ANIMATE.0, ANIMATE.1).await;
    wait_until(|| async { !model(&isle).await.2 }).await;
    let frames = settled(&isle).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), frames);
}

/// The focused node's role and label in a surface's tree.
async fn focused(shell: &ShellProxy<'_>) -> (String, String) {
    let tree: serde_json::Value =
        serde_json::from_str(&shell.tree("probe").await.unwrap()).unwrap();
    let node = tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["focused"] == true)
        .expect("a focused node")
        .clone();
    (
        node["role"].as_str().unwrap().to_string(),
        node["label"].as_str().unwrap_or("").to_string(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_names_its_controls_and_tab_walks_them() {
    let isle = Isle::start("shell-a11y").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    wait_until(|| async { focused(&shell).await.0 == "TextInput" }).await;

    let tree: serde_json::Value =
        serde_json::from_str(&shell.tree("probe").await.unwrap()).unwrap();
    let has = |role: &str, label: &str| {
        tree["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["role"] == role && n["label"] == label)
    };
    assert!(has("Window", "probe"));
    assert!(has("Button", "Pressed 0"));
    assert!(has("Button", "Animate"));
    assert!(has("TextInput", "Type here"));

    let mut seen = Vec::new();
    for _ in 0..6 {
        tap(&isle, 15).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        seen.push(focused(&shell).await);
    }
    for wanted in [("Button", "Pressed 0"), ("Button", "Animate"), ("TextInput", "Type here")] {
        assert!(
            seen.iter().any(|(r, l)| r == wanted.0 && l == wanted.1),
            "Tab never reached {wanted:?}: {seen:?}"
        );
    }
}

fn crash_records(isle: &Isle) -> usize {
    fs::read_dir(isle.dir.join("state/isle/crashes"))
        .map(|d| d.count())
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panic_anywhere_in_the_probe_rebuilds_it_with_its_model() {
    let mut isle = Isle::start("shell-panic").await;
    let client = isle.client("isle.test.Panic", "445566");
    isle.wait_for_window("isle.test.Panic").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    for code in [37, 18, 18, 25] {
        tap(&isle, code).await;
    }
    wait_until(|| async { model(&isle).await.0 == "keep" }).await;

    for (n, fault) in ["render", "event", "draw"].into_iter().enumerate() {
        isle.test().await.panic_probe(fault).await.unwrap();
        if fault == "event" {
            tap(&isle, 57).await;
        }
        let started = std::time::Instant::now();
        wait_until(|| async { probe(&isle).await.panics == n as u32 + 1 }).await;
        let info = probe(&isle).await;
        assert!(info.open && !info.disabled, "{fault}: {info:?}");
        // The rebuilt field takes focus again and blinks; Tab moves focus on.
        tap(&isle, 15).await;
        settled(&isle).await;
        let image = shot(&isle, &format!("after-{fault}")).await;
        assert_eq!(image.at(1175, 391), RED, "{fault}: the probe is drawn again");
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(model(&isle).await.0, "keep", "{fault}: the model outlives the tree");
    }
    assert_eq!(crash_records(&isle), 0, "a surface's panic is not a crash");
    // The window and the compositor carry on.
    assert!(isle.windows().await.iter().any(|w| w.app_id == "isle.test.Panic"));
    shell.close("probe").await.unwrap();
    client.wait_for("keyboard-enter").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sixth_panic_in_a_minute_turns_the_probe_off_until_restarted() {
    let isle = Isle::start("shell-panic-limit").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    for n in 1..=6u32 {
        isle.test().await.panic_probe("render").await.unwrap();
        wait_until(|| async {
            let info = probe(&isle).await;
            info.panics == n || info.disabled
        })
        .await;
    }
    let info = probe(&isle).await;
    assert!(info.disabled && !info.open, "{info:?}");
    assert!(!shell.open("probe").await.unwrap() || !probe(&isle).await.open);

    assert!(shell.restart("probe").await.unwrap());
    let info = probe(&isle).await;
    assert!(!info.disabled && info.panics == 0, "{info:?}");
    shell.open("probe").await.unwrap();
    assert!(probe(&isle).await.open);
    assert_eq!(crash_records(&isle), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_token_edit_reaches_the_probe_and_a_broken_file_keeps_the_last_good() {
    let isle = Isle::start("shell-tokens").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    tap(&isle, 15).await;
    settled(&isle).await;
    assert_eq!(shot(&isle, "tokens-before").await.at(1100, 364), FILL);

    let file = isle.dir.join("config/tokens.kdl");
    fs::write(&file, "version 1\ncolour { dark { window-raised \"#102030\" } }\n").unwrap();
    let started = std::time::Instant::now();
    wait_until(|| async { shot(&isle, "tokens-after").await.at(1100, 364) == [0x10, 0x20, 0x30] })
        .await;
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
    assert!(shell.tokens_status().await.unwrap().ok);

    fs::write(&file, "version 1\ncolour { dark { window-raised \"#1020\n").unwrap();
    wait_until(|| async { !shell.tokens_status().await.unwrap().ok }).await;
    let status = shell.tokens_status().await.unwrap();
    assert!(status.line >= 2, "{status:?}");
    settled(&isle).await;
    assert_eq!(shot(&isle, "tokens-broken").await.at(1100, 364), [0x10, 0x20, 0x30]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_appearance_setting_redraws_the_probe_in_light() {
    let isle = Isle::start("shell-light").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    tap(&isle, 15).await;
    settled(&isle).await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("appearance.mode", &zvariant::Value::from("light"))
        .await
        .unwrap();
    wait_until(|| async { shot(&isle, "light").await.at(1100, 364) == [0xff, 0xff, 0xff] }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_open_is_drawn_by_the_compositor_and_the_probe_draws_once() {
    let isle = Isle::start("shell-transition").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    let frames = test.frames("HEADLESS-1").await.unwrap();
    shell.open("probe").await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let moved = test.frames("HEADLESS-1").await.unwrap() - frames;
    let draws = probe(&isle).await.draws;
    assert!(moved >= 10, "only {moved} frames in the open");
    // The tree draws as it mounts and as its rows' fade runs, never for the open itself.
    assert!(draws < moved, "{draws} draws for {moved} frames");

    shell.close("probe").await.unwrap();
    let draws = probe(&isle).await.draws;
    let frames = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(test.frames("HEADLESS-1").await.unwrap() - frames >= 5, "the close moves");
    assert_eq!(probe(&isle).await.draws, draws, "a closing surface keeps its last frame");
}

#[tokio::test(flavor = "multi_thread")]
async fn skia_cache_stays_under_its_cap_over_a_hundred_opens() {
    let isle = Isle::start("shell-cache").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;
    test.transitions(false).await.unwrap();
    let mut first = None;
    let mut highest = 0;
    for cycle in 0..100 {
        let draws = probe(&isle).await.draws;
        shell.open("probe").await.unwrap();
        // Through the rows' fade, so its layers reach the cache.
        wait_until(|| async { probe(&isle).await.draws > draws }).await;
        tokio::time::sleep(Duration::from_millis(if cycle == 0 { 300 } else { 30 })).await;
        let (used, limit) = shell.cache().await.unwrap();
        assert!(used <= limit, "{used} over the cap {limit}");
        highest = highest.max(used);
        shell.close("probe").await.unwrap();
        if cycle == 0 {
            first = Some(shell.cache().await.unwrap().0);
        }
    }
    let (after, _) = shell.cache().await.unwrap();
    let first = first.unwrap();
    eprintln!("cache after the first close {first}, after the hundredth {after}, highest {highest}");
    assert!(after <= first + 1024 * 1024, "{after} grew from {first}");
}

/// Relative luminance of an sRGB colour, for a contrast ratio.
fn luminance(c: [f64; 3]) -> f64 {
    let linear = |v: f64| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2])
}

/// The contrast of text-2, rgba(242,243,245,0.64), drawn over `glass`.
fn text_2_contrast(glass: [u8; 3]) -> f64 {
    let g = glass.map(|c| c as f64 / 255.0);
    let text = [242.0, 243.0, 245.0].map(|c: f64| c / 255.0);
    let over = [0, 1, 2].map(|i| text[i] * 0.64 + g[i] * 0.36);
    let (a, b) = (luminance(over), luminance(g));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Inside the probe's glass strip, clear of its label.
const GLASS: (usize, usize) = (1100, 700);

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_s_glass_blurs_what_is_behind_it_and_holds_its_floor() {
    let mut isle = Isle::start("shell-glass").await;
    let _red = isle.client_with(&["--app-id", "isle.test.Red", "--colour", "ff0000", "--size", "1000x600"]);
    isle.wait_for_window("isle.test.Red").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    tap(&isle, 15).await;
    settled(&isle).await;

    let image = shot(&isle, "glass-red").await;
    let [r, g, b] = image.at(GLASS.0, GLASS.1);
    assert!(r as i32 > g as i32 + 40 && r as i32 > b as i32 + 40, "red shows through: {r},{g},{b}");
    assert_ne!([r, g, b], [255, 0, 0], "and is tinted");
    assert!(text_2_contrast([r, g, b]) >= 4.4, "{}", text_2_contrast([r, g, b]));

    // Nothing behind it changes, so it is not blurred again.
    let runs = probe(&isle).await.glass_runs;
    assert!(runs >= 1);
    click(&isle, PRESS.0, PRESS.1).await;
    settled(&isle).await;
    assert_eq!(probe(&isle).await.glass_runs, runs, "the probe redrew, the backdrop did not");
}

#[tokio::test(flavor = "multi_thread")]
async fn over_white_the_glass_tint_rises_to_hold_secondary_text() {
    let mut isle = Isle::start("shell-glass-white").await;
    let _white = isle.client_with(&["--app-id", "isle.test.White", "--colour", "ffffff", "--size", "1000x600"]);
    isle.wait_for_window("isle.test.White").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    tap(&isle, 15).await;
    wait_until(|| async { probe(&isle).await.glass_floor > 0.0 }).await;
    settled(&isle).await;
    let glass = shot(&isle, "glass-white").await.at(GLASS.0, GLASS.1);
    let ratio = text_2_contrast(glass);
    assert!(ratio >= 4.5, "text-2 at {ratio:.2} over {glass:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn glass_over_an_edge_is_blurred_the_right_way_up() {
    let mut isle = Isle::start("shell-glass-edge").await;
    // Centred, so its bottom edge crosses the glass strip at y 680.
    let _red = isle.client_with(&["--app-id", "isle.test.Edge", "--colour", "ff0000", "--size", "1000x280"]);
    isle.wait_for_window("isle.test.Edge").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    shell.open("probe").await.unwrap();
    tap(&isle, 15).await;
    settled(&isle).await;
    let image = shot(&isle, "glass-edge").await;
    let red = |y: usize| image.at(GLASS.0, y)[0] as i32 - image.at(GLASS.0, y)[2] as i32;
    let (above, at, below) = (red(645), red(680), red(716));
    assert!(above > at && at > below, "red fades across the edge: {above} {at} {below}");
    // Blur 48 spreads the edge across the whole strip.
    assert!(above - below < 60 && below > 5, "{above} {at} {below}");
}
