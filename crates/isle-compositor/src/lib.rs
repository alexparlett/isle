//! The compositor: one calloop loop, its backends, outputs, input and shells (B9).

pub mod backend;
pub mod build_info;
pub mod crash;
mod grabs;
mod handlers;
mod input;
pub mod redraw;
mod render;
pub mod state;
pub mod watchdog;

use std::time::Duration;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
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
    let (winit, events) = Winit::new()?;
    let backend = Backend::Winit(winit);
    let isle = Isle::new(
        display,
        event_loop.handle(),
        event_loop.get_signal(),
        heartbeat,
        backend.seat_name(),
    );
    let mut state = State { backend, isle };
    backend::winit::setup(&mut state, events)?;

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
