//! A pretend HDR screen: the headless output drawn through the HDR pass into 10 bits (P22).
//! Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::time::Duration;

use isle_ipc::{displays::DisplaysProxy, settings::SettingsProxy};

mod common;

use common::{Isle, wait_until};

fn pq(nits: f64) -> u32 {
    let y = (nits / 10000.0).clamp(0.0, 1.0).powf(2610.0 / 16384.0);
    (((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y)).powf(78.84375) * 1023.0).round() as u32
}

fn near(got: (u32, u32, u32), want: u32) -> bool {
    [got.0, got.1, got.2].iter().all(|v| v.abs_diff(want) <= 2)
}

/// A white window on HEADLESS-1 made HDR, and the pixel at its centre.
async fn white_window(isle: &mut Isle) -> (i32, i32) {
    let _client = isle.client("isle.test.White", "ffffff");
    let window = isle.wait_for_window("isle.test.White").await;
    isle.test().await.make_hdr("HEADLESS-1", 302.0, 203.0).await.unwrap();
    (window.x + window.width / 2, window.y + window.height / 2)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_sdr_window_shows_at_the_reference_white() {
    let mut isle = Isle::start("hdr-white").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    let white = pq(203.0);
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), white) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sdr_brightness_applies_without_relighting() {
    let mut isle = Isle::start("hdr-sdr-white").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(203.0)) }).await;
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let info = displays.list().await.unwrap().into_iter().find(|d| d.name == "HEADLESS-1").unwrap();
    let identity = format!("{} {} {}", info.make, info.model, info.serial);
    displays
        .configure(&format!(r#"{{"{identity}": {{"sdr-white": 280}}}}"#))
        .await
        .unwrap();
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(280.0)) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_still_hdr_desktop_presents_no_frames() {
    let mut isle = Isle::start("hdr-idle").await;
    let (x, y) = white_window(&mut isle).await;
    let test = isle.test().await;
    wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(203.0)) }).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let before = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    // The island's clock may turn a minute in the window.
    assert!(test.frames("HEADLESS-1").await.unwrap() - before <= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn hdr_switched_on_and_kept_stays_on_through_other_settings_and_switches_off() {
    let isle = Isle::start("hdr-flow").await;
    let test = isle.test().await;
    test.make_hdr_capable("HEADLESS-1", 302.0).await.unwrap();
    let displays = DisplaysProxy::new(&isle.bus).await.unwrap();
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let info = || async {
        displays.list().await.unwrap().into_iter().find(|d| d.name == "HEADLESS-1").unwrap()
    };
    let before = info().await;
    assert_eq!((before.hdr_peak, before.hdr), (302.0, false));
    let identity = format!("{} {} {}", before.make, before.model, before.serial);

    let id = displays.configure(&format!(r#"{{"{identity}": {{"hdr": true}}}}"#)).await.unwrap();
    wait_until(|| async { info().await.hdr }).await;
    assert!(displays.keep(id).await.unwrap());

    // Settings' sliders write their keys straight to the file.
    for (key, value) in [("sdr-colour", zvariant::Value::from(0.5f64)), ("sdr-white", zvariant::Value::from(260i64))] {
        settings.set(&format!("output[{identity}].{key}"), &value).await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(info().await.hdr, "{key} turned HDR off");
    }

    let id = displays.configure(&format!(r#"{{"{identity}": {{"hdr": false}}}}"#)).await.unwrap();
    wait_until(|| async { !info().await.hdr }).await;
    assert!(displays.keep(id).await.unwrap());
    settings
        .set(&format!("output[{identity}].sdr-colour"), &zvariant::Value::from(1.0f64))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!info().await.hdr, "a setting turned HDR back on");
}

mod clients {
    use wayland_protocols::wp::color_management::v1::client::wp_color_manager_v1::{
        Feature, Primaries, TransferFunction,
    };

    use super::*;

    fn pq_eotf(e: f64) -> f64 {
        let p = e.clamp(0.0, 1.0).powf(1.0 / 78.84375);
        ((p - 0.8359375).max(0.0) / (18.8515625 - 18.6875 * p)).powf(1.0 / (2610.0 / 16384.0)) * 10000.0
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_manager_advertises_what_gamescope_and_mesa_look_for() {
        let mut isle = Isle::start("hdr-features").await;
        let client = isle.client("isle.test.Cm", "888888");
        client.wait_for("cm done").await;
        for feature in [
            Feature::Parametric,
            Feature::SetPrimaries,
            Feature::SetLuminances,
            Feature::SetMasteringDisplayPrimaries,
            Feature::ExtendedTargetVolume,
            Feature::WindowsScrgb,
        ] {
            assert!(client.printed(&format!("cm feature {}", feature as u32)), "{feature:?}");
        }
        for tf in [TransferFunction::St2084Pq, TransferFunction::Gamma22, TransferFunction::ExtLinear] {
            assert!(client.printed(&format!("cm tf {}", tf as u32)), "{tf:?}");
        }
        assert!(!client.printed(&format!("cm tf {}", TransferFunction::Srgb as u32)));
        for primaries in [Primaries::Srgb, Primaries::Bt2020] {
            assert!(client.printed(&format!("cm primaries {}", primaries as u32)), "{primaries:?}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_preferred_description_follows_the_screen() {
        let mut isle = Isle::start("hdr-preferred").await;
        let client = isle.client("isle.test.Cm", "888888");
        isle.wait_for_window("isle.test.Cm").await;
        client.send("preferred");
        client.wait_for("preferred done").await;
        assert!(client.printed(&format!("preferred tf {}", TransferFunction::Gamma22 as u32)));

        client.clear();
        isle.test().await.make_hdr("HEADLESS-1", 302.0, 250.0).await.unwrap();
        client.wait_for("preferred changed").await;
        client.send("preferred");
        client.wait_for("preferred done").await;
        assert!(client.printed(&format!("preferred tf {}", TransferFunction::St2084Pq as u32)));
        assert!(client.printed(&format!("preferred primaries {}", Primaries::Bt2020 as u32)));
        assert!(client.printed("preferred luminances 50 10000 250"));
        // Above the reference, so gamescope offers HDR.
        assert!(client.printed_part("preferred target 0 302"));
    }

    /// An HDR10 window mastered to 1000 cd/m² in `colour`, and the code at its centre.
    async fn hdr10(isle: &mut Isle, colour: &str) -> (i32, i32) {
        let client = isle.client("isle.test.Hdr10", colour);
        let window = isle.wait_for_window("isle.test.Hdr10").await;
        isle.test().await.make_hdr("HEADLESS-1", 302.0, 203.0).await.unwrap();
        client.send("describe pq 1000");
        client.wait_for("described").await;
        std::mem::forget(client);
        (window.x + window.width / 2, window.y + window.height / 2)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_screenshot_is_sdr_with_hdr10_white_at_sdr_white() {
        let mut isle = Isle::start("hdr-shot").await;
        let _sdr = isle.client_with(&["--app-id", "isle.test.Grey", "--colour", "808080", "--size", "800x600"]);
        let grey = isle.wait_for_window("isle.test.Grey").await;
        let (x, y) = hdr10(&mut isle, "ffffff").await;
        let test = isle.test().await;
        wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), pq(302.0)) }).await;
        let shot = isle.shot("HEADLESS-1", "hdr").await;
        // The HDR10 window opens centred over the larger grey one, which shows at its edge.
        assert_eq!(shot.at(x as usize, y as usize), [255, 255, 255]);
        let edge = (grey.x + 20) as usize;
        let grey_px = shot.at(edge, (grey.y + grey.height / 2) as usize);
        assert!(grey_px.iter().all(|&c| c.abs_diff(0x80) <= 1), "{grey_px:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn hdr10_below_the_peak_passes_through() {
        let mut isle = Isle::start("hdr-pass-through").await;
        let (x, y) = hdr10(&mut isle, "828282").await;
        let test = isle.test().await;
        let want = pq(pq_eotf(130.0 / 255.0));
        wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), want) }).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn hdr10_above_the_peak_is_tone_mapped_to_it() {
        let mut isle = Isle::start("hdr-tone-map").await;
        let (x, y) = hdr10(&mut isle, "ffffff").await;
        let test = isle.test().await;
        let peak = pq(302.0);
        wait_until(|| async { near(test.screen_code("HEADLESS-1", x, y).await.unwrap(), peak) }).await;
    }
}
