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
    loop {
        // Longer than the scrollbar's fade after its timeout.
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let now = test.frames("HEADLESS-1").await.unwrap();
        if now == last {
            return now;
        }
        last = now;
    }
}

const RED: [u8; 3] = [255, 0, 0];
const FILL: [u8; 3] = [28, 28, 34];

#[tokio::test(flavor = "multi_thread")]
async fn the_probe_draws_at_its_place_and_scale() {
    let isle = Isle::start("shell-draw").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    assert!(!shell.open("nothing").await.unwrap());
    assert!(shell.open("probe").await.unwrap());
    let info = probe(&isle).await;
    assert!(info.open);
    assert_eq!(
        (info.x, info.y, info.width, info.height),
        (720, 360, 480, 360)
    );
    settled(&isle).await;

    let image = shot(&isle, "scale1").await;
    assert_eq!(image.at(727, 367), RED, "the marker's last pixel");
    assert_eq!(image.at(728, 368), FILL, "the panel past the marker");
    assert_eq!(image.at(1195, 364), FILL, "the panel's top-right padding");
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
    assert_eq!(image.at(494, 194), RED, "the marker is 16 pixels at scale 2");
    assert_eq!(image.at(498, 198), FILL);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_open_still_probe_presents_nothing_and_a_closed_one_wakes_nothing() {
    let isle = Isle::start("shell-idle").await;
    let shell = ShellProxy::new(&isle.bus).await.unwrap();
    let test = isle.test().await;

    shell.open("probe").await.unwrap();
    let frames = settled(&isle).await;
    let wakes = probe(&isle).await.wakes;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), frames);
    assert_eq!(probe(&isle).await.wakes, wakes, "a still surface is never woken");

    shell.close("probe").await.unwrap();
    settled(&isle).await;
    let image = shot(&isle, "closed").await;
    assert_eq!(image.at(727, 367), [28, 28, 31], "nothing where the probe was");
    let wakes = probe(&isle).await.wakes;
    let frames = test.frames("HEADLESS-1").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(probe(&isle).await.wakes, wakes);
    assert_eq!(test.frames("HEADLESS-1").await.unwrap(), frames);
}
