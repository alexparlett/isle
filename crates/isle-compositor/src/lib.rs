//! The compositor: one calloop loop, its backends, outputs, input and shells (B9).

pub mod backend;
pub mod build_info;
pub mod crash;
mod dbus;
mod grabs;
mod handlers;
mod input;
pub mod redraw;
mod render;
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
use tracing::info;

use crate::{
    backend::{Backend, winit::Winit},
    state::{Isle, State},
    watchdog::Heartbeat,
};

/// A main-loop dispatch stuck this long ends the process (choice 7).
const HANG_LIMIT: Duration = Duration::from_secs(10);

pub struct Options {
    pub watchdog_abort: bool,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let heartbeat = Heartbeat::new();
    crash::install_panic_hook(heartbeat.clone());
    heartbeat.spawn_watchdog(options.watchdog_abort.then_some(HANG_LIMIT));
    info!("isle-compositor {}", build_info::describe());

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
    worker::spawn(worker::ToLoop::new(jobs), events.clone());

    let isle = Isle::new(
        display,
        event_loop.handle(),
        event_loop.get_signal(),
        heartbeat,
        events,
        backend.seat_name(),
    );
    let mut state = State { backend, isle };
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
