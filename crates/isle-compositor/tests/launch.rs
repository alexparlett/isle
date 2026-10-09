#![cfg(feature = "test-hooks")]

mod common;

use std::fs;

use common::{Isle, wait_until};

#[tokio::test(flavor = "multi_thread")]
async fn an_entry_and_its_action_start_in_front_with_their_token() {
    let isle = Isle::start("launch").await;
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    fs::write(
        isle.dir.join("data/applications/isle.test.Launched.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Launched\nExec={client} --app-id isle.test.Launched --colour 22aa44 %U\nActions=other;\n\n[Desktop Action other]\nName=Other\nExec={client} --app-id isle.test.Other --colour 4422aa\n"
        ),
    )
    .unwrap();
    let windows = isle.windows_proxy().await;
    wait_until(|| async { windows.launch("isle.test.Launched", "", &[]).await.unwrap() }).await;
    let launched = isle.wait_for_window("isle.test.Launched").await;
    wait_until(|| async { isle.window(launched.id).await.focused }).await;

    assert!(
        windows
            .launch("isle.test.Launched", "other", &[])
            .await
            .unwrap()
    );
    let other = isle.wait_for_window("isle.test.Other").await;
    wait_until(|| async { isle.window(other.id).await.focused }).await;

    assert!(!windows.launch("isle.test.Nothing", "", &[]).await.unwrap());
    assert!(
        !windows
            .launch("isle.test.Launched", "missing", &[])
            .await
            .unwrap()
    );
    assert!(isle.log().contains("launching"), "{}", isle.log());
}

/// Records what Isle asks of an `org.freedesktop.Application`.
#[derive(Clone, Default)]
struct App(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

#[zbus::interface(name = "org.freedesktop.Application")]
impl App {
    fn activate(&self, platform: std::collections::HashMap<String, zbus::zvariant::OwnedValue>) {
        let token = platform.contains_key("activation-token");
        self.0
            .lock()
            .unwrap()
            .push(format!("activate token={token}"));
    }

    fn open(
        &self,
        uris: Vec<String>,
        _platform: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    ) {
        self.0
            .lock()
            .unwrap()
            .push(format!("open {}", uris.join(" ")));
    }

    fn activate_action(
        &self,
        name: String,
        _parameter: Vec<zbus::zvariant::OwnedValue>,
        _platform: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    ) {
        self.0.lock().unwrap().push(format!("action {name}"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dbus_activatable_entry_is_asked_over_the_bus() {
    let isle = Isle::start("launch-dbus").await;
    let client = env!("CARGO_BIN_EXE_isle-test-client");
    let entry = |id: &str| {
        format!(
            "[Desktop Entry]\nType=Application\nName={id}\nDBusActivatable=true\nExec={client} --app-id isle.test.Fallback --colour 22aa44\nActions=other;\n\n[Desktop Action other]\nName=Other\nExec={client} --app-id isle.test.Other --colour 4422aa\n"
        )
    };
    fs::write(
        isle.dir
            .join("data/applications/isle.test.Activated.desktop"),
        entry("Activated"),
    )
    .unwrap();
    fs::write(
        isle.dir.join("data/applications/isle.test.Absent.desktop"),
        entry("Absent"),
    )
    .unwrap();
    let app = App::default();
    let _service = zbus::connection::Builder::address(isle.bus_address().as_str())
        .unwrap()
        .name("isle.test.Activated")
        .unwrap()
        .serve_at("/isle/test/Activated", app.clone())
        .unwrap()
        .build()
        .await
        .unwrap();
    let windows = isle.windows_proxy().await;
    wait_until(|| async {
        windows
            .launch("isle.test.Activated", "", &[])
            .await
            .unwrap()
    })
    .await;
    assert!(
        windows
            .launch("isle.test.Activated", "other", &[])
            .await
            .unwrap()
    );
    assert!(
        windows
            .launch("isle.test.Activated", "", &["file:///tmp/a".into()])
            .await
            .unwrap()
    );
    wait_until(|| async { app.0.lock().unwrap().len() == 3 }).await;
    let mut calls = app.0.lock().unwrap().clone();
    calls.sort();
    assert_eq!(
        calls,
        ["action other", "activate token=true", "open file:///tmp/a"]
    );

    // Nothing owns the name, so the entry starts by Exec.
    assert!(windows.launch("isle.test.Absent", "", &[]).await.unwrap());
    isle.wait_for_window("isle.test.Fallback").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_command_runs_in_a_shell() {
    let isle = Isle::start("launch-run").await;
    let marker = isle.dir.join("ran");
    let test = isle.test().await;
    assert!(
        test.run(&format!("echo \"$((6 * 7))\" > '{}'", marker.display()))
            .await
            .unwrap()
    );
    wait_until(|| async { fs::read_to_string(&marker).is_ok_and(|t| t.trim() == "42") }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_appimage_dropped_in_applications_becomes_an_app() {
    if std::process::Command::new("mksquashfs")
        .arg("-version")
        .output()
        .is_err()
    {
        eprintln!("mksquashfs missing; skipped");
        return;
    }
    let isle = Isle::start("launch-appimage").await;
    let src = isle.dir.join("appimage-src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("tool.desktop"),
        "[Desktop Entry]\nType=Application\nName=Tool\nExec=tool\n",
    )
    .unwrap();
    let squash = isle.dir.join("payload.squashfs");
    assert!(
        std::process::Command::new("mksquashfs")
            .arg(&src)
            .arg(&squash)
            .args(["-quiet", "-noappend", "-all-root"])
            .status()
            .unwrap()
            .success()
    );
    // An ELF header marked as a type 2 AppImage whose section headers end at 64.
    let mut image = vec![0u8; 64];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[8..11].copy_from_slice(b"AI\x02");
    image[0x28..0x30].copy_from_slice(&64u64.to_le_bytes());
    image.extend(fs::read(&squash).unwrap());
    let apps = isle.dir.join("home/Applications");
    wait_until(|| async { apps.is_dir() }).await;
    let path = apps.join("Tool.AppImage");
    fs::write(&path, image).unwrap();

    let entry = isle
        .dir
        .join("data/applications/isle-appimage-tool.desktop");
    wait_until(|| async { entry.exists() }).await;
    let text = fs::read_to_string(&entry).unwrap();
    assert!(text.contains("Name=Tool"), "{text}");
    let windows = isle.windows_proxy().await;
    wait_until(|| async { windows.launch("isle-appimage-tool", "", &[]).await.unwrap() }).await;

    fs::remove_file(&path).unwrap();
    wait_until(|| async { !entry.exists() }).await;
    wait_until(|| async { !windows.launch("isle-appimage-tool", "", &[]).await.unwrap() }).await;
}
