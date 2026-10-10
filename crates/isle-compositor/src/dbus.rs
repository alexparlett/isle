//! The compositor's D-Bus API, served on the worker (`platform.md`, Compositor D-Bus API).

use std::error::Error;

use isle_ipc::{
    BUS_NAME, DISPLAYS_PATH, DisplayInfo, SESSION_PATH, SETTINGS_PATH, SHELL_PATH, SettingKey,
    SettingsStatus, SurfaceInfo, VERSION_PATH, WINDOWS_PATH, WindowInfo,
};
use isle_settings::{SetError, Value, schema_of};
use tokio::sync::broadcast;
use tracing::info;
use zbus::{fdo, interface, object_server::SignalEmitter};
use zvariant::OwnedValue;

use crate::{
    settings::SharedStore,
    worker::{Event, LoopTimeout, ToLoop},
};

impl From<LoopTimeout> for fdo::Error {
    fn from(err: LoopTimeout) -> Self {
        fdo::Error::TimedOut(err.to_string())
    }
}

struct Version;

#[interface(name = "io.github.alexparlett.Isle.Version1")]
impl Version {
    #[zbus(property)]
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    #[zbus(property)]
    fn commit(&self) -> &str {
        crate::build_info::COMMIT
    }

    #[zbus(property)]
    fn built(&self) -> u64 {
        crate::build_info::built()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// The compositor's process id, which changes when the supervisor restarts it.
    #[zbus(property)]
    fn pid(&self) -> u32 {
        std::process::id()
    }

    #[zbus(property)]
    fn features(&self) -> Vec<String> {
        crate::build_info::features()
    }
}

struct Session {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Session1")]
impl Session {
    async fn quit(&self) -> fdo::Result<()> {
        self.to_loop
            .call(|state| state.isle.loop_signal.stop())
            .await?;
        Ok(())
    }

    async fn lock(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.lock("the bus")).await?)
    }

    /// Idle as JSON: how long, dimmed, the screens off, the steps come, what holds it.
    async fn idle(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.idle_status()).await?)
    }

    #[zbus(property)]
    async fn locked(&self) -> fdo::Result<bool> {
        Ok(self.to_loop.call(|state| state.isle.lock.locked).await?)
    }

    /// The session was locked or unlocked; `Locked` says which.
    #[zbus(signal)]
    async fn lock_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

struct Windows {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Windows1")]
impl Windows {
    async fn list(&self) -> fdo::Result<Vec<WindowInfo>> {
        Ok(self.to_loop.call(|state| state.isle.window_infos()).await?)
    }

    async fn focus(&self, id: u64) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.activate(id)).await?)
    }

    async fn close(&self, id: u64) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.close_window(id)).await?)
    }

    async fn apps(&self) -> fdo::Result<Vec<isle_ipc::AppInfo>> {
        Ok(self.to_loop.call(|state| state.isle.app_infos()).await?)
    }

    async fn activate_app(&self, app: String) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.activate_app(&app)).await?)
    }

    async fn quit_app(&self, app: String) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.quit_app(&app)).await?)
    }

    /// Sends the polite signal to an app running in the background with no window.
    async fn stop_app(&self, app: String) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.stop_app(&app)).await?)
    }

    /// Starts an app by desktop id, or one of its entry's actions by id when not empty.
    async fn launch(&self, app: String, action: String, uris: Vec<String>) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                state.launch(crate::launch::Launch::App {
                    app,
                    action: (!action.is_empty()).then_some(action),
                    uris,
                })
            })
            .await?)
    }

    /// Opens a link or file in the app that handles it.
    async fn open(&self, target: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.launch(crate::launch::Launch::Open(target)))
            .await?)
    }

    async fn minimise(&self, id: u64) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.minimise(id)).await?)
    }

    async fn unminimise(&self, id: u64) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.unminimise(id)).await?)
    }

    async fn set_app_hidden(&self, app: String, hidden: bool) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.set_app_hidden(&app, hidden))
            .await?)
    }

    async fn tile(&self, id: u64, zone: String) -> fdo::Result<bool> {
        let Some(zone) = crate::wm::geometry::Zone::from_name(&zone) else {
            return Ok(false);
        };
        Ok(self.to_loop.call(move |state| state.tile(id, zone)).await?)
    }

    async fn restore(&self, id: u64) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.restore_box(id)).await?)
    }

    async fn set_box(&self, id: u64, x: i32, y: i32, width: i32, height: i32) -> fdo::Result<bool> {
        if width < 1 || height < 1 {
            return Err(fdo::Error::InvalidArgs("a box is at least 1 by 1".into()));
        }
        Ok(self
            .to_loop
            .call(move |state| {
                state.set_box(id, smithay::utils::Rectangle::new((x, y).into(), (width, height).into()))
            })
            .await?)
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

struct Desktops {
    to_loop: ToLoop,
}

struct Wallpaper {
    to_loop: ToLoop,
}

struct IslandIface {
    to_loop: ToLoop,
}

struct ServicesIface {
    to_loop: ToLoop,
}

struct NotificationsIface {
    to_loop: ToLoop,
}

struct FocusIface {
    to_loop: ToLoop,
}

struct CaptureIface {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Capture1")]
impl CaptureIface {
    async fn shoot(&self, what: String, copy: bool) -> fdo::Result<()> {
        if !matches!(what.as_str(), "screen" | "pick" | "toolbar" | "colour") {
            return Err(fdo::Error::InvalidArgs(format!("no shot called {what}")));
        }
        Ok(self
            .to_loop
            .call(move |state| match what.as_str() {
                "screen" => state.shoot_screens(copy),
                "pick" => state.open_pick(crate::freeze::Purpose::Shot { copy }),
                "toolbar" => state.open_toolbar(copy),
                _ => state.open_pick(crate::freeze::Purpose::Colour),
            })
            .await?)
    }

    async fn record(&self, action: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                match action.as_str() {
                    "start" => {
                        if let Some(output) = state.isle.main_output() {
                            state.start_recording(crate::cast::Source::Screen(output));
                        }
                    }
                    _ => {
                        state.stop_recording();
                    }
                }
                state.recording()
            })
            .await?)
    }

    async fn status(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.capture_status().to_string()).await?)
    }

    async fn stop_stream(&self, id: u64) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                let there = state.isle.casting.get(id).is_some();
                state.end_cast(id, crate::cast::Ended::Stopped);
                there
            })
            .await?)
    }
}

#[interface(name = "io.github.alexparlett.Isle.Focus1")]
impl FocusIface {
    async fn status(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.focus_status()).await?)
    }

    async fn set(&self, mode: String, minutes: u32) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| {
                let minutes = (minutes > 0).then_some(i64::from(minutes));
                state.focus_set(Some(&mode), minutes, crate::focus_modes::Origin::Control)
            })
            .await?)
    }

    async fn off(&self) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(|state| state.focus_set(None, None, crate::focus_modes::Origin::Control))
            .await?)
    }

    #[zbus(signal)]
    async fn changed(emitter: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;
}

#[interface(name = "io.github.alexparlett.Isle.Notifications1")]
impl NotificationsIface {
    async fn list(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.notifications_status()).await?)
    }

    async fn close(&self, id: u32) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.notification_close(id)).await?)
    }

    async fn invoke(&self, id: u32, key: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.notification_invoke(id, &key))
            .await?)
    }

    async fn reply(&self, id: u32, text: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.notification_reply(id, &text))
            .await?)
    }

    async fn clear(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.notifications_clear()).await?)
    }

    #[zbus(signal)]
    async fn changed(emitter: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;
}

struct MenusIface {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Menus1")]
impl MenusIface {
    /// The front app's menu bar as JSON.
    async fn bar(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.bar_json()).await?)
    }

    /// Chooses the item at a path of labels, a title first; false when there is none.
    async fn activate(&self, path: Vec<String>) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                let path: Vec<&str> = path.iter().map(String::as_str).collect();
                state.bar_activate(&path)
            })
            .await?)
    }

    /// The front app's menu items whose labels hold `query`, with their paths, as JSON.
    async fn search(&self, query: String) -> fdo::Result<String> {
        Ok(self.to_loop.call(move |state| state.menu_search(&query)).await?)
    }

    /// A window's own menus as JSON, and where they came from: `dbusmenu`, `gmenu`, `none`
    /// for the fallback, or `looking` while Isle asks.
    async fn show(&self, id: u64) -> fdo::Result<String> {
        Ok(self.to_loop.call(move |state| state.menus_json(id)).await?)
    }

    /// Reveals the menu bar on the focused screen, a title's menu open from the keyboard.
    async fn reveal(&self, title: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                let Some(output) = state.isle.output_with_focus().or_else(|| state.isle.active_output()) else {
                    return false;
                };
                state.reveal_bar(&output, true, None);
                let index = state.isle.bar.titles.iter().position(|t| t.label == title);
                if let Some(index) = index {
                    state.menu_bar_action(isle_shell::menubar::BarAction::Title(index));
                }
                true
            })
            .await?)
    }
}

#[interface(name = "io.github.alexparlett.Isle.Island1")]
impl IslandIface {
    /// Each output's island as JSON.
    async fn status(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.island_status()).await?)
    }
}

#[interface(name = "io.github.alexparlett.Isle.Services1")]
impl ServicesIface {
    async fn status(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.services.status()).await?)
    }
}

#[interface(name = "io.github.alexparlett.Isle.Wallpaper1")]
impl Wallpaper {
    async fn status(&self) -> fdo::Result<Vec<isle_ipc::WallpaperInfo>> {
        Ok(self
            .to_loop
            .call(|state| {
                let outputs: Vec<(String, String)> = state
                    .isle
                    .space
                    .outputs()
                    .map(|o| (o.name(), crate::state::Isle::output_identity(o)))
                    .collect();
                state
                    .isle
                    .wallpapers
                    .status(&outputs)
                    .into_iter()
                    .map(|s| isle_ipc::WallpaperInfo {
                        output: s.output,
                        source: s.source,
                        file: s.file.map(|f| f.display().to_string()).unwrap_or_default(),
                        problem: s.problem.unwrap_or_default(),
                    })
                    .collect()
            })
            .await?)
    }
}

/// The output named, or the one with focus for an empty name.
fn output_named(state: &crate::state::State, name: &str) -> Option<smithay::output::Output> {
    if name.is_empty() {
        state.isle.output_with_focus()
    } else {
        state.isle.space.outputs().find(|o| o.name() == name).cloned()
    }
}

#[interface(name = "io.github.alexparlett.Isle.Desktops1")]
impl Desktops {
    async fn list(&self) -> fdo::Result<Vec<isle_ipc::DesktopInfo>> {
        Ok(self.to_loop.call(|state| state.isle.desktop_infos()).await?)
    }

    async fn switch(&self, output: String, index: u32) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| match output_named(state, &output) {
                Some(o) if index > 0 => state.switch_desktop(&o, index as usize - 1),
                _ => false,
            })
            .await?)
    }

    async fn add(&self, output: String) -> fdo::Result<u32> {
        Ok(self
            .to_loop
            .call(move |state| {
                output_named(state, &output)
                    .and_then(|o| state.add_desktop(&o))
                    .map_or(0, |i| i as u32 + 1)
            })
            .await?)
    }

    async fn remove(&self, output: String, index: u32) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| match output_named(state, &output) {
                Some(o) if index > 0 => state.remove_desktop(&o, index as usize - 1),
                _ => false,
            })
            .await?)
    }

    async fn move_window(&self, id: u64, index: u32) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| index > 0 && state.move_to_desktop(id, index as usize - 1))
            .await?)
    }

    async fn set_full_screen(&self, id: u64, on: bool) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                if on {
                    state.enter_full_screen(id, false)
                } else {
                    state.leave_full_screen(id)
                }
            })
            .await?)
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

struct Shell {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Shell1")]
impl Shell {
    async fn list(&self) -> fdo::Result<Vec<SurfaceInfo>> {
        Ok(self
            .to_loop
            .call(|state| {
                state
                    .isle
                    .shell
                    .surfaces()
                    .into_iter()
                    .map(|s| SurfaceInfo {
                        name: s.name,
                        output: s.output,
                        open: s.open,
                        x: s.geometry.loc.x,
                        y: s.geometry.loc.y,
                        width: s.geometry.size.w,
                        height: s.geometry.size.h,
                        panics: s.panics,
                        disabled: s.disabled,
                        wakes: s.wakes,
                        draws: s.draws,
                        glass_runs: s.glass_runs,
                        glass_floor: s.glass_floor,
                        draw_p50: s.draw_times.0,
                        draw_p95: s.draw_times.1,
                        draw_max: s.draw_times.2,
                        glass_p50: s.glass_times.0,
                        glass_p95: s.glass_times.1,
                        glass_max: s.glass_times.2,
                    })
                    .collect()
            })
            .await?)
    }

    /// Opens Spotlight as `how` says: `open`, `apps`, `emoji`; `close` closes it.
    async fn spotlight(&self, how: String) -> fdo::Result<bool> {
        let scope = match how.as_str() {
            "open" => Some(isle_search::Scope::General),
            "apps" => Some(isle_search::Scope::Apps),
            "emoji" => Some(isle_search::Scope::Emoji),
            "close" => None,
            _ => return Err(fdo::Error::InvalidArgs(format!("no way to open Spotlight called {how}"))),
        };
        Ok(self
            .to_loop
            .call(move |state| {
                match scope {
                    Some(scope) => {
                        state.close_spotlight();
                        state.open_spotlight(scope);
                    }
                    None => state.close_spotlight(),
                }
                state.isle.spotlight.open()
            })
            .await?)
    }

    /// Spotlight's rows for `query` in `scope` as JSON, run without opening it.
    async fn search(&self, query: String, scope: String) -> fdo::Result<String> {
        let scope = isle_search::Scope::parse(&scope)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("no scope called {scope}")))?;
        let (sources, usage) = self.to_loop.call(|state| state.search_sources()).await?;
        tokio::task::spawn_blocking(move || crate::spotlight::search_all(sources, usage, query, scope))
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    async fn open(&self, name: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.open_surface(&name))
            .await?)
    }

    async fn close(&self, name: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.close_surface(&name))
            .await?)
    }

    /// Bytes in Skia's resource cache and its limit; zero before any surface has drawn.
    async fn cache(&self) -> fdo::Result<(u64, u64)> {
        Ok(self
            .to_loop
            .call(|state| {
                let (used, limit) = state.isle.shell.cache_usage().unwrap_or_default();
                (used as u64, limit as u64)
            })
            .await?)
    }

    /// Whether the user's tokens.kdl is in force, or where it went wrong.
    async fn tokens_status(&self) -> fdo::Result<SettingsStatus> {
        Ok(self
            .to_loop
            .call(|state| match &state.isle.tokens_error {
                None => SettingsStatus {
                    ok: true,
                    file: crate::tokens::tokens_path().display().to_string(),
                    line: 0,
                    column: 0,
                    message: String::new(),
                },
                Some(err) => SettingsStatus {
                    ok: false,
                    file: err.file.display().to_string(),
                    line: err.line as u32,
                    column: err.column as u32,
                    message: err.message.clone(),
                },
            })
            .await?)
    }

    /// Turns a surface that panicked too often back on, closed.
    async fn restart(&self, name: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| {
                let found = state.isle.shell.restart(&name);
                state.dispatch_shell();
                found
            })
            .await?)
    }

    /// The surface's accessibility tree as JSON: each node's id, role, label, value,
    /// children and focus.
    async fn tree(&self, name: String) -> fdo::Result<String> {
        self.to_loop
            .call(move |state| state.isle.shell.tree(&name))
            .await?
            .ok_or_else(|| fdo::Error::InvalidArgs("no such surface".into()))
    }
}

struct Displays {
    to_loop: ToLoop,
}

#[interface(name = "io.github.alexparlett.Isle.Displays1")]
impl Displays {
    async fn list(&self) -> fdo::Result<Vec<DisplayInfo>> {
        Ok(self
            .to_loop
            .call(|state| state.isle.display_infos())
            .await?)
    }

    /// Releases every output and probes the connectors again, as one step.
    async fn redetect(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.redetect()).await?)
    }

    /// Turns every screen off now; any input lights them.
    async fn sleep(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.displays_sleep()).await?)
    }

    /// The output's frames after `since`, with what its planes carried.
    async fn trace(&self, output: String, since: u64) -> fdo::Result<isle_ipc::PlaneTrace> {
        self.to_loop
            .call(move |state| state.isle.plane_trace(&output, since))
            .await?
            .ok_or_else(|| fdo::Error::InvalidArgs("no such display".into()))
    }

    /// Clears the output's trace counters.
    async fn reset_trace(&self, output: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.isle.reset_plane_trace(&output))
            .await?
            .ok_or_else(|| fdo::Error::InvalidArgs("no such display".into()))
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

struct Keyboard {
    to_loop: ToLoop,
    store: SharedStore,
    events: broadcast::Sender<Event>,
}

impl Keyboard {
    /// Installs a recording on the loop and waits for its key, a minute at most.
    async fn recording(&self, detect: bool) -> fdo::Result<isle_ipc::RecordInfo> {
        use crate::keyboard::inject::Recording;
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.to_loop
            .call(move |state| {
                state.cancel_recording();
                state.isle.recording = Some(if detect {
                    Recording::Keyboard(reply)
                } else {
                    Recording::Chord(reply)
                });
            })
            .await?;
        let recorded = match tokio::time::timeout(std::time::Duration::from_secs(60), answer).await {
            Ok(Ok(recorded)) => recorded,
            _ => {
                let _ = self.to_loop.call(|state| state.cancel_recording()).await;
                return Err(fdo::Error::TimedOut("no key came".into()));
            }
        };
        Ok(isle_ipc::RecordInfo {
            chord: recorded.chord,
            holder: recorded.holder,
            cancelled: recorded.cancelled,
            keyboard: recorded.keyboard,
            iso: recorded.iso,
        })
    }
}

#[interface(name = "io.github.alexparlett.Isle.Keyboard1")]
impl Keyboard {
    async fn list(&self) -> fdo::Result<Vec<isle_ipc::KeyboardInfo>> {
        Ok(self.to_loop.call(|state| state.keyboard_infos()).await?)
    }

    async fn status(&self) -> fdo::Result<isle_ipc::KeyboardStatus> {
        Ok(self
            .to_loop
            .call(|state| {
                let (layout, caps_lock, num_lock) = state.keyboard_status();
                isle_ipc::KeyboardStatus {
                    layout,
                    caps_lock,
                    num_lock,
                    inhibited_by: state.inhibiting_app(),
                    taken_back: state.isle.keys_taken_back.is_some(),
                }
            })
            .await?)
    }

    /// Sets a keyboard's profile by its name: `mac`, `pc` or `auto`; writes the settings.
    async fn set_profile(&self, name: String, profile: String) -> fdo::Result<()> {
        let mut store = self.store.lock().await;
        let key = format!("keyboard-device[{name}].profile");
        let keys = store
            .set(&key, Value::Str(profile))
            .map_err(|err| match err {
                SetError::Invalid(message) => fdo::Error::InvalidArgs(message),
                other => fdo::Error::Failed(other.to_string()),
            })?;
        crate::settings::publish(&self.to_loop, &self.events, store.current().clone(), keys);
        Ok(())
    }

    /// Whether the user's translation.kdl is in force, and where it is wrong if not.
    async fn tables_status(&self) -> fdo::Result<SettingsStatus> {
        Ok(self
            .to_loop
            .call(|state| {
                let file = crate::keyboard::tables_path().display().to_string();
                match &state.isle.keyboards.tables_error {
                    None => SettingsStatus {
                        ok: true,
                        file,
                        line: 0,
                        column: 0,
                        message: String::new(),
                    },
                    Some(err) => SettingsStatus {
                        ok: false,
                        file,
                        line: err.line as u32,
                        column: err.column as u32,
                        message: err.message.clone(),
                    },
                }
            })
            .await?)
    }

    /// The Mac chord that sends an app `chord`, for a menu item it declares by it; empty
    /// when none does.
    async fn mac_chord(&self, app: String, chord: String) -> fdo::Result<String> {
        let sent = isle_keys::engine::Stroke::parse(&chord)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("`{chord}` is not a chord")))?;
        Ok(self
            .to_loop
            .call(move |state| {
                state
                    .isle
                    .keyboards
                    .tables()
                    .mac_chord(&[app.as_str()], &sent)
                    .map(|c| c.words())
                    .unwrap_or_default()
            })
            .await?)
    }

    /// Suspends desktop chords and translation and returns the next chord; Escape cancels,
    /// Delete clears, focus moving ends it. Gives up after a minute.
    async fn record(&self) -> fdo::Result<isle_ipc::RecordInfo> {
        self.recording(false).await
    }

    /// Returns the keyboard the next key comes from.
    async fn detect(&self) -> fdo::Result<isle_ipc::RecordInfo> {
        self.recording(true).await
    }

    /// Types text into a window, focusing it, untranslated; false when it is gone.
    async fn type_text(&self, window: u64, text: String) -> fdo::Result<bool> {
        Ok(self.to_loop.call(move |state| state.type_into(window, &text)).await?)
    }

    /// Sends a window a Mac chord as its own (Cmd+V is a terminal's Ctrl+Shift+V).
    async fn send_chord(&self, window: u64, chord: String) -> fdo::Result<bool> {
        let chord = isle_keys::chord::Chord::parse(&chord)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("`{chord}` is not a chord")))?;
        Ok(self.to_loop.call(move |state| state.send_chord_to(window, chord)).await?)
    }

    /// Takes the keys back from the window holding them, or gives them back.
    async fn take_keys_back(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.toggle_keys_back()).await?)
    }

    /// Every action of the shortcut set with its chords.
    async fn shortcuts(&self) -> Vec<isle_ipc::ShortcutInfo> {
        let schema = schema_of::<isle_settings::schema::Settings>();
        let store = self.store.lock().await;
        store
            .current()
            .shortcuts
            .chords()
            .into_iter()
            .map(|(id, chords)| isle_ipc::ShortcutInfo {
                id: id.to_owned(),
                label: schema
                    .iter()
                    .find(|k| k.key == format!("shortcuts.{id}"))
                    .map(|k| k.doc.clone())
                    .unwrap_or_default(),
                chords: chords.to_vec(),
                packet: isle_keys::shortcuts::find(id)
                    .and_then(|s| s.packet)
                    .unwrap_or_default()
                    .to_owned(),
            })
            .collect()
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

#[cfg(feature = "test-hooks")]
struct Test {
    to_loop: ToLoop,
}

#[cfg(feature = "test-hooks")]
#[interface(name = "io.github.alexparlett.Isle.Test1")]
impl Test {
    /// Posts to the island as a service would; the message names what was wrong.
    async fn island(&self, json: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.island_post(&json))
            .await?
            .map_err(fdo::Error::InvalidArgs)
    }

    /// Acts on Control Centre as its controls would: `{"page": "id"}`, `{"toggle": "id"}`,
    /// `{"choose": [module, row]}`, `{"slide": [module, row, value]}`,
    /// `{"switch": [module, row, on]}`, `{"expand": [module, row]}`.
    async fn panel(&self, json: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.panel_test_action(&json))
            .await?
            .map_err(fdo::Error::InvalidArgs)
    }

    /// Acts on the banners as their controls would: `{"hover": true}`, `{"open": id}`,
    /// `{"action": [id, key]}`, `{"close": id}`, `{"reply": id}`, `{"send": [id, text]}`.
    async fn banner(&self, json: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.banner_test_action(&json))
            .await?
            .map_err(fdo::Error::InvalidArgs)
    }

    /// The lock as JSON: whether locked, the dots typed, the status, failures, pending, and
    /// the minutes left of a lockout.
    async fn lock_status(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.lock_status()).await?)
    }

    /// Moves idle's clock on by `millis`, as that long with no input would.
    async fn idle_advance(&self, millis: u64) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.idle_advance(std::time::Duration::from_millis(millis)))
            .await?)
    }

    /// Queues a prompt as an agent would, from JSON: `kind`, `title`, `message`, `fields`
    /// (`id`, `label`, `secret`, `show`), `choices` (`id`, `label`, `checked`), `action`,
    /// `cancel`, `details`, `code`, `typed`, `confirm`, `pid`, `user_started`, `checks`.
    async fn prompt_ask(&self, json: String) -> fdo::Result<u64> {
        let ask = crate::prompts::test_ask(&json).map_err(fdo::Error::InvalidArgs)?;
        Ok(self
            .to_loop
            .call(move |state| {
                let (send, mut answers) = tokio::sync::mpsc::unbounded_channel();
                let id = state.prompt_ask(ask, send);
                // The answers are recorded by the queue for the test to read.
                if let Some(runtime) = &state.isle.worker {
                    runtime.spawn(async move { while answers.recv().await.is_some() {} });
                }
                id
            })
            .await?)
    }

    /// The prompt queue as JSON: the one showing, whether armed, those waiting and why, the
    /// answers the test's prompts received.
    async fn prompts(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.prompts_status()).await?)
    }

    /// The session dialog and the session's end as JSON.
    async fn session(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.session_status()).await?)
    }

    /// Presses what a test cannot reach by key: `force-quit {app}`, `show {app}`, and the
    /// lock's `restart` and `shut-down`.
    async fn session_act(&self, action: String) -> fdo::Result<()> {
        use isle_shell::{lock::LockAction, prompts::session::SessionAction};
        Ok(self
            .to_loop
            .call(move |state| match action.split_once(' ') {
                Some(("force-quit", app)) => {
                    state.session_action(SessionAction::ForceQuit(app.to_owned()))
                }
                Some(("show", app)) => state.session_action(SessionAction::ShowApp(app.to_owned())),
                _ if action == "restart" => state.lock_power(LockAction::Restart),
                _ if action == "shut-down" => state.lock_power(LockAction::ShutDown),
                _ => {}
            })
            .await?)
    }

    /// The clipboard's text when Isle offers it.
    async fn clipboard(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle_clipboard_text().unwrap_or_default()).await?)
    }

    /// Spotlight's model as JSON.
    async fn spotlight(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.spotlight_status()).await?)
    }

    /// Does what the pointer would in Spotlight: `hover {key}`, `click {key}`, `quick {n}`.
    async fn spotlight_act(&self, action: String) -> fdo::Result<()> {
        use isle_shell::spotlight::SpotlightAction;
        let action = match action.split_once(' ') {
            Some(("hover", key)) => SpotlightAction::Hover(key.to_owned()),
            Some(("click", key)) => SpotlightAction::Click(key.to_owned()),
            Some(("secondary", key)) => SpotlightAction::Secondary(key.to_owned()),
            Some(("quick", n)) => SpotlightAction::Quick(n.parse().unwrap_or(0)),
            Some(("contents", on)) => SpotlightAction::Contents(on == "true"),
            Some(("drag", keys)) => {
                let (from, to) = keys.split_once(' ').ok_or_else(|| fdo::Error::InvalidArgs(action.clone()))?;
                let (from, to) = (from.to_owned(), to.to_owned());
                return Ok(self
                    .to_loop
                    .call(move |state| {
                        state.spotlight_action(SpotlightAction::DragFrom(from));
                        state.spotlight_action(SpotlightAction::DragTo(to));
                    })
                    .await?);
            }
            Some(("menu", label)) => {
                let label = label.to_owned();
                return Ok(self.to_loop.call(move |state| state.spotlight_menu_choose(&label)).await?);
            }
            None if action == "filter" => SpotlightAction::Filter,
            _ => return Err(fdo::Error::InvalidArgs(action)),
        };
        Ok(self.to_loop.call(move |state| state.spotlight_action(action)).await?)
    }

    /// Starts a library game by its key (`steam:570`) as Spotlight does.
    async fn launch_game(&self, key: String) -> fdo::Result<bool> {
        let Some(id) = isle_games::GameId::parse(&key) else {
            return Ok(false);
        };
        Ok(self.to_loop.call(move |state| state.launch_game(&id)).await?)
    }

    /// Runs a command line as Spotlight's Run does.
    async fn run(&self, command: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| state.launch(crate::launch::Launch::Command(command)))
            .await?)
    }

    /// Force Quit's panel and the alerts hung on windows as JSON.
    async fn force_quit(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.isle.force_quit_status()).await?)
    }

    /// Presses what a test cannot reach by key: the sheet's `confirm`, `select {app}`, and an
    /// alert's `force-quit {window}`.
    async fn force_quit_act(&self, action: String) -> fdo::Result<()> {
        use isle_shell::{alert::AlertAction, force_quit::ForceQuitAction};
        Ok(self
            .to_loop
            .call(move |state| match action.split_once(' ') {
                Some(("select", app)) => {
                    state.force_quit_action(ForceQuitAction::Select(app.to_owned()))
                }
                Some(("force-quit", id)) => {
                    if let Ok(id) = id.parse() {
                        state.alert_action(id, AlertAction::ForceQuit);
                    }
                }
                _ if action == "confirm" => state.force_quit_action(ForceQuitAction::Confirm),
                _ => {}
            })
            .await?)
    }

    /// A prompt's answer was wrong: it asks again with `message`.
    async fn prompt_wrong(&self, id: u64, message: String) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.prompt_wrong(id, message))
            .await?)
    }

    async fn prompt_done(&self, id: u64) -> fdo::Result<()> {
        Ok(self.to_loop.call(move |state| state.prompt_done(id)).await?)
    }

    /// The island's Show or Cancel on a waiting prompt.
    async fn prompt_activity(&self, action: String) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.prompt_activity(&action))
            .await?)
    }

    /// Makes the lock's face panic once, as a fault in its tree would.
    async fn panic_lock(&self) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(|state| {
                state.isle.lock.face.injected += 1;
                state.refresh_lock_face();
            })
            .await?)
    }

    /// Ends the lock with no password, as only a test may.
    async fn unlock(&self) -> fdo::Result<()> {
        Ok(self.to_loop.call(|state| state.unlock("the Test interface")).await?)
    }

    /// What `output`'s last frame was made of, front to back: `client` for any client's
    /// surface, `shell`, `wallpaper`, `solid` and `cursor`.
    async fn frame_kinds(&self, output: String) -> fdo::Result<Vec<String>> {
        Ok(self
            .to_loop
            .call(move |state| {
                state
                    .isle
                    .lock
                    .frame_kinds
                    .borrow()
                    .get(&output)
                    .map(|k| k.iter().map(|k| k.to_string()).collect())
                    .unwrap_or_default()
            })
            .await?)
    }

    /// The banner stack as JSON.
    async fn banners(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.banners_status()).await?)
    }

    /// Drops the tray's watcher as a crash would; it owns its name again by itself.
    async fn tray_kill(&self) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(|state| {
                if let Some(kill) = &state.isle.services.tray_kill {
                    kill.notify_one();
                }
            })
            .await?)
    }

    /// Gives an output a backlight named `test` at `current` of `max`, set through logind.
    async fn fake_screen(&self, connector: String, current: u32, max: u32) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.fake_screen(connector, current, max))
            .await?)
    }

    fn stall(&self, millis: u32) {
        self.to_loop.post(move |state| {
            let _beat = state.isle.heartbeat.beat("test-stall");
            std::thread::sleep(std::time::Duration::from_millis(millis.into()));
        });
    }

    /// A key by its evdev code, from a test keyboard with the Mac profile.
    async fn key(&self, code: u32, pressed: bool) -> fdo::Result<()> {
        use smithay::backend::input::{InputTime, KeyState};
        let key_state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        Ok(self
            .to_loop
            .call(move |state| {
                if state.isle.keyboards.get("test").is_none() {
                    state.keyboard_added(
                        "test",
                        "Isle test keyboard",
                        Some(isle_keys::profile::Profile::Mac),
                    );
                }
                state.on_device_key(
                    "test",
                    "Isle test keyboard",
                    code,
                    key_state,
                    InputTime::now(),
                )
            })
            .await?)
    }

    /// Adds a keyboard by name, as if plugged in; its id for `key_on`.
    async fn add_keyboard(&self, name: String) -> fdo::Result<String> {
        Ok(self
            .to_loop
            .call(move |state| {
                let id = format!("test:{}", state.isle.keyboards.devices.len());
                state.keyboard_added(&id, &name, None);
                id
            })
            .await?)
    }

    async fn remove_keyboard(&self, id: String) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.keyboard_removed(&id))
            .await?)
    }

    /// Held chords told of their release so far.
    async fn held_releases(&self) -> fdo::Result<u32> {
        Ok(self.to_loop.call(|state| state.isle.held_releases).await?)
    }

    /// The switcher while held, as JSON, or `null`.
    async fn switcher(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.switcher_json()).await?)
    }

    /// Each update of the switcher's previews in microseconds, while measuring.
    async fn switcher_updates(&self) -> fdo::Result<Vec<u64>> {
        Ok(self
            .to_loop
            .call(|state| std::mem::take(&mut state.isle.switcher_updates))
            .await?)
    }

    /// Times the switcher's panel has been drawn.
    async fn switcher_draws(&self) -> fdo::Result<u32> {
        Ok(self.to_loop.call(|state| state.isle.switcher_drawn_count).await?)
    }

    /// A key from a keyboard `add_keyboard` made.
    async fn key_on(&self, id: String, code: u32, pressed: bool) -> fdo::Result<()> {
        use smithay::backend::input::{InputTime, KeyState};
        let key_state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        self.to_loop
            .call(move |state| {
                let Some(name) = state.isle.keyboards.get(&id).map(|d| d.name.clone()) else {
                    return Err(fdo::Error::InvalidArgs(format!("no keyboard {id}")));
                };
                state.on_device_key(&id, &name, code, key_state, InputTime::now());
                Ok(())
            })
            .await?
    }

    /// Moves the pointer to a point in the global space.
    async fn pointer_to(&self, x: f64, y: f64) -> fdo::Result<()> {
        use smithay::backend::input::InputTime;
        Ok(self
            .to_loop
            .call(move |state| state.on_pointer_to((x, y).into(), InputTime::now()))
            .await?)
    }

    /// Moves the pointer by a relative amount, as a mouse does, held to the outputs.
    async fn pointer_by(&self, dx: f64, dy: f64) -> fdo::Result<()> {
        use smithay::backend::input::InputTime;
        Ok(self
            .to_loop
            .call(move |state| state.on_pointer_by((dx, dy).into(), (dx, dy).into(), InputTime::now()))
            .await?)
    }

    /// A button by its evdev code: 272 is the left button.
    async fn button(&self, button: u32, pressed: bool) -> fdo::Result<()> {
        use smithay::backend::input::{ButtonState, InputTime};
        let button_state = if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };
        Ok(self
            .to_loop
            .call(move |state| state.on_button(button, button_state, InputTime::now()))
            .await?)
    }

    /// Scrolls as a mouse wheel does, in steps of 120 per notch; positive is down.
    async fn axis(&self, v120_x: i32, v120_y: i32) -> fdo::Result<()> {
        use smithay::{
            backend::input::{Axis, AxisSource, InputTime},
            input::pointer::AxisFrame,
        };
        Ok(self
            .to_loop
            .call(move |state| {
                let mut frame = AxisFrame::new(InputTime::now()).source(AxisSource::Wheel);
                for (axis, v120) in [(Axis::Horizontal, v120_x), (Axis::Vertical, v120_y)] {
                    if v120 != 0 {
                        frame = frame
                            .value(axis, v120 as f64 * 15.0 / 120.0)
                            .v120(axis, v120);
                    }
                }
                state.on_axis(frame);
            })
            .await?)
    }

    /// Starts or stops the full-output probe's tile fades.
    async fn animate_probe_full(&self, on: bool) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| {
                let probe = state.isle.probe_full.clone();
                state.isle.shell.with_root("probe-full", || probe.set_animating(on));
                state.dispatch_shell();
            })
            .await?)
    }

    /// Times every surface draw, waiting for the GPU after each.
    async fn measure(&self, on: bool) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.shell.set_measure(on))
            .await?)
    }

    /// Turns shell transitions off or on.
    async fn transitions(&self, on: bool) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.shell.set_animated(on))
            .await?)
    }

    /// The X display once Xwayland's window manager runs, else -1.
    async fn x_display(&self) -> fdo::Result<i32> {
        Ok(self
            .to_loop
            .call(|state| match (&state.isle.xwm, state.isle.xdisplay) {
                (Some(_), Some(display)) => display as i32,
                _ => -1,
            })
            .await?)
    }

    /// Turns window motion off or on, apart from the shell's transitions.
    async fn window_motion(&self, on: bool) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.window_motion = on)
            .await?)
    }

    /// Bytes the allocator has handed out and not had back, from glibc's `mallinfo2`.
    fn malloc_in_use(&self) -> u64 {
        // Safety: mallinfo2 only reads the allocator's counters.
        let info = unsafe { libc::mallinfo2() };
        (info.uordblks + info.hblkhd) as u64
    }

    /// The counts inside a surface's tree, for finding what grows.
    async fn surface_debug(&self, name: String) -> fdo::Result<String> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.shell.debug(&name).unwrap_or_default())
            .await?)
    }

    /// Makes the probe panic when it next renders, handles a key, or draws: `render`, `event`
    /// or `draw`.
    async fn panic_probe(&self, fault: String) -> fdo::Result<()> {
        use isle_shell::probe::Fault;
        let fault = match fault.as_str() {
            "render" => Fault::Render,
            "event" => Fault::Event,
            "draw" => Fault::Draw,
            other => return Err(fdo::Error::InvalidArgs(format!("no fault {other}"))),
        };
        Ok(self
            .to_loop
            .call(move |state| {
                let probe = state.isle.probe.clone();
                state.isle.shell.with_root("probe", || probe.arm(fault));
                state.dispatch_shell();
            })
            .await?)
    }

    /// The probe surface's model: the field's text, the presses counted, whether it animates.
    async fn probe(&self) -> fdo::Result<(String, u32, bool)> {
        Ok(self
            .to_loop
            .call(|state| {
                let model = state.isle.probe.model.borrow();
                (model.text.clone(), model.presses, model.animating)
            })
            .await?)
    }

    /// Adds a virtual display; headless only.
    #[allow(clippy::too_many_arguments)]
    async fn add_output(
        &self,
        name: String,
        make: String,
        model: String,
        serial: String,
        width: i32,
        height: i32,
    ) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| match &mut state.backend {
                crate::backend::Backend::Headless(headless) => headless
                    .add_output(
                        &mut state.isle,
                        &name,
                        (&make, &model, &serial),
                        (width, height),
                    )
                    .map(|_| ()),
                _ => Err("outputs are added only to the headless backend".into()),
            })
            .await?
            .map_err(fdo::Error::Failed)
    }

    async fn remove_output(&self, name: String) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| match &mut state.backend {
                crate::backend::Backend::Headless(headless) => {
                    headless.remove_output(&mut state.isle, &name)
                }
                _ => false,
            })
            .await?)
    }

    /// Frames presented on an output since it was added.
    async fn frames(&self, output: String) -> fdo::Result<u64> {
        self.to_loop
            .call(move |state| {
                state
                    .isle
                    .space
                    .outputs()
                    .find(|o| o.name() == output)
                    .map(|o| crate::redraw::OutputState::of(o).borrow().frames_presented)
            })
            .await?
            .ok_or_else(|| fdo::Error::InvalidArgs("no such output".into()))
    }

    /// Draws an output and writes it to `path` as a PNG.
    async fn screenshot(&self, output: String, path: String) -> fdo::Result<()> {
        let pixels = self
            .to_loop
            .call(move |state| {
                let target = state
                    .isle
                    .space
                    .outputs()
                    .find(|o| o.name() == output)
                    .cloned();
                match target {
                    Some(target) => {
                        state.draw_shell(&target);
                        state.backend.screenshot(&state.isle, &target)
                    }
                    None => Err("no such output".into()),
                }
            })
            .await?
            .map_err(fdo::Error::Failed)?;
        tokio::task::spawn_blocking(move || write_png(&path, &pixels))
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Starts a stream: `{"screen": name}`, `{"window": id}` or `{"portion": name, "x",
    /// "y", "w", "h"}`, with `"cursor": "embedded"` to draw the pointer in. Returns its id.
    async fn cast_start(&self, spec: String) -> fdo::Result<u64> {
        self.to_loop
            .call(move |state| state.test_cast_start(&spec))
            .await?
            .map_err(fdo::Error::Failed)
    }

    /// Starts a recording of a source given as `CastStart` takes it.
    async fn record(&self, spec: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.test_record(&spec))
            .await?
            .map_err(fdo::Error::Failed)
    }

    /// The recording as JSON, or `null`.
    async fn recording(&self) -> fdo::Result<String> {
        Ok(self
            .to_loop
            .call(|state| state.recording_status().to_string())
            .await?)
    }

    /// Stops the recording as its Stop does.
    async fn stop_recording(&self) -> fdo::Result<bool> {
        Ok(self.to_loop.call(|state| state.stop_recording()).await?)
    }

    /// Screen shares as JSON: the picker and the sessions.
    async fn share(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.share_status().to_string()).await?)
    }

    /// Each stream frame's render time on the loop in microseconds since measuring began.
    async fn cast_times(&self) -> fdo::Result<Vec<u64>> {
        Ok(self.to_loop.call(|state| state.isle.casting.times.clone()).await?)
    }

    /// The capture protocols' sessions as JSON.
    async fn copies(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.copy_status().to_string()).await?)
    }

    /// The streams as JSON.
    async fn casts(&self) -> fdo::Result<String> {
        Ok(self
            .to_loop
            .call(|state| state.casts_status().to_string())
            .await?)
    }

    /// Ends a stream from Isle's side.
    async fn cast_stop(&self, id: u64) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.end_cast(id, crate::cast::Ended::Stopped))
            .await?)
    }

    /// The capture toolbar as JSON.
    async fn toolbar(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.toolbar_status()).await?)
    }

    /// The thumbnail after a shot as JSON, or `null`.
    async fn thumbnail(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.thumbnail_status()).await?)
    }

    /// Acts as an item of the thumbnail's menu would: `save-screenshots`,
    /// `save-documents`, `save-clipboard`, `copy`, `trash` or `close`.
    async fn thumbnail_act(&self, item: String) -> fdo::Result<()> {
        self.to_loop
            .call(move |state| state.thumbnail_test_act(&item))
            .await?
            .map_err(fdo::Error::Failed)
    }

    /// Shift+Cmd+4's pick as JSON, or `null`.
    async fn pick(&self) -> fdo::Result<String> {
        Ok(self.to_loop.call(|state| state.pick_status()).await?)
    }

    /// Takes a still as `Isle::test_capture` reads `spec` and returns the files written.
    async fn capture(&self, spec: String) -> fdo::Result<Vec<String>> {
        let written = self
            .to_loop
            .call(move |state| state.test_capture(&spec))
            .await?
            .map_err(fdo::Error::Failed)?;
        let written = tokio::time::timeout(std::time::Duration::from_secs(10), written)
            .await
            .map_err(|_| fdo::Error::Failed("timed out".into()))?
            .map_err(|_| fdo::Error::Failed("nothing written".into()))?;
        Ok(written
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect())
    }

    fn panic_service(&self) {
        self.to_loop
            .post(|state| state.isle.notify(crate::worker::Event::PanicService));
    }

    fn panic_core(&self) {
        self.to_loop
            .post(|_| panic!("a core panic injected through the Test interface"));
    }
}

struct Settings {
    store: SharedStore,
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
}

fn to_variant(value: Value) -> OwnedValue {
    let value = match value {
        Value::Bool(b) => zvariant::Value::from(b),
        Value::Int(i) => zvariant::Value::from(i),
        Value::Float(f) => zvariant::Value::from(f),
        Value::Str(s) => zvariant::Value::from(s),
        Value::List(l) => zvariant::Value::from(l),
    };
    OwnedValue::try_from(value).expect("a value without file descriptors")
}

fn from_variant(value: &zvariant::Value<'_>) -> fdo::Result<Value> {
    Ok(match value {
        zvariant::Value::Bool(b) => Value::Bool(*b),
        zvariant::Value::I32(i) => Value::Int((*i).into()),
        zvariant::Value::U32(i) => Value::Int((*i).into()),
        zvariant::Value::I64(i) => Value::Int(*i),
        zvariant::Value::F64(f) => Value::Float(*f),
        zvariant::Value::Str(s) => Value::Str(s.to_string()),
        zvariant::Value::Array(a) => Value::List(
            a.iter()
                .map(|v| match v {
                    zvariant::Value::Str(s) => Ok(s.to_string()),
                    _ => Err(fdo::Error::InvalidArgs("a list holds strings".into())),
                })
                .collect::<fdo::Result<_>>()?,
        ),
        zvariant::Value::Value(inner) => return from_variant(inner),
        other => {
            return Err(fdo::Error::InvalidArgs(format!(
                "a setting is a bool, number, string or list of strings, not {other}"
            )));
        }
    })
}

#[interface(name = "io.github.alexparlett.Isle.Settings1")]
impl Settings {
    async fn get(&self, key: String) -> fdo::Result<OwnedValue> {
        let value = self
            .store
            .lock()
            .await
            .get(&key)
            .map_err(fdo::Error::InvalidArgs)?;
        Ok(to_variant(value))
    }

    /// Validates, writes and applies; returns the keys that changed.
    async fn set(&self, key: String, value: zvariant::Value<'_>) -> fdo::Result<Vec<String>> {
        let value = from_variant(&value)?;
        let mut store = self.store.lock().await;
        let keys = store.set(&key, value).map_err(|err| match err {
            SetError::Invalid(message) => fdo::Error::InvalidArgs(message),
            other => fdo::Error::Failed(other.to_string()),
        })?;
        crate::settings::publish(
            &self.to_loop,
            &self.events,
            store.current().clone(),
            keys.clone(),
        );
        Ok(keys)
    }

    async fn schema(&self) -> Vec<SettingKey> {
        schema_of::<isle_settings::schema::Settings>()
            .into_iter()
            .map(|k| SettingKey {
                key: k.key,
                kind: k.kind.into(),
                default: k.default,
                constraint: k.constraint,
                doc: k.doc,
            })
            .collect()
    }

    /// Reads settings.kdl and puts it in force, after a safe start.
    async fn use_mine(&self) -> fdo::Result<Vec<String>> {
        let mut store = self.store.lock().await;
        match store.use_file() {
            isle_settings::Reload::Rejected(err) => Err(fdo::Error::Failed(err.to_string())),
            isle_settings::Reload::Applied(keys) => {
                crate::settings::publish(
                    &self.to_loop,
                    &self.events,
                    store.current().clone(),
                    keys.clone(),
                );
                self.to_loop.post(|state| {
                    state.isle.safe_start = false;
                    state.island_settings_status(None);
                });
                let _ = self.events.send(Event::SettingsStatus);
                Ok(keys)
            }
            isle_settings::Reload::Unchanged => {
                self.to_loop.post(|state| {
                    state.isle.safe_start = false;
                    state.island_settings_status(None);
                });
                let _ = self.events.send(Event::SettingsStatus);
                Ok(Vec::new())
            }
        }
    }

    async fn status(&self) -> SettingsStatus {
        let store = self.store.lock().await;
        match store.error() {
            None => SettingsStatus {
                ok: true,
                file: store.path().display().to_string(),
                line: 0,
                column: 0,
                message: String::new(),
            },
            Some(err) => SettingsStatus {
                ok: false,
                file: err.file.display().to_string(),
                line: err.line as u32,
                column: err.column as u32,
                message: err.message.clone(),
            },
        }
    }

    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, keys: Vec<String>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

#[cfg(feature = "test-hooks")]
fn write_png(
    path: &str,
    pixels: &crate::backend::Pixels,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, pixels.width, pixels.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels.rgba)?;
    Ok(())
}

pub async fn serve(
    to_loop: ToLoop,
    events: broadcast::Sender<Event>,
    store: SharedStore,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut receiver = events.subscribe();
    let builder = zbus::connection::Builder::session()?
        .serve_at(
            SETTINGS_PATH,
            Settings {
                store: store.clone(),
                to_loop: to_loop.clone(),
                events: events.clone(),
            },
        )?
        .serve_at(VERSION_PATH, Version)?
        .serve_at(
            SESSION_PATH,
            Session {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            WINDOWS_PATH,
            Windows {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            SHELL_PATH,
            Shell {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            DISPLAYS_PATH,
            Displays {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::DESKTOPS_PATH,
            Desktops {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::MENUS_PATH,
            MenusIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::ISLAND_PATH,
            IslandIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::FOCUS_PATH,
            FocusIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::NOTIFICATIONS_PATH,
            NotificationsIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::CAPTURE_PATH,
            CaptureIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::SERVICES_PATH,
            ServicesIface {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::WALLPAPER_PATH,
            Wallpaper {
                to_loop: to_loop.clone(),
            },
        )?
        .serve_at(
            isle_ipc::KEYBOARD_PATH,
            Keyboard {
                to_loop: to_loop.clone(),
                store: store.clone(),
                events: events.clone(),
            },
        )?;
    #[cfg(feature = "test-hooks")]
    let builder = builder.serve_at(
        isle_ipc::TEST_PATH,
        Test {
            to_loop: to_loop.clone(),
        },
    )?;
    let connection = builder.name(BUS_NAME)?.build().await?;
    info!(name = BUS_NAME, "on the session bus");

    let server = connection.object_server();
    loop {
        match receiver.recv().await {
            Ok(Event::WindowsChanged) => {
                let iface = server.interface::<_, Windows>(WINDOWS_PATH).await?;
                Windows::changed(iface.signal_emitter()).await?;
            }
            Ok(Event::SettingsChanged(keys)) => {
                let iface = server.interface::<_, Settings>(SETTINGS_PATH).await?;
                Settings::changed(iface.signal_emitter(), keys).await?;
            }
            Ok(Event::SettingsStatus) => {
                let iface = server.interface::<_, Settings>(SETTINGS_PATH).await?;
                Settings::status_changed(iface.signal_emitter()).await?;
            }
            Ok(Event::DesktopsChanged) => {
                let iface = server
                    .interface::<_, Desktops>(isle_ipc::DESKTOPS_PATH)
                    .await?;
                Desktops::changed(iface.signal_emitter()).await?;
            }
            Ok(Event::KeyboardsChanged) => {
                let iface = server
                    .interface::<_, Keyboard>(isle_ipc::KEYBOARD_PATH)
                    .await?;
                Keyboard::changed(iface.signal_emitter()).await?;
            }
            Ok(Event::FocusChanged) => {
                let iface = server.interface::<_, FocusIface>(isle_ipc::FOCUS_PATH).await?;
                FocusIface::changed(iface.signal_emitter()).await?;
            }
            Ok(Event::NotificationsChanged) => {
                let iface = server
                    .interface::<_, NotificationsIface>(isle_ipc::NOTIFICATIONS_PATH)
                    .await?;
                NotificationsIface::changed(iface.signal_emitter()).await?;
            }
            Ok(Event::LockChanged) => {
                let iface = server.interface::<_, Session>(SESSION_PATH).await?;
                Session::lock_changed(iface.signal_emitter()).await?;
            }
            Ok(Event::DisplaysChanged) => {
                let iface = server.interface::<_, Displays>(DISPLAYS_PATH).await?;
                Displays::changed(iface.signal_emitter()).await?;
            }
            #[cfg(feature = "test-hooks")]
            Ok(Event::PanicService) => {
                panic!("a service panic injected through the Test interface")
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {}
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}
