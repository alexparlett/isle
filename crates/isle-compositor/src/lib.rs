//! The compositor: one calloop loop, its backends, outputs, input and shells (B9).

pub mod backend;
mod binds;
pub mod build_info;
pub mod crash;
mod dbus;
mod grabs;
mod handlers;
mod input;
pub mod redraw;
mod render;
pub mod settings;
pub mod state;
pub mod watchdog;
mod windows;
pub mod worker;

use std::time::Duration;

use smithay::reexports::{
    calloop::{EventLoop, channel},
    wayland_server::Display,
};
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{
    backend::{Backend, winit::Winit},
    state::{Isle, State},
    watchdog::Heartbeat,
};

/// A main-loop dispatch stuck this long ends the process (choice 7).
const HANG_LIMIT: Duration = Duration::from_secs(10);

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let heartbeat = Heartbeat::new();
    crash::install_panic_hook(heartbeat.clone());
    info!("isle-compositor {}", build_info::describe());

    let store: isle_settings::Store<isle_settings::schema::Settings> =
        isle_settings::Store::open(settings::settings_path());
    if let Some(err) = store.error() {
        warn!("settings.kdl rejected, starting on the defaults: {err}");
    }
    let initial = store.current().clone();
    heartbeat.set_abort(initial.debug.watchdog_abort);
    heartbeat.spawn_watchdog(HANG_LIMIT);

    let mut event_loop: EventLoop<'static, State> = EventLoop::try_new()?;
    let display: Display<State> = Display::new()?;
    let (winit, winit_events) = Winit::new()?;
    let backend = Backend::Winit(winit);

    let (jobs, job_source) = channel::channel::<worker::Job>();
    event_loop
        .handle()
        .insert_source(job_source, |event, _, state| {
            if let channel::Event::Msg(job) = event {
                let _beat = state.isle.heartbeat.beat("worker-job");
                job(state);
            }
        })
        .map_err(|e| e.error)?;
    let (events, _) = broadcast::channel(64);
    let store = std::sync::Arc::new(tokio::sync::Mutex::new(store));
    worker::spawn(worker::ToLoop::new(jobs), events.clone(), store);

    let isle = Isle::new(
        display,
        event_loop.handle(),
        event_loop.get_signal(),
        heartbeat,
        events,
        initial.clone(),
        backend.seat_name(),
    );
    let mut state = State { backend, isle };
    state.apply_settings(initial, None);
    backend::winit::setup(&mut state, winit_events)?;

    // Children connect to Isle, not to the compositor this one may be nested in.
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.isle.socket_name) };

    event_loop.run(None, &mut state, |state| {
        state.redraw_queued();
        state.isle.space.refresh();
        state.isle.popups.cleanup();
        let _ = state.isle.display_handle.flush_clients();
    })?;
    info!("exiting");
    Ok(())
}
