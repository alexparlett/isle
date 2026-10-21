//! Files reached from the compositor's own surfaces: Quick Look over `QuickLook1`, reveals
//! over FileManager1 and searches over its `Application` interface, each call carrying an
//! activation token (`p15-files.md` choices 26, 32 and 33).

use std::collections::HashMap;

use smithay::wayland::xdg_activation::XdgActivationTokenData;
use tracing::warn;
use zvariant::Value;

use crate::state::State;

pub const FILES: &str = "io.github.alexparlett.Isle.Files";
const PATH: &str = "/io/github/alexparlett/Isle/QuickLook1";
const INTERFACE: &str = "io.github.alexparlett.Isle.QuickLook1";

impl State {
    /// Whether Files is installed, which the rows that lead to it wait on.
    pub fn files_installed(&self) -> bool {
        self.isle.apps.get(FILES).is_some()
    }

    /// A token the window Files raises for this call takes focus with.
    fn files_token(&mut self) -> String {
        let (token, _) = self.isle.xdg_activation_state.create_external_token(XdgActivationTokenData::default());
        token.to_string()
    }

    /// The item selected in a window of whatever owns FileManager1.
    pub fn show_in_files(&mut self, path: &std::path::Path) {
        self.file_manager("ShowItems", path);
    }

    /// The folder shown in a window of whatever owns FileManager1.
    pub fn show_folder_in_files(&mut self, path: &std::path::Path) {
        self.file_manager("ShowFolders", path);
    }

    fn file_manager(&mut self, method: &'static str, path: &std::path::Path) {
        let (uri, token) = (isle_fs::recents::uri(path), self.files_token());
        let Some(worker) = &self.isle.worker else {
            return;
        };
        worker.spawn(async move {
            let Ok(conn) = isle_bus::session().await else {
                return;
            };
            let call = conn
                .call_method(
                    Some("org.freedesktop.FileManager1"),
                    "/org/freedesktop/FileManager1",
                    Some("org.freedesktop.FileManager1"),
                    method,
                    &(vec![uri], token),
                )
                .await;
            if let Err(err) = call {
                warn!("could not show a file: {err}");
            }
        });
    }

    /// Spotlight's query as a search of Home in a Files window.
    pub fn show_all_in_files(&mut self, query: String) {
        let token = self.files_token();
        let Some(worker) = &self.isle.worker else {
            return;
        };
        worker.spawn(async move {
            let Ok(conn) = isle_bus::session().await else {
                return;
            };
            let platform: HashMap<&str, Value> =
                HashMap::from([("activation-token", Value::from(token.as_str())), ("desktop-startup-id", Value::from(token.as_str()))]);
            let params = vec![Value::from(query.as_str()), Value::from("")];
            let call = conn
                .call_method(
                    Some(FILES),
                    "/io/github/alexparlett/Isle/Files",
                    Some("org.freedesktop.Application"),
                    "ActivateAction",
                    &("search", params, platform),
                )
                .await;
            if let Err(err) = call {
                warn!("Files did not search: {err}");
            }
        });
    }

    /// Shows `uri` in Quick Look over the shell surface `caller`, in `mode`: `none`, `markup`
    /// or `trim`. The call starts Files when it is not running.
    pub fn quick_look_show(&self, uri: String, caller: &'static str, mode: &'static str) {
        self.quick_look_call("Show", Some((uri, caller, mode)));
    }

    pub fn quick_look_close(&self) {
        self.quick_look_call("Close", None);
    }

    fn quick_look_call(&self, method: &'static str, show: Option<(String, &'static str, &'static str)>) {
        let Some(worker) = &self.isle.worker else {
            return;
        };
        worker.spawn(async move {
            let Ok(conn) = isle_bus::session().await else {
                return;
            };
            let call = match show {
                Some((uri, caller, mode)) => {
                    let body = (vec![uri], 0u32, caller, (0i32, 0i32, 0i32, 0i32), mode, "");
                    conn.call_method(Some(FILES), PATH, Some(INTERFACE), method, &body).await
                }
                None => conn.call_method(Some(FILES), PATH, Some(INTERFACE), method, &()).await,
            };
            if let Err(err) = call {
                warn!("Quick Look: {err}");
            }
        });
    }
}
