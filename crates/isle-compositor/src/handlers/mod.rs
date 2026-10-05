//! The Wayland protocols' handlers.

mod compositor;
mod layer_shell;
mod xdg_shell;

use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dispatch2,
    input::{
        Seat, SeatHandler, SeatState,
        dnd::{DnDGrab, DndGrabHandler, GrabType, Source},
        pointer::{CursorImageStatus, Focus},
        tablet::TabletSeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
        wayland_server::{Resource, protocol::wl_surface::WlSurface},
    },
    utils::Serial,
    wayland::{
        compositor::{get_parent, with_states},
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        fractional_scale::{FractionalScaleHandler, with_fractional_scale},
        output::OutputHandler,
        pointer_constraints::PointerConstraintsHandler,
        selection::{
            SelectionHandler,
            data_device::{
                DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler, set_data_device_focus,
            },
            primary_selection::{
                PrimarySelectionHandler, PrimarySelectionState, set_primary_focus,
            },
        },
        shell::{
            kde::decoration::{KdeDecorationHandler, KdeDecorationState},
            xdg::{ToplevelSurface, decoration::XdgDecorationHandler},
        },
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
};
use tracing::debug;

use crate::state::State;

impl SeatHandler for State {
    type KeyboardFocus = crate::focus::Focus;
    type PointerFocus = crate::focus::Focus;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.isle.seat_state
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&crate::focus::Focus>) {
        let dh = &self.isle.display_handle;
        let client = focused
            .and_then(|f| f.surface())
            .and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client.clone());
        set_primary_focus(dh, seat, client);
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        self.isle.cursor_status = image;
        self.backend.update_cursor(&self.isle);
        self.isle.queue_redraw_all();
    }
}

impl PointerConstraintsHandler for State {}

impl TabletSeatHandler for State {
    type ToolFocus = WlSurface;
}

impl SelectionHandler for State {
    type SelectionUserData = ();
}

impl DataDeviceHandler for State {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.isle.data_device_state
    }
}

impl DndGrabHandler for State {}

impl WaylandDndGrabHandler for State {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        _icon: Option<WlSurface>,
        seat: Seat<Self>,
        serial: Serial,
        type_: GrabType,
    ) {
        match type_ {
            GrabType::Pointer => {
                let pointer = seat.get_pointer().unwrap();
                let start = pointer.grab_start_data().unwrap();
                let grab = DnDGrab::new_pointer(&self.isle.display_handle, start, source, seat);
                pointer.set_grab(self, grab, serial, Focus::Keep);
            }
            GrabType::Touch => source.cancel(),
        }
    }
}

impl PrimarySelectionHandler for State {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.isle.primary_selection_state
    }
}

impl OutputHandler for State {}

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.isle.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        if self.backend.import_dmabuf(&dmabuf) {
            let _ = notifier.successful::<State>();
        } else {
            notifier.failed();
        }
    }
}

/// Server side is preferred; a client that asks for its own frame gets it.
impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.set_decoration(&toplevel, DecorationMode::ServerSide);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: DecorationMode) {
        self.set_decoration(&toplevel, mode);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.set_decoration(&toplevel, DecorationMode::ServerSide);
    }
}

impl State {
    fn set_decoration(&mut self, toplevel: &ToplevelSurface, mode: DecorationMode) {
        toplevel.with_pending_state(|state| state.decoration_mode = Some(mode));
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
        self.set_server_side(toplevel.wl_surface(), mode == DecorationMode::ServerSide);
    }

    fn set_server_side(&mut self, surface: &WlSurface, on: bool) {
        let Some(record) = self
            .isle
            .wm
            .records
            .iter_mut()
            .find(|r| r.surface() == Some(surface))
        else {
            return;
        };
        if record.server_side != on {
            record.server_side = on;
            self.isle.sync_bars();
            self.isle.queue_redraw_all();
        }
    }
}

impl KdeDecorationHandler for State {
    fn kde_decoration_state(&self) -> &KdeDecorationState {
        &self.isle.kde_decoration_state
    }

    fn new_decoration(
        &mut self,
        surface: &WlSurface,
        _decoration: &smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
    ) {
        self.set_server_side(surface, true);
    }

    fn request_mode(
        &mut self,
        surface: &WlSurface,
        decoration: &smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
        mode: smithay::reexports::wayland_server::WEnum<
            smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::Mode,
        >,
    ) {
        use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::Mode;
        if let smithay::reexports::wayland_server::WEnum::Value(mode) = mode {
            decoration.mode(mode);
            self.set_server_side(surface, mode == Mode::Server);
        }
    }
}

impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.isle.xdg_activation_state
    }

    /// A token is valid only when made from input the focused client received.
    fn token_created(&mut self, _token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        let Some((serial, seat)) = data.serial else {
            return false;
        };
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        Seat::from_resource(&seat) == Some(self.isle.seat.clone())
            && keyboard
                .last_enter()
                .is_some_and(|enter| serial.is_no_older_than(&enter))
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        let Some(id) = self.isle.wm.for_surface(&surface).filter(|r| r.mapped).map(|r| r.id) else {
            return;
        };
        if data.timestamp.elapsed().as_secs() < 10 {
            self.activate(id);
        } else {
            debug!("activation without a fresh token: the window wants attention");
            self.isle.wm.get_mut(id).unwrap().attention = true;
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }
}

impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        let mut root = surface.clone();
        while let Some(parent) = get_parent(&root) {
            root = parent;
        }
        let output = self
            .isle
            .window_for_surface(&root)
            .and_then(|w| self.isle.space.outputs_for_element(&w).into_iter().next())
            .or_else(|| self.isle.active_output());
        if let Some(output) = output {
            with_states(&surface, |states| {
                with_fractional_scale(states, |fractional| {
                    fractional.set_preferred_scale(output.current_scale().fractional_scale());
                });
            });
        }
    }
}

delegate_dispatch2!(State);
