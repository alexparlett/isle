//! What Cleaner hands to Files (`p19b-cleaner.md` choice 23): Show in Files through
//! `FileManager1`, Quick Look through Files' `QuickLook1`, and a path on the clipboard.

use std::path::{Path, PathBuf};

const FILES: &str = "io.github.alexparlett.Isle.Files";

/// The bus and the worker, read at a component's top since both are context; handlers take
/// this.
#[derive(Clone)]
pub struct Ties {
    pub bus: Option<zbus::Connection>,
    pub worker: isle_app::Worker,
}

impl Ties {
    pub fn get() -> Self {
        Self {
            bus: isle_app::session_bus(),
            worker: isle_app::worker(),
        }
    }
}

pub fn show_in_files(ties: &Ties, paths: Vec<PathBuf>) {
    let Some(bus) = ties.bus.clone() else {
        return;
    };
    ties.worker.spawn(async move {
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

pub fn quick_look(ties: &Ties, path: &Path) {
    let Some(bus) = ties.bus.clone() else {
        return;
    };
    let uri = isle_fs::recents::uri(path);
    ties.worker.spawn(async move {
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
