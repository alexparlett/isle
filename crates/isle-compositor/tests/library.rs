//! The game library: games launched through their launchers, and gamescope named after the
//! game it hosts. Needs `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use common::{Isle, wait_until};

/// Steam's fixture library under the test's data home, and a `steam.desktop` whose program
/// writes what it was given to `launched`.
fn fake_steam(dir: &Path) {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../isle-games/tests/fixtures/steam/steamapps");
    let steamapps = dir.join("data/Steam/steamapps");
    fs::create_dir_all(&steamapps).unwrap();
    for name in ["libraryfolders.vdf", "appmanifest_1466060.acf"] {
        fs::copy(fixture.join(name), steamapps.join(name)).unwrap();
    }
    let program = dir.join("steam.sh");
    fs::write(
        &program,
        format!(
            "#!/bin/sh\necho \"$@\" > '{}'\n",
            dir.join("launched").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        dir.join("data/applications/steam.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Steam\nExec={} %U\n",
            program.display()
        ),
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_game_starts_through_its_launchers_entry() {
    let isle = Isle::start_prepared("library-launch", "", fake_steam).await;
    let test = isle.test().await;
    wait_until(|| async { test.launch_game("steam:1466060").await.unwrap() }).await;
    let launched = isle.dir.join("launched");
    wait_until(|| async {
        fs::read_to_string(&launched).is_ok_and(|t| t.trim() == "steam://rungameid/1466060")
    })
    .await;
    assert!(!test.launch_game("steam:1").await.unwrap());
    assert!(!test.launch_game("nonsense").await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn gamescope_started_by_steam_is_the_game_it_hosts() {
    let mut isle = Isle::start_prepared("library-gamescope", "", fake_steam).await;
    let test = isle.test().await;
    // The library is read once the worker is up.
    wait_until(|| async { test.launch_game("steam:1466060").await.unwrap() }).await;
    let _client = isle.client_env("gamescope", &[("SteamGameId", "1466060")]);
    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .any(|w| w.app == "steam_app_1466060")
    })
    .await;
    let apps = isle.windows_proxy().await.apps().await.unwrap();
    let game = apps.iter().find(|a| a.app == "steam_app_1466060").unwrap();
    assert_eq!(game.name, "Tainted Grail: The Fall of Avalon");
}
