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
    type SelectionUserData = crate::clipboard::Owner;

    fn new_selection(
        &mut self,
        ty: smithay::wayland::selection::SelectionTarget,
        source: Option<smithay::wayland::selection::SelectionSource>,
        _seat: Seat<Self>,
    ) {
        let types = source.map(|s| s.mime_types());
        self.tell_xwayland(ty, types.clone());
        if ty == smithay::wayland::selection::SelectionTarget::Clipboard {
            self.clipboard_offered(types.unwrap_or_default());
        }
    }

    fn send_selection(
        &mut self,
        ty: smithay::wayland::selection::SelectionTarget,
        mime_type: String,
        fd: std::os::fd::OwnedFd,
        _seat: Seat<Self>,
        owner: &Self::SelectionUserData,
    ) {
        let owner = owner.clone();
        self.send_selection_to(ty, mime_type, fd, &owner);
    }
}

impl DataDeviceHandler for State {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.isle.data_device_state
    }
}

impl DndGrabHandler for State {
    fn dropped(
        &mut self,
        _target: Option<smithay::input::dnd::DndTarget<'_, Self>>,
        _validated: bool,
        _seat: Seat<Self>,
        _location: smithay::utils::Point<f64, smithay::utils::Logical>,
    ) {
        self.isle.dnd_icon = None;
        self.isle.queue_redraw_all();
    }

    fn cancelled(&mut self, _seat: Seat<Self>, _location: smithay::utils::Point<f64, smithay::utils::Logical>) {
        self.isle.dnd_icon = None;
        self.isle.queue_redraw_all();
    }
}

impl WaylandDndGrabHandler for State {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        icon: Option<WlSurface>,
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
                self.isle.dnd_icon = icon;
                self.isle.queue_redraw_all();
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

/// xdg-decoration: the client's choice where it makes one, else server side while Isle
/// draws bars; with bars off every client is told to draw its own.
impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.set_decoration(&toplevel, None);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: DecorationMode) {
        self.set_decoration(&toplevel, Some(mode));
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.set_decoration(&toplevel, None);
    }
}

impl State {
    fn set_decoration(&mut self, toplevel: &ToplevelSurface, requested: Option<DecorationMode>) {
        if let Some(record) = self
            .isle
            .wm
            .records
            .iter_mut()
            .find(|r| r.surface().as_ref() == Some(toplevel.wl_surface()))
        {
            record.decoration = Some(crate::wm::Decoration::Xdg(requested));
        }
        let mode = if self.isle.settings.windows.title_bars {
            requested.unwrap_or(DecorationMode::ServerSide)
        } else {
            DecorationMode::ClientSide
        };
        toplevel.with_pending_state(|state| state.decoration_mode = Some(mode));
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
        self.set_server_side(toplevel.wl_surface(), mode == DecorationMode::ServerSide);
    }

    /// Title bars turned on or off: every client that negotiates hears again who draws.
    pub fn renegotiate_decorations(&mut self) {
        use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::Mode as Kde;
        let negotiated: Vec<(u64, crate::wm::Decoration)> = self
            .isle
            .wm
            .records
            .iter()
            .filter_map(|r| Some((r.id, r.decoration.clone()?)))
            .collect();
        for (id, decoration) in negotiated {
            match decoration {
                crate::wm::Decoration::Xdg(requested) => {
                    if let Some(toplevel) = self.isle.wm.get(id).and_then(|r| r.window.toplevel()).cloned() {
                        self.set_decoration(&toplevel, requested);
                    }
                }
                crate::wm::Decoration::Kde { object, requested } => {
                    let server = self.isle.settings.windows.title_bars && requested == Kde::Server;
                    object.mode(if server { Kde::Server } else { Kde::Client });
                    if let Some(surface) = self.isle.wm.get(id).and_then(|r| r.surface()) {
                        self.set_server_side(&surface, server);
                    }
                }
            }
        }
    }

    fn set_server_side(&mut self, surface: &WlSurface, on: bool) {
        let Some(record) = self
            .isle
            .wm
            .records
            .iter_mut()
            .find(|r| r.surface().as_ref() == Some(surface))
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

    /// KDE server-decoration: the global's default is server side, which holds while Isle
    /// draws bars; with bars off the client is told to draw its own.
    fn new_decoration(
        &mut self,
        surface: &WlSurface,
        decoration: &smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
    ) {
        use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration::Mode as Kde;
        if let Some(record) = self.isle.wm.records.iter_mut().find(|r| r.surface().as_ref() == Some(surface)) {
            record.decoration = Some(crate::wm::Decoration::Kde {
                object: decoration.clone(),
                requested: Kde::Server,
            });
        }
        let server = self.isle.settings.windows.title_bars;
        if !server {
            decoration.mode(Kde::Client);
        }
        self.set_server_side(surface, server);
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
            if let Some(record) = self.isle.wm.records.iter_mut().find(|r| r.surface().as_ref() == Some(surface)) {
                record.decoration = Some(crate::wm::Decoration::Kde {
                    object: decoration.clone(),
                    requested: mode,
                });
            }
            let server = mode == Mode::Server && self.isle.settings.windows.title_bars;
            decoration.mode(if server { Mode::Server } else if mode == Mode::None { Mode::None } else { Mode::Client });
            self.set_server_side(surface, server);
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
            self.want_attention(id, false);
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
