//! Pointer lock and confinement, and relative motion. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Client, Isle};

const CMD: u32 = 125;
const SHIFT: u32 = 42;
const TAB: u32 = 15;
const FOUR: u32 = 5;
const ESC: u32 = 1;
const LEFT_BUTTON: u32 = 272;

fn count(client: &Client, line: &str) -> usize {
    client.lines.lock().unwrap().iter().filter(|l| *l == line).count()
}

async fn wait_count(client: &Client, line: &str, n: usize) {
    common::wait_until(|| async { count(client, line) >= n }).await;
}

/// A game-like window under the pointer and focused, with relative motion asked for.
async fn game(isle: &mut Isle, name: &str) -> Client {
    let client = isle.client(name, "3366ff");
    isle.wait_for_window(name).await;
    client.wait_for("keyboard-enter").await;
    isle.test().await.pointer_to(960.0, 540.0).await.unwrap();
    client.send("relative");
    client
}

#[tokio::test(flavor = "multi_thread")]
async fn a_locked_pointer_stays_and_motion_arrives_relative() {
    let mut isle = Isle::start("pointer-lock").await;
    let client = game(&mut isle, "isle.test.Locked").await;
    client.send("lock-pointer");
    client.wait_for("locked").await;
    client.clear();
    let test = isle.test().await;
    test.pointer_by(40.0, -12.0).await.unwrap();
    client.wait_for("relative 40 -12 40 -12").await;
    test.pointer_by(500.0, 0.0).await.unwrap();
    client.wait_for("relative 500 0 500 0").await;
    assert!(
        !client.lines.lock().unwrap().iter().any(|l| l.starts_with("motion ")),
        "{:?}",
        client.lines.lock().unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_tab_and_the_pick_let_the_pointer_go_and_give_it_back() {
    let mut isle = Isle::start("pointer-release").await;
    let other = isle.client("isle.test.Other", "ff3366");
    isle.wait_for_window("isle.test.Other").await;
    let client = game(&mut isle, "isle.test.Held").await;
    client.send("lock-pointer");
    wait_count(&client, "locked", 1).await;

    isle.chord(&[CMD, TAB]).await;
    other.wait_for("keyboard-enter").await;
    wait_count(&client, "unlocked", 1).await;
    isle.chord(&[CMD, TAB]).await;
    wait_count(&client, "locked", 2).await;

    isle.chord(&[SHIFT, CMD, FOUR]).await;
    wait_count(&client, "unlocked", 2).await;
    isle.chord(&[ESC]).await;
    wait_count(&client, "locked", 3).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cmd_tab_to_a_game_on_another_desktop_gives_it_the_pointer() {
    let mut isle = Isle::start("pointer-desktop").await;
    let other = isle.client("isle.test.Other", "ff3366");
    isle.wait_for_window("isle.test.Other").await;
    let client = game(&mut isle, "isle.test.Away").await;
    let id = isle.wait_for_window("isle.test.Away").await.id;
    client.send("lock-pointer");
    wait_count(&client, "locked", 1).await;
    let desktops = isle_ipc::desktops::DesktopsProxy::new(&isle.bus).await.unwrap();
    desktops.add("").await.unwrap();
    assert!(desktops.move_window(id, 2).await.unwrap());
    desktops.switch("", 1).await.unwrap();
    wait_count(&client, "unlocked", 1).await;
    isle.test().await.pointer_by(1.0, 0.0).await.unwrap();
    common::wait_until(|| async {
        other.lines.lock().unwrap().iter().any(|l| l.starts_with("pointer-enter "))
    })
    .await;

    // The pointer has not moved since desktop 1's window was under it.
    isle.chord(&[CMD, TAB]).await;
    wait_count(&client, "locked", 2).await;
    client.clear();
    let test = isle.test().await;
    test.pointer_by(40.0, -12.0).await.unwrap();
    client.wait_for("relative 40 -12 40 -12").await;
    test.button(LEFT_BUTTON, true).await.unwrap();
    test.button(LEFT_BUTTON, false).await.unwrap();
    client.wait_for("button 272 released").await;
    let rows = desktops.list().await.unwrap();
    assert!(rows[1].current, "{rows:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_confined_pointer_stays_on_its_window() {
    let mut isle = Isle::start("pointer-confine").await;
    let client = game(&mut isle, "isle.test.Confined").await;
    client.send("confine-pointer");
    client.wait_for("confined").await;
    client.clear();
    let test = isle.test().await;
    // The window is 400 wide and centred, so 150 to the right stays on it and 500 more
    // would leave it.
    let last_motion = || {
        client
            .lines
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|l| l.starts_with("motion "))
            .cloned()
    };
    test.pointer_by(150.0, 0.0).await.unwrap();
    client.wait_for("relative 150 0 150 0").await;
    let inside = last_motion().unwrap();
    assert!(inside.starts_with("motion 350 "), "{inside}");
    test.pointer_by(500.0, 0.0).await.unwrap();
    client.wait_for("relative 500 0 500 0").await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(last_motion(), Some(inside));
}
