//! Xwayland and Isle's X11 window manager on Smithay's (B5), with v1's X11 rules as its
//! policy (`compositor.md` Xwayland).

use std::{os::fd::OwnedFd, process::Stdio, time::Duration};

use smithay::{
    desktop::Window,
    input::pointer::GrabStartData,
    reexports::calloop::timer::{TimeoutAction, Timer},
    utils::{Logical, Rectangle, SERIAL_COUNTER},
    wayland::{
        selection::{
            SelectionTarget,
            data_device::{
                clear_data_device_selection, current_data_device_selection_userdata,
                request_data_device_client_selection, set_data_device_selection,
            },
            primary_selection::{
                clear_primary_selection, current_primary_selection_userdata,
                request_primary_client_selection, set_primary_selection,
            },
        },
        xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    },
    xwayland::{
        X11Surface, X11Wm, XWayland, XWaylandEvent, XwmHandler,
        xwm::{Reorder, ResizeEdge, XwmId},
    },
};
use tracing::{error, info, warn};

use crate::{
    state::State,
    wm::{Mode, geometry::Zone},
};

/// Starts Xwayland on `display`, or the first free display, and returns its number. Its
/// window manager starts when it is ready; if it dies it is started again on the same display.
pub fn start(state: &mut State, display: Option<u32>) -> Option<u32> {
    let (xwayland, client) = match XWayland::spawn(
        &state.isle.display_handle,
        display,
        std::iter::empty::<(String, String)>(),
        std::iter::empty::<String>(),
        true,
        Stdio::null(),
        Stdio::null(),
        |_| (),
    ) {
        Ok(spawned) => spawned,
        Err(err) => {
            warn!("Xwayland did not start: {err}");
            return None;
        }
    };
    let number = xwayland.display_number();
    let dh = state.isle.display_handle.clone();
    let inserted =
        state
            .isle
            .loop_handle
            .insert_source(xwayland, move |event, _, state| match event {
                XWaylandEvent::Ready { x11_socket, .. } => {
                    match X11Wm::start_wm(
                        state.isle.loop_handle.clone(),
                        &dh,
                        x11_socket,
                        client.clone(),
                    ) {
                        Ok(wm) => {
                            info!(display = number, "Xwayland is ready");
                            state.isle.xwm = Some(wm);
                            state.isle.xwayland_deaths = 0;
                        }
                        Err(err) => error!("the X11 window manager did not start: {err}"),
                    }
                }
                XWaylandEvent::Error => {
                    warn!("Xwayland failed as it started");
                    restart_later(state, number);
                }
            });
    match inserted {
        Ok(token) => state.isle.xwayland_source = Some(token),
        Err(err) => {
            error!("no Xwayland source: {err}");
            return None;
        }
    }
    Some(number)
}

/// Starts Xwayland again on `display` after a pause that doubles with each death.
fn restart_later(state: &mut State, display: u32) {
    // The old server's source holds the display's lock until it is dropped.
    if let Some(token) = state.isle.xwayland_source.take() {
        state.isle.loop_handle.remove(token);
    }
    state.isle.xwayland_deaths += 1;
    let wait = Duration::from_secs(1 << state.isle.xwayland_deaths.min(5));
    let _ = state
        .isle
        .loop_handle
        .insert_source(Timer::from_duration(wait), move |_, _, state| {
            start(state, Some(display));
            TimeoutAction::Drop
        });
}

/// Whether an X11 window is left alone: drawn where it asks, never placed, focused,
/// decorated, animated or shadowed. Override-redirect windows, and untitled ones, which
/// Steam's menus and drag icons are (v1's `x11-untitled`, D40).
fn unmanaged(window: &X11Surface) -> bool {
    window.is_override_redirect() || window.title().is_empty()
}

/// Whether an X11 window wears Isle's bar, by the rules a window manager answers to: its
/// type is a normal, dialog or utility window (EWMH); its Motif hints ask for a title; it
/// draws no frame of its own (`_GTK_FRAME_EXTENTS`); it is its rectangle (no SHAPE); and it
/// is not full screen.
fn wants_bar(window: &X11Surface) -> bool {
    use smithay::xwayland::xwm::{MwmDecorationsHint as D, WmWindowType as T};
    let typed = matches!(
        window.window_type(),
        None | Some(T::Normal | T::Dialog | T::Utility)
    );
    // ALL means every part except the ones also listed.
    let titled = window.motif_hints().decorations.is_none_or(|d| {
        if d.contains(D::ALL) {
            !d.contains(D::TITLE)
        } else {
            d.contains(D::TITLE)
        }
    });
    let own_frame = window.frame_extents() != Default::default();
    typed && titled && !own_frame && !window.is_shaped() && !window.is_fullscreen()
}

impl State {
    /// Motif hints or the shape changed: the bar follows.
    fn x11_bar_changed(&mut self, id: u64, window: &X11Surface) {
        let server_side = wants_bar(window);
        if let Some(record) = self.isle.wm.get_mut(id)
            && record.server_side != server_side
        {
            record.server_side = server_side;
            self.isle.sync_bars();
        }
        self.isle.queue_redraw_all();
    }

    fn x11_record(&self, window: &X11Surface) -> Option<u64> {
        self.isle
            .wm
            .records
            .iter()
            .find(|r| r.x11() == Some(window))
            .map(|r| r.id)
    }

    fn map_unmanaged(&mut self, window: X11Surface) {
        let _ = window.set_mapped(true);
        if self
            .isle
            .unmanaged
            .iter()
            .any(|w| w.x11_surface() == Some(&window))
        {
            return;
        }
        self.isle.unmanaged.push(Window::new_x11_window(window));
        self.isle.queue_redraw_all();
    }

    fn forget_unmanaged(&mut self, window: &X11Surface) {
        self.isle
            .unmanaged
            .retain(|w| w.x11_surface() != Some(window));
        self.isle.queue_redraw_all();
    }
}

impl XWaylandShellHandler for State {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.isle.xwayland_shell_state
    }

    /// A window that took focus before Xwayland gave it a surface takes the keyboard now.
    fn surface_associated(
        &mut self,
        _xwm: XwmId,
        _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        window: X11Surface,
    ) {
        let Some(id) = self.x11_record(&window) else {
            return;
        };
        if self.isle.wm.recent.first() == Some(&id) && self.isle.wm.is_shown(id) {
            let window = self.isle.window_by_id(id);
            self.set_focus(window.as_ref());
        }
        self.isle.sync_bars();
        self.isle.queue_redraw_all();
    }
}

impl XwmHandler for State {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.isle.xwm.as_mut().expect("the window manager runs")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if unmanaged(&window) {
            self.map_unmanaged(window);
            return;
        }
        let id = match self.x11_record(&window) {
            Some(id) => id,
            None => self.isle.add_window(Window::new_x11_window(window.clone())),
        };
        let record = self.isle.wm.get_mut(id).unwrap();
        record.pid = window.pid().map(|p| p as i32);
        record.server_side = wants_bar(&window);
        record.rect = window.geometry();
        let _ = window.set_mapped(true);
        if window.is_fullscreen() {
            self.isle.wm.get_mut(id).unwrap().wants_full_screen = true;
        }
        self.map_window(id);
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        self.map_unmanaged(window);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        self.forget_unmanaged(&window);
        if let Some(id) = self.x11_record(&window) {
            self.animate_leaving(id, None);
            self.unmap_window(id);
        }
        if !window.is_override_redirect() {
            let _ = window.set_mapped(false);
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        self.forget_unmanaged(&window);
        if let Some(surface) = self
            .x11_record(&window)
            .and_then(|id| self.isle.wm.get(id))
            .and_then(|r| r.surface())
        {
            self.remove_window(&surface);
        } else if let Some(id) = self.x11_record(&window) {
            self.isle.wm.records.retain(|r| r.id != id);
            self.isle.wm.recent.retain(|r| *r != id);
        }
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let mut geometry = window.geometry();
        if let Some(w) = w {
            geometry.size.w = w as i32;
        }
        if let Some(h) = h {
            geometry.size.h = h as i32;
        }
        // A managed window is placed by the window manager; one not yet mapped, or left
        // alone, keeps where it asks to be.
        let managed = self
            .x11_record(&window)
            .filter(|id| self.isle.wm.get(*id).is_some_and(|r| r.mapped));
        match managed {
            Some(id) => {
                if matches!(self.isle.wm.get(id).map(|r| &r.mode), Some(Mode::Floating)) {
                    if let Some(record) = self.isle.wm.get(id) {
                        record.configure_size(geometry.size);
                    }
                } else {
                    let _ = window.configure(None);
                }
            }
            None => {
                if let Some(x) = x {
                    geometry.loc.x = x;
                }
                if let Some(y) = y {
                    geometry.loc.y = y;
                }
                let _ = window.configure(geometry);
            }
        }
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _geometry: Rectangle<i32, Logical>,
        _above: Option<u32>,
    ) {
        if self
            .isle
            .unmanaged
            .iter()
            .any(|w| w.x11_surface() == Some(&window))
        {
            self.isle.queue_redraw_all();
        }
    }

    fn property_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        property: smithay::xwayland::xwm::WmWindowProperty,
    ) {
        let Some(id) = self.x11_record(&window) else {
            return;
        };
        use smithay::xwayland::xwm::WmWindowProperty as P;
        match property {
            P::Title | P::Class => {
                self.isle.refresh_identity(id);
                self.isle.sync_bars();
                self.isle.notify(crate::worker::Event::WindowsChanged);
            }
            P::MotifHints | P::WindowType | P::FrameExtents => self.x11_bar_changed(id, &window),
            _ => {}
        }
    }

    fn shape_changed(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.x11_bar_changed(id, &window);
        }
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.tile(id, Zone::Fill);
        }
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.restore_box(id);
        }
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.enter_full_screen(id, false);
        }
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.leave_full_screen(id);
        }
    }

    /// Answered with a minimise, never another state (v1's sticky maximise toggle).
    fn minimize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.minimise(id);
        }
    }

    fn unminimize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.unminimise(id);
        }
    }

    fn modal_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.isle.wm.get_mut(id).unwrap().modal = true;
        }
    }

    fn unmodal_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(id) = self.x11_record(&window) {
            self.isle.wm.get_mut(id).unwrap().modal = false;
        }
    }

    /// `_NET_WM_MOVERESIZE`: Isle's own resize, from the pointer as it is.
    fn resize_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32, edge: ResizeEdge) {
        let Some(id) = self.x11_record(&window) else {
            return;
        };
        use crate::grabs::Edges;
        let edges = match edge {
            ResizeEdge::Top => Edges::TOP,
            ResizeEdge::Bottom => Edges::BOTTOM,
            ResizeEdge::Left => Edges::LEFT,
            ResizeEdge::Right => Edges::RIGHT,
            ResizeEdge::TopLeft => Edges::TOP | Edges::LEFT,
            ResizeEdge::TopRight => Edges::TOP | Edges::RIGHT,
            ResizeEdge::BottomLeft => Edges::BOTTOM | Edges::LEFT,
            ResizeEdge::BottomRight => Edges::BOTTOM | Edges::RIGHT,
        };
        self.begin_edge_resize(id, edges, x11_button(button));
    }

    /// `_NET_WM_MOVERESIZE`: Isle's own move with snapping and zones.
    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32) {
        let Some(id) = self.x11_record(&window) else {
            return;
        };
        let Some(window) = self.isle.window_by_id(id) else {
            return;
        };
        let pointer = self.isle.seat.get_pointer().unwrap();
        let start_data = GrabStartData {
            focus: None,
            button: x11_button(button),
            location: pointer.current_location(),
        };
        if let Some(grab) = crate::grabs::MoveGrab::new(self, start_data, window) {
            pointer.set_grab(
                self,
                grab,
                SERIAL_COUNTER.next_serial(),
                smithay::input::pointer::Focus::Clear,
            );
        }
    }

    /// `_NET_ACTIVE_WINDOW`: granted to the app in front, else the window wants attention.
    fn active_window_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _timestamp: u32,
        _currently_active: Option<X11Surface>,
    ) {
        let Some(id) = self.x11_record(&window) else {
            return;
        };
        let app = self.isle.wm.get(id).map(|r| r.app.clone());
        if app.is_some() && app == self.isle.front_app() {
            self.activate(id);
        } else if let Some(record) = self.isle.wm.get_mut(id) {
            record.attention = true;
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }

    fn allow_selection_access(&mut self, xwm: XwmId, _selection: SelectionTarget) -> bool {
        let Some(focus) = self.focused_window() else {
            return false;
        };
        self.isle
            .wm
            .get(focus)
            .and_then(|r| r.x11())
            .is_some_and(|x| x.xwm_id() == Some(xwm))
    }

    fn send_selection(
        &mut self,
        _xwm: XwmId,
        selection: SelectionTarget,
        mime_type: String,
        fd: OwnedFd,
    ) {
        let seat = self.isle.seat.clone();
        let result = match selection {
            SelectionTarget::Clipboard => {
                request_data_device_client_selection(&seat, mime_type, fd)
                    .map_err(|e| e.to_string())
            }
            SelectionTarget::Primary => {
                request_primary_client_selection(&seat, mime_type, fd).map_err(|e| e.to_string())
            }
        };
        if let Err(err) = result {
            warn!("the Wayland selection did not reach Xwayland: {err}");
        }
    }

    fn new_selection(&mut self, _xwm: XwmId, selection: SelectionTarget, mime_types: Vec<String>) {
        let dh = self.isle.display_handle.clone();
        match selection {
            SelectionTarget::Clipboard => {
                set_data_device_selection(&dh, &self.isle.seat, mime_types, ())
            }
            SelectionTarget::Primary => set_primary_selection(&dh, &self.isle.seat, mime_types, ()),
        }
    }

    fn cleared_selection(&mut self, _xwm: XwmId, selection: SelectionTarget) {
        let dh = self.isle.display_handle.clone();
        match selection {
            SelectionTarget::Clipboard => {
                if current_data_device_selection_userdata(&self.isle.seat).is_some() {
                    clear_data_device_selection(&dh, &self.isle.seat);
                }
            }
            SelectionTarget::Primary => {
                if current_primary_selection_userdata(&self.isle.seat).is_some() {
                    clear_primary_selection(&dh, &self.isle.seat);
                }
            }
        }
    }

    /// Xwayland died: its windows go with it, and it starts again on the same display.
    fn disconnected(&mut self, _xwm: XwmId) {
        warn!("Xwayland went away");
        self.isle.xwm = None;
        self.isle.unmanaged.clear();
        let gone: Vec<u64> = self
            .isle
            .wm
            .records
            .iter()
            .filter(|r| r.x11().is_some())
            .map(|r| r.id)
            .collect();
        for id in gone {
            self.unmap_window(id);
            self.isle.shell.detach(id);
            self.isle.wm.records.retain(|r| r.id != id);
            self.isle.wm.recent.retain(|r| *r != id);
        }
        self.isle.notify(crate::worker::Event::WindowsChanged);
        if let Some(display) = self.isle.xdisplay {
            restart_later(self, display);
        }
    }
}

/// X11's button numbers as evdev codes.
fn x11_button(button: u32) -> u32 {
    match button {
        2 => 0x112,
        3 => 0x111,
        _ => 0x110,
    }
}

impl crate::state::Isle {
    /// Keeps X11's stacking order the registry's, so Xwayland knows what covers what.
    pub fn restack_x11(&mut self) {
        let Some(xwm) = self.xwm.as_mut() else {
            return;
        };
        let windows: Vec<X11Surface> = self
            .wm
            .records
            .iter()
            .filter_map(|r| r.x11().cloned())
            .collect();
        if let Err(err) = xwm.update_stacking_order_upwards(windows.iter()) {
            warn!("X11 stacking: {err}");
        }
    }
}
