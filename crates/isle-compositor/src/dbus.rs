//! The compositor's D-Bus API, served on the worker (`platform.md`, Compositor D-Bus API).

use std::error::Error;

use isle_ipc::{
    BUS_NAME, DISPLAYS_PATH, DisplayInfo, SESSION_PATH, VERSION_PATH, WINDOWS_PATH, WindowInfo,
};
use tokio::sync::broadcast;
use tracing::info;
use zbus::{fdo, interface, object_server::SignalEmitter};

use crate::worker::{Event, LoopTimeout, ToLoop};

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

pub async fn serve(
    to_loop: ToLoop,
    mut events: broadcast::Receiver<Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let builder = zbus::connection::Builder::session()?
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
        match events.recv().await {
            Ok(Event::WindowsChanged) => {
                let iface = server.interface::<_, Windows>(WINDOWS_PATH).await?;
                Windows::changed(iface.signal_emitter()).await?;
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
