//! Files against apps.md's acceptance bars that a headless Isle can check: gone when closed
//! (3), whole with no UDisks2 (4), killed without harm and its panel answered (7), a large
//! folder drawn before its listing ends (9). Needs `--features test-hooks` and `isle-files`
//! built.
#![cfg(feature = "test-hooks")]

use std::{path::PathBuf, time::Duration};

mod common;

use common::{Client, Isle, keys::*, wait_until};

const APP: &str = "io.github.alexparlett.Isle.Files";
const W: u32 = 17;

fn files_bin() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle-files");
    path.exists().then(|| path.to_string_lossy().into_owned())
}

/// Files in a home of its own, as an Isle session starts it.
fn start(isle: &mut Isle, bin: &str, args: &[&str]) -> (Client, PathBuf) {
    let home = isle.dir.join("bars-home");
    std::fs::create_dir_all(home.join("Documents")).unwrap();
    let env = |k: &str, v: &std::path::Path| format!("{k}={}", v.display());
    let vars = [
        "XDG_CURRENT_DESKTOP=Isle".to_owned(),
        env("HOME", &home),
        env("XDG_CONFIG_HOME", &home.join(".config")),
        env("XDG_STATE_HOME", &isle.dir.join("bars-state")),
        env("XDG_DATA_HOME", &home.join(".local/share")),
        env("XDG_CACHE_HOME", &home.join(".cache")),
    ];
    let mut all: Vec<&str> = vars.iter().map(String::as_str).collect();
    all.push(bin);
    all.extend_from_slice(args);
    (isle.spawn_logged("env", &all), home)
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_its_window_ends_it_and_a_kill_answers_its_panel() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-bars-close").await;
    let (app, _) = start(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    // Bar 3: gone within five seconds of its last window closing.
    isle.chord(&[SUPER, W]).await;
    let closed = std::time::Instant::now();
    wait_until(|| async { app.exited() }).await;
    assert!(closed.elapsed() < Duration::from_secs(5));

    // Bar 7: killed while a panel is open, the request answers cancel and the session goes on.
    let (again, _) = start(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    again.wait_for_part("serving FileManager1").await;
    let frontend = zbus::connection::Builder::address(isle.bus_address().as_str()).unwrap().build().await.unwrap();
    frontend.request_name("org.freedesktop.portal.Desktop").await.unwrap();
    let asked = tokio::spawn(async move {
        let reply = frontend
            .call_method(
                Some("org.freedesktop.impl.portal.desktop.isle"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.impl.portal.FileChooser"),
                "OpenFile",
                &(
                    zbus::zvariant::ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/k").unwrap(),
                    "isle.test.App",
                    "",
                    "",
                    std::collections::HashMap::<String, zbus::zvariant::OwnedValue>::new(),
                ),
            )
            .await
            .unwrap();
        reply.body().deserialize::<(u32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>)>().unwrap().0
    });
    wait_until(|| async { isle.windows().await.iter().filter(|w| w.app_id == APP).count() == 2 }).await;
    unsafe { libc::kill(again.pid(), libc::SIGKILL) };
    wait_until(|| async { again.exited() }).await;
    assert_eq!(asked.await.unwrap(), 1, "the panel's request answered cancel");
    let shell = isle_ipc::shell::ShellProxy::new(&isle.bus).await.unwrap();
    assert!(!shell.list().await.unwrap().is_empty(), "the shell answers");
    let (_third, _) = start(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_udisks_it_answers_and_its_menus_are_its_own() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    // The private system bus has no UDisks2.
    let mut isle = Isle::start("files-bars-daemons").await;
    let (app, _) = start(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    let menus = isle_ipc::menus::MenusProxy::new(&isle.bus).await.unwrap();
    wait_until(|| async { menus.bar().await.unwrap().contains("Enclosing Folder") }).await;
    // Bar 4: a command taken at once with the daemon missing.
    assert!(menus.activate(&["Go", "Home"]).await.unwrap());
    app.wait_for_part("id=\"go-home\"").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_large_folder_draws_before_its_listing_ends() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-bars-large").await;
    let big = isle.dir.join("bars-home/big");
    std::fs::create_dir_all(&big).unwrap();
    for n in 0..50_000 {
        std::fs::File::create(big.join(format!("file {n:05}.txt"))).unwrap();
    }
    let (app, _) = start(&mut isle, &bin, &[big.to_str().unwrap()]);
    isle.wait_for_window(APP).await;
    wait_until(|| async { app.printed_part("listed entries=50000") }).await;
    // Bar 9: the first batch on screen before the listing ends.
    let ms = |part: &str| {
        app.lines
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|l| l.contains(part) && l.contains("ms="))
            .and_then(|l| l.split("ms=").nth(1).and_then(|r| r.split_whitespace().next()).and_then(|n| n.parse::<u64>().ok()))
            .unwrap()
    };
    let (first, all) = (ms("first batch"), ms("listed entries=50000"));
    println!("50,000 entries: first batch at {first} ms, the listing done at {all} ms");
    assert!(first < all, "the first batch came before the listing ended");
    tokio::time::sleep(Duration::from_millis(300)).await;
    isle.keep_shot("files-large-folder").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "a measurement: run by name"]
async fn idle_memory() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-memory").await;
    let (app, _) = start(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    tokio::time::sleep(Duration::from_secs(30)).await;
    let rollup = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", app.pid())).unwrap();
    let field = |name: &str| {
        rollup.lines().find(|l| l.starts_with(name)).and_then(|l| l.split_whitespace().nth(1)).unwrap_or("0").to_owned()
    };
    println!(
        "Files: Pss {} kB, Pss_Anon {} kB, Pss_File {} kB, Private_Clean {} kB, Private_Dirty {} kB, Rss {} kB",
        field("Pss:"),
        field("Pss_Anon:"),
        field("Pss_File:"),
        field("Private_Clean:"),
        field("Private_Dirty:"),
        field("Rss:")
    );
}
