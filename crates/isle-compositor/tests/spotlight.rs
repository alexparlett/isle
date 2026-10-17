//! Spotlight: opening, the field, the keys and the surfaces it gives way to. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use common::{Isle, keys::*, wait_until};
use isle_ipc::session::SessionProxy;

const SPACE: u32 = 57;
const ESCAPE: u32 = 1;
const TAB: u32 = 15;
const BACKSPACE: u32 = 14;
const I: u32 = 23;
const E: u32 = 18;
const N: u32 = 49;
const ONE: u32 = 2;
const SLASH: u32 = 53;

/// The evdev code of a lowercase letter, digit or space on the US layout.
fn code(c: char) -> u32 {
    const ROWS: [(&str, u32); 4] = [
        ("1234567890", 2),
        ("qwertyuiop", 16),
        ("asdfghjkl", 30),
        ("zxcvbnm", 44),
    ];
    if c == ' ' {
        return SPACE;
    }
    ROWS.iter()
        .find_map(|(row, first)| row.find(c).map(|i| first + i as u32))
        .unwrap_or_else(|| panic!("no key for {c:?}"))
}

/// Cmd+Space and `text`.
async fn search(isle: &Isle, text: &str) {
    let keys: Vec<u32> = text.chars().map(code).collect();
    open_and_type(isle, &keys).await;
    wait_until(|| async { status(isle).await["text"] == text }).await;
}

async fn titles(isle: &Isle) -> Vec<String> {
    status(isle).await["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["title"].as_str().map(str::to_owned))
        .collect()
}

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.spotlight().await.unwrap()).unwrap()
}

async fn press(isle: &Isle, key: u32) {
    let test = isle.test().await;
    test.key(key, true).await.unwrap();
    test.key(key, false).await.unwrap();
}

/// Cmd+Space, then `keys` at once, with no wait between.
async fn open_and_type(isle: &Isle, keys: &[u32]) {
    let test = isle.test().await;
    test.key(SUPER, true).await.unwrap();
    test.key(SPACE, true).await.unwrap();
    test.key(SPACE, false).await.unwrap();
    test.key(SUPER, false).await.unwrap();
    for key in keys {
        test.key(*key, true).await.unwrap();
        test.key(*key, false).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_typed_with_the_chord_reach_the_field() {
    let mut isle = Isle::start("spotlight-keys").await;
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;
    open_and_type(&isle, &[F, I, R, E]).await;
    wait_until(|| async { status(&isle).await["text"] == "fire" }).await;
    window.wait_for("keyboard-leave").await;
    assert!(!window.printed("key 33"), "no key reached the window");

    press(&isle, BACKSPACE).await;
    wait_until(|| async { status(&isle).await["text"] == "fir" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["text"] == "" }).await;
    press(&isle, ESCAPE).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    window.wait_for("keyboard-enter").await;
    assert_eq!(
        status(&isle).await["kept"],
        serde_json::Value::Null,
        "an emptied query is not kept"
    );

    open_and_type(&isle, &[N, E]).await;
    wait_until(|| async { status(&isle).await["text"] == "ne" }).await;
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    assert_eq!(status(&isle).await["kept"], "ne");
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(&isle).await["text"] == "ne" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn scopes_come_from_prefixes_and_quick_keys() {
    let isle = Isle::start("spotlight-scopes").await;
    open_and_type(&isle, &[SLASH]).await;
    wait_until(|| async { status(&isle).await["scope"] == "Files { contents: false }" }).await;
    press(&isle, BACKSPACE).await;
    wait_until(|| async { status(&isle).await["scope"] == "General" }).await;
    isle.chord(&[SUPER, ONE]).await;
    wait_until(|| async { status(&isle).await["scope"] == "Apps" }).await;
    isle.test().await.spotlight_act("quick 1").await.unwrap();
    wait_until(|| async { status(&isle).await["scope"] == "General" }).await;
    press(&isle, TAB).await;
    assert_eq!(status(&isle).await["open"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_switcher_and_the_lock_close_it() {
    let mut isle = Isle::start("spotlight-closes").await;
    let window = isle.client("isle.test.Front", "3366ff");
    window.wait_for("keyboard-enter").await;
    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    isle.chord(&[SUPER, TAB]).await;
    wait_until(|| async { status(&isle).await["open"] == false }).await;

    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    SessionProxy::new(&isle.bus)
        .await
        .unwrap()
        .lock()
        .await
        .unwrap();
    wait_until(|| async { status(&isle).await["open"] == false }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prompt_takes_the_keys_and_gives_them_back() {
    let isle = Isle::start("spotlight-prompt").await;
    open_and_type(&isle, &[F]).await;
    wait_until(|| async { status(&isle).await["text"] == "f" }).await;
    let test = isle.test().await;
    let prompt = serde_json::json!({
        "kind": "polkit",
        "title": "Test needs your password",
        "message": "Authentication is needed to run a test.",
        "fields": [{"id": "password", "label": "Password for test", "secret": true}],
        "action": "Allow",
    });
    let id = test.prompt_ask(&prompt.to_string()).await.unwrap();
    wait_until(|| async {
        let p: serde_json::Value = serde_json::from_str(&test.prompts().await.unwrap()).unwrap();
        p["showing"] == id
    })
    .await;
    press(&isle, I).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(status(&isle).await["text"], "f", "the prompt has the keys");
    assert_eq!(status(&isle).await["open"], true, "and Spotlight stays");
    test.prompt_done(id).await.unwrap();
    wait_until(|| async {
        press(&isle, I).await;
        status(&isle).await["text"]
            .as_str()
            .is_some_and(|t| t.starts_with("fi"))
    })
    .await;
}

const RETURN: u32 = 28;

#[tokio::test(flavor = "multi_thread")]
async fn an_app_starts_from_its_name_and_is_raised_the_second_time() {
    let mut isle = Isle::start("spotlight-launch").await;
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    std::fs::write(
        isle.dir.join("data/applications/isle.test.Launched.desktop"),
        format!("[Desktop Entry]\nType=Application\nName=Launchable\nExec={client} --app-id isle.test.Launched --colour 22aa44\n"),
    )
    .unwrap();
    wait_until(|| async {
        search(&isle, "launcha").await;
        let found = titles(&isle).await.contains(&"Launchable".to_owned());
        if !found {
            press(&isle, ESCAPE).await;
            press(&isle, ESCAPE).await;
        }
        found
    })
    .await;
    assert_eq!(status(&isle).await["lines"][0]["header"], "Top hit");
    press(&isle, RETURN).await;
    let first = isle.wait_for_window("isle.test.Launched").await;
    wait_until(|| async { isle.window(first.id).await.focused }).await;

    let _other = isle.client("isle.test.Other", "3366ff");
    wait_until(|| async { !isle.window(first.id).await.focused }).await;
    search(&isle, "launcha").await;
    press(&isle, RETURN).await;
    wait_until(|| async { isle.window(first.id).await.focused }).await;
    let launched = isle
        .windows()
        .await
        .iter()
        .filter(|w| w.app_id == "isle.test.Launched")
        .count();
    assert_eq!(launched, 1, "raised, not started again");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_window_and_a_shell_action_are_found_and_done() {
    let mut isle = Isle::start("spotlight-windows").await;
    let a = isle.client_with(&[
        "--app-id",
        "isle.test.A",
        "--title",
        "quarterly report",
        "--colour",
        "3366ff",
    ]);
    a.wait_for("keyboard-enter").await;
    let wa = isle.wait_for_window("isle.test.A").await;
    let b = isle.client("isle.test.B", "22aa44");
    b.wait_for("keyboard-enter").await;
    search(&isle, "quarterly").await;
    assert!(titles(&isle).await.contains(&"quarterly report".to_owned()));
    press(&isle, RETURN).await;
    wait_until(|| async { isle.window(wa.id).await.focused }).await;

    search(&isle, "hide").await;
    let lines = status(&isle).await["lines"].clone();
    assert_eq!(lines[1]["title"], "Hide", "{lines}");
    assert_eq!(lines[1]["trailing"], "⌘H", "the action shows its chord");
    press(&isle, RETURN).await;
    wait_until(|| async { isle.window(wa.id).await.hidden }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_front_apps_menu_items_are_found_and_chosen() {
    let mut isle = Isle::start("spotlight-menus").await;
    let client = isle.client_with(&[
        "--app-id",
        "isle.test.Qt",
        "--colour",
        "3366ff",
        "--appmenu",
    ]);
    client.wait_for("menus exported").await;
    client.wait_for("keyboard-enter").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    search(&isle, "word wrap").await;
    let lines = status(&isle).await["lines"].clone();
    let found = lines
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["title"] == "Word Wrap")
        .cloned();
    let found = found.unwrap_or_else(|| panic!("{lines}"));
    assert_eq!(found["detail"], "Edit");
    client.clear();
    press(&isle, RETURN).await;
    client.wait_for("menu activated Word Wrap").await;
}

/// Desktop entries named `names`, each running the test client under its own app id.
fn entries(isle: &Isle, names: &[(&str, &str, &str)]) {
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    for (id, name, category) in names {
        std::fs::write(
            isle.dir.join(format!("data/applications/{id}.desktop")),
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nCategories={category};\nExec={client} --app-id {id} --colour 22aa44\n"
            ),
        )
        .unwrap();
    }
}

fn favourites(isle: &Isle) -> String {
    let text = std::fs::read_to_string(isle.dir.join("config/settings.kdl")).unwrap_or_default();
    text.lines()
        .find(|l| l.trim_start().starts_with("favourites"))
        .unwrap_or("")
        .trim()
        .to_owned()
}

async fn keys_listed(isle: &Isle) -> Vec<String> {
    status(isle).await["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["key"].as_str().map(str::to_owned))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn favourites_are_added_moved_and_removed_and_written() {
    let isle = Isle::start("spotlight-favourites").await;
    entries(
        &isle,
        &[
            ("isle.test.Alpha", "Alpha", "Utility"),
            ("isle.test.Beta", "Beta", "Network"),
            ("isle.test.Gamma", "Gamma", "Network"),
        ],
    );
    let test = isle.test().await;
    wait_until(|| async {
        isle.chord(&[SUPER, SPACE]).await;
        isle.chord(&[SUPER, ONE]).await;
        let found = keys_listed(&isle).await.len() >= 3;
        if !found {
            press(&isle, ESCAPE).await;
            press(&isle, ESCAPE).await;
        }
        found
    })
    .await;
    assert_eq!(status(&isle).await["scope"], "Apps");

    for key in ["app:isle.test.Gamma", "app:isle.test.Alpha"] {
        test.spotlight_act(&format!("secondary {key}"))
            .await
            .unwrap();
        wait_until(|| async {
            status(&isle).await["menu"]
                .as_array()
                .is_some_and(|m| m.iter().any(|l| l == "Add to favourites"))
        })
        .await;
        test.spotlight_act("menu Add to favourites").await.unwrap();
    }
    wait_until(|| async {
        favourites(&isle) == r#"favourites "isle.test.Gamma" "isle.test.Alpha""#
    })
    .await;
    let listed = keys_listed(&isle).await;
    assert_eq!(
        &listed[..2],
        ["fav:app:isle.test.Gamma", "fav:app:isle.test.Alpha"],
        "{listed:?}"
    );

    test.spotlight_act("drag fav:app:isle.test.Alpha fav:app:isle.test.Gamma")
        .await
        .unwrap();
    wait_until(|| async {
        favourites(&isle) == r#"favourites "isle.test.Alpha" "isle.test.Gamma""#
    })
    .await;
    test.spotlight_act("drag app:isle.test.Beta fav:app:isle.test.Gamma")
        .await
        .unwrap();
    wait_until(|| async {
        favourites(&isle) == r#"favourites "isle.test.Alpha" "isle.test.Beta" "isle.test.Gamma""#
    })
    .await;
    test.spotlight_act("drag fav:app:isle.test.Alpha app:isle.test.Gamma")
        .await
        .unwrap();
    wait_until(|| async {
        favourites(&isle) == r#"favourites "isle.test.Beta" "isle.test.Gamma""#
    })
    .await;

    test.spotlight_act("filter").await.unwrap();
    wait_until(|| async {
        status(&isle).await["menu"]
            .as_array()
            .is_some_and(|m| m.iter().any(|l| l == "Internet"))
    })
    .await;
    test.spotlight_act("menu Internet").await.unwrap();
    wait_until(|| async { status(&isle).await["category"] == "Internet" }).await;
    let listed = keys_listed(&isle).await;
    assert_eq!(
        listed,
        ["app:isle.test.Beta", "app:isle.test.Gamma"],
        "a category alone, no favourites"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_badge_shows_on_its_tile() {
    let mut isle = Isle::start("spotlight-badge").await;
    entries(&isle, &[("isle.test.Badged", "Badged", "Utility")]);
    let client = isle.client("isle.test.Badged", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("badge count=7");
    client.wait_for("badge sent").await;
    wait_until(|| async {
        isle.chord(&[SUPER, SPACE]).await;
        isle.chord(&[SUPER, ONE]).await;
        let badge = status(&isle).await["badges"]["app:isle.test.Badged"].clone();
        if badge != 7 {
            press(&isle, ESCAPE).await;
            press(&isle, ESCAPE).await;
        }
        badge == 7
    })
    .await;
}

/// Opens Spotlight in `prefix`'s scope and types `text`.
async fn scoped_search(isle: &Isle, prefix: u32, text: &str) {
    let mut keys = vec![prefix];
    keys.extend(text.chars().map(code));
    open_and_type(isle, &keys).await;
    wait_until(|| async { status(isle).await["text"] == text }).await;
}

async fn found_in(isle: &Isle, prefix: u32, text: &str, title: &str) -> bool {
    scoped_search(isle, prefix, text).await;
    let mut seen = false;
    for _ in 0..40 {
        if titles(isle).await.iter().any(|t| t == title) {
            seen = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    isle.chord(&[SUPER, SPACE]).await;
    wait_until(|| async { status(isle).await["open"] == false }).await;
    seen
}

const QUESTION_SHIFT: u32 = 53;

#[tokio::test(flavor = "multi_thread")]
async fn files_are_found_by_name_live_and_by_content() {
    let isle = Isle::start_prepared("spotlight-files", "", |dir| {
        let home = dir.join("home");
        std::fs::create_dir_all(home.join("notes/work")).unwrap();
        std::fs::create_dir_all(home.join(".hidden")).unwrap();
        std::fs::write(home.join("notes/plan.md"), "the plan\nquarterly numbers\n").unwrap();
        std::fs::write(home.join(".hidden/planb.md"), "").unwrap();
    })
    .await;
    wait_until(|| async { found_in(&isle, SLASH, "plan", "plan.md").await }).await;
    assert!(
        !found_in(&isle, SLASH, "planb", "planb.md").await,
        "hidden folders are left out"
    );

    let home = isle.dir.join("home");
    std::fs::create_dir_all(home.join("notes/fresh")).unwrap();
    std::fs::write(home.join("notes/fresh/minutes.txt"), "").unwrap();
    wait_until(|| async { found_in(&isle, SLASH, "minutes", "minutes.txt").await }).await;
    std::fs::remove_dir_all(home.join("notes/fresh")).unwrap();
    wait_until(|| async { !found_in(&isle, SLASH, "minutes", "minutes.txt").await }).await;

    // ? is Shift+/ on the US layout.
    let test = isle.test().await;
    open_and_type(&isle, &[]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    test.key(SHIFT, true).await.unwrap();
    press(&isle, QUESTION_SHIFT).await;
    test.key(SHIFT, false).await.unwrap();
    wait_until(|| async { status(&isle).await["scope"] == "Files { contents: true }" }).await;
    for c in "quarterly".chars() {
        press(&isle, code(c)).await;
    }
    wait_until(|| async {
        let lines = status(&isle).await["lines"].clone();
        lines.as_array().unwrap().iter().any(|l| {
            l["title"] == "plan.md"
                && l["detail"]
                    .as_str()
                    .is_some_and(|d| d.starts_with("quarterly numbers"))
        })
    })
    .await;
}

const SHIFT_PERIOD_GT: u32 = 52;

#[tokio::test(flavor = "multi_thread")]
async fn a_sum_is_answered_and_copied_and_the_web_row_comes_last() {
    let isle = Isle::start("spotlight-calc").await;
    // 12*48: * is Shift+8 on the US layout.
    let test = isle.test().await;
    open_and_type(&isle, &[code('1'), code('2')]).await;
    test.key(SHIFT, true).await.unwrap();
    press(&isle, code('8')).await;
    test.key(SHIFT, false).await.unwrap();
    for c in "48".chars() {
        press(&isle, code(c)).await;
    }
    wait_until(|| async { status(&isle).await["text"] == "12*48" }).await;
    wait_until(|| async { status(&isle).await["lines"][1]["title"] == "576" }).await;
    let lines = status(&isle).await["lines"].clone();
    assert_eq!(lines[0]["header"], "Calculation");
    assert_eq!(
        lines.as_array().unwrap().last().unwrap()["title"],
        "Search the web for “12*48”"
    );
    press(&isle, RETURN).await;
    wait_until(|| async { test.clipboard().await.unwrap() == "576" }).await;
    assert_eq!(status(&isle).await["open"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_command_runs_and_is_kept() {
    let isle = Isle::start("spotlight-run").await;
    let test = isle.test().await;
    open_and_type(&isle, &[]).await;
    wait_until(|| async { status(&isle).await["open"] == true }).await;
    test.key(SHIFT, true).await.unwrap();
    press(&isle, SHIFT_PERIOD_GT).await;
    test.key(SHIFT, false).await.unwrap();
    wait_until(|| async { status(&isle).await["scope"] == "Run" }).await;
    for c in "touch ran".chars() {
        press(&isle, code(c)).await;
    }
    wait_until(|| async { status(&isle).await["lines"][0]["title"] == "touch ran" }).await;
    press(&isle, RETURN).await;
    let ran = isle.dir.join("home/ran");
    wait_until(|| async { ran.exists() }).await;
    let commands = isle.dir.join("state/isle/commands.json");
    wait_until(|| async {
        std::fs::read_to_string(&commands).is_ok_and(|t| t.contains("touch ran"))
    })
    .await;
}

/// Ctrl+Cmd+Space, then `name`, then Return.
async fn pick_emoji(isle: &Isle, name: &str) {
    let test = isle.test().await;
    test.key(CTRL, true).await.unwrap();
    isle.chord(&[SUPER, SPACE]).await;
    test.key(CTRL, false).await.unwrap();
    wait_until(|| async { status(isle).await["scope"] == "Emoji" }).await;
    for c in name.chars() {
        press(isle, code(c)).await;
    }
    wait_until(|| async { status(isle).await["text"] == name }).await;
    wait_until(|| async {
        status(isle).await["selected"]
            .as_str()
            .is_some_and(|s| s.starts_with("emoji:"))
    })
    .await;
    press(isle, RETURN).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_emoji_is_pasted_and_the_clipboard_comes_back() {
    let mut isle = Isle::start("spotlight-emoji").await;
    let client = isle.client("isle.test.Typing", "3366ff");
    client.wait_for("keyboard-enter").await;
    client.send("copy before");
    client.wait_for("copied").await;
    pick_emoji(&isle, "partying face").await;
    client.wait_for("pasted 🥳").await;
    let test = isle.test().await;
    wait_until(|| async { test.clipboard().await.unwrap() == "before" }).await;

    client.clear();
    pick_emoji(&isle, "people holding hands").await;
    client.wait_for("pasted 🧑\u{200d}🤝\u{200d}🧑").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_emoji_reaches_an_x11_window() {
    let mut isle = Isle::start("spotlight-emoji-x11").await;
    let client = isle
        .x11_client(&["--class", "isle.test.X11", "--colour", "ff0000"])
        .await;
    client.wait_for("focus-in").await;
    pick_emoji(&isle, "partying face").await;
    client.wait_for("pasted 🥳").await;
}

const FOUR: u32 = 5;
const P: u32 = 25;
const Y: u32 = 21;
const DELETE: u32 = 111;

async fn clip_titles(isle: &Isle) -> Vec<(String, String)> {
    status(isle).await["lines"]
        .as_array()
        .unwrap()
        .iter()
        .scan(String::new(), |section, l| {
            if let Some(h) = l["header"].as_str() {
                *section = h.to_owned();
                return Some(None);
            }
            Some(l["title"].as_str().map(|t| (section.clone(), t.to_owned())))
        })
        .flatten()
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn copies_are_kept_secrets_are_not_and_a_pick_pastes() {
    let mut isle = Isle::start("spotlight-clipboard").await;
    let a = isle.client("isle.test.Copier", "3366ff");
    a.wait_for("keyboard-enter").await;
    let index = isle.dir.join("state/isle/clipboard/index.json");
    let kept = |text: &'static str| {
        let index = index.clone();
        async move { std::fs::read_to_string(&index).is_ok_and(|t| t.contains(text)) }
    };
    a.send("copy first copy");
    wait_until(|| kept("first copy")).await;
    a.send("copy-secret hunter2");
    a.wait_for("copied").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    a.send("copy second copy");
    wait_until(|| kept("second copy")).await;
    let kept = std::fs::read_to_string(&index).unwrap();
    assert!(
        kept.contains("first copy") && !kept.contains("hunter2"),
        "{kept}"
    );

    open_and_type(&isle, &[]).await;
    isle.chord(&[SUPER, FOUR]).await;
    wait_until(|| async { status(&isle).await["scope"] == "Clipboard" }).await;
    wait_until(|| async { clip_titles(&isle).await.len() == 2 }).await;
    assert_eq!(
        clip_titles(&isle).await,
        [
            ("Recent".to_owned(), "second copy".to_owned()),
            ("Recent".to_owned(), "first copy".to_owned())
        ]
    );
    // Pin the older one; it leads under Pinned.
    press(&isle, 108).await;
    isle.chord(&[SUPER, P]).await;
    wait_until(|| async {
        clip_titles(&isle).await.first() == Some(&("Pinned".to_owned(), "first copy".to_owned()))
    })
    .await;
    // Cmd+Y shows it whole; Escape puts the list back.
    isle.chord(&[SUPER, Y]).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    press(&isle, ESCAPE).await;
    assert_eq!(
        status(&isle).await["scope"],
        "Clipboard",
        "Escape closed the preview, not the scope"
    );

    // Return pastes the pinned one into the window that had focus.
    a.clear();
    press(&isle, RETURN).await;
    a.wait_for("pasted first copy").await;

    open_and_type(&isle, &[]).await;
    isle.chord(&[SUPER, FOUR]).await;
    wait_until(|| async { clip_titles(&isle).await.len() == 2 }).await;
    press(&isle, 108).await;
    isle.chord(&[SUPER, DELETE]).await;
    wait_until(|| async { clip_titles(&isle).await.len() == 1 }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_dragged_out_arrives_as_its_link() {
    let mut isle = Isle::start_prepared("spotlight-drag", "", |dir| {
        std::fs::create_dir_all(dir.join("home/notes")).unwrap();
        std::fs::write(dir.join("home/notes/plan.md"), "the plan").unwrap();
    })
    .await;
    let target = isle.client("isle.test.Target", "3366ff");
    target.wait_for("keyboard-enter").await;
    let w = isle.wait_for_window("isle.test.Target").await;
    wait_until(|| async { found_in(&isle, SLASH, "plan", "plan.md").await }).await;
    scoped_search(&isle, SLASH, "plan").await;
    wait_until(|| async { titles(&isle).await.iter().any(|t| t == "plan.md") }).await;
    let test = isle.test().await;
    // The first row: under the panel's padding and its first header.
    test.pointer_to(960.0, 352.0).await.unwrap();
    test.button(272, true).await.unwrap();
    for x in [800.0, 500.0, 200.0, 100.0] {
        test.pointer_to(x, 352.0).await.unwrap();
    }
    wait_until(|| async { status(&isle).await["open"] == false }).await;
    let (cx, cy) = ((w.x + w.width / 2) as f64, (w.y + w.height / 2) as f64);
    test.pointer_to(cx - 10.0, cy).await.unwrap();
    test.pointer_to(cx, cy).await.unwrap();
    target.wait_for("dnd enter").await;
    test.button(272, false).await.unwrap();
    let link = format!(
        "dropped file://{}",
        isle.dir.join("home/notes/plan.md").display()
    );
    target.wait_for(&link).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn search_runs_without_spotlight_and_tab_scopes_to_an_app() {
    let mut isle = Isle::start("spotlight-cli").await;
    entries(&isle, &[("isle.test.Editor", "Editor", "Development")]);
    let editor = isle.client_with(&[
        "--app-id",
        "isle.test.Editor",
        "--title",
        "draft notes",
        "--colour",
        "3366ff",
    ]);
    editor.wait_for("keyboard-enter").await;
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async {
        let found: serde_json::Value =
            serde_json::from_str(&shell.search("edit", "general").await.unwrap()).unwrap();
        found
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["title"] == "Editor" && r["group"] == "Apps")
    })
    .await;
    let found: serde_json::Value =
        serde_json::from_str(&shell.search("draft", "windows").await.unwrap()).unwrap();
    assert_eq!(found[0]["title"], "draft notes");
    assert!(shell.search("x", "nowhere").await.is_err());
    assert_eq!(status(&isle).await["open"], false, "nothing opened");

    assert!(shell.spotlight("apps").await.unwrap());
    assert_eq!(status(&isle).await["scope"], "Apps");
    assert!(!shell.spotlight("close").await.unwrap());

    search(&isle, "editor").await;
    wait_until(|| async { status(&isle).await["selected"] == "app:isle.test.Editor" }).await;
    press(&isle, TAB).await;
    wait_until(|| async { status(&isle).await["scope"] == "App(\"isle.test.Editor\")" }).await;
    wait_until(|| async { titles(&isle).await.contains(&"draft notes".to_owned()) }).await;
    press(&isle, BACKSPACE).await;
    wait_until(|| async { status(&isle).await["text"] == "editor" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_source_turned_off_leaves_general_search_and_keeps_its_scope() {
    let isle = Isle::start("spotlight-sources-off").await;
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    let calculates = |json: String| {
        let found: serde_json::Value = serde_json::from_str(&json).unwrap();
        found.as_array().unwrap().iter().any(|r| r["group"] == "Calculation")
    };
    assert!(calculates(shell.search("2+2", "general").await.unwrap()));
    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus).await.unwrap();
    let off = zvariant::Value::from(vec!["calculator".to_owned()]);
    settings.set("spotlight.sources-off", &off).await.unwrap();
    wait_until(|| async { !calculates(shell.search("2+2", "general").await.unwrap()) }).await;
    assert!(calculates(shell.search("2+2", "calculator").await.unwrap()));
}

/// Writes Spotlight's look to `$ISLE_SHOT_DIR` for a person to see; run by name.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn look() {
    let Some(dir) = std::env::var_os("ISLE_SHOT_DIR") else {
        return;
    };
    let mut isle = Isle::start("spotlight-look").await;
    for (id, name) in [
        ("org.mozilla.firefox", "Firefox"),
        ("org.gnome.Nautilus", "Files"),
        ("fish", "Fish"),
    ] {
        std::fs::write(
            isle.dir.join(format!("data/applications/{id}.desktop")),
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec=true\nIcon={id}\nActions=new;\n\n[Desktop Action new]\nName=New Window\nExec=true\n"),
        )
        .unwrap();
    }
    let _w = isle.client_with(&[
        "--app-id",
        "isle.test.A",
        "--title",
        "final draft",
        "--colour",
        "3366ff",
    ]);
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    search(&isle, "fi").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let test = isle.test().await;
    let path = std::path::Path::new(&dir).join("spotlight-field.png");
    test.screenshot("HEADLESS-1", path.to_str().unwrap())
        .await
        .unwrap();
    isle.chord(&[SUPER, ONE]).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    test.spotlight_act("secondary app:org.mozilla.firefox")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    test.screenshot(
        "HEADLESS-1",
        std::path::Path::new(&dir)
            .join("spotlight-apps.png")
            .to_str()
            .unwrap(),
    )
    .await
    .unwrap();
    press(&isle, ESCAPE).await;
    press(&isle, ESCAPE).await;
    press(&isle, ESCAPE).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    test.key(CTRL, true).await.unwrap();
    isle.chord(&[SUPER, SPACE]).await;
    test.key(CTRL, false).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let emoji = std::path::Path::new(&dir).join("spotlight-emoji.png");
    test.screenshot("HEADLESS-1", emoji.to_str().unwrap())
        .await
        .unwrap();
}
