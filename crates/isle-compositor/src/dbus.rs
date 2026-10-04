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
        Ok(self
            .to_loop
            .call(move |state| match state.isle.window_by_id(id) {
                Some(window) => {
                    state.set_focus(Some(&window));
                    true
                }
                None => false,
            })
            .await?)
    }

    async fn close(&self, id: u64) -> fdo::Result<bool> {
        Ok(self
            .to_loop
            .call(move |state| match state.isle.window_by_id(id) {
                Some(window) => {
                    window.toplevel().unwrap().send_close();
                    true
                }
                None => false,
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
                    })
                    .collect()
            })
            .await?)
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
    fn stall(&self, millis: u32) {
        self.to_loop.post(move |state| {
            let _beat = state.isle.heartbeat.beat("test-stall");
            std::thread::sleep(std::time::Duration::from_millis(millis.into()));
        });
    }

    /// A key by its evdev code, as a keyboard sends it.
    async fn key(&self, code: u32, pressed: bool) -> fdo::Result<()> {
        use smithay::backend::input::{InputTime, KeyState};
        let key_state = if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };
        let keycode = smithay::input::keyboard::Keycode::new(code + 8);
        Ok(self
            .to_loop
            .call(move |state| state.on_key(keycode, key_state, InputTime::now()))
            .await?)
    }

    /// Moves the pointer to a point in the global space.
    async fn pointer_to(&self, x: f64, y: f64) -> fdo::Result<()> {
        use smithay::backend::input::InputTime;
        Ok(self
            .to_loop
            .call(move |state| state.on_pointer_to((x, y).into(), InputTime::now()))
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

    /// Turns shell transitions off or on.
    async fn transitions(&self, on: bool) -> fdo::Result<()> {
        Ok(self
            .to_loop
            .call(move |state| state.isle.shell.set_animated(on))
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
                let _ = self.events.send(Event::SettingsStatus);
                Ok(keys)
            }
            isle_settings::Reload::Unchanged => {
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
                store,
                to_loop: to_loop.clone(),
                events,
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
