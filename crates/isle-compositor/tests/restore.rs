//! Session restore: the record kept from the window list, frozen while the session ends, and
//! the apps it names brought back with each window on the desktop it left. Needs
//! `--features test-hooks`.
#![cfg(feature = "test-hooks")]

mod common;

use std::{fs, path::Path, time::Duration};

use common::{Isle, wait_until};
use isle_ipc::desktops::DesktopsProxy;

const RETURN: u32 = 28;
const Q: u32 = 16;
const SHIFT: u32 = 42;
const CMD: u32 = 125;

const OUTPUT: &str = "Isle Headless 1";

/// A desktop entry for `id` that starts the test client with `args`.
fn entry(dir: &Path, id: &str, args: &str) {
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    fs::write(
        dir.join(format!("data/applications/{id}.desktop")),
        format!(
            "[Desktop Entry]\nType=Application\nName={id}\nExec={client} --app-id {id} {args}\n"
        ),
    )
    .unwrap();
}

async fn status(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.restore_status().await.unwrap()).unwrap()
}

fn app_ids(status: &serde_json::Value) -> Vec<String> {
    status["record"]["apps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Restore held for the test, and Login Items read from the test's folders alone.
fn env(dir: &Path, more: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut env = vec![
        ("ISLE_RESTORE".to_owned(), "manual".to_owned()),
        (
            "XDG_CONFIG_HOME".to_owned(),
            dir.join("xdg-config").display().to_string(),
        ),
        (
            "XDG_CONFIG_DIRS".to_owned(),
            dir.join("xdg-none").display().to_string(),
        ),
    ];
    env.extend(more.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    env
}

async fn start(name: &str, settings: &str, prepare: impl FnOnce(&Path)) -> Isle {
    Isle::start_env(name, settings, prepare, |dir| env(dir, &[])).await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_record_keeps_apps_back_to_front_with_their_desktops_and_freezes_at_the_end() {
    let mut isle = start(
        "restore-record",
        "windows {\n    games \"isle.test.Game\"\n}\n",
        |dir| {
            entry(dir, "isle.test.A", "");
            entry(dir, "isle.test.B", "");
            entry(dir, "isle.test.Game", "");
        },
    )
    .await;
    assert!(isle.test().await.restore_now().await.unwrap().is_empty());
    let desktops = DesktopsProxy::new(&isle.bus).await.unwrap();
    assert_eq!(desktops.add("").await.unwrap(), 2);

    let a = isle.client_with(&[
        "--app-id",
        "isle.test.A",
        "--do",
        "window",
        "--do",
        "dialog",
    ]);
    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .filter(|w| w.app == "isle.test.A")
            .count()
            == 3
    })
    .await;
    let a_windows: Vec<_> = isle
        .windows()
        .await
        .into_iter()
        .filter(|w| w.app == "isle.test.A")
        .collect();
    let second = a_windows.iter().find(|w| w.title == "window 1").unwrap().id;
    let first = a_windows
        .iter()
        .find(|w| w.title == "test client")
        .unwrap()
        .id;
    assert!(desktops.move_window(second, 2).await.unwrap());
    let windows = isle.windows_proxy().await;
    assert!(windows.minimise(first).await.unwrap());

    let _no_entry = isle.client("isle.test.NoEntry", "22aa44");
    let _game = isle.client("isle.test.Game", "aa2244");
    isle.wait_for_window("isle.test.NoEntry").await;
    isle.wait_for_window("isle.test.Game").await;
    // In front of the game, whose window would take the chords.
    let b = isle.client("isle.test.B", "3366ff");
    b.wait_for("keyboard-enter").await;

    wait_until(|| async {
        let s = status(&isle).await;
        app_ids(&s) == ["isle.test.A", "isle.test.B"] && s["record"]["front"] == "isle.test.B"
    })
    .await;
    let s = status(&isle).await;
    let a_record = &s["record"]["apps"][0]["windows"];
    assert_eq!(
        a_record.as_array().unwrap().len(),
        2,
        "the dialog is left out: {s}"
    );
    assert_eq!(a_record[0]["output"], OUTPUT);
    assert_eq!(a_record[0]["desktop"], 0);
    assert_eq!(a_record[0]["minimised"], true);
    assert_eq!(a_record[1]["desktop"], 1);
    assert_eq!(s["record"]["apps"][1]["windows"][0]["desktop"], 1);

    // Written once it settles, and not again for a title.
    let file = isle.dir.join("state/isle/restore.json");
    wait_until(|| async { fs::read_to_string(&file).is_ok_and(|t| t.contains("isle.test.B")) })
        .await;
    let written = fs::metadata(&file).unwrap().modified().unwrap();
    b.send("title Another title");
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), written);

    // Log out freezes it as it was before the apps quit; a stop called off thaws it.
    b.send("keep-open");
    isle.chord(&[SHIFT, CMD, Q]).await;
    wait_until(|| async {
        let s: serde_json::Value =
            serde_json::from_str(&isle.test().await.session().await.unwrap()).unwrap();
        s["armed"] == true
    })
    .await;
    isle.chord(&[RETURN]).await;
    wait_until(|| async { status(&isle).await["frozen"] == true }).await;
    wait_until(|| async { a.exited() }).await;
    assert_eq!(
        app_ids(&status(&isle).await),
        ["isle.test.A", "isle.test.B"]
    );
    let on_disk: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(on_disk["data"]["apps"][0]["id"], "isle.test.A");
    wait_until(|| async {
        let s: serde_json::Value =
            serde_json::from_str(&isle.test().await.session().await.unwrap()).unwrap();
        s["stop"] == "open isle.test.B"
    })
    .await;
    isle.test()
        .await
        .session_act("show isle.test.B")
        .await
        .unwrap();
    wait_until(|| async { status(&isle).await["frozen"] == false }).await;
    wait_until(|| async { app_ids(&status(&isle).await) == ["isle.test.B"] }).await;
}

/// A record of `apps` as the state file holds it.
fn record(apps: serde_json::Value, front: &str, skip: bool) -> String {
    serde_json::json!({
        "version": 1,
        "data": { "apps": apps, "front": front, "skip": skip, "reopen_windows": !skip },
    })
    .to_string()
}

fn prepare_restore(dir: &Path, record: &str) {
    fs::create_dir_all(dir.join("state/isle")).unwrap();
    fs::write(dir.join("state/isle/restore.json"), record).unwrap();
    fs::write(
        dir.join("state/isle/desktops.json"),
        format!("{{\"version\":1,\"data\":{{\"counts\":{{\"{OUTPUT}\":3}}}}}}"),
    )
    .unwrap();
    entry(dir, "isle.test.A", "--do window");
    entry(dir, "isle.test.B", "");
    entry(dir, "isle.test.Item", "");
    fs::create_dir_all(dir.join("xdg-config/autostart")).unwrap();
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    fs::write(
        dir.join("xdg-config/autostart/isle.test.Item.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Item\nExec={client} --app-id isle.test.Item\n"
        ),
    )
    .unwrap();
}

async fn start_restoring(name: &str, record: String, more: &[(&str, &str)]) -> Isle {
    Isle::start_env(
        name,
        "",
        move |dir| prepare_restore(dir, &record),
        |dir| env(dir, more),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn restore_brings_apps_back_on_their_desktops_after_login_items() {
    let apps = serde_json::json!([
        { "id": "isle.test.Item", "windows": [{ "output": OUTPUT, "desktop": 0 }] },
        { "id": "isle.test.A", "windows": [
            { "output": OUTPUT, "desktop": 2 },
            { "output": "Gone Screen 9", "desktop": 7, "minimised": true },
        ] },
        { "id": "isle.test.B", "windows": [{ "output": OUTPUT, "desktop": 1 }] },
    ]);
    let isle = start_restoring("restore-back", record(apps, "isle.test.B", false), &[]).await;
    let restored = isle.test().await.restore_now().await.unwrap();
    assert_eq!(
        restored,
        ["isle.test.A", "isle.test.B"],
        "the Login Item's app is left to it"
    );

    wait_until(|| async {
        isle.windows()
            .await
            .iter()
            .filter(|w| w.app.starts_with("isle.test."))
            .count()
            == 4
    })
    .await;
    let windows = isle.windows().await;
    let find = |app: &str, title: &str| {
        windows
            .iter()
            .find(|w| w.app == app && w.title == title)
            .unwrap()
            .clone()
    };
    let a1 = find("isle.test.A", "test client");
    let a2 = find("isle.test.A", "window 1");
    let b = find("isle.test.B", "test client");
    let item = find("isle.test.Item", "test client");
    // The app maps its windows in its own order; each takes the next noted place. An
    // absent output gives the main screen, and the desktop is clamped to its row.
    assert_eq!((a1.desktop, a2.desktop), (3, 3), "{windows:?}");
    assert_eq!(a2.output, "HEADLESS-1");
    assert_eq!(
        [a1.minimised, a2.minimised].iter().filter(|m| **m).count(),
        1
    );
    assert_eq!(b.desktop, 2);
    assert!(b.focused, "the front app has focus");
    assert!(!a1.focused);
    assert!(!item.focused);

    // A restored app is told so.
    let environ = fs::read(format!("/proc/{}/environ", a1.pid)).unwrap();
    assert!(environ.split(|b| *b == 0).any(|v| v == b"ISLE_RESTORED=1"));

    // The record taken again keeps them as they came back, A's windows behind the Login
    // Item's, which was not restored.
    wait_until(|| async {
        let s = status(&isle).await;
        app_ids(&s) == ["isle.test.A", "isle.test.Item", "isle.test.B"]
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_skip_restores_nothing_and_is_cleared() {
    let apps = serde_json::json!([
        { "id": "isle.test.B", "windows": [{ "output": OUTPUT, "desktop": 0 }] },
    ]);
    let isle = start_restoring("restore-skip", record(apps, "isle.test.B", true), &[]).await;
    assert!(isle.test().await.restore_now().await.unwrap().is_empty());
    let s = status(&isle).await;
    assert_eq!(s["record"]["skip"], false);
    assert_eq!(
        s["record"]["reopen_windows"], false,
        "the checkbox as last left"
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!isle.windows().await.iter().any(|w| w.app == "isle.test.B"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_restores_despite_the_skip_but_a_safe_start_does_not() {
    let apps = serde_json::json!([
        { "id": "isle.test.B", "windows": [{ "output": OUTPUT, "desktop": 0 }] },
    ]);
    let restart = start_restoring(
        "restore-restart",
        record(apps.clone(), "isle.test.B", true),
        &[("ISLE_RESTART", "1")],
    )
    .await;
    assert_eq!(
        restart.test().await.restore_now().await.unwrap(),
        ["isle.test.B"]
    );
    restart.wait_for_window("isle.test.B").await;
    drop(restart);

    let safe = start_restoring(
        "restore-safe",
        record(apps, "isle.test.B", false),
        &[("ISLE_SAFE_START", "1")],
    )
    .await;
    assert!(safe.test().await.restore_now().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn noted_windows_wait_no_longer_than_their_app_maps_them() {
    let apps = serde_json::json!([
        { "id": "isle.test.B", "windows": [{ "output": OUTPUT, "desktop": 1 }] },
    ]);
    let mut isle =
        start_restoring("restore-pending", record(apps, "isle.test.B", false), &[]).await;
    assert_eq!(
        isle.test().await.restore_now().await.unwrap(),
        ["isle.test.B"]
    );
    isle.wait_for_window("isle.test.B").await;
    wait_until(|| async {
        status(&isle).await["pending"]
            .as_array()
            .unwrap()
            .is_empty()
    })
    .await;
    // B's next window is a new one: it opens on the desktop shown.
    let more = isle.client("isle.test.B", "22aa44");
    more.wait_for("keyboard-enter").await;
    let shown: Vec<_> = isle
        .windows()
        .await
        .into_iter()
        .filter(|w| w.app == "isle.test.B")
        .collect();
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(|w| w.desktop == 2), "{shown:?}");
}

async fn session(isle: &Isle) -> serde_json::Value {
    serde_json::from_str(&isle.test().await.session().await.unwrap()).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dialogs_checkbox_unticked_skips_the_next_sign_in() {
    let mut isle = start("restore-checkbox", "", |dir| entry(dir, "isle.test.B", "")).await;
    isle.test().await.restore_now().await.unwrap();
    let b = isle.client("isle.test.B", "3366ff");
    b.wait_for("keyboard-enter").await;

    isle.chord(&[SHIFT, CMD, Q]).await;
    wait_until(|| async { session(&isle).await["armed"] == true }).await;
    assert_eq!(session(&isle).await["reopen"], true, "checked as last left");
    isle.test().await.session_act("reopen false").await.unwrap();
    wait_until(|| async { session(&isle).await["reopen"] == false }).await;
    isle.keep_shot("session-reopen").await;
    isle.chord(&[RETURN]).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while isle.compositor_exit().is_none() {
        assert!(std::time::Instant::now() < deadline, "never exited");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let file = isle.dir.join("state/isle/restore.json");
    let on_disk: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(on_disk["data"]["skip"], true);
    assert_eq!(on_disk["data"]["reopen_windows"], false);
    assert_eq!(
        on_disk["data"]["apps"][0]["id"], "isle.test.B",
        "noted before it quit"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn reopen_apps_off_hides_the_checkbox_and_notes_nothing() {
    let isle = start(
        "restore-off",
        "windows {\n    reopen-apps #false\n}\n",
        |dir| entry(dir, "isle.test.B", ""),
    )
    .await;
    isle.test().await.restore_now().await.unwrap();
    let mut isle = isle;
    let b = isle.client("isle.test.B", "3366ff");
    b.wait_for("keyboard-enter").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(app_ids(&status(&isle).await).is_empty());
    isle.chord(&[SHIFT, CMD, Q]).await;
    wait_until(|| async { session(&isle).await["armed"] == true }).await;
    assert!(session(&isle).await["reopen"].is_null());
}

#[tokio::test(flavor = "multi_thread")]
async fn restored_windows_are_only_ever_on_their_own_desktops_and_launch_at_once() {
    let apps = serde_json::json!([
        { "id": "isle.test.A", "windows": [
            { "output": OUTPUT, "desktop": 2 },
            { "output": OUTPUT, "desktop": 2 },
        ] },
        { "id": "isle.test.B", "windows": [{ "output": OUTPUT, "desktop": 0 }] },
    ]);
    let isle = Isle::start_env(
        "restore-first-frame",
        "",
        move |dir| {
            prepare_restore(dir, &record(apps, "isle.test.B", false));
            entry(dir, "isle.test.A", "--colour 22aa44 --do window");
        },
        |dir| env(dir, &[]),
    )
    .await;
    let asked = std::time::Instant::now();
    let restored = isle.test().await.restore_now().await.unwrap();
    assert!(
        asked.elapsed() < Duration::from_secs(1),
        "every app launched within a second"
    );
    assert_eq!(restored, ["isle.test.A", "isle.test.B"]);

    // Every look at the window list while they map finds A's windows on desktop 3 only.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let windows = isle.windows().await;
        for w in windows.iter().filter(|w| w.app == "isle.test.A") {
            assert_eq!(w.desktop, 3, "{w:?}");
        }
        let a = windows.iter().filter(|w| w.app == "isle.test.A").count();
        if a == 2 && windows.iter().any(|w| w.app == "isle.test.B") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never mapped: {}",
            isle.log()
        );
    }
    // Desktop 1, the front app's, is shown, and nothing of A's is drawn on it.
    let shot = isle.shot("HEADLESS-1", "restored").await;
    for y in (0..shot.height).step_by(16) {
        for x in (0..shot.width).step_by(16) {
            assert_ne!(shot.at(x, y), [0x22, 0xaa, 0x44], "A drawn at {x},{y}");
        }
    }
}
