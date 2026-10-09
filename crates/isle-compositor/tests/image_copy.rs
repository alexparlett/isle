//! The standard capture protocols through grim: a screen, a window by its foreign handle,
//! and the lock; and a session streaming to the test client, listed with Stop. Needs
//! `--features test-hooks` and grim.
#![cfg(feature = "test-hooks")]

mod common;

use std::process::Command;

use common::{Isle, wait_until};
use isle_ipc::session::SessionProxy;

/// grim's PNG of `args`, or `None` when it fails.
fn grim(isle: &Isle, args: &[&str]) -> Option<(u32, u32, Vec<u8>)> {
    let path = isle
        .dir
        .join(format!("grim-{}.png", args.join("-").replace('/', "_")));
    let status = Command::new("grim")
        .args(args)
        .arg(&path)
        .env("WAYLAND_DISPLAY", isle.socket())
        .env("XDG_RUNTIME_DIR", isle.dir.join("runtime"))
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).ok()?));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let rgb = match info.color_type {
        png::ColorType::Rgba => buf
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        _ => buf,
    };
    Some((info.width, info.height, rgb))
}

fn at(shot: &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 3] {
    let i = ((y * shot.0 + x) * 3) as usize;
    [shot.2[i], shot.2[i + 1], shot.2[i + 2]]
}

async fn copies(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.copies().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn grim_takes_the_screen_a_window_and_the_lock() {
    if Command::new("grim").arg("-h").output().is_err() {
        return;
    }
    let mut isle = Isle::start("image-copy").await;
    let window = isle.client("isle.test.Red", "ff0000");
    window.wait_for("keyboard-enter").await;
    let info = isle.wait_for_window("isle.test.Red").await;
    let cx = (info.x + info.width / 2) as u32;
    let cy = (info.y + info.height / 2) as u32;

    let screen = grim(&isle, &[]).expect("grim took the screen");
    assert_eq!((screen.0, screen.1), (1920, 1080));
    assert_eq!(at(&screen, cx, cy), [255, 0, 0]);

    wait_until(|| async { copies(&isle).await["toplevels"][info.id.to_string()].is_string() })
        .await;
    let identifier = copies(&isle).await["toplevels"][info.id.to_string()]
        .as_str()
        .unwrap()
        .to_owned();
    let alone = grim(&isle, &["-T", &identifier]).expect("grim took the window");
    assert_eq!(alone.0 as i32, info.width);
    assert_eq!(at(&alone, alone.0 / 2, alone.1 / 2), [255, 0, 0]);

    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    window.wait_for("keyboard-leave").await;
    let locked = grim(&isle, &[]).expect("grim took the locked screen");
    assert_ne!(
        at(&locked, cx, cy),
        [255, 0, 0],
        "the window shows through the lock"
    );
    assert!(
        grim(&isle, &["-T", &identifier]).is_none(),
        "a window was given while locked"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_capture_session_is_listed_in_sharing_and_stop_ends_it() {
    let mut isle = Isle::start("image-copy-stop").await;
    let client = isle.client("isle.test.Copier", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("copy-screen");
    client.wait_for("copied 3").await;
    wait_until(|| async { isle.island_output().await["privacy"] == 1 }).await;
    wait_until(|| async { isle.island_output().await["form"] == "pill" }).await;
    assert_ne!(isle.island_output().await["text"], "");
    isle.open_panel(None).await;
    isle.panel(serde_json::json!({"activity": ["sharing", "stop-copy:1"]}))
        .await;
    client.wait_for("copy stopped").await;
    wait_until(|| async { isle.island_output().await["privacy"] == 0 }).await;
    assert!(copies(&isle).await["sessions"].as_array().unwrap().is_empty());
}
