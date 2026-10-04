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
            protocol::wl_surface::WlSurface,
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
    pub next_window_id: u64,
    pub settings: Settings,
    pub shell: isle_shell::Shell,
    #[cfg(feature = "test-hooks")]
    pub probe: std::rc::Rc<isle_shell::probe::Probe>,
    /// Keys whose press ran a bind; their release is not sent to clients.
    pub suppressed_keys: std::collections::HashSet<smithay::input::keyboard::Keycode>,
    /// The keyboard's focus before a shell surface took it, given back when it closes.
    pub shell_keyboard_return: Vec<(isle_shell::ShellFocus, Option<Focus>)>,

    pub space: Space<Window>,
    /// Toplevels that have not committed a buffer yet; they are placed when they do.
    pub unmapped: Vec<Window>,
    pub popups: PopupManager,
    pub cursor_status: CursorImageStatus,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub kde_decoration_state: KdeDecorationState,
    pub xdg_activation_state: XdgActivationState,
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
    pub fn new(
        display: Display<State>,
        loop_handle: LoopHandle<'static, State>,
        loop_signal: LoopSignal,
        heartbeat: Heartbeat,
        events: broadcast::Sender<Event>,
        settings: Settings,
        seat_name: String,
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

        let compositor_state = CompositorState::new::<State>(&dh);
        let xdg_shell_state = XdgShellState::new::<State>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<State>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<State>(&dh);
        let kde_decoration_state = KdeDecorationState::new::<State>(&dh, KdeMode::Client);
        let xdg_activation_state = XdgActivationState::new::<State>(&dh);
        let shm_state = ShmState::new::<State>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<State>(&dh);
        let data_device_state = DataDeviceState::new::<State>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<State>(&dh);
        let viewporter_state = ViewporterState::new::<State>(&dh);
        let fractional_scale_state = FractionalScaleManagerState::new::<State>(&dh);
        let presentation_state = PresentationState::new::<State>(&dh, clock.id() as u32);
        let single_pixel_buffer_state = SinglePixelBufferState::new::<State>(&dh);
        let cursor_shape_state = CursorShapeManagerState::new::<State>(&dh);

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
        #[allow(unused_mut)]
        let mut shell = isle_shell::Shell::new(ping, Vec::new());
        #[cfg(feature = "test-hooks")]
        let probe = std::rc::Rc::new(isle_shell::probe::Probe::default());
        #[cfg(feature = "test-hooks")]
        shell.register(probe.clone());

        Self {
            display_handle: dh,
            loop_handle,
            loop_signal,
            clock,
            socket_name,
            heartbeat,
            events,
            next_window_id: 1,
            settings,
            shell,
            #[cfg(feature = "test-hooks")]
            probe,
            suppressed_keys: Default::default(),
            shell_keyboard_return: Vec::new(),
            space: Space::default(),
            unmapped: Vec::new(),
            popups: PopupManager::default(),
            cursor_status: CursorImageStatus::default_named(),
            compositor_state,
            xdg_shell_state,
            layer_shell_state,
            xdg_decoration_state,
            kde_decoration_state,
            xdg_activation_state,
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

    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .chain(self.unmapped.iter())
            .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == surface))
            .cloned()
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
                .map(|(s, p)| (s, (p + location).to_f64()))
        };
        layer_at(Layer::Overlay)
            .or_else(|| layer_at(Layer::Top))
            .or_else(|| {
                self.space
                    .element_under(pos)
                    .and_then(|(window, location)| {
                        window
                            .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                            .map(|(s, p)| (s, (p + location).to_f64()))
                    })
            })
            .or_else(|| layer_at(Layer::Bottom))
            .or_else(|| layer_at(Layer::Background))
            .map(|(surface, at)| (Focus::Surface(surface), at))
    }

    /// The part of `output` layer surfaces leave free, in the global space.
    pub fn work_area(&self, output: &Output) -> smithay::utils::Rectangle<i32, Logical> {
        let origin = self.space.output_geometry(output).unwrap_or_default().loc;
        let mut zone = layer_map_for_output(output).non_exclusive_zone();
        zone.loc += origin;
        zone
    }

    /// The output new windows open on: the one under the pointer.
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
        for w in self.space.elements() {
            let active = window == Some(w);
            if w.set_activated(active) {
                w.toplevel().unwrap().send_pending_configure();
            }
        }
        let window = window?;
        self.space.raise_element(window, true);
        Some(window.toplevel()?.wl_surface().into())
    }

    /// Runs the shell surfaces whose wakers fired and redraws the outputs they changed.
    pub fn dispatch_shell(&mut self) -> Vec<(String, isle_shell::Action)> {
        let dispatched = self.shell.dispatch();
        let mut actions: Vec<(String, isle_shell::Action)> = dispatched
            .disabled
            .into_iter()
            .map(|name| (name, isle_shell::Action::Close))
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
                .map(|(name, _, action)| (name, action)),
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
                keyboard.set_focus(self, Some(Focus::Shell(focus)), serial);
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
                        keyboard.set_focus(self, Some(previous), serial);
                    }
                    None => self.focus_top(),
                }
            }
        }
        self.dispatch_shell();
        self.isle.queue_redraw_all();
        found
    }

    /// Runs the shell surfaces whose wakers fired, then what they asked for, in order.
    pub fn dispatch_shell(&mut self) {
        for (name, action) in self.isle.dispatch_shell() {
            match action {
                isle_shell::Action::Close => {
                    self.close_surface(&name);
                }
                isle_shell::Action::Key(code) => {
                    use smithay::backend::input::{InputTime, KeyState};
                    let keycode = smithay::input::keyboard::Keycode::new(code + 8);
                    self.on_key(keycode, KeyState::Pressed, InputTime::now());
                    self.on_key(keycode, KeyState::Released, InputTime::now());
                }
            }
        }
    }

    /// Gives `window` the keyboard, or clears it.
    pub fn set_focus(&mut self, window: Option<&Window>) {
        let surface = self.isle.focus_window(window);
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        keyboard.set_focus(self, surface, serial);
        self.isle.queue_redraw_all();
        self.isle.notify(Event::WindowsChanged);
    }

    /// Focuses the topmost window, after the focused one went away.
    pub fn focus_top(&mut self) {
        let top = self.isle.space.elements().last().cloned();
        self.set_focus(top.as_ref());
    }
}
