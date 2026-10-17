//! The compositor: one calloop loop, its backends, outputs, input and shells (B9).

pub mod backend;
mod badges;
mod banners;
mod binds;
mod bluetooth_agent;
pub mod build_info;
pub mod crash;
mod cursor;
mod desktop_notes;
mod dbus;
mod files;
mod focus;
mod focus_modes;
pub mod freeze;
mod force_quit;
mod games;
mod grabs;
mod handlers;
mod idle;
mod image_copy;
mod input;
mod keyboard;
mod island;
pub mod capture;
pub mod cast;
mod centre;
mod child_env;
mod clipboard;
mod control_centre;
mod control_edit;
mod keyring;
mod launch;
mod lock;
mod lock_face;
mod menubar;
mod menus;
mod network_secrets;
mod notifications;
mod spotlight;
mod night_light;
mod steam;
mod switcher;
mod vpn_secrets;
mod top_edge;
mod osd;
mod outputs;
mod polkit;
mod processes;
mod prompts;
mod screensaver;
mod services;
mod session_end;
mod pointer;
pub mod present;
pub mod redraw;
mod sync;
mod render;
pub mod settings;
pub mod state;
pub mod state_writer;
mod portal;
mod recording;
mod settings_link;
mod share;
mod thumbnail;
mod titlebar_overlay;
mod toolbar;
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
    // Read before any thread starts; no child sees it.
    let ready_fd = std::env::var("ISLE_READY_FD")
        .ok()
        .and_then(|fd| fd.parse::<i32>().ok());
    unsafe { std::env::remove_var("ISLE_READY_FD") };
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

    let (token_source, token_text, token_error) = match tokens::load(&tokens::tokens_path()) {
        Ok((source, text)) => (source, text, None),
        Err(err) => {
            warn!("tokens.kdl rejected, the shipped tokens stand: {err}");
            (isle_tokens::Source::shipped(), String::new(), Some(err))
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
    isle.tokens_text = token_text;
    isle.tokens_error = token_error;
    isle.keyboards.set_tables(keyboard::load_tables(&keyboard::tables_path()));
    isle.keyboards.system = isle_keys::keymap::Layouts::system();
    isle.worker = Some(runtime.clone());
    isle.to_loop = Some(to_loop.clone());
    isle.settings_store = Some(store);
    isle.load_places(&runtime);
    isle.load_usage(&runtime);
    isle.load_capture_state(&runtime);
    isle.load_desktops(&runtime);
    isle.wallpapers.load(&runtime);
    isle.safe_start = safe_start;
    if lock::marked() {
        isle.start_locked();
    }
    let mut state = State { backend, isle };
    state.apply_settings(initial, None);
    match (winit_events, session_events) {
        (Some(events), _) => backend::winit::setup(&mut state, events)?,
        (_, Some(notifier)) => backend::udev::setup(&mut state, notifier)?,
        _ => backend::headless::setup(&mut state)?,
    }
    state.sync_island();
    state.locked_at_start();
    state.arm_idle();
    state.pace_hidden_windows();
    state.start_night_light();

    // Children connect to Isle, not to the compositor this one may be nested in.
    child_env::set(
        "WAYLAND_DISPLAY",
        state.isle.socket_name.to_string_lossy().into_owned(),
    );
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
            state.crash_note();
        }
        Ok((_, Err(isle_notifications::Taken))) => {
            warn!("another program serves notifications; Isle's banners will be empty")
        }
        Err(err) => warn!("no notification server: {err}"),
    }
    // Players and games ask it to hold the screen awake from their start.
    let saver = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let owned = screensaver::own(&conn, to_loop.clone()).await?;
        Ok::<_, zbus::Error>((conn, owned))
    });
    match saver {
        Ok((conn, true)) => {
            let (loop_s, events) = (to_loop.clone(), state.isle.events.clone());
            runtime.spawn(worker::supervise("screensaver", move || {
                screensaver::follow(conn.clone(), loop_s.clone(), events.clone())
            }));
        }
        Ok((_, false)) => warn!("another program serves org.freedesktop.ScreenSaver"),
        Err(err) => warn!("no ScreenSaver interface: {err}"),
    }
    // gnome-keyring, gcr's SSH agent and pinentry ask Isle's prompter, never gcr's own.
    let keyring = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let owned = keyring::own(&conn, prompts::PromptClient(to_loop.clone())).await?;
        Ok::<_, zbus::Error>((conn, owned))
    });
    match keyring {
        Ok((conn, true)) => std::mem::forget(conn),
        Ok((_, false)) => warn!("another program serves the keyring's prompts"),
        Err(err) => warn!("no keyring prompter: {err}"),
    }
    // Apps look for the watcher once at start, as for the registrar.
    let tray = runtime.block_on(async {
        let conn = zbus::Connection::session().await?;
        let owned = isle_tray::own(&conn).await?;
        Ok::<_, isle_bus::Error>(owned.ok().map(|host| (conn, host)))
    });
    let tray = tray.unwrap_or_else(|err| {
        warn!("no tray watcher: {err}");
        None
    });
    state.isle.services.tray_kill = Some(services::tray::start(&runtime, &to_loop, tray));
    // The portal frontend reads a backend once at its start: Isle owns the name before the
    // session's apps can start the frontend.
    *state.isle.portal_appearance.lock().unwrap() = portal::appearance_of(state.isle.shell.theme());
    state.isle.portal = portal::own(&runtime, to_loop.clone(), state.isle.portal_appearance.clone());
    let mut ready_fd = ready_fd;
    if !start_clients(&mut state, &mut ready_fd) {
        info!("Xwayland and the session's apps wait for the GPU");
    }

    event_loop.run(None, &mut state, |state| {
        if !state.isle.clients_started {
            start_clients(state, &mut ready_fd);
        }
        // The space learns which outputs each window overlaps in its refresh, and a
        // redraw reads that.
        state.isle.space.refresh();
        state.isle.popups.cleanup();
        state.switcher_after_dispatch();
        state.sync_foreign_toplevels();
        state.idle_after_dispatch();
        // Input this iteration reaches clients before a frame is drawn, not after.
        let _ = state.isle.display_handle.flush_clients();
        state.redraw_queued();
        let _ = state.isle.display_handle.flush_clients();
    })?;
    info!("exiting");
    if let backend::Backend::Udev(udev) = &mut state.backend {
        udev.restore_gamma();
    }
    match state.isle.exit_error.take() {
        Some(err) => Err(err.into()),
        None => Ok(()),
    }
}

/// Once the GPU serves dmabufs, starts Xwayland and tells the supervisor Isle is up, so the
/// session's apps start. Clients read linux-dmabuf and syncobj once, as they start: an
/// Xwayland started before them runs without glamor and draws every X11 window and game in
/// shared memory. Returns whether that has happened.
fn start_clients(state: &mut State, ready_fd: &mut Option<i32>) -> bool {
    if state.isle.clients_started {
        return true;
    }
    if !state.backend.renders() {
        return false;
    }
    state.isle.clients_started = true;
    state.warm_up();
    let display = xwayland::start(state, None);
    state.isle.xdisplay = display;
    if let Some(display) = display {
        child_env::set("DISPLAY", format!(":{display}"));
    }
    if let Some(fd) = ready_fd.take() {
        report_ready(fd, &state.isle.socket_name, display);
    }
    true
}

/// Tells the supervisor the socket is up, on the fd it named in `ISLE_READY_FD`.
fn report_ready(fd: i32, socket: &std::ffi::OsStr, display: Option<u32>) {
    use std::{io::Write, os::fd::FromRawFd};
    // Safety: the supervisor opened this fd for us alone; it is closed here.
    let mut pipe = unsafe { std::fs::File::from_raw_fd(fd) };
    let _ = writeln!(pipe, "WAYLAND_DISPLAY={}", socket.to_string_lossy());
    if let Some(display) = display {
        let _ = writeln!(pipe, "DISPLAY=:{display}");
    }
}
