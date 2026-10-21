//! Files' Quick Look shown from the compositor's own surfaces: Spotlight's ⌘Y and the capture
//! thumbnail's click, over `QuickLook1` (`p15-files.md` choices 26 and 33).

use tracing::warn;

use crate::state::State;

const FILES: &str = "io.github.alexparlett.Isle.Files";
const PATH: &str = "/io/github/alexparlett/Isle/QuickLook1";
const INTERFACE: &str = "io.github.alexparlett.Isle.QuickLook1";

impl State {
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
