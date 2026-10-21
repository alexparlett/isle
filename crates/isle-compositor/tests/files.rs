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

fn write_png(path: &std::path::Path, w: u32, h: u32, rgb: [u8; 3]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    let data: Vec<u8> = (0..w * h).flat_map(|_| rgb).collect();
    writer.write_image_data(&data).unwrap();
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
    write_png(&home.join("Pictures/red.png"), 400, 300, [220, 40, 40]);
    std::fs::write(home.join("Downloads/blob.bin"), [0u8, 159, 146, 150, 1, 2, 3]).unwrap();
    write_png(&home.join("Pictures/tall.png"), 100, 300, [40, 160, 60]);
    let env = |k: &str, v: &std::path::Path| format!("{k}={}", v.display());
    let vars = [
        "XDG_CURRENT_DESKTOP=Isle".to_owned(),
        env("HOME", &home),
        env("XDG_CONFIG_HOME", &home.join(".config")),
        env("XDG_STATE_HOME", &isle.dir.join("files-state")),
        env("XDG_DATA_HOME", &home.join(".local/share")),
        env("XDG_CACHE_HOME", &home.join(".cache")),
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

#[tokio::test(flavor = "multi_thread")]
async fn pictures_show_their_thumbnails_in_icons() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-thumbnails").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    let uri = format!("file://{}", home.join("Pictures").display());
    isle.bus
        .call_method(
            Some("org.freedesktop.FileManager1"),
            "/org/freedesktop/FileManager1",
            Some("org.freedesktop.FileManager1"),
            "ShowFolders",
            &(vec![uri.as_str()], ""),
        )
        .await
        .unwrap();
    app.wait_for_part("show-folders").await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.chord(&[SUPER, KEY_1]).await;
    app.wait_for_part("id=\"as-icons\"").await;
    // The helper writes the shared cache; the cell draws what it wrote.
    // Icons at 64 points are drawn from the 128 pixel thumbnails.
    let thumb_dir = home.join(".cache/thumbnails/normal");
    wait_until(|| async { std::fs::read_dir(&thumb_dir).map(|d| d.count() >= 2).unwrap_or(false) }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let shot = isle.keep_shot("files-thumbnails").await;
    let w = isle.wait_for_window(APP).await;
    let mut red = false;
    for y in (w.y.max(0) as usize)..(w.y as usize + 300) {
        for x in (w.x as usize + 220)..(w.x as usize + 700) {
            let [r, g, b] = shot.at(x, y);
            if r > 200 && g < 60 && b < 60 {
                red = true;
            }
        }
    }
    assert!(red, "the red picture is drawn as itself");
}

const BACKSPACE: u32 = 14;
const KEY_Z: u32 = 44;
const KEY_D: u32 = 32;

async fn reveal(isle: &Isle, app: &Client, path: &std::path::Path) {
    let uri = format!("file://{}", path.display());
    isle.bus
        .call_method(
            Some("org.freedesktop.FileManager1"),
            "/org/freedesktop/FileManager1",
            Some("org.freedesktop.FileManager1"),
            "ShowItems",
            &(vec![uri.as_str()], ""),
        )
        .await
        .unwrap();
    app.wait_for_part("show-items").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn move_to_trash_undo_and_duplicate() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-jobs").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    let todo = home.join("todo.txt");
    reveal(&isle, &app, &todo).await;

    isle.chord(&[SUPER, BACKSPACE]).await;
    app.wait_for_part("id=\"move-to-trash\"").await;
    let trashed = home.join(".local/share/Trash/files/todo.txt");
    wait_until(|| async { trashed.exists() && !todo.exists() }).await;
    assert!(home.join(".local/share/Trash/info/todo.txt.trashinfo").exists(), "its info written");

    isle.chord(&[SUPER, KEY_Z]).await;
    app.wait_for_part("id=\"undo\"").await;
    wait_until(|| async { todo.exists() && !trashed.exists() }).await;

    reveal(&isle, &app, &todo).await;
    isle.chord(&[SUPER, KEY_D]).await;
    wait_until(|| async { home.join("todo copy.txt").exists() }).await;
    isle.keep_shot("files-after-jobs").await;
}

const ENTER: u32 = 28;
const KEY_C: u32 = 46;
const KEY_V: u32 = 47;
const KEY_O: u32 = 24;

/// Types lower-case letters, digits' row and dots by their evdev codes.
async fn type_text(isle: &Isle, text: &str) {
    for c in text.chars() {
        let code = match c {
            'a' => 30, 'b' => 48, 'c' => 46, 'd' => 32, 'e' => 18, 'f' => 33, 'g' => 34, 'h' => 35,
            'i' => 23, 'j' => 36, 'k' => 37, 'l' => 38, 'm' => 50, 'n' => 49, 'o' => 24, 'p' => 25,
            'q' => 16, 'r' => 19, 's' => 31, 't' => 20, 'u' => 22, 'v' => 47, 'w' => 17, 'x' => 45,
            'y' => 21, 'z' => 44, '.' => 52, _ => continue,
        };
        isle.chord(&[code]).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_in_place_copy_paste_and_drag_onto_a_folder() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-rename").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    let w = isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // A drag from todo.txt's row onto Downloads' moves it there: Home's rows are Development,
    // Documents, Downloads, Pictures, notes.md, todo.txt under a 52 point toolbar and a 24
    // point header.
    let row = |i: f64| (w.x as f64 + 400., w.y as f64 + 52. + 24. + 28. * i + 14.);
    let test = isle.test().await;
    let (from, to) = (row(5.), row(2.));
    test.pointer_to(from.0, from.1).await.unwrap();
    test.button(272, true).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    for step in 1..=8 {
        let f = step as f64 / 8.;
        test.pointer_to(from.0, from.1 + (to.1 - from.1) * f).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }
    isle.keep_shot("files-dragging").await;
    test.button(272, false).await.unwrap();
    wait_until(|| async { home.join("Downloads/todo.txt").exists() && !home.join("todo.txt").exists() }).await;

    // Return opens the name; a new name with its extension renames at once.
    reveal(&isle, &app, &home.join("notes.md")).await;
    isle.chord(&[ENTER]).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    type_text(&isle, "ideas.md").await;
    isle.keep_shot("files-renaming").await;
    isle.chord(&[ENTER]).await;
    wait_until(|| async { home.join("ideas.md").exists() && !home.join("notes.md").exists() }).await;

    // ⌘C, Documents, ⌘V: a copy.
    reveal(&isle, &app, &home.join("ideas.md")).await;
    isle.chord(&[SUPER, KEY_C]).await;
    isle.chord(&[SUPER, SHIFT, KEY_O]).await;
    app.wait_for_part("id=\"go-documents\"").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    isle.chord(&[SUPER, KEY_V]).await;
    wait_until(|| async { home.join("Documents/ideas.md").exists() && home.join("ideas.md").exists() }).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_nothing_opens_asks_in_open_with() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-open-with").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    reveal(&isle, &app, &home.join("Downloads/blob.bin")).await;
    isle.chord(&[SUPER, KEY_O]).await;
    app.wait_for_part("open with path=").await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("files-open-with").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugged_stick_is_listed_mounted_by_a_click_and_ejected() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-volumes").await;
    let udisks = isle_fakes::udisks::UDisks::start(&isle.system_address()).await.unwrap();
    udisks.plug("STICK").await.unwrap();
    let (app, _home) = start_files(&mut isle, &bin, &[]);
    let w = isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-stick").await;

    // Locations: the computer, then the stick, under Recents and four Favourites.
    let y = w.y as f64 + 52. + 8. + 28. + 12. + 24. + 4. * 28. + 12. + 24. + 28. + 14.;
    let test = isle.test().await;
    test.pointer_to(w.x as f64 + 100., y).await.unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    wait_until(|| async { udisks.mounted("STICK").await }).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("files-stick-mounted").await;

    // Its Eject button at the row's trailing end.
    test.pointer_to(w.x as f64 + 220. - 8. - 8. - 10., y).await.unwrap();
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    wait_until(|| async { !udisks.mounted("STICK").await }).await;
}

const KEY_F: u32 = 33;
const ESC: u32 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn search_filters_as_typed_walks_on_return_and_takes_spotlights_query() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-search").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    std::fs::write(home.join("Development/isle/notebook.txt"), "the island\n").unwrap();
    std::fs::create_dir_all(home.join("Development/isle/target")).unwrap();
    std::fs::write(home.join("Development/isle/target/notes.o"), "").unwrap();
    std::fs::write(home.join("Development/isle/.gitignore"), "target\n").unwrap();
    let w = isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    // ⌘F and typing filter home at once; Return walks everything under it, the ignored
    // folder left out.
    isle.chord(&[SUPER, KEY_F]).await;
    app.wait_for_part("id=\"find\"").await;
    type_text(&isle, "note").await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.keep_shot("files-search-filter").await;
    isle.chord(&[ENTER]).await;
    app.wait_for_part("searched found=2").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    isle.keep_shot("files-search-walk").await;
    isle.chord(&[ESC]).await;

    // Spotlight's Show all in Files: a tab searching home.
    let path = "/io/github/alexparlett/Isle/Files";
    let params = [zbus::zvariant::Value::from("isle"), zbus::zvariant::Value::from("")];
    let reply = isle
        .bus
        .call_method(
            Some(APP),
            path,
            Some("org.freedesktop.Application"),
            "ActivateAction",
            &("search", &params[..], std::collections::HashMap::<&str, zbus::zvariant::Value>::new()),
        )
        .await;
    assert!(reply.is_ok(), "{reply:?}");
    app.wait_for_part("searched found=1").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    isle.keep_shot("files-search-spotlight").await;

    // The scope bar's + adds a Kind row, under the tab bar the second tab brought.
    let test = isle.test().await;
    let plus = (w.x as f64 + w.width as f64 - 80., w.y as f64 + 52. + 30. + 18.);
    test.pointer_to(plus.0, plus.1).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    test.button(272, true).await.unwrap();
    test.button(272, false).await.unwrap();
    app.wait_for_part("searched found=1").await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    isle.keep_shot("files-search-criteria").await;
}

const KEY_I: u32 = 23;

#[tokio::test(flavor = "multi_thread")]
async fn get_info_opens_a_window_per_item_and_from_file_manager1() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-info").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    reveal(&isle, &app, &home.join("Pictures/red.png")).await;
    isle.chord(&[SUPER, KEY_I]).await;
    app.wait_for_part("id=\"get-info\"").await;
    wait_until(|| async { isle.windows().await.iter().filter(|w| w.app_id == APP).count() == 2 }).await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-get-info").await;

    let uri = format!("file://{}", home.join("notes.md").display());
    isle.bus
        .call_method(
            Some("org.freedesktop.FileManager1"),
            "/org/freedesktop/FileManager1",
            Some("org.freedesktop.FileManager1"),
            "ShowItemProperties",
            &(vec![uri.as_str()], ""),
        )
        .await
        .unwrap();
    wait_until(|| async { isle.windows().await.iter().filter(|w| w.app_id == APP).count() == 3 }).await;
}

const KEY_COMMA: u32 = 51;

#[tokio::test(flavor = "multi_thread")]
async fn files_settings_open_on_cmd_comma_and_apply_to_the_views() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-settings").await;
    let (app, _home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    // Hidden extensions and no Trash row, set as Settings sets them.
    let settings = isle_ipc::settings::SettingsProxy::new(&isle.bus).await.unwrap();
    for key in ["show-extensions", "sidebar.trash"] {
        settings
            .set(&format!("app[{APP}].{key}"), &zbus::zvariant::Value::from(false))
            .await
            .unwrap();
    }
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-settings-applied").await;

    isle.chord(&[SUPER, KEY_COMMA]).await;
    app.wait_for_part("id=\"settings\"").await;
    wait_until(|| async { isle.windows().await.iter().filter(|w| w.app_id == APP).count() == 2 }).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    isle.keep_shot("files-settings-window").await;
}

/// A FileChooser call as the portal frontend makes it; blocks until the panel answers.
async fn file_chooser(
    frontend: zbus::Connection,
    method: &'static str,
    handle: &'static str,
    options: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
) -> (u32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>) {
    let reply = frontend
        .call_method(
            Some("org.freedesktop.impl.portal.desktop.isle"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.impl.portal.FileChooser"),
            method,
            &(zbus::zvariant::ObjectPath::try_from(handle).unwrap(), "isle.test.App", "", "", options),
        )
        .await
        .unwrap();
    reply.body().deserialize().unwrap()
}

fn owned<'a>(v: impl Into<zbus::zvariant::Value<'a>>) -> zbus::zvariant::OwnedValue {
    v.into().try_to_owned().unwrap()
}

fn bytes_of(path: &std::path::Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    let mut b = path.as_os_str().as_bytes().to_vec();
    b.push(0);
    b
}

#[tokio::test(flavor = "multi_thread")]
async fn the_portal_hands_open_and_save_to_files_panels() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-panels").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    let frontend = zbus::connection::Builder::address(isle.bus_address().as_str()).unwrap().build().await.unwrap();
    frontend.request_name("org.freedesktop.portal.Desktop").await.unwrap();
    let panels = || async { isle.windows().await.iter().filter(|w| w.app_id == APP).count() };

    // A folder request: Return chooses the folder shown.
    let mut options = std::collections::HashMap::new();
    options.insert("directory".to_owned(), owned(true));
    options.insert("current_folder".to_owned(), owned(bytes_of(&home.join("Pictures"))));
    let asked = tokio::spawn(file_chooser(frontend.clone(), "OpenFile", "/org/freedesktop/portal/desktop/request/1_1/a", options));
    app.wait_for_part("choose").await;
    wait_until(|| async { panels().await == 2 }).await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-panel-open").await;
    isle.chord(&[ENTER]).await;
    let (response, results) = asked.await.unwrap();
    assert_eq!(response, 0);
    let uris: Vec<String> = results["uris"].try_clone().unwrap().try_into().unwrap();
    assert!(uris[0].ends_with("/Pictures"), "{uris:?}");
    wait_until(|| async { panels().await == 1 }).await;

    // Save with a name: Return in the name field answers with the path.
    let mut options = std::collections::HashMap::new();
    options.insert("current_name".to_owned(), owned("shot.png"));
    options.insert("current_folder".to_owned(), owned(bytes_of(&home.join("Pictures"))));
    let asked = tokio::spawn(file_chooser(frontend.clone(), "SaveFile", "/org/freedesktop/portal/desktop/request/1_1/b", options));
    wait_until(|| async { panels().await == 2 }).await;
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    isle.keep_shot("files-panel-save").await;
    isle.chord(&[ENTER]).await;
    let (response, results) = asked.await.unwrap();
    assert_eq!(response, 0);
    let uris: Vec<String> = results["uris"].try_clone().unwrap().try_into().unwrap();
    assert!(uris[0].ends_with("/Pictures/shot.png"), "{uris:?}");

    // Escape cancels.
    let asked = tokio::spawn(file_chooser(frontend.clone(), "OpenFile", "/org/freedesktop/portal/desktop/request/1_1/c", Default::default()));
    wait_until(|| async { panels().await == 2 }).await;
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    isle.chord(&[ESC]).await;
    assert_eq!(asked.await.unwrap().0, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panel_is_a_sheet_on_the_asking_window() {
    let Some(bin) = files_bin() else {
        eprintln!("skipped: build isle-files first");
        return;
    };
    let mut isle = Isle::start("files-panel-sheet").await;
    let (app, home) = start_files(&mut isle, &bin, &[]);
    isle.wait_for_window(APP).await;
    app.wait_for_part("serving FileManager1").await;
    let asking = isle.client("isle.test.Red", "ff0000");
    asking.wait_for("keyboard-enter").await;
    asking.send("export");
    let handle = loop {
        let line = asking.lines.lock().unwrap().iter().find_map(|l| l.strip_prefix("exported ").map(str::to_owned));
        if let Some(handle) = line {
            break handle;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    let parent = isle.wait_for_window("isle.test.Red").await.id;
    isle.windows_proxy().await.set_box(parent, 200, 100, 1400, 860).await.unwrap();
    wait_until(|| async { isle.window(parent).await.width == 1400 }).await;
    let frontend = zbus::connection::Builder::address(isle.bus_address().as_str()).unwrap().build().await.unwrap();
    frontend.request_name("org.freedesktop.portal.Desktop").await.unwrap();

    let mut options = std::collections::HashMap::new();
    options.insert("directory".to_owned(), owned(true));
    options.insert("current_folder".to_owned(), owned(bytes_of(&home.join("Documents"))));
    let parent_handle: &'static str = Box::leak(format!("wayland:{handle}").into_boxed_str());
    let asked = tokio::spawn(async move {
        frontend
            .call_method(
                Some("org.freedesktop.impl.portal.desktop.isle"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.impl.portal.FileChooser"),
                "OpenFile",
                &(zbus::zvariant::ObjectPath::try_from("/org/freedesktop/portal/desktop/request/1_1/s").unwrap(), "isle.test.Red", parent_handle, "Choose a folder", options),
            )
            .await
            .unwrap()
            .body()
            .deserialize::<(u32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>)>()
            .unwrap()
    });
    let sheet = loop {
        let found = isle.windows().await.into_iter().find(|w| w.app_id == APP && w.parent == parent);
        if let Some(found) = found {
            break found;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    };
    let host = isle.window(parent).await;
    assert!(sheet.x > host.x && sheet.x + sheet.width < host.x + host.width, "{sheet:?} inside {host:?}");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    isle.keep_shot("files-panel-sheet").await;
    isle.chord(&[ENTER]).await;
    let (response, _) = asked.await.unwrap();
    assert_eq!(response, 0);
}
