//! The compositor: one calloop loop, its backends, outputs, input and shells (B9).

pub mod backend;
mod badges;
mod banners;
mod binds;
pub mod build_info;
pub mod crash;
mod cursor;
mod dbus;
mod focus;
mod focus_modes;
mod games;
mod grabs;
mod handlers;
mod input;
mod keyboard;
mod island;
mod centre;
mod control_centre;
mod launch;
mod menubar;
mod menus;
mod notifications;
mod switcher;
mod top_edge;
mod osd;
mod outputs;
mod processes;
mod services;
pub mod redraw;
mod render;
pub mod settings;
pub mod state;
pub mod state_writer;
pub mod tokens;
mod wallpaper;
pub mod watchdog;
pub mod worker;
mod wm;
mod xwayland;

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

pub fn run(kind: backend::Kind) -> Result<(), Box<dyn std::error::Error>> {
    // The supervisor reads the ready pipe to its end; no child may hold it open.
    if let Some(fd) = std::env::var("ISLE_READY_FD")
        .ok()
        .and_then(|fd| fd.parse::<i32>().ok())
    {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    let heartbeat = Heartbeat::new();
    crash::install_panic_hook(heartbeat.clone());
    info!("isle-compositor {}", build_info::describe());

    let safe_start = std::env::var("ISLE_SAFE_START").as_deref() == Ok("1");
    let store: isle_settings::Store<isle_settings::schema::Settings> = if safe_start {
        isle_settings::Store::defaults(settings::settings_path())
    } else {
        isle_settings::Store::open(settings::settings_path())
    };
    if let Some(err) = store.error() {
        warn!("starting on the default settings: {err}");
    }
    let initial = store.current().clone();
    heartbeat.set_abort(initial.debug.watchdog_abort);
    heartbeat.spawn_watchdog(HANG_LIMIT);

    let mut event_loop: EventLoop<'static, State> = EventLoop::try_new()?;
    let display: Display<State> = Display::new()?;
    let mut winit_events = None;
    let mut session_events = None;
    let backend = match kind {
        backend::Kind::Udev => {
            let (udev, notifier) = backend::udev::Udev::new(initial.render_device.as_deref())?;
            session_events = Some(notifier);
            Backend::Udev(udev)
        }
        backend::Kind::Winit => {
            let (winit, events) = Winit::new()?;
            winit_events = Some(events);
            Backend::Winit(winit)
        }
        backend::Kind::Headless => Backend::Headless(backend::headless::Headless::new()?),
    };

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
    let to_loop = worker::ToLoop::new(jobs);
    let runtime = worker::spawn(to_loop.clone(), events.clone(), store.clone());

    let (token_source, token_error) = match tokens::load(&tokens::tokens_path()) {
        Ok(source) => (source, None),
        Err(err) => {
            warn!("tokens.kdl rejected, the shipped tokens stand: {err}");
            (isle_tokens::Source::shipped(), Some(err))
        }
    };
    let theme = tokens::theme(&token_source, &initial);
    let mut isle = Isle::new(
        display,
        event_loop.handle(),
        event_loop.get_signal(),
        heartbeat,
        events,
        initial.clone(),
        backend.seat_name(),
        theme,
    );
    isle.tokens = token_source;
    isle.tokens_error = token_error;
    isle.keyboards.set_tables(keyboard::load_tables(&keyboard::tables_path()));
    isle.keyboards.system = isle_keys::keymap::Layouts::system();
    isle.worker = Some(runtime.clone());
    isle.to_loop = Some(to_loop.clone());
    isle.settings_store = Some(store);
    isle.load_places(&runtime);
    isle.load_desktops(&runtime);
    isle.wallpapers.load(&runtime);
    isle.safe_start = safe_start;
    let mut state = State { backend, isle };
    state.apply_settings(initial, None);
    match (winit_events, session_events) {
        (Some(events), _) => backend::winit::setup(&mut state, events)?,
        (_, Some(notifier)) => backend::udev::setup(&mut state, notifier)?,
        _ => backend::headless::setup(&mut state)?,
    }
    state.sync_island();

    // Children connect to Isle, not to the compositor this one may be nested in.
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.isle.socket_name) };
    let display = xwayland::start(&mut state, None);
    state.isle.xdisplay = display;
    if let Some(display) = display {
        unsafe { std::env::set_var("DISPLAY", format!(":{display}")) };
    }
    // Qt asks once per process whether the registrar has an owner, so Isle owns it before
    // any app can start.
    let owned = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let owned = isle_menus::registrar::own(&conn).await?;
        zbus::Result::Ok((conn, owned))
    });
    match owned {
        Ok((conn, Ok(registrations))) => {
            state.isle.menus.conn = Some(conn);
            let watched = registrations.clone();
            let loop_ = to_loop.clone();
            runtime.spawn(async move {
                loop {
                    watched.registered().await;
                    loop_.post(|state| state.menus_registered());
                }
            });
            state.isle.menus.registrations = Some(registrations);
        }
        Ok((conn, Err(isle_menus::registrar::Taken(owner)))) => {
            state.isle.menus.conn = Some(conn);
            state.isle.menus.taken = Some(owner);
        }
        Err(err) => warn!("no menu registrar: {err}"),
    }
    // Apps that notify at start find a server, as they find the registrar.
    state.isle.load_focus(&runtime);
    state.focus_evaluate(focus_modes::Origin::Auto);
    let first_id = state.isle.load_notifications(&runtime);
    let served = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let loop_n = to_loop.clone();
        let served = isle_notifications::serve(&conn, first_id, move |incoming| {
            loop_n.post(move |state| state.notification_incoming(incoming));
        })
        .await?;
        Ok::<_, isle_bus::Error>((conn, served))
    });
    match served {
        Ok((conn, Ok(handle))) => {
            state.isle.notifications.server = Some(handle);
            std::mem::forget(conn);
        }
        Ok((_, Err(isle_notifications::Taken))) => {
            warn!("another program serves notifications; Isle's banners will be empty")
        }
        Err(err) => warn!("no notification server: {err}"),
    }
    report_ready(&state.isle.socket_name, display);

    event_loop.run(None, &mut state, |state| {
        // The space learns which outputs each window overlaps in its refresh, and a
        // redraw reads that.
        state.isle.space.refresh();
        state.isle.popups.cleanup();
        state.switcher_after_dispatch();
        state.redraw_queued();
        let _ = state.isle.display_handle.flush_clients();
    })?;
    info!("exiting");
    Ok(())
}

/// Tells the supervisor the socket is up, on the fd it named in `ISLE_READY_FD`.
fn report_ready(socket: &std::ffi::OsStr, display: Option<u32>) {
    use std::{io::Write, os::fd::FromRawFd};
    let Some(fd) = std::env::var("ISLE_READY_FD")
        .ok()
        .and_then(|fd| fd.parse::<i32>().ok())
    else {
        return;
    };
    unsafe { std::env::remove_var("ISLE_READY_FD") };
    // Safety: the supervisor opened this fd for us alone; it is closed here.
    let mut pipe = unsafe { std::fs::File::from_raw_fd(fd) };
    let _ = writeln!(pipe, "WAYLAND_DISPLAY={}", socket.to_string_lossy());
    if let Some(display) = display {
        let _ = writeln!(pipe, "DISPLAY=:{display}");
    }
}
