//! Media from MPRIS: the island's activity, Now Playing's transport, scrubbing, the keys, and
//! the most recently playing player taking the module.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};
use isle_fakes::mpris::Mpris;

const KEY_PLAYPAUSE: u32 = 164;

#[tokio::test(flavor = "multi_thread")]
async fn a_playing_player_leads_the_island_and_the_module_drives_it() {
    let isle = Isle::start("media-transport").await;
    let player = Mpris::start(&isle.bus_address(), "test", true)
        .await
        .unwrap();
    wait_until(|| async {
        let island = isle.island_output().await;
        island["form"] == "pill" && island["text"] == "Track 1"
    })
    .await;
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("now-playing").await["state"] == "Track 1" }).await;

    isle.panel(serde_json::json!({ "toggle": "now-playing" }))
        .await;
    wait_until(|| async { player.status().await == "Paused" }).await;
    isle.chord(&[KEY_PLAYPAUSE]).await;
    wait_until(|| async { player.status().await == "Playing" }).await;

    isle.panel_settled().await;
    isle.keep_shot("now-playing-grid").await;
    isle.open_panel(Some("now-playing")).await;
    isle.panel_settled().await;
    isle.keep_shot("now-playing-page").await;
    isle.panel(serde_json::json!({ "choose": ["now-playing", "next"] }))
        .await;
    wait_until(|| async { isle.module("now-playing").await["state"] == "Track 2" }).await;
    isle.panel(serde_json::json!({ "slide": ["now-playing", "position", 0.5] }))
        .await;
    wait_until(|| async { player.position().await == 90_000_000 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_player_that_starts_playing_takes_the_module() {
    let isle = Isle::start("media-recent").await;
    let first = Mpris::start(&isle.bus_address(), "first", true)
        .await
        .unwrap();
    let second = Mpris::start(&isle.bus_address(), "second", false)
        .await
        .unwrap();
    isle.open_panel(None).await;
    wait_until(|| async { isle.module("now-playing").await["state"] == "Track 1" }).await;
    first.play_pause().await;
    second.play_pause().await;
    wait_until(|| async {
        isle.services().await["media"]["state"]["players"][0]["bus"]
            == "org.mpris.MediaPlayer2.second"
    })
    .await;
    second.vanish().await;
    wait_until(|| async {
        let players = isle.services().await["media"]["state"]["players"].clone();
        players.as_array().is_some_and(|p| p.len() == 1)
    })
    .await;
    // Nothing plays now: the island has no media face.
    wait_until(|| async { isle.island_output().await["text"] != "Track 1" }).await;
}
