//! The compositor's D-Bus API, served on the worker (`platform.md`, Compositor D-Bus API).

use std::error::Error;

use isle_ipc::{
    BUS_NAME, DISPLAYS_PATH, DisplayInfo, SESSION_PATH, SETTINGS_PATH, SettingKey, SettingsStatus,
    VERSION_PATH, WINDOWS_PATH, WindowInfo,
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
