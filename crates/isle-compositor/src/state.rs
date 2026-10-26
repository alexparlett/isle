//! The compositor's state: the backend beside everything else, so rendering can borrow both.

use std::{ffi::OsString, sync::Arc, time::Duration};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType, layer_map_for_output},
    input::{Seat, SeatState, keyboard::XkbConfig, pointer::CursorImageStatus},
    output::Output,
    reexports::{
        calloop::{Interest, LoopHandle, LoopSignal, Mode, PostAction, generic::Generic},
        wayland_server::{
            Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        dmabuf::DmabufState,
        fractional_scale::FractionalScaleManagerState,
        output::OutputManagerState,
        presentation::PresentationState,
        selection::{data_device::DataDeviceState, primary_selection::PrimarySelectionState},
        shell::{
            kde::decoration::KdeDecorationState,
            wlr_layer::{Layer, WlrLayerShellState},
            xdg::{XdgShellState, decoration::XdgDecorationState},
        },
        shm::ShmState,
        single_pixel_buffer::SinglePixelBufferState,
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
        xdg_activation::XdgActivationState,
        xdg_foreign::XdgForeignState,
    },
};
use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration_manager::Mode as KdeMode;
use tracing::{info, warn};

use isle_settings::schema::Settings;
use tokio::sync::broadcast;

use crate::{backend::Backend, focus::Focus, watchdog::Heartbeat, worker::Event};

pub struct State {
    pub backend: Backend,
    pub isle: Isle,
}

pub struct Isle {
    pub display_handle: DisplayHandle,
    pub loop_handle: LoopHandle<'static, State>,
    pub loop_signal: LoopSignal,
    pub clock: Clock<Monotonic>,
    pub socket_name: OsString,
    pub heartbeat: Heartbeat,
    pub events: broadcast::Sender<Event>,
    pub settings: Settings,
    pub shell: isle_shell::Shell,
    /// The design tokens in force, and why the user's file is not, when it is not.
    pub tokens: isle_tokens::Source,
    /// The user's tokens.kdl in force, for apps; empty when the shipped tokens stand alone.
    pub tokens_text: String,
    pub tokens_error: Option<isle_tokens::TokenError>,
    #[cfg(feature = "test-hooks")]
    pub probe: std::rc::Rc<isle_shell::probe::Probe>,
    #[cfg(feature = "test-hooks")]
    pub probe_full: std::rc::Rc<isle_shell::probe::ProbeFull>,
    /// The keyboard's focus before a shell surface took it, given back when it closes.
    pub shell_keyboard_return: Vec<(isle_shell::ShellFocus, Option<Focus>)>,
    pub keyboards: crate::keyboard::Keyboards,
    /// The worker, for blocking reads off the loop, and its way back.
    pub worker: Option<tokio::runtime::Handle>,
    pub to_loop: Option<crate::worker::ToLoop>,
    /// Processes Feral GameMode has registered as games.
    pub gamemode_pids: std::collections::HashSet<i32>,
    pub inhibit_state: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
    pub x11_grabs: crate::keyboard::inhibit::X11Grabs,
    /// The window the keys were taken back from, until focus moves.
    pub keys_taken_back: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
    pub recording: Option<crate::keyboard::inject::Recording>,
    /// Slow Keys: keys pressed and not yet held long enough, and keys that were.
    pub slow_pending: std::collections::HashMap<(String, u32), smithay::reexports::calloop::RegistrationToken>,
    pub slow_accepted: std::collections::HashSet<(String, u32)>,
    /// The settings store, for the few settings the keyboard writes itself.
    pub settings_store: Option<crate::settings::SharedStore>,
    /// Keys a recording took, whose releases go nowhere either.
    pub swallowed: std::collections::HashSet<(String, u32)>,
    pub xwayland_grab_state: smithay::wayland::xwayland_keyboard_grab::XWaylandKeyboardGrabState,
    pub virtual_keyboard_state: smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState,
    pub bindings: crate::binds::Bindings,
    /// A repeating shortcut's key and its timer.
    pub repeat: Option<(u32, smithay::reexports::calloop::RegistrationToken)>,
    /// A shortcut holding its chord until these modifiers are let go.
    pub held_chord: Option<(&'static str, isle_keys::chord::Mods)>,
    #[cfg(feature = "test-hooks")]
    pub held_releases: u32,

    /// The windows shown now; `wm` keeps every window.
    pub space: Space<Window>,
    pub wm: crate::wm::Wm,
    pub xwm: Option<smithay::xwayland::X11Wm>,
    /// The X display Xwayland serves, kept across its restarts.
    pub xdisplay: Option<u32>,
    /// The X11 windows by id, bottom to top, as X11 last stacked them.
    pub x11_order: Vec<u64>,
    /// Xwayland is started and the supervisor told Isle is up: both wait for the GPU.
    pub clients_started: bool,
    /// Why the main loop was stopped, when Isle cannot go on and the supervisor should
    /// start it again.
    pub exit_error: Option<String>,
    pub xwayland_deaths: u32,
    pub xwayland_source: Option<smithay::reexports::calloop::RegistrationToken>,
    pub xwayland_shell_state: smithay::wayland::xwayland_shell::XWaylandShellState,
    /// X11 windows left alone: override-redirect and untitled ones, front last.
    pub unmanaged: Vec<Window>,
    pub big_picture: crate::steam::BigPicture,
    pub night_light: crate::night_light::NightLight,
    pub night_timer: Option<smithay::reexports::calloop::RegistrationToken>,
    /// The warmth last put on the screens.
    pub night_applied: f64,
    pub night_fading: bool,
    pub frame_phases: crate::redraw::FramePhases,
    /// The surface whose pointer lock or confinement is active.
    pub constrained: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
    /// Explicit sync, once a device that can wait on sync points is open.
    pub syncobj_state: Option<smithay::wayland::drm_syncobj::DrmSyncobjState>,
    /// The surfaces with a fifo barrier or a timed commit outstanding, which each frame
    /// visits; the others are not walked.
    pub paced: std::collections::HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        smithay::reexports::wayland_server::Weak<
            smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        >,
    >,
    /// Each surface's armed wait for its next timed commit.
    pub commit_timers: std::collections::HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        (
            smithay::wayland::commit_timing::Timestamp,
            smithay::reexports::calloop::RegistrationToken,
        ),
    >,
    /// The window whose bar shows Option's glyphs.
    pub bar_option: Option<u64>,
    /// A display change in force and not yet kept, and the last change's id.
    pub display_change: Option<crate::displays::Pending>,
    pub next_display_change: u64,
    /// Surfaces that asked for a transparent bar, and the lights' inset over each.
    pub titlebar_overlays: std::collections::HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        (i32, i32),
    >,
    /// Windows floating over a caller, by their surface (`isle_float_v1`).
    pub floats: std::collections::HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        crate::titlebar_overlay::Float,
    >,
    pub zoom: crate::wm::ZoomState,
    pub zoom_menu: std::rc::Rc<isle_shell::zones::ZoneSurface>,
    pub zone_panel: std::rc::Rc<isle_shell::zones::ZoneSurface>,
    /// Pointer buttons held now, by evdev code.
    pub buttons_down: std::collections::HashSet<u32>,
    /// Presses the desktop took to close a menu, whose releases it takes too.
    pub taken_buttons: std::collections::HashSet<u32>,
    pub places: isle_state::domains::Places,
    pub desktops: crate::wm::desktops::Desktops,
    pub wallpapers: crate::wallpaper::Wallpapers,
    pub island: crate::island::IslandState,
    pub lock: crate::lock::LockState,
    pub idle: crate::idle::IdleState,
    pub prompts: crate::prompts::PromptsState,
    pub session: crate::session_end::SessionState,
    pub force_quit: crate::force_quit::ForceQuitState,
    pub spotlight: crate::spotlight::SpotlightState,
    /// The screen streams Isle produces.
    /// Each screen's name, shown while Settings identifies the displays.
    pub identify: std::rc::Rc<isle_shell::identify::IdentifySurface>,
    pub casting: crate::cast::Casting,
    /// ext-image-copy-capture, its sources and the foreign-toplevel list.
    pub image_copy: crate::image_copy::ImageCopy,
    /// Screen shares through the portal.
    pub share: crate::share::ShareState,
    pub share_picker: std::rc::Rc<isle_shell::capture::picker::PickerSurface>,
    /// The portal backend's connection, while Isle owns its name.
    pub portal: Option<zbus::Connection>,
    pub portal_appearance: crate::portal::Appearance,
    /// Apps' Inhibits by request handle: the app's name, the flags, the reason.
    pub portal_holds: std::collections::HashMap<String, (String, u32, String)>,
    /// Session-state monitors, by session handle.
    pub portal_monitors: std::collections::HashSet<String>,
    /// The screen recording running, if one is.
    pub recording_now: Option<crate::recording::Recording>,
    /// Presses marked in the recording, in the global space, while they fade.
    pub clicks: Vec<(smithay::utils::Point<f64, smithay::utils::Logical>, std::time::Instant)>,
    /// A portal's PickColor waiting on the pick.
    pub pick_reply: Option<tokio::sync::oneshot::Sender<Option<[u8; 3]>>>,
    /// Shift+Cmd+4's frozen screens while the pick is open.
    pub freeze: Option<crate::freeze::Freeze>,
    pub capture_readout: std::rc::Rc<isle_shell::capture::readout::ReadoutSurface>,
    /// The thumbnail after a shot, while it shows.
    pub thumbnail: Option<crate::thumbnail::Thumbnail>,
    pub capture_thumbnail: std::rc::Rc<isle_shell::capture::thumbnail::ThumbnailSurface>,
    /// Shift+Cmd+5's toolbar while it is open.
    pub toolbar: Option<crate::toolbar::Toolbar>,
    pub countdown: Option<crate::toolbar::Countdown>,
    pub capture_toolbar: std::rc::Rc<isle_shell::capture::toolbar::ToolbarSurface>,
    pub capture_state: isle_state::domains::Capture,
    pub capture_saver: Option<crate::state_writer::Saver<isle_state::domains::Capture>>,
    /// The emoji the colour font draws, by their place in the table, once checked.
    pub emoji_drawable: std::sync::Arc<std::sync::OnceLock<Vec<usize>>>,
    /// The file-name index, once built.
    pub files: Option<isle_search::files::Shared>,
    /// Where the index reaches, sent to its watcher when the settings change.
    pub files_reach: Option<tokio::sync::watch::Sender<Option<isle_search::files::Reach>>>,
    pub clipboard: crate::clipboard::ClipboardState,
    pub idle_notifier_state: smithay::wayland::idle_notify::IdleNotifierState<State>,
    /// The outputs changed under the lock; its face is shown again on the next dispatch.
    pub lock_outputs_changed: bool,
    /// Surfaces that panicked in the last dispatch.
    pub panicked: Vec<String>,
    /// Lets the sleep logind is waiting on go ahead.
    pub may_sleep: Option<tokio::sync::watch::Sender<bool>>,
    /// Sleep waits for every screen to show the lock.
    pub sleep_waiting: bool,
    /// Asleep: nothing is drawn until resume.
    pub asleep: bool,
    pub services: crate::services::Services,
    pub notifications: crate::notifications::NotificationsState,
    pub banners: crate::banners::BannersState,
    pub focus: crate::focus_modes::FocusState,
    /// The apps whose banners show, whose attention events wait.
    pub banners_apps: Vec<String>,
    pub top_edge: crate::top_edge::TopEdge,
    pub hot_corners: crate::hot_corners::HotCorners,
    pub bar: crate::menubar::BarState,
    pub menus: crate::menus::MenuState,
    /// The settings file holds a rejected hand edit: its file, line, column and message.
    pub settings_error: Option<(String, u32, u32, String)>,
    /// Started on the defaults after repeated crashes, until `isle settings use-mine`.
    pub safe_start: bool,
    pub window_programs: Option<crate::wm::draw::Programs>,
    /// Each window's shadow layers, kept so their ids hold still across frames.
    pub shadow_cache: std::cell::RefCell<
        std::collections::HashMap<(u64, usize), (crate::wm::draw::ShadowShape, crate::wm::draw::ShadowElement)>,
    >,
    /// The dim over windows that have a modal dialog attached, by window.
    pub scrims: std::cell::RefCell<
        std::collections::HashMap<u64, smithay::backend::renderer::element::solid::SolidColorBuffer>,
    >,
    /// Windows drawn from their last frame while they go.
    pub leaving: Vec<crate::wm::motion::Leaving>,
    pub split: crate::wm::split::SplitState,
    /// Outputs whose desktop is shown aside, by identity.
    pub shown_desktops: std::collections::HashMap<String, crate::wm::show_desktop::Shown>,
    /// Windows move; the Test interface turns it off so screenshots need not wait.
    pub window_motion: bool,
    /// The desktop entries, which name apps.
    pub apps: std::sync::Arc<isle_apps::Index>,
    /// Apps being quit, with the processes that get the polite signal once their windows go.
    pub quitting: Vec<(String, Vec<i32>)>,
    pub processes: crate::processes::Processes,
    /// The icon of the drag in progress, drawn at the pointer over everything.
    pub dnd_icon: Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
    /// Cmd+Tab's switcher while its chord is held.
    pub switcher: Option<crate::switcher::SwitcherState>,
    pub mission: Option<crate::mission::MissionState>,
    pub mission_back: std::rc::Rc<isle_shell::mission::MissionSurface>,
    pub mission_front: std::rc::Rc<isle_shell::mission::MissionSurface>,
    /// The windows or desktops changed while Mission Control is open.
    pub mission_dirty: std::cell::Cell<bool>,
    /// A desktop's menu open from Mission Control: its output and place.
    pub desk_menu: Option<(smithay::output::Output, usize)>,
    pub stage_surface: std::rc::Rc<isle_shell::stage::StageSurface>,
    pub stage_reveal: crate::wm::stage::Reveal,
    /// Buttons pressed while Mission Control was open, whose releases are its own too.
    pub mission_buttons: std::collections::HashSet<u32>,
    /// Buttons whose press ran a shortcut, whose releases go nowhere either.
    pub shortcut_buttons: std::collections::HashSet<u32>,
    pub switcher_surface: std::rc::Rc<isle_shell::switcher::SwitcherSurface>,
    /// The windows changed since the switcher last read them.
    pub switcher_dirty: std::cell::Cell<bool>,
    #[cfg(feature = "test-hooks")]
    pub switcher_drawn_count: u32,
    #[cfg(feature = "test-hooks")]
    pub switcher_was_drawn: bool,
    /// Each update of the switcher's panel and previews in microseconds, while measuring.
    #[cfg(feature = "test-hooks")]
    pub switcher_updates: Vec<u64>,
    pub games: std::sync::Arc<isle_games::Library>,
    /// LauncherEntry's badges and progress by desktop id.
    pub badges: std::collections::HashMap<String, isle_apps::badges::LauncherEntry>,
    /// App icon lookups, kept for the appearance they were made in.
    pub icons: std::cell::RefCell<Option<(bool, isle_apps::icons::Icons)>>,
    /// Pings awaiting their pong, by client and the window that prompted them.
    pub pings: Vec<(smithay::wayland::shell::xdg::ShellClient, u64)>,
    /// The pointer's image before the busy one replaced it over a window not responding.
    pub cursor_before_busy: Option<CursorImageStatus>,
    /// The zone a window being dragged would tile to.
    pub zone_preview: Option<crate::wm::ZonePreview>,
    pub places_saver: Option<crate::state_writer::Saver<isle_state::domains::Places>>,
    pub popups: PopupManager,
    pub cursor_status: CursorImageStatus,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub xdg_dialog_state: smithay::wayland::shell::xdg::dialog::XdgDialogState,
    pub kde_decoration_state: KdeDecorationState,
    pub xdg_activation_state: XdgActivationState,
    pub xdg_foreign_state: XdgForeignState,
    pub shm_state: ShmState,
    pub dmabuf_state: DmabufState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<State>,
    pub data_device_state: DataDeviceState,
    pub primary_selection_state: PrimarySelectionState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_state: FractionalScaleManagerState,
    pub presentation_state: PresentationState,
    pub single_pixel_buffer_state: SinglePixelBufferState,
    pub cursor_shape_state: CursorShapeManagerState,

    pub seat: Seat<State>,
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

impl Isle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display: Display<State>,
        loop_handle: LoopHandle<'static, State>,
        loop_signal: LoopSignal,
        heartbeat: Heartbeat,
        events: broadcast::Sender<Event>,
        settings: Settings,
        seat_name: String,
        theme: std::rc::Rc<isle_tokens::Theme>,
    ) -> Self {
        let dh = display.handle();
        let clock = Clock::new();

        let socket = ListeningSocketSource::new_auto().expect("a Wayland socket");
        let socket_name = socket.socket_name().to_os_string();
        loop_handle
            .insert_source(socket, |stream, _, state| {
                let _beat = state.isle.heartbeat.beat("wayland-accept");
                if let Err(err) = state
                    .isle
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                {
                    warn!(?err, "could not add a Wayland client");
                }
            })
            .expect("the Wayland socket source");
        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    let _beat = state.isle.heartbeat.beat("wayland");
                    // Safety: the display is never dropped while the loop runs.
                    unsafe { display.get_mut().dispatch_clients(state).unwrap() };
                    Ok(PostAction::Continue)
                },
            )
            .expect("the Wayland display source");
        info!(socket = ?socket_name, "listening");

        let image_copy = crate::image_copy::ImageCopy::new(&dh);
        let compositor_state = CompositorState::new::<State>(&dh);
        let xdg_shell_state = XdgShellState::new::<State>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<State>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<State>(&dh);
        let xwayland_shell_state =
            smithay::wayland::xwayland_shell::XWaylandShellState::new::<State>(&dh);
        let xdg_dialog_state =
            smithay::wayland::shell::xdg::dialog::XdgDialogState::new::<State>(&dh);
        let kde_decoration_state = KdeDecorationState::new::<State>(&dh, KdeMode::Server);
        let xdg_activation_state = XdgActivationState::new::<State>(&dh);
        let xdg_foreign_state = XdgForeignState::new::<State>(&dh);
        crate::menus::protocols::create_globals(&dh);
        crate::titlebar_overlay::create_global(&dh);
        let shm_state = ShmState::new::<State>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<State>(&dh);
        crate::background_effect::create_global(&dh);
        let data_device_state = DataDeviceState::new::<State>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<State>(&dh);
        let viewporter_state = ViewporterState::new::<State>(&dh);
        let fractional_scale_state = FractionalScaleManagerState::new::<State>(&dh);
        let presentation_state = PresentationState::new::<State>(&dh, clock.id() as u32);
        let single_pixel_buffer_state = SinglePixelBufferState::new::<State>(&dh);
        smithay::wayland::fifo::FifoManagerState::new::<State>(&dh);
        smithay::wayland::commit_timing::CommitTimingManagerState::new::<State>(&dh);
        smithay::wayland::content_type::ContentTypeState::new::<State>(&dh);
        smithay::wayland::pointer_constraints::PointerConstraintsState::new::<State>(&dh);
        smithay::wayland::relative_pointer::RelativePointerManagerState::new::<State>(&dh);
        smithay::wayland::tearing_control::TearingControlState::new::<State>(&dh);
        let cursor_shape_state = CursorShapeManagerState::new::<State>(&dh);
        let inhibit_state =
            smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState::new::<State>(&dh);
        let xwayland_grab_state =
            smithay::wayland::xwayland_keyboard_grab::XWaylandKeyboardGrabState::new::<State>(&dh);
        // Its keys are a client's own, sent with its own keymap and never translated.
        smithay::wayland::idle_inhibit::IdleInhibitManagerState::new::<State>(&dh);
        let idle_notifier_state =
            smithay::wayland::idle_notify::IdleNotifierState::<State>::new(&dh, loop_handle.clone());
        let virtual_keyboard_state =
            smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState::new::<State, _>(&dh, |_| true);
        smithay::wayland::virtual_pointer::VirtualPointerManagerState::new::<State, _>(&dh, |_| true);

        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name);
        seat.add_keyboard(XkbConfig::default(), 600, 25)
            .expect("a keyboard with the default keymap");
        seat.add_pointer();

        let (ping, ping_source) =
            smithay::reexports::calloop::ping::make_ping().expect("the shell's wake source");
        loop_handle
            .insert_source(ping_source, |_, _, state| {
                let _beat = state.isle.heartbeat.beat("shell");
                state.dispatch_shell();
            })
            .expect("the shell's wake source");
        let mut shell = isle_shell::Shell::new(ping, theme);
        let zoom_menu = isle_shell::zones::ZoneSurface::menu();
        let switcher_surface = isle_shell::switcher::SwitcherSurface::new();
        let banners = crate::banners::BannersState::new();
        shell.register(banners.surface.clone());
        shell.register(zoom_menu.clone());
        let zone_panel = isle_shell::zones::ZoneSurface::panel();
        shell.register(zone_panel.clone());
        let island = crate::island::IslandState::new(shell.theme());
        shell.register(island.surface.clone());
        shell.register(island.notice.clone());
        let bar = crate::menubar::BarState::new();
        shell.register(bar.surface.clone());
        shell.register(bar.menu.clone());
        shell.register(bar.window_menu.clone());
        shell.register(bar.about.clone());
        shell.register(switcher_surface.clone());
        let mission_back = isle_shell::mission::MissionSurface::back();
        let mission_front = isle_shell::mission::MissionSurface::front();
        shell.register(mission_back.clone());
        shell.register(mission_front.clone());
        let stage_surface = isle_shell::stage::StageSurface::new();
        shell.register(stage_surface.clone());
        let lock = crate::lock::LockState::default();
        shell.register(lock.face.surface.clone());
        shell.register(lock.face.minimal.clone());
        let prompts = crate::prompts::PromptsState::new();
        shell.register(prompts.surface.clone());
        let session = crate::session_end::SessionState::new();
        shell.register(session.surface.clone());
        let force_quit = crate::force_quit::ForceQuitState::new();
        shell.register(force_quit.surface.clone());
        let spotlight = crate::spotlight::SpotlightState::new();
        shell.register(spotlight.surface.clone());
        let identify = isle_shell::identify::IdentifySurface::new();
        shell.register(identify.clone());
        let capture_readout = isle_shell::capture::readout::ReadoutSurface::new();
        shell.register(capture_readout.clone());
        shell.register(isle_shell::capture::camera::CameraSurface::new());
        let capture_thumbnail = isle_shell::capture::thumbnail::ThumbnailSurface::new();
        shell.register(capture_thumbnail.clone());
        let capture_toolbar = isle_shell::capture::toolbar::ToolbarSurface::new();
        shell.register(capture_toolbar.clone());
        let share_picker = isle_shell::capture::picker::PickerSurface::new();
        shell.register(share_picker.clone());
        #[cfg(feature = "test-hooks")]
        let probe = std::rc::Rc::new(isle_shell::probe::Probe::default());
        #[cfg(feature = "test-hooks")]
        shell.register(probe.clone());
        #[cfg(feature = "test-hooks")]
        let probe_full = std::rc::Rc::new(isle_shell::probe::ProbeFull::default());
        #[cfg(feature = "test-hooks")]
        shell.register(probe_full.clone());
        #[cfg(feature = "test-hooks")]
        shell.register(std::rc::Rc::new(isle_shell::gallery::Gallery));

        Self {
            display_handle: dh,
            loop_handle,
            loop_signal,
            clock,
            socket_name,
            heartbeat,
            events,
            settings,
            shell,
            tokens: isle_tokens::Source::shipped(),
            tokens_text: String::new(),
            tokens_error: None,
            #[cfg(feature = "test-hooks")]
            probe,
            #[cfg(feature = "test-hooks")]
            probe_full,
            shell_keyboard_return: Vec::new(),
            keyboards: Default::default(),
            worker: None,
            to_loop: None,
            gamemode_pids: Default::default(),
            inhibit_state,
            x11_grabs: Default::default(),
            keys_taken_back: None,
            recording: None,
            slow_pending: Default::default(),
            slow_accepted: Default::default(),
            settings_store: None,
            swallowed: Default::default(),
            xwayland_grab_state,
            virtual_keyboard_state,
            bindings: Default::default(),
            repeat: None,
            held_chord: None,
            #[cfg(feature = "test-hooks")]
            held_releases: 0,
            space: Space::default(),
            wm: Default::default(),
            xwm: None,
            xdisplay: None,
            x11_order: Vec::new(),
            clients_started: false,
            exit_error: None,
            xwayland_deaths: 0,
            xwayland_source: None,
            xwayland_shell_state,
            unmanaged: Vec::new(),
            syncobj_state: None,
            constrained: None,
            big_picture: Default::default(),
            night_light: Default::default(),
            night_timer: None,
            night_applied: crate::night_light::NEUTRAL,
            night_fading: false,
            frame_phases: Default::default(),
            paced: Default::default(),
            commit_timers: Default::default(),
            bar_option: None,
            display_change: None,
            next_display_change: 0,
            titlebar_overlays: Default::default(),
            floats: Default::default(),
            zoom: Default::default(),
            zoom_menu,
            zone_panel,
            buttons_down: Default::default(),
            taken_buttons: Default::default(),
            places: Default::default(),
            desktops: Default::default(),
            wallpapers: Default::default(),
            island,
            lock,
            idle: Default::default(),
            prompts,
            session,
            force_quit,
            casting: Default::default(),
            image_copy,
            recording_now: None,
            identify,
            clicks: Vec::new(),
            portal: None,
            portal_appearance: Default::default(),
            portal_holds: Default::default(),
            portal_monitors: Default::default(),
            share: Default::default(),
            share_picker,
            freeze: None,
            pick_reply: None,
            capture_readout,
            thumbnail: None,
            capture_thumbnail,
            toolbar: None,
            countdown: None,
            capture_toolbar,
            capture_state: Default::default(),
            capture_saver: None,
            spotlight,
            emoji_drawable: Default::default(),
            files: None,
            files_reach: None,
            clipboard: crate::clipboard::ClipboardState::load(
                isle_state::StateDir::default_dir().join("clipboard"),
            ),
            idle_notifier_state,
            lock_outputs_changed: false,
            panicked: Vec::new(),
            may_sleep: None,
            sleep_waiting: false,
            asleep: false,
            top_edge: Default::default(),
            hot_corners: Default::default(),
            bar,
            menus: Default::default(),
            settings_error: None,
            safe_start: false,
            window_programs: None,
            shadow_cache: Default::default(),
            leaving: Vec::new(),
            split: Default::default(),
            shown_desktops: Default::default(),
            scrims: Default::default(),
            window_motion: true,
            apps: Default::default(),
            quitting: Vec::new(),
            processes: Default::default(),
            dnd_icon: None,
            switcher: None,
            switcher_surface,
            switcher_dirty: Default::default(),
            mission: None,
            mission_back,
            mission_front,
            mission_dirty: Default::default(),
            desk_menu: None,
            stage_surface,
            stage_reveal: Default::default(),
            mission_buttons: Default::default(),
            shortcut_buttons: Default::default(),
            #[cfg(feature = "test-hooks")]
            switcher_drawn_count: 0,
            #[cfg(feature = "test-hooks")]
            switcher_was_drawn: false,
            #[cfg(feature = "test-hooks")]
            switcher_updates: Vec::new(),
            games: Default::default(),
            badges: Default::default(),
            services: Default::default(),
            notifications: Default::default(),
            banners,
            banners_apps: Vec::new(),
            focus: Default::default(),
            icons: Default::default(),
            pings: Vec::new(),
            cursor_before_busy: None,
            zone_preview: None,
            places_saver: None,
            popups: PopupManager::default(),
            cursor_status: CursorImageStatus::default_named(),
            compositor_state,
            xdg_shell_state,
            layer_shell_state,
            xdg_decoration_state,
            xdg_dialog_state,
            kde_decoration_state,
            xdg_activation_state,
            xdg_foreign_state,
            shm_state,
            dmabuf_state: DmabufState::new(),
            output_manager_state,
            seat_state,
            data_device_state,
            primary_selection_state,
            viewporter_state,
            fractional_scale_state,
            presentation_state,
            single_pixel_buffer_state,
            cursor_shape_state,
            seat,
        }
    }

    pub fn update_frame_budget(&self) {
        let fastest = self
            .space
            .outputs()
            .filter_map(|o| o.current_mode())
            .map(|m| m.refresh)
            .max()
            .filter(|r| *r > 0)
            .unwrap_or(60_000);
        self.heartbeat
            .set_budget(Duration::from_secs_f64(1000.0 / fastest as f64));
    }

    /// The surface under `pos` and its origin: overlay and top layers, then windows,
    /// then bottom and background layers.
    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(Focus, Point<f64, Logical>)> {
        let output = self.space.output_under(pos).next()?;
        let origin = self.space.output_geometry(output)?.loc;
        let shell_at = |layer| {
            self.shell
                .focus_at(output, layer, pos - origin.to_f64())
                .map(|(focus, at)| (Focus::Shell(focus), at + origin.to_f64()))
        };
        // While locked the pointer reaches the lock's surfaces and nothing else; over a
        // prompt's scrim, the prompt and the island.
        if self.lock.locked {
            return shell_at(isle_shell::Layer::Lock);
        }
        if self.scrimmed() {
            return shell_at(isle_shell::Layer::Prompt)
                .or_else(|| shell_at(isle_shell::Layer::Overlay));
        }
        let surface_under = || self.client_surface_under(output, origin, pos);
        shell_at(isle_shell::Layer::Overlay)
            .or_else(|| {
                let on_layer = {
                    let map = layer_map_for_output(output);
                    [Layer::Overlay, Layer::Top]
                        .into_iter()
                        .any(|layer| map.layer_under(layer, pos - origin.to_f64()).is_some())
                };
                on_layer.then(surface_under).flatten()
            })
            .or_else(|| shell_at(isle_shell::Layer::Top))
            .or_else(surface_under)
            .or_else(|| shell_at(isle_shell::Layer::Background))
    }

    /// The client surface under `pos`: overlay and top layers, then windows, then bottom
    /// and background layers.
    fn client_surface_under(
        &self,
        output: &Output,
        origin: Point<i32, Logical>,
        pos: Point<f64, Logical>,
    ) -> Option<(Focus, Point<f64, Logical>)> {
        let map = layer_map_for_output(output);
        let layer_at = |layer: Layer| {
            let surface = map.layer_under(layer, pos - origin.to_f64())?;
            let geometry = map.layer_geometry(surface)?;
            let location = origin + geometry.loc;
            surface
                .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (Focus::Surface(s), (p + location).to_f64()))
        };
        layer_at(Layer::Overlay)
            .or_else(|| layer_at(Layer::Top))
            .or_else(|| self.window_part_under(pos))
            .or_else(|| layer_at(Layer::Bottom))
            .or_else(|| layer_at(Layer::Background))
    }

    /// The front window's part under `pos`: its popups and surfaces, its bar, or the band
    /// around its frame.
    fn window_part_under(&self, pos: Point<f64, Logical>) -> Option<(Focus, Point<f64, Logical>)> {
        for window in self.unmanaged.iter().rev() {
            let Some(x11) = window.x11_surface() else {
                continue;
            };
            let at = x11.last_configure().loc;
            if let Some((s, p)) = window.surface_under(pos - at.to_f64(), WindowSurfaceType::ALL) {
                return Some((Focus::Surface(s), (p + at).to_f64()));
            }
        }
        for window in self.space.elements().rev() {
            let id = crate::wm::window_id(window);
            let Some(location) = self.space.element_location(window) else {
                continue;
            };
            let surface_at = location - window.geometry().loc;
            if let Some((s, p)) =
                window.surface_under(pos - surface_at.to_f64(), WindowSurfaceType::POPUP)
            {
                return Some((Focus::Surface(s), (p + surface_at).to_f64()));
            }
            if let Some(hit) = self.alert_at(id, pos) {
                return Some(hit);
            }
            if let Some(hit) = self.sheet_at(id, pos) {
                return Some(hit);
            }
            if let Some(part) = self.bar_or_edge_at(id, pos) {
                let at = match &part {
                    Focus::Shell(_) => self.frame(id).map(|f| f.loc.to_f64()).unwrap_or_default(),
                    _ => pos,
                };
                return Some((part, at));
            }
            if let Some((s, p)) =
                window.surface_under(pos - surface_at.to_f64(), WindowSurfaceType::TOPLEVEL | WindowSurfaceType::SUBSURFACE)
            {
                return Some((Focus::Surface(s), (p + surface_at).to_f64()));
            }
        }
        None
    }

    /// The output under the pointer, else the first.
    pub fn active_output(&self) -> Option<Output> {
        let pointer = self.seat.get_pointer()?.current_location();
        self.space
            .output_under(pointer)
            .next()
            .or_else(|| self.space.outputs().next())
            .cloned()
    }

    /// Raises `window`, activates it and gives it the keyboard; `None` clears focus.
    pub fn focus_window(&mut self, window: Option<&Window>) -> Option<Focus> {
        for record in &self.wm.records {
            let active = window == Some(&record.window);
            if record.window.set_activated(active)
                && let Some(toplevel) = record.window.toplevel()
                && toplevel.is_initial_configure_sent()
            {
                toplevel.send_pending_configure();
            }
        }
        let window = window?;
        let id = crate::wm::window_id(window);
        self.wm.raise(id);
        self.wm.touch(id);
        self.sync_space();
        if let Some(sheet) = self.sheet_focus(id) {
            return Some(sheet);
        }
        use smithay::wayland::seat::WaylandFocus;
        let surface = window.wl_surface()?.into_owned();
        Some(match window.x11_surface() {
            Some(x11) => Focus::X11 {
                window: Box::new(x11.clone()),
                surface,
            },
            None => Focus::Surface(surface),
        })
    }

    /// Runs the shell surfaces whose wakers fired and redraws the outputs they changed.
    pub fn dispatch_shell(&mut self) -> Vec<(String, Option<isle_shell::ShellFocus>, isle_shell::Action)> {
        let dispatched = self.shell.dispatch();
        self.panicked.extend(dispatched.panicked.iter().cloned());
        let mut actions: Vec<(String, Option<isle_shell::ShellFocus>, isle_shell::Action)> = dispatched
            .disabled
            .into_iter()
            .map(|name| (name, None, isle_shell::Action::Close))
            .collect();
        for output in dispatched.redraw {
            self.queue_redraw(&output);
        }
        let pointer_focus = self.seat.get_pointer().and_then(|p| p.current_focus());
        for (focus, _) in dispatched.cursors {
            if pointer_focus == Some(Focus::Shell(focus.clone())) {
                self.set_shell_cursor(&focus);
            }
        }
        actions.extend(
            dispatched
                .actions
                .into_iter()
                .map(|(name, focus, action)| (name, Some(focus), action)),
        );
        actions
    }

    /// The pointer is over a shell surface, which names its cursor.
    pub fn set_shell_cursor(&mut self, focus: &isle_shell::ShellFocus) {
        let icon = self.shell.cursor(focus);
        let status = CursorImageStatus::Named(icon);
        if self.cursor_status != status {
            self.cursor_status = status;
            self.queue_redraw_all();
        }
    }

    /// Tells the shell where the outputs are; the first is the main one.
    pub fn sync_shell_outputs(&mut self) {
        let places = self
            .space
            .outputs()
            .filter_map(|output| {
                Some(isle_shell::OutputPlace {
                    output: output.clone(),
                    geometry: self.space.output_geometry(output)?,
                    scale: output.current_scale().fractional_scale(),
                })
            })
            .collect();
        self.shell.set_outputs(places);
        self.dispatch_shell();
        self.refresh_island();
        self.lock_outputs_changed = true;
    }

    pub fn queue_redraw_all(&mut self) {
        let outputs: Vec<_> = self.space.outputs().cloned().collect();
        for output in &outputs {
            self.queue_redraw(output);
        }
    }

    pub fn queue_redraw_for_window(&mut self, window: &Window) {
        let outputs = self.space.outputs_for_element(window);
        for output in &outputs {
            self.queue_redraw(output);
        }
    }
}

impl State {
    /// Opens a shell surface; one that takes keys takes the keyboard.
    pub fn open_surface(&mut self, name: &str) -> bool {
        let found = self.isle.shell.open(name);
        if let Some(focus) = self.isle.shell.keyboard_taker(name) {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            if previous != Some(Focus::Shell(focus.clone())) {
                self.isle
                    .shell_keyboard_return
                    .push((focus.clone(), previous));
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(Focus::Shell(focus)), serial);
            }
        }
        self.dispatch_shell();
        found
    }

    /// Closes a shell surface, giving the keyboard back to what had it before (shell.md
    /// rule 11).
    pub fn close_surface(&mut self, name: &str) -> bool {
        let taker = self.isle.shell.keyboard_taker(name);
        let found = self.isle.shell.close(name);
        if let Some(focus) = taker
            && let Some(index) = self
                .isle
                .shell_keyboard_return
                .iter()
                .position(|(f, _)| *f == focus)
        {
            let (_, previous) = self.isle.shell_keyboard_return.remove(index);
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            if keyboard.current_focus() == Some(Focus::Shell(focus)) {
                match previous.filter(smithay::utils::IsAlive::alive) {
                    Some(previous) => {
                        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                        self.focus_keyboard(Some(previous), serial);
                    }
                    None => self.focus_next(),
                }
            }
        }
        self.dispatch_shell();
        self.isle.queue_redraw_all();
        found
    }

    /// Runs the shell surfaces whose wakers fired, then what they asked for, in order.
    pub fn dispatch_shell(&mut self) {
        let actions = self.isle.dispatch_shell();
        for name in std::mem::take(&mut self.isle.panicked) {
            self.lock_panicked(&name);
        }
        if std::mem::take(&mut self.isle.lock_outputs_changed) && self.isle.lock.locked {
            self.show_lock_face();
        }
        for (name, focus, action) in actions {
            match action {
                isle_shell::Action::Window(action) => {
                    if let Some(id) = focus.and_then(|f| self.isle.shell.attached_key(&f)) {
                        self.bar_action(id, action);
                    }
                }
                isle_shell::Action::Close if name == "zoom-menu" => self.close_zoom_menu(),
                isle_shell::Action::Close => {
                    self.close_surface(&name);
                }
                isle_shell::Action::Zone(action) => self.zone_action(action),
                isle_shell::Action::Island(action) => self.island_action(focus, action),
                isle_shell::Action::MenuBar(action) => self.menu_bar_action(action),
                isle_shell::Action::Menu(event) => self.window_menu_event(event),
                isle_shell::Action::Switcher(action) => self.switcher_pointer(action),
                isle_shell::Action::Stage(action) => self.stage_action(action),
                isle_shell::Action::Banner(action) => self.banner_action(action),
                isle_shell::Action::Lock(action) => self.lock_action(action),
                isle_shell::Action::Prompt(action) => self.prompt_action(action),
                isle_shell::Action::Session(action) => self.session_action(action),
                isle_shell::Action::ForceQuit(action) => self.force_quit_action(action),
                isle_shell::Action::Spotlight(action) => self.spotlight_action(action),
                isle_shell::Action::Toolbar(action) => self.toolbar_action(action),
                isle_shell::Action::Picker(action) => self.picker_action(action),
                isle_shell::Action::Alert(action) => {
                    if let Some(id) = focus
                        .and_then(|f| self.isle.shell.attached_key(&f))
                        .and_then(crate::force_quit::alert_window)
                    {
                        self.alert_action(id, action);
                    }
                }
                isle_shell::Action::Key(code) => {
                    use crate::keyboard::ISLE_DEVICE;
                    use smithay::backend::input::{InputTime, KeyState};
                    self.on_device_key(
                        ISLE_DEVICE,
                        "Isle",
                        code,
                        KeyState::Pressed,
                        InputTime::now(),
                    );
                    self.on_device_key(
                        ISLE_DEVICE,
                        "Isle",
                        code,
                        KeyState::Released,
                        InputTime::now(),
                    );
                }
            }
        }
    }

    /// Gives `window` the keyboard, or clears it; a window with a modal dialog gives it to
    /// the dialog.
    pub fn set_focus(&mut self, window: Option<&Window>) {
        // A window taking the keyboard dismisses Spotlight, which would stay up deaf.
        if window.is_some() && self.isle.spotlight.open() {
            self.close_spotlight();
        }
        let modal = window
            .and_then(|w| self.isle.wm.modal_of(crate::wm::window_id(w)))
            .and_then(|id| self.isle.window_by_id(id));
        let window = modal.as_ref().or(window);
        // Activating an app answers every window of it that asked for attention.
        let app = window
            .and_then(|w| self.isle.wm.get(crate::wm::window_id(w)))
            .map(|r| r.app.clone());
        for record in self.isle.wm.records.iter_mut().filter(|r| Some(&r.app) == app.as_ref()) {
            record.attention = false;
            record.urgent = false;
        }
        if let Some(app) = &app {
            crate::badges::calm(&mut self.isle.badges, app);
        }
        tracing::info!(window = window.map(crate::wm::window_id), ?app, "focus");
        let surface = self.isle.focus_window(window);
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        self.focus_keyboard(surface, serial);
        // Its menus are looked up as it takes focus, so the bar has them when revealed.
        if let Some(window) = window {
            self.menus_window(crate::wm::window_id(window));
        }
        self.isle.sync_bars();
        self.isle.queue_redraw_all();
        self.isle.notify(Event::WindowsChanged);
    }
}
