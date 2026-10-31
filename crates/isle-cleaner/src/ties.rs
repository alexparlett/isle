//! What Cleaner hands to Files (`p19b-cleaner.md` choice 23): Show in Files through
//! `FileManager1`, Quick Look through Files' `QuickLook1`, and a path on the clipboard.

use std::path::{Path, PathBuf};

const FILES: &str = "io.github.alexparlett.Isle.Files";

pub fn show_in_files(paths: Vec<PathBuf>) {
    let Some(bus) = isle_app::session_bus() else {
        return;
    };
    isle_app::worker().spawn(async move {
        let uris: Vec<String> = paths.iter().map(|p| isle_fs::recents::uri(p)).collect();
        let _ = bus
            .call_method(
                Some("org.freedesktop.FileManager1"),
                "/org/freedesktop/FileManager1",
                Some("org.freedesktop.FileManager1"),
                "ShowItems",
                &(uris, ""),
            )
            .await;
    });
}

pub fn quick_look(path: &Path) {
    let Some(bus) = isle_app::session_bus() else {
        return;
    };
    let uri = isle_fs::recents::uri(path);
    isle_app::worker().spawn(async move {
        let body = (vec![uri], 0u32, "", (0i32, 0i32, 0i32, 0i32), "none", "");
        let _ = bus
            .call_method(
                Some(FILES),
                "/io/github/alexparlett/Isle/QuickLook1",
                Some("io.github.alexparlett.Isle.QuickLook1"),
                "Show",
                &body,
            )
            .await;
    });
}

pub fn copy_path(app: &isle_app::AppHandle, path: &Path) {
    if let Some(data) = app.data() {
        let bytes = path.as_os_str().as_encoded_bytes().to_vec();
        data.set_selection(vec![
            ("text/plain;charset=utf-8".into(), bytes.clone()),
            ("text/plain".into(), bytes),
        ]);
    }
}
