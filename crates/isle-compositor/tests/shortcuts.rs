//! The shortcut set: every chord reaches its action and no client, clashes are refused,
//! held chords are told of their release, the layout chord and repeating keys. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use isle_ipc::{keyboard::KeyboardProxy, settings::SettingsProxy};
use isle_keys::{chord::Chord, keyinfo::KeyInfo, keymap::KeymapSpec, xkb};

mod common;

use common::{Isle, wait_until};

const CMD: u32 = 125;
const OPTION: u32 = 56;
const CTRL: u32 = 29;
const SHIFT: u32 = 42;
const TAB: u32 = 15;
const LEFT: u32 = 105;
const RIGHT: u32 = 106;
const SPACE: u32 = 57;

/// The test keyboard's keymap: US with the Mac variant.
fn keys() -> KeyInfo {
    let spec = KeymapSpec {
        model: "pc104".into(),
        layout: "us".into(),
        variant: "mac".into(),
        options: String::new(),
    };
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    KeyInfo::new(&spec.compile(&context).unwrap())
}

/// The codes that type `chord`: its modifiers, then its key.
fn codes(keys: &KeyInfo, chord: &Chord) -> Option<(Vec<u32>, u32)> {
    let mut mods = Vec::new();
    for (on, code) in [
        (chord.mods.ctrl, CTRL),
        (chord.mods.option, OPTION),
        (chord.mods.shift, SHIFT),
        (chord.mods.cmd, CMD),
    ] {
        if on {
            mods.push(code);
        }
    }
    Some((mods, keys.code_for(chord.sym)?))
}

async fn press(isle: &Isle, mods: &[u32], key: u32) {
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

/// Shortcuts whose actions close, hide or move the window under test, lock or end the
/// session; their own tests drive those by their chords.
const DISRUPTIVE: &[&str] = &[
    "lock",
    "control-centre",
    "notification-centre",
    "menu-bar",
    "help-search",
    "window-menu",
    "close-window",
    "quit-app",
    "hide",
    "hide-others",
    "minimise",
    "minimise-app",
    "full-screen",
    "log-out-now",
    "next-display",
    "previous-display",
    "desktop-left",
    "desktop-right",
    "carry-left",
    "carry-right",
    "zoom-menu",
    "next-layout",
];

#[tokio::test(flavor = "multi_thread")]
async fn every_chord_in_the_set_reaches_its_action_and_no_client() {
    let mut isle = Isle::start("shortcuts-all").await;
    let client = isle.client("isle.test.Shortcuts", "3366ff");
    client.wait_for("keyboard-enter").await;
    let keys = keys();
    let shortcuts = KeyboardProxy::new(&isle.bus)
        .await
        .unwrap()
        .shortcuts()
        .await
        .unwrap();
    assert!(shortcuts.len() > 70, "the whole set is listed");
    let mut pressed = 0;
    for s in shortcuts
        .iter()
        .filter(|s| !DISRUPTIVE.contains(&s.id.as_str()))
    {
        for text in &s.chords {
            let chord = Chord::parse(text).unwrap();
            let Some((mods, key)) = codes(&keys, &chord) else {
                panic!("no key types {text} for {}", s.id);
            };
            client.clear();
            press(&isle, &mods, key).await;
            pressed += 1;
            let lines = client.lines.lock().unwrap().clone();
            assert!(
                !lines.iter().any(|l| l == &format!("key {key} pressed")),
                "{text} ({}) reached the client: {lines:?}",
                s.id
            );
        }
    }
    assert!(pressed > 50, "pressed {pressed}");
    // Cmd+Q is the desktop's even though Q is not, and a plain key still types.
    client.clear();
    press(&isle, &[], 16).await;
    client.wait_for("sym q pressed").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chord_bound_twice_is_refused_naming_both_and_the_old_set_stays() {
    let mut isle = Isle::start("shortcuts-clash").await;
    let settings = SettingsProxy::new(&isle.bus).await.unwrap();
    let err = settings
        .set("shortcuts.hide", &zvariant::Value::from(vec!["Cmd+Space"]))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("spotlight") && err.contains("hide"), "{err}");

    let file = isle.dir.join("config/settings.kdl");
    std::fs::write(
        &file,
        "shortcuts {\n    hide \"Cmd+H\"\n    minimise \"Cmd+H\"\n}\n",
    )
    .unwrap();
    wait_until(|| async { !settings.status().await.unwrap().ok }).await;
    let status = settings.status().await.unwrap();
    assert!(
        status.message.contains("hide") && status.message.contains("minimise"),
        "{}",
        status.message
    );
    assert!(
        [2, 3].contains(&status.line),
        "a binding's line: {}",
        status.line
    );

    // The defaults stay in force: Cmd+M still minimises.
    let client = isle.client("isle.test.Clash", "3366ff");
    client.wait_for("keyboard-enter").await;
    let window = isle.wait_for_window("isle.test.Clash").await;
    press(&isle, &[CMD], 50).await;
    wait_until(|| async { isle.window(window.id).await.minimised }).await;

    // An old `binds` block is ignored.
    std::fs::write(&file, "binds {\n    terminal \"Super+Return\"\n}\n").unwrap();
    wait_until(|| async { settings.status().await.unwrap().ok }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_switcher_s_chord_is_told_of_cmd_s_release_every_time() {
    let mut isle = Isle::start("shortcuts-held").await;
    let client = isle.client("isle.test.Held", "3366ff");
    client.wait_for("keyboard-enter").await;
    let test = isle.test().await;
    for attempt in 1..=100u32 {
        test.key(CMD, true).await.unwrap();
        for key in [TAB, TAB, LEFT, RIGHT] {
            test.key(key, true).await.unwrap();
            test.key(key, false).await.unwrap();
        }
        assert_eq!(
            test.held_releases().await.unwrap(),
            attempt - 1,
            "not before Cmd goes"
        );
        test.key(CMD, false).await.unwrap();
        assert_eq!(test.held_releases().await.unwrap(), attempt);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ctrl_space_is_the_app_s_with_one_layout_and_switches_with_two() {
    let mut isle = Isle::start("shortcuts-layout").await;
    let client = isle.client("isle.test.Layout", "3366ff");
    client.wait_for("keyboard-enter").await;
    press(&isle, &[CTRL], SPACE).await;
    client.wait_for("sym space pressed").await;

    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.layouts", &zvariant::Value::from(vec!["us", "de"]))
        .await
        .unwrap();
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        keyboard
            .status()
            .await
            .unwrap()
            .layout
            .starts_with("English")
    })
    .await;
    client.clear();
    press(&isle, &[CTRL], SPACE).await;
    wait_until(|| async {
        keyboard
            .status()
            .await
            .unwrap()
            .layout
            .starts_with("German")
    })
    .await;
    assert!(!client.printed("sym space pressed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn volume_up_held_repeats_and_stops_with_its_key() {
    let isle = Isle::start("shortcuts-repeat").await;
    SettingsProxy::new(&isle.bus)
        .await
        .unwrap()
        .set("keyboard.repeat-delay", &zvariant::Value::from(100i64))
        .await
        .unwrap();
    let test = isle.test().await;
    const VOLUME_UP: u32 = 115;
    let runs = |log: &str| log.matches("Media(VolumeUp)").count();
    test.key(VOLUME_UP, true).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    test.key(VOLUME_UP, false).await.unwrap();
    let held = runs(&isle.log());
    assert!(held >= 3, "ran {held} times while held");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(runs(&isle.log()), held, "it stopped with its key");
}
