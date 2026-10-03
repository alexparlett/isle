//! The compositor's state: the backend beside everything else, so rendering can borrow both.

use std::{ffi::OsString, sync::Arc, time::Duration};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
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

use crate::{backend::Backend, watchdog::Heartbeat, worker::Event};

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
    /// Keys whose press ran a bind; their release is not sent to clients.
    pub suppressed_keys: std::collections::HashSet<smithay::input::keyboard::Keycode>,

    pub space: Space<Window>,
    /// Toplevels that have not committed a buffer yet; they are placed when they do.
    pub unmapped: Vec<Window>,
    pub popups: PopupManager,
    pub cursor_status: CursorImageStatus,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
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
            suppressed_keys: Default::default(),
            space: Space::default(),
            unmapped: Vec::new(),
            popups: PopupManager::default(),
            cursor_status: CursorImageStatus::default_named(),
            compositor_state,
            xdg_shell_state,
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

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
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
    pub fn focus_window(&mut self, window: Option<&Window>) -> Option<WlSurface> {
        for w in self.space.elements() {
            let active = window == Some(w);
            if w.set_activated(active) {
                w.toplevel().unwrap().send_pending_configure();
            }
        }
        let window = window?;
        self.space.raise_element(window, true);
        Some(window.toplevel()?.wl_surface().clone())
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
