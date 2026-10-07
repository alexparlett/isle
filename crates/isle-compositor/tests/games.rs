//! Games, windows holding the keys, Isle's own keys and the recording grab. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::{keyboard::KeyboardProxy, settings::SettingsProxy};

mod common;

use common::{Client, Isle, wait_until};

const CMD: u32 = 125;
const OPTION: u32 = 56;
const CTRL: u32 = 29;
const SHIFT: u32 = 42;
const TAB: u32 = 15;
const ESCAPE: u32 = 1;
const C: u32 = 46;
const H: u32 = 35;
const K: u32 = 37;

async fn chord(isle: &Isle, mods: &[u32], key: u32) {
    let test = isle.test().await;
    for m in mods {
        test.key(*m, true).await.unwrap();
    }
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
    for m in mods.iter().rev() {
        test.key(*m, false).await.unwrap();
    }
}

async fn game_window(isle: &Isle, app: &str) -> u64 {
    let id = isle.wait_for_window(app).await.id;
    wait_until(|| async { isle.window(id).await.game }).await;
    id
}

/// Over a game Ctrl is Ctrl and Cmd is Super, nothing is swapped, and only the escape set
/// acts: Cmd+Tab does, Cmd+H does not.
async fn plays_raw(isle: &Isle, game: &Client, id: u64) {
    let test = isle.test().await;
    game.clear();
    chord(isle, &[CMD], C).await;
    game.wait_for(&format!("key {C} released")).await;
    assert!(
        game.printed(&format!("key {CMD} pressed")),
        "Cmd reaches a game as Super"
    );
    assert!(!game.printed(&format!("key {CTRL} pressed")));

    let pc = test
        .add_keyboard("AT Translated Set 2 keyboard")
        .await
        .unwrap();
    game.clear();
    test.key_on(&pc, OPTION, true).await.unwrap();
    test.key_on(&pc, OPTION, false).await.unwrap();
    game.wait_for(&format!("key {OPTION} released")).await;

    let before = test.held_releases().await.unwrap();
    chord(isle, &[CMD], TAB).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        before + 1,
        "Cmd+Tab acts"
    );
    game.clear();
    chord(isle, &[CMD], H).await;
    game.wait_for(&format!("key {H} released")).await;
    assert!(!isle.window(id).await.hidden, "Cmd+H is the game's");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_steam_labelled_window_is_a_game_and_plays_raw() {
    let mut isle = Isle::start("games-label").await;
    let game = isle.client("steam_app_1", "3366ff");
    game.wait_for("keyboard-enter").await;
    let id = game_window(&isle, "steam_app_1").await;
    plays_raw(&isle, &game, id).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_process_steam_started_is_a_game() {
    let mut isle = Isle::start("games-env").await;
    let game = isle.client_env("isle.test.NativeGame", &[("SteamGameId", "440")]);
    game.wait_for("keyboard-enter").await;
    let id = game_window(&isle, "isle.test.NativeGame").await;
    plays_raw(&isle, &game, id).await;
}

struct FakeGameMode {
    pids: Vec<i32>,
}

#[zbus::interface(name = "com.feralinteractive.GameMode")]
impl FakeGameMode {
    fn list_games(&self) -> Vec<(i32, zvariant::OwnedObjectPath)> {
        self.pids
            .iter()
            .map(|p| {
                (
                    *p,
                    zvariant::OwnedObjectPath::try_from("/com/feralinteractive/GameMode/Games/1")
                        .unwrap(),
                )
            })
            .collect()
    }

    #[zbus(signal)]
    async fn game_registered(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        pid: i32,
        path: zvariant::ObjectPath<'_>,
    ) -> zbus::Result<()>;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_process_gamemode_registered_is_a_game() {
    let mut isle = Isle::start("games-gamemode").await;
    let game = isle.client("isle.test.Registered", "3366ff");
    game.wait_for("keyboard-enter").await;
    let id = isle.wait_for_window("isle.test.Registered").await.id;
    assert!(!isle.window(id).await.game);

    let path = "/com/feralinteractive/GameMode";
    let service = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .serve_at(
            path,
            FakeGameMode {
                pids: vec![game.pid()],
            },
        )
        .unwrap()
        .name("com.feralinteractive.GameMode")
        .unwrap()
        .build()
        .await
        .unwrap();
    let iface = service
        .object_server()
        .interface::<_, FakeGameMode>(path)
        .await
        .unwrap();
    let games = zvariant::ObjectPath::try_from("/com/feralinteractive/GameMode/Games/1").unwrap();
    FakeGameMode::game_registered(iface.signal_emitter(), game.pid(), games)
        .await
        .unwrap();
    wait_until(|| async { isle.window(id).await.game }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_not_games_entry_overrides_the_label() {
    let mut isle = Isle::start("games-override").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set(
            "windows.not-games",
            &zvariant::Value::from(vec!["steam_app_2"]),
        )
        .await
        .unwrap();
    let app = isle.client("steam_app_2", "3366ff");
    app.wait_for("keyboard-enter").await;
    app.clear();
    chord(&isle, &[CMD], C).await;
    app.wait_for(&format!("key {C} released")).await;
    assert!(app.printed(&format!("key {CTRL} pressed")), "translated");
    assert!(!isle.wait_for_window("steam_app_2").await.game);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_holding_the_keys_gets_them_until_cmd_escape() {
    let mut isle = Isle::start("inhibit").await;
    let test = isle.test().await;
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    let other = isle.client("isle.test.Other", "ff6633");
    other.wait_for("keyboard-enter").await;
    let vm = isle.client("isle.test.Vm", "3366ff");
    vm.wait_for("keyboard-enter").await;
    vm.send("inhibit");
    vm.wait_for("inhibitor active").await;
    assert_eq!(
        keyboard.status().await.unwrap().inhibited_by,
        "isle.test.Vm"
    );

    vm.clear();
    chord(&isle, &[CMD], TAB).await;
    vm.wait_for(&format!("key {TAB} released")).await;
    assert!(vm.printed(&format!("key {CMD} pressed")));
    assert_eq!(
        test.held_releases().await.unwrap(),
        0,
        "the switcher did not run"
    );

    chord(&isle, &[CMD], ESCAPE).await;
    vm.wait_for("inhibitor inactive").await;
    assert!(keyboard.status().await.unwrap().taken_back);
    chord(&isle, &[CMD], TAB).await;
    assert_eq!(
        test.held_releases().await.unwrap(),
        1,
        "the desktop has Cmd+Tab back"
    );
    let windows = isle.windows_proxy().await;
    let other = isle.wait_for_window("isle.test.Other").await.id;
    wait_until(|| async { isle.window(other).await.focused }).await;

    // Back to it, the inhibitor holds again; Cmd+Escape takes the keys back.
    vm.clear();
    windows
        .focus(isle.wait_for_window("isle.test.Vm").await.id)
        .await
        .unwrap();
    vm.wait_for("inhibitor active").await;
    chord(&isle, &[CMD], ESCAPE).await;
    vm.wait_for("inhibitor inactive").await;

    // Focus away and back gives the keys back.
    vm.clear();
    windows
        .focus(isle.wait_for_window("isle.test.Other").await.id)
        .await
        .unwrap();
    windows
        .focus(isle.wait_for_window("isle.test.Vm").await.id)
        .await
        .unwrap();
    vm.wait_for("inhibitor active").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn isle_types_into_a_window_untranslated_and_unshifted() {
    let mut isle = Isle::start("inject").await;
    let test = isle.test().await;
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    let target = isle.client("kitty", "3366ff");
    target.wait_for("keyboard-enter").await;
    let target_id = isle.wait_for_window("kitty").await.id;
    let other = isle.client("isle.test.Other", "ff6633");
    other.wait_for("keyboard-enter").await;
    target.clear();

    test.key(SHIFT, true).await.unwrap();
    test.key(CMD, true).await.unwrap();
    assert!(keyboard.type_text(target_id, "hé🙂").await.unwrap());
    target.wait_for("keyboard-enter").await;
    target.wait_for("sym h pressed").await;
    target.wait_for("sym eacute pressed").await;
    target.wait_for("sym U1F642 pressed").await;

    target.clear();
    assert!(keyboard.send_chord(target_id, "Cmd+V").await.unwrap());
    target.wait_for("sym V released").await;
    let lines = target.lines.lock().unwrap().clone();
    assert!(lines.contains(&format!("key {CTRL} pressed")), "{lines:?}");
    test.key(CMD, false).await.unwrap();
    test.key(SHIFT, false).await.unwrap();
    assert!(!keyboard.type_text(9999, "x").await.unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recording_returns_the_chord_and_who_holds_it() {
    let mut isle = Isle::start("record").await;
    let client = isle.client("isle.test.Settings", "3366ff");
    client.wait_for("keyboard-enter").await;
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();

    let recorded = {
        let keyboard = keyboard.clone();
        tokio::spawn(async move { keyboard.record().await.unwrap() })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    chord(&isle, &[SHIFT, CMD], K).await;
    let info = recorded.await.unwrap();
    assert_eq!(
        (info.chord.as_str(), info.holder.as_str()),
        ("Shift+Cmd+K", "")
    );

    let recorded = {
        let keyboard = keyboard.clone();
        tokio::spawn(async move { keyboard.record().await.unwrap() })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    chord(&isle, &[CMD], H).await;
    let info = recorded.await.unwrap();
    assert_eq!(
        (info.chord.as_str(), info.holder.as_str()),
        ("Cmd+H", "hide")
    );
    assert!(
        !isle.wait_for_window("isle.test.Settings").await.hidden,
        "Cmd+H was recorded, not run"
    );

    let recorded = {
        let keyboard = keyboard.clone();
        tokio::spawn(async move { keyboard.record().await.unwrap() })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    chord(&isle, &[], ESCAPE).await;
    assert!(recorded.await.unwrap().cancelled);

    let test = isle.test().await;
    let pc = test
        .add_keyboard("AT Translated Set 2 keyboard")
        .await
        .unwrap();
    let detected = {
        let keyboard = keyboard.clone();
        tokio::spawn(async move { keyboard.detect().await.unwrap() })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    test.key_on(&pc, 86, true).await.unwrap();
    test.key_on(&pc, 86, false).await.unwrap();
    let info = detected.await.unwrap();
    assert_eq!(info.keyboard, "AT Translated Set 2 keyboard");
    assert!(info.iso);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_x11_window_steam_labels_is_a_game() {
    let mut isle = Isle::start("games-x11").await;
    let game = isle
        .x11_client(&["--class", "isle.test.SteamX11", "--steam-game", "440"])
        .await;
    game.wait_for("focus-in").await;
    game_window(&isle, "isle.test.SteamX11").await;
}
