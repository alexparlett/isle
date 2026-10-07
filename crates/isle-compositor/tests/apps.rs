#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, wait_until};

#[tokio::test(flavor = "multi_thread")]
async fn an_app_whose_windows_close_and_that_keeps_running_is_in_the_background() {
    let mut isle = Isle::start("background").await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let stays = isle.client_with(&["--app-id", "isle.test.Stays", "--do", "linger"]);
    let window = isle.wait_for_window("isle.test.Stays").await;
    let windows = isle.windows_proxy().await;
    let app = |name: &'static str| {
        let windows = windows.clone();
        async move {
            windows
                .apps()
                .await
                .unwrap()
                .into_iter()
                .find(|a| a.app == name)
        }
    };
    assert!(!app("isle.test.Stays").await.unwrap().background);
    assert!(
        !windows.stop_app("isle.test.Stays").await.unwrap(),
        "it has a window"
    );

    assert!(windows.close(window.id).await.unwrap());
    stays.wait_for("closed").await;
    wait_until(|| async { app("isle.test.Stays").await.is_some_and(|a| a.background) }).await;
    let apps = windows.apps().await.unwrap();
    assert_eq!(
        apps.last().unwrap().app,
        "isle.test.Stays",
        "background apps come last"
    );
    assert_eq!(apps.last().unwrap().windows, 0);

    assert!(windows.stop_app("isle.test.Stays").await.unwrap());
    wait_until(|| async { stays.exited() }).await;
    wait_until(|| async { app("isle.test.Stays").await.is_none() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_exits_with_its_window_is_not_in_the_background() {
    let mut isle = Isle::start("nobackground").await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let goes = isle.client("isle.test.Goes", "ff0000");
    let window = isle.wait_for_window("isle.test.Goes").await;
    let windows = isle.windows_proxy().await;
    assert!(windows.close(window.id).await.unwrap());
    wait_until(|| async { goes.exited() }).await;
    wait_until(|| async {
        windows
            .apps()
            .await
            .unwrap()
            .iter()
            .all(|a| a.app != "isle.test.Goes")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn x11_attention_and_urgency_mark_the_app_until_it_is_activated() {
    let mut isle = Isle::start("x11attention").await;
    let asks = isle
        .x11_client(&["--class", "isle.test.Asks", "--colour", "ff0000"])
        .await;
    isle.wait_for_window("isle.test.Asks").await;
    let _front = isle
        .x11_client(&["--class", "isle.test.Front", "--colour", "0000ff"])
        .await;
    let front = isle.wait_for_window("isle.test.Front").await;
    wait_until(|| async { isle.window(front.id).await.focused }).await;
    let windows = isle.windows_proxy().await;
    let asking = || {
        let windows = windows.clone();
        async move {
            let app = windows
                .apps()
                .await
                .unwrap()
                .into_iter()
                .find(|a| a.app == "isle.test.Asks")
                .unwrap();
            (app.attention, app.urgent)
        }
    };

    asks.send("activate");
    wait_until(|| async { asking().await == (true, false) }).await;
    asks.send("urgent");
    wait_until(|| async { asking().await == (true, true) }).await;
    assert!(windows.activate_app("isle.test.Asks").await.unwrap());
    wait_until(|| async { asking().await == (false, false) }).await;

    asks.send("calm");
    assert!(windows.activate_app("isle.test.Front").await.unwrap());
    asks.send("demand");
    wait_until(|| async { asking().await == (true, false) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn launcher_entry_badges_progress_and_urgency_follow_their_sender() {
    let mut isle = Isle::start("badges").await;
    let _other = isle.client("isle.test.Other", "0000ff");
    isle.wait_for_window("isle.test.Other").await;
    let badged = isle.client("isle.test.Badged", "ff0000");
    isle.wait_for_window("isle.test.Badged").await;
    let windows = isle.windows_proxy().await;
    let app = || {
        let windows = windows.clone();
        async move {
            windows
                .apps()
                .await
                .unwrap()
                .into_iter()
                .find(|a| a.app == "isle.test.Badged")
                .unwrap()
        }
    };
    assert_eq!(app().await.badge, -1);

    badged.send("badge count=4");
    badged.wait_for("badge sent").await;
    wait_until(|| async { app().await.badge == 4 }).await;
    badged.clear();
    badged.send("badge progress=0.4");
    badged.wait_for("badge sent").await;
    wait_until(|| async { (app().await.progress - 0.4).abs() < 1e-9 }).await;
    assert_eq!(app().await.badge, 4, "a partial update keeps the count");

    assert!(windows.activate_app("isle.test.Other").await.unwrap());
    badged.clear();
    badged.send("badge urgent=true");
    badged.wait_for("badge sent").await;
    wait_until(|| async { app().await.urgent }).await;
    assert!(windows.activate_app("isle.test.Badged").await.unwrap());
    wait_until(|| async { !app().await.urgent }).await;

    badged.send("quit");
    wait_until(|| async { badged.exited() }).await;
    wait_until(|| async { windows.apps().await.unwrap().iter().all(|a| a.badge == -1) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_steam_game_takes_its_name_and_cover_from_the_library() {
    let mut isle = Isle::start_prepared("steamnames", "", |dir| {
        let root = dir.join("data/Steam");
        let apps = root.join("steamapps");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(
            apps.join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                root.display()
            ),
        )
        .unwrap();
        std::fs::write(
            apps.join("appmanifest_4183110.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"4183110\"\n\t\"name\"\t\t\"A Test Game\"\n}\n",
        )
        .unwrap();
        let cache = root.join("appcache/librarycache/4183110/abc");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("library_600x900.jpg"), b"").unwrap();
    })
    .await;
    let _game = isle.client("steam_app_4183110", "ff0000");
    isle.wait_for_window("steam_app_4183110").await;
    let windows = isle.windows_proxy().await;
    let game = || {
        let windows = windows.clone();
        async move {
            windows
                .apps()
                .await
                .unwrap()
                .into_iter()
                .find(|a| a.app == "steam_app_4183110")
                .unwrap()
        }
    };
    wait_until(|| async { game().await.name == "A Test Game" }).await;
    let game = game().await;
    assert!(
        game.icon.ends_with("4183110/abc/library_600x900.jpg"),
        "{}",
        game.icon
    );
}
