//! Software Update: the check by source, a run through a fake paru with its progress, its
//! notes and Stop, and a restart owed in the island, the system menu and the note.
#![cfg(feature = "test-hooks")]

mod common;

use std::path::Path;

use common::{Isle, wait_until};
use isle_ipc::{menus::MenusProxy, notifications::NotificationsProxy, updates::UpdatesProxy};
use serde_json::Value;

const CTRL: u32 = 29;
const F2: u32 = 60;

/// A paru that lists one AUR update and, for a run, does what `mode` in its folder says.
const PARU: &str = r#"#!/bin/sh
here=$(dirname "$0")
if [ "$1" = "-Qua" ]; then echo "xremap-hypr-bin 0.15.9-1 -> 0.15.15-1"; exit 0; fi
mode=$(cat "$here/mode" 2>/dev/null)
echo ':: Synchronizing package databases...'
case "$mode" in
  refuse) echo 'Error executing command as another user: Request dismissed'; exit 1;;
  lock) echo 'error: failed to synchronize all databases (unable to lock database)'; exit 1;;
esac
echo 'Packages (2) alpha-1.0-1  beta-2.0-1'
echo ':: Processing package changes...'
[ "$mode" = slow ] && sleep 30
sleep 1
echo 'upgrading alpha...'
sleep 1
echo 'upgrading beta...'
echo ':: Running post-transaction hooks...'
exit 0
"#;

const CHECKUPDATES: &str =
    "#!/bin/sh\necho 'glibc 2.44-1 -> 2.44-2'\necho 'mesa 1:26.2.2-2 -> 1:26.2.6-1'\nexit 0\n";

fn script(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Isle's source as a git repository a commit ahead of the installed package.
fn isle_source(dir: &Path) {
    let src = dir.join("isle-src");
    std::fs::create_dir_all(&src).unwrap();
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(&src)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .unwrap()
            .success();
        assert!(ok);
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(src.join("f"), "1").unwrap();
    git(&["add", "f"]);
    git(&["commit", "-qm", "one"]);
    std::fs::create_dir_all(dir.join("root/var/lib/pacman/local/isle-git-r1.g000000000000-1")).unwrap();
}

async fn start(name: &str, owed: bool) -> Isle {
    Isle::start_env(
        name,
        "wallpaper {\n    source \"#1c1c1f\"\n}\n",
        |dir| {
            std::fs::create_dir_all(dir.join("bin")).unwrap();
            script(&dir.join("bin/paru"), PARU);
            script(&dir.join("bin/checkupdates"), CHECKUPDATES);
            isle_source(dir);
            if owed {
                // The running kernel's modules are gone: the kernel was updated.
                std::fs::create_dir_all(dir.join("root/usr/lib/modules/0.0.0-new")).unwrap();
            }
        },
        move |dir| {
            let cli = Path::new(env!("CARGO_BIN_EXE_isle-compositor")).with_file_name("isle");
            let mut env = vec![
                (
                    "ISLE_PARU".into(),
                    dir.join("bin/paru").display().to_string(),
                ),
                (
                    "ISLE_CHECKUPDATES".into(),
                    dir.join("bin/checkupdates").display().to_string(),
                ),
                ("ISLE_CLI".into(), cli.display().to_string()),
                ("ISLE_UPDATE_DIRECT".into(), "1".into()),
                (
                    "ISLE_UPDATES_ROOT".into(),
                    dir.join("root").display().to_string(),
                ),
                (
                    "ISLE_UPDATES_SOURCE".into(),
                    format!("file://{}#branch=main", dir.join("isle-src").display()),
                ),
            ];
            env.push(("PATH".into(), std::env::var("PATH").unwrap_or_default()));
            env
        },
    )
    .await
}

async fn status(isle: &Isle) -> Value {
    let text = UpdatesProxy::new(&isle.bus)
        .await
        .unwrap()
        .status()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

async fn notes(isle: &Isle) -> Vec<Value> {
    let text = NotificationsProxy::new(&isle.bus)
        .await
        .unwrap()
        .list()
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    v["entries"].as_array().cloned().unwrap_or_default()
}

async fn note(isle: &Isle, summary: &str) -> Option<Value> {
    notes(isle)
        .await
        .into_iter()
        .find(|e| e["summary"] == summary)
}

async fn system_menu(isle: &Isle) -> Value {
    if bar(isle).await["open"] != 0 {
        isle.chord(&[CTRL, F2]).await;
        wait_until(|| async { bar(isle).await["open"] == 0 }).await;
    }
    bar(isle).await["titles"][0].clone()
}

async fn bar(isle: &Isle) -> Value {
    let text = MenusProxy::new(&isle.bus)
        .await
        .unwrap()
        .bar()
        .await
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn item<'a>(menu: &'a Value, label: &str) -> Option<&'a Value> {
    menu["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["label"] == label)
}

async fn install(isle: &Isle, mode: &str) -> String {
    std::fs::write(isle.dir.join("bin/mode"), mode).unwrap();
    UpdatesProxy::new(&isle.bus)
        .await
        .unwrap()
        .install()
        .await
        .unwrap()
}

async fn state(isle: &Isle) -> String {
    status(isle).await["progress"]["state"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_check_lists_each_source_and_the_menu_counts_them() {
    let isle = start("updates-check", false).await;
    UpdatesProxy::new(&isle.bus)
        .await
        .unwrap()
        .check()
        .await
        .unwrap();
    let s = status(&isle).await;
    let sources: Vec<&str> = s["pending"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["source"].as_str().unwrap())
        .collect();
    assert_eq!(
        sources,
        ["Repositories", "Repositories", "Aur", "Isle"],
        "{s}"
    );
    let isle_update = &s["pending"][3];
    assert_eq!(isle_update["name"], "isle-git");
    assert_eq!(isle_update["to"].as_str().unwrap().len(), 12);

    let system = system_menu(&isle).await;
    let update = item(&system, "Software Update…").unwrap();
    assert_eq!(
        (update["enabled"].clone(), update["badge"].clone()),
        (Value::Bool(true), Value::from("4"))
    );
    assert!(item(&system, "Restart to finish updating…").is_none());

    isle.open_panel(None).await;
    wait_until(|| async { isle.module("updates").await["state"] == "4 updates" }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_counts_its_packages_shows_its_glyph_and_says_when_it_is_done() {
    let isle = start("updates-run", false).await;
    assert_eq!(install(&isle, "ok").await, "");
    wait_until(|| async {
        let s = status(&isle).await;
        s["progress"]["state"] == "Running"
            && s["progress"]["done"] == 1
            && s["progress"]["total"] == 2
    })
    .await;
    assert!(
        isle.glyphs().await.contains(&"updates".to_owned()),
        "{:?}",
        isle.glyphs().await
    );
    assert_eq!(install(&isle, "ok").await, "An update is already running.");
    wait_until(|| async { state(&isle).await == "Done" }).await;
    wait_until(|| async { note(&isle, "Update installed").await.is_some() }).await;
    let n = note(&isle, "Update installed").await.unwrap();
    assert_eq!(n["body"], "2 packages were updated.");
    wait_until(|| async { !isle.glyphs().await.contains(&"updates".to_owned()) }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_run_says_why_and_a_refused_one_says_permission() {
    let isle = start("updates-fail", false).await;
    assert_eq!(install(&isle, "lock").await, "");
    wait_until(|| async { state(&isle).await == "Failed" }).await;
    wait_until(|| async { note(&isle, "Couldn't install updates").await.is_some() }).await;
    let n = note(&isle, "Couldn't install updates").await.unwrap();
    assert_eq!(
        n["body"],
        "Another program is installing packages. Try again when it has finished."
    );
    assert!(
        n["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a[0] == "details"),
        "{n}"
    );

    assert_eq!(install(&isle, "refuse").await, "");
    wait_until(|| async { state(&isle).await == "Refused" }).await;
    assert_eq!(
        status(&isle).await["progress"]["reason"],
        "Permission was not given."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_ends_a_run_before_pacman_and_nothing_is_posted() {
    let isle = start("updates-stop", false).await;
    assert_eq!(install(&isle, "slow").await, "");
    wait_until(|| async { status(&isle).await["progress"]["total"] == 2 }).await;
    assert_eq!(
        status(&isle).await["can_stop"],
        !Path::new("/var/lib/pacman/db.lck").exists()
    );
    let why = UpdatesProxy::new(&isle.bus)
        .await
        .unwrap()
        .stop()
        .await
        .unwrap();
    assert_eq!(why, "");
    wait_until(|| async { state(&isle).await == "Stopped" }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(note(&isle, "Couldn't install updates").await.is_none());
    assert!(note(&isle, "Update installed").await.is_none());
    let why = UpdatesProxy::new(&isle.bus)
        .await
        .unwrap()
        .stop()
        .await
        .unwrap();
    assert_eq!(why, "No update is running.");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_owed_shows_in_the_island_the_menu_and_the_note() {
    let isle = start("updates-restart", true).await;
    wait_until(|| async {
        status(&isle).await["restart"] == serde_json::json!(["The kernel was updated"])
    })
    .await;
    wait_until(|| async { isle.glyphs().await.contains(&"updates".to_owned()) }).await;
    let system = system_menu(&isle).await;
    assert_eq!(
        item(&system, "Restart to finish updating…").unwrap()["enabled"],
        true
    );

    assert_eq!(install(&isle, "ok").await, "");
    wait_until(|| async { note(&isle, "Update installed").await.is_some() }).await;
    let n = note(&isle, "Update installed").await.unwrap();
    assert_eq!(n["body"], "Restart to finish updating.");
    assert!(
        n["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a[0] == "restart"),
        "{n}"
    );

    isle.open_panel(Some("updates")).await;
    let labels: Vec<String> = isle.rows().await.iter().map(|r| r.to_string()).collect();
    assert!(
        labels.iter().any(|r| r.contains("The kernel was updated")),
        "{labels:?}"
    );
    assert!(labels.iter().any(|r| r.contains("Restart…")), "{labels:?}");
}
