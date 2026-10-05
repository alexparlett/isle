use smithay::{
    desktop::{
        PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window,
        find_popup_root_surface, get_popup_toplevel_coords,
    },
    input::{
        Seat,
        pointer::{Focus, GrabStartData},
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            Resource,
            protocol::{wl_seat, wl_surface::WlSurface},
        },
    },
    utils::{Rectangle, Serial},
    wayland::shell::xdg::{
        PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    },
};
use tracing::warn;

use crate::{
    grabs::{MoveGrab, ResizeGrab},
    state::State,
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.isle.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        self.isle.add_window(Window::new_wayland_window(surface));
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.remove_window(surface.wl_surface());
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        self.unconstrain_popup(&surface);
        if let Err(err) = self.isle.popups.track_popup(PopupKind::Xdg(surface)) {
            warn!(?err, "could not track a popup");
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        self.isle.queue_redraw_all();
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let wl_surface = surface.wl_surface();
        let Some(start_data) = check_grab(&seat, wl_surface, serial) else {
            return;
        };
        let Some(window) = self.isle.window_for_surface(wl_surface) else {
            return;
        };
        let Some(initial) = self.isle.space.element_location(&window) else {
            return;
        };
        let grab = MoveGrab {
            start_data,
            window,
            initial,
        };
        seat.get_pointer()
            .unwrap()
            .set_grab(self, grab, serial, Focus::Clear);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: wl_seat::WlSeat,
        serial: Serial,
        edges: xdg_toplevel::ResizeEdge,
    ) {
        let Some(seat) = Seat::from_resource(&seat) else {
            return;
        };
        let wl_surface = surface.wl_surface();
        let Some(start_data) = check_grab(&seat, wl_surface, serial) else {
            return;
        };
        let Some(window) = self.isle.window_for_surface(wl_surface) else {
            return;
        };
        let Some(location) = self.isle.space.element_location(&window) else {
            return;
        };
        let size = window.geometry().size;
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
        });
        surface.send_pending_configure();
        let grab = ResizeGrab::start(
            start_data,
            window,
            edges.into(),
            Rectangle::new(location, size),
        );
        seat.get_pointer()
            .unwrap()
            .set_grab(self, grab, serial, Focus::Clear);
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let Some(seat) = Seat::<State>::from_resource(&seat) else {
            return;
        };
        let kind = PopupKind::Xdg(surface);
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Ok(mut grab) = self.isle.popups.grab_popup(root.into(), kind, &seat, serial) else {
            return;
        };
        if let Some(keyboard) = seat.get_keyboard() {
            if keyboard.is_grabbed()
                && !(keyboard.has_grab(serial)
                    || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }
        if let Some(pointer) = seat.get_pointer() {
            if pointer.is_grabbed()
                && !(pointer.has_grab(serial)
                    || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
    }
}

/// The grab start data for a client's move or resize, if the client holds the
/// pointer's implicit grab from a press on that surface.
fn check_grab(
    seat: &Seat<State>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<State>> {
    let pointer = seat.get_pointer()?;
    if !pointer.has_grab(serial) {
        return None;
    }
    let start_data = pointer.grab_start_data()?;
    let (focus, _) = start_data.focus.as_ref()?;
    if !smithay::wayland::seat::WaylandFocus::same_client_as(focus, &surface.id()) {
        return None;
    }
    Some(start_data)
}

impl State {
    /// Keeps a popup inside the output its toplevel is on.
    fn unconstrain_popup(&self, popup: &PopupSurface) {
        let kind = PopupKind::Xdg(popup.clone());
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Some(window) = self.isle.window_for_surface(&root) else {
            return;
        };
        let Some(window_geometry) = self.isle.space.element_geometry(&window) else {
            return;
        };
        let Some(output) = self
            .isle
            .space
            .outputs_for_element(&window)
            .into_iter()
            .next()
        else {
            return;
        };
        let mut target = self.isle.space.output_geometry(&output).unwrap();
        target.loc -= get_popup_toplevel_coords(&kind);
        target.loc -= window_geometry.loc;
        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }

    pub(super) fn commit_popup(&mut self, surface: &WlSurface) {
        self.isle.popups.commit(surface);
        if let Some(PopupKind::Xdg(popup)) = self.isle.popups.find_popup(surface) {
            if !popup.is_initial_configure_sent() {
                // The initial configure is always allowed.
                popup.send_configure().expect("initial popup configure");
            }
            self.isle.queue_redraw_all();
        }
    }
}
