//! Files in a headless Isle: its window at home, FileManager1's reveal, one instance. Needs
//! `--features test-hooks` and `isle-files` built.
#![cfg(feature = "test-hooks")]

use std::path::PathBuf;

mod common;

use common::{Client, Isle, keys::*, wait_until};

const APP: &str = "io.github.alexparlett.Isle.Files";

fn files_bin() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/isle-files");
    path.exists().then(|| path.to_string_lossy().into_owned())
}

/// A home with folders and files, and Files started in it as an Isle session starts apps.
fn start_files(isle: &mut Isle, bin: &str, args: &[&str]) -> (Client, PathBuf) {
    let home = isle.dir.join("files-home");
    for dir in ["Documents", "Downloads", "Development/isle", "Pictures"] {
        std::fs::create_dir_all(home.join(dir)).unwrap();
    }
    for (name, body) in [("notes.md", "# Notes\n"), ("todo.txt", "milk\n"), (".hidden", "")] {
        std::fs::write(home.join(name), body).unwrap();
    }
    std::fs::write(home.join("Development/isle/Cargo.toml"), "[workspace]\n").unwrap();
    let env = |k: &str, v: &std::path::Path| format!("{k}={}", v.display());
    let vars = [
        "XDG_CURRENT_DESKTOP=Isle".to_owned(),
        env("HOME", &home),
        env("XDG_CONFIG_HOME", &home.join(".config")),
        env("XDG_STATE_HOME", &isle.dir.join("files-state")),
        env("XDG_DATA_HOME", &home.join(".local/share")),
    ];
    let mut all: Vec<&str> = vars.iter().map(String::as_str).collect();
    all.push(bin);
    all.extend_from_slice(args);
    (isle.spawn_logged("env", &all), home)
}

#[tokio::test(flavor = "multi_thread")]
async fn files_opens_home_and_reveals_through_file_manager1() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-window").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-home").await;

    // Show in folder from another app: a tab in the window with the item selected.
    let uri = format!("file://{}", home.join("Development/isle/Cargo.toml").display());
    let reply = isle
        .bus
        .call_method(
            Some("org.freedesktop.FileManager1"),
            "/org/freedesktop/FileManager1",
            Some("org.freedesktop.FileManager1"),
            "ShowItems",
            &(vec![uri.as_str()], ""),
        )
        .await;
    assert!(reply.is_ok(), "{reply:?}");
    app.wait_for_part("show-items").await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-revealed").await;
    let windows: Vec<_> = isle.windows().await.into_iter().filter(|w| w.app_id == APP).collect();
    assert_eq!(windows.len(), 1, "a reveal is a tab, not a window");

    // A second launch hands its folder to the running Files.
    let (second, _) = start_files(&mut isle, &bin, &[home.join("Pictures").to_str().unwrap()]);
    wait_until(|| async { second.exited() }).await;
    app.wait_for_part("Pictures").await;
}

// evdev's codes for the keys these chords use.
const KEY_1: u32 = 2;
const KEY_3: u32 = 4;
const KEY_P: u32 = 25;
const KEY_SLASH: u32 = 53;
const DOWN: u32 = 108;

#[tokio::test(flavor = "multi_thread")]
async fn the_three_views_the_preview_and_the_status_bar() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-views").await;
    let (app, _home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    let settle = || tokio::time::sleep(std::time::Duration::from_millis(500));
    settle().await;

    // The first item selected, the preview and the status bar on, then each view.
    isle.chord(&[DOWN]).await;
    isle.chord(&[SUPER, SHIFT, KEY_P]).await;
    isle.chord(&[SUPER, KEY_SLASH]).await;
    settle().await;
    isle.keep_shot("files-list").await;
    isle.chord(&[SUPER, KEY_1]).await;
    app.wait_for_part("id=\"as-icons\"").await;
    settle().await;
    isle.keep_shot("files-icons").await;
    isle.chord(&[SUPER, KEY_3]).await;
    app.wait_for_part("id=\"as-columns\"").await;
    settle().await;
    isle.keep_shot("files-columns").await;
}
