//! The translation tables end to end: every shipped rule reaches an app it applies to as
//! the engine sends it, close and quit go to an app's own chords, untranslated apps get raw
//! keys, and a broken user file leaves the shipped tables in force. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

use std::collections::BTreeMap;

use isle_ipc::keyboard::KeyboardProxy;
use isle_keys::{
    chord::Chord,
    engine::{Ctx, Engine, Out},
    keyinfo::KeyInfo,
    keymap::KeymapSpec,
    tables::Tables,
    xkb,
};

mod common;

use common::{Isle, wait_until};

const CMD: u32 = 125;
const OPTION: u32 = 56;
const CTRL: u32 = 29;
const SHIFT: u32 = 42;

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

fn mod_codes(chord: &Chord) -> Vec<u32> {
    [
        (chord.mods.ctrl, CTRL),
        (chord.mods.option, OPTION),
        (chord.mods.shift, SHIFT),
        (chord.mods.cmd, CMD),
    ]
    .into_iter()
    .filter_map(|(on, code)| on.then_some(code))
    .collect()
}

/// The key lines a client should print for `chord` in an app known by `names`, from the
/// engine run here on the same keymap and tables.
fn expected(keys: &KeyInfo, tables: &Tables, names: &[&str], chord: &Chord) -> Vec<String> {
    let app = tables.for_app(names);
    let ctx = Ctx {
        keys,
        translate: true,
        option_level3: true,
        rules: &app.rules,
    };
    let mut engine = Engine::default();
    let mods = mod_codes(chord);
    let key = keys.code_for(chord.sym).expect("a key for the chord");
    let mut outs: Vec<Out<()>> = Vec::new();
    for m in &mods {
        outs.extend(engine.press(*m, &ctx, |_| None));
    }
    outs.extend(engine.press(key, &ctx, |_| None));
    outs.extend(engine.release(key, &ctx));
    for m in mods.iter().rev() {
        outs.extend(engine.release(*m, &ctx));
    }
    assert!(
        engine.sent().is_empty(),
        "the engine left keys held for {chord}"
    );
    outs.into_iter()
        .filter_map(|o| match o {
            Out::Press(c) => Some(format!("key {c} pressed")),
            Out::Release(c) => Some(format!("key {c} released")),
            Out::Action(()) => None,
        })
        .collect()
}

async fn type_chord(isle: &Isle, keys: &KeyInfo, chord: &Chord) {
    let test = isle.test().await;
    let mods = mod_codes(chord);
    let key = keys.code_for(chord.sym).unwrap();
    for m in &mods {
        test.key(*m, true).await.unwrap();
    }
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
    for m in mods.iter().rev() {
        test.key(*m, false).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn every_shipped_rule_reaches_an_app_it_applies_to() {
    let mut isle = Isle::start("translation-all").await;
    let keys = keys();
    let tables = Tables::shipped();
    let mut by_app: BTreeMap<String, Vec<Chord>> = BTreeMap::new();
    for rule in tables.list() {
        let app = rule.app.unwrap_or("isle.test.Plain").to_owned();
        by_app.entry(app).or_default().push(*rule.chord);
    }
    let mut checked = 0;
    for (app, chords) in by_app {
        let client = isle.client(&app, "3366ff");
        client.wait_for("keyboard-enter").await;
        for chord in chords {
            let want = expected(&keys, &tables, &[app.as_str()], &chord);
            client.clear();
            type_chord(&isle, &keys, &chord).await;
            client.wait_for(want.last().unwrap()).await;
            let got: Vec<String> = client
                .lines
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.starts_with("key "))
                .cloned()
                .collect();
            assert_eq!(got, want, "{chord} in {app}");
            checked += 1;
        }
        drop(client);
    }
    assert_eq!(checked, tables.list().len(), "every rule was typed");
}

#[tokio::test(flavor = "multi_thread")]
async fn close_and_quit_go_to_an_app_s_own_chords() {
    let mut isle = Isle::start("translation-delegates").await;
    let keys = keys();
    let browser = isle.client("chromium", "3366ff");
    browser.wait_for("keyboard-enter").await;
    browser.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+W").unwrap()).await;
    browser.wait_for("sym w released").await;
    assert!(browser.printed(&format!("key {CTRL} pressed")));
    assert!(!browser.printed("closed"), "the browser closes its own tab");

    let firefox = isle.client("org.mozilla.firefox", "ff6633");
    firefox.wait_for("keyboard-enter").await;
    firefox.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+Q").unwrap()).await;
    firefox.wait_for("sym q released").await;

    let plain = isle.client("isle.test.Plain", "33ff66");
    plain.wait_for("keyboard-enter").await;
    plain.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+W").unwrap()).await;
    plain.wait_for("closed").await;
    assert!(!plain.printed("sym w pressed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_untranslated_app_gets_cmd_as_super() {
    let mut isle = Isle::start("translation-raw").await;
    let keys = keys();
    let viewer = isle.client("org.remmina.Remmina", "3366ff");
    viewer.wait_for("keyboard-enter").await;
    viewer.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+C").unwrap()).await;
    viewer.wait_for("sym c released").await;
    assert!(viewer.printed(&format!("key {CMD} pressed")));
    assert!(!viewer.printed(&format!("key {CTRL} pressed")));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_user_file_is_refused_and_a_good_one_lies_over_the_shipped() {
    let mut isle = Isle::start("translation-user").await;
    let keys = keys();
    let keyboard = KeyboardProxy::new(&isle.bus).await.unwrap();
    let file = isle.dir.join("config/translation.kdl");
    std::fs::write(
        &file,
        "version 1\ngroup \"terminals\" {\n    \"Cmd+Nope\" \"Home\"\n}\n",
    )
    .unwrap();
    wait_until(|| async { !keyboard.tables_status().await.unwrap().ok }).await;
    let status = keyboard.tables_status().await.unwrap();
    assert_eq!(status.line, 3, "{}", status.message);

    let kitty = isle.client("kitty", "3366ff");
    kitty.wait_for("keyboard-enter").await;
    kitty.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+C").unwrap()).await;
    kitty.wait_for("sym C released").await;

    std::fs::write(
        &file,
        "version 1\ngroup \"terminals\" {\n    \"Cmd+K\" \"Ctrl+L\"\n}\n",
    )
    .unwrap();
    wait_until(|| async { keyboard.tables_status().await.unwrap().ok }).await;
    kitty.clear();
    type_chord(&isle, &keys, &Chord::parse("Cmd+K").unwrap()).await;
    kitty.wait_for("sym l released").await;
    assert_eq!(
        keyboard.mac_chord("kitty", "Ctrl+Shift+V").await.unwrap(),
        "Cmd+V"
    );
}
