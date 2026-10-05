//! What the seat's pointer and keyboard can focus: a client's surface or a shell surface.

use std::{borrow::Cow, sync::Arc};

use isle_shell::ShellFocus;
use smithay::{
    backend::input::{Axis, KeyState},
    input::{
        Seat,
        dnd::{DndFocus, Source},
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent, MotionEvent,
            PointerTarget, RelativeMotionEvent,
        },
    },
    reexports::wayland_server::{
        DisplayHandle, Resource, backend::ObjectId, protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Serial},
    wayland::{seat::WaylandFocus, selection::data_device::WlOfferData},
};

use crate::state::State;

#[derive(Clone, Debug, PartialEq)]
pub enum Focus {
    Surface(WlSurface),
    Shell(ShellFocus),
    /// The resize band around a decorated window's frame; pointer only.
    Edge { window: u64, edges: u32 },
}

impl From<WlSurface> for Focus {
    fn from(surface: WlSurface) -> Self {
        Focus::Surface(surface)
    }
}

impl From<&WlSurface> for Focus {
    fn from(surface: &WlSurface) -> Self {
        Focus::Surface(surface.clone())
    }
}

impl IsAlive for Focus {
    fn alive(&self) -> bool {
        match self {
            Focus::Surface(s) => s.alive(),
            Focus::Shell(s) => s.alive(),
            Focus::Edge { .. } => true,
        }
    }
}

impl WaylandFocus for Focus {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        match self {
            Focus::Surface(s) => Some(Cow::Borrowed(s)),
            Focus::Shell(_) | Focus::Edge { .. } => None,
        }
    }

    fn same_client_as(&self, object_id: &ObjectId) -> bool {
        match self {
            Focus::Surface(s) => s.id().same_client_as(object_id),
            Focus::Shell(_) | Focus::Edge { .. } => false,
        }
    }
}

impl Focus {
    pub fn surface(&self) -> Option<&WlSurface> {
        match self {
            Focus::Surface(s) => Some(s),
            Focus::Shell(_) | Focus::Edge { .. } => None,
        }
    }
}

macro_rules! gesture {
    ($name:ident, $event:ty) => {
        fn $name(&self, seat: &Seat<State>, data: &mut State, event: &$event) {
            if let Focus::Surface(s) = self {
                PointerTarget::$name(s, seat, data, event);
            }
        }
    };
}

impl PointerTarget<State> for Focus {
    fn enter(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        match self {
            Focus::Surface(s) => PointerTarget::enter(s, seat, data, event),
            Focus::Shell(f) => {
                data.isle.shell.pointer_motion(f, event.location);
                data.isle.set_shell_cursor(f);
            }
            Focus::Edge { edges, .. } => {
                data.isle.cursor_status =
                    smithay::input::pointer::CursorImageStatus::Named(resize_cursor(*edges));
                data.backend.update_cursor(&data.isle);
                data.isle.queue_redraw_all();
            }
        }
    }

    fn motion(&self, seat: &Seat<State>, data: &mut State, event: &MotionEvent) {
        match self {
            Focus::Surface(s) => PointerTarget::motion(s, seat, data, event),
            Focus::Shell(f) => data.isle.shell.pointer_motion(f, event.location),
            Focus::Edge { .. } => {}
        }
    }

    fn relative_motion(&self, seat: &Seat<State>, data: &mut State, event: &RelativeMotionEvent) {
        if let Focus::Surface(s) = self {
            PointerTarget::relative_motion(s, seat, data, event);
        }
    }

    fn button(&self, seat: &Seat<State>, data: &mut State, event: &ButtonEvent) {
        match self {
            Focus::Surface(s) => PointerTarget::button(s, seat, data, event),
            Focus::Shell(f) => data.isle.shell.pointer_button(
                f,
                event.button,
                event.state == smithay::backend::input::ButtonState::Pressed,
            ),
            Focus::Edge { .. } => {}
        }
    }

    fn axis(&self, seat: &Seat<State>, data: &mut State, frame: AxisFrame) {
        match self {
            Focus::Surface(s) => PointerTarget::axis(s, seat, data, frame),
            Focus::Shell(f) => {
                let discrete = (frame.v120.is_some()).then(|| {
                    let (x, y) = frame.v120.unwrap_or_default();
                    (x as f64, y as f64)
                });
                let value = |axis| match axis {
                    Axis::Horizontal => frame.axis.0,
                    Axis::Vertical => frame.axis.1,
                };
                data.isle.shell.pointer_axis(
                    f,
                    value(Axis::Horizontal),
                    value(Axis::Vertical),
                    discrete,
                );
            }
            Focus::Edge { .. } => {}
        }
    }

    fn frame(&self, seat: &Seat<State>, data: &mut State) {
        if let Focus::Surface(s) = self {
            PointerTarget::frame(s, seat, data);
        }
    }

    gesture!(gesture_swipe_begin, GestureSwipeBeginEvent);
    gesture!(gesture_swipe_update, GestureSwipeUpdateEvent);
    gesture!(gesture_swipe_end, GestureSwipeEndEvent);
    gesture!(gesture_pinch_begin, GesturePinchBeginEvent);
    gesture!(gesture_pinch_update, GesturePinchUpdateEvent);
    gesture!(gesture_pinch_end, GesturePinchEndEvent);
    gesture!(gesture_hold_begin, GestureHoldBeginEvent);
    gesture!(gesture_hold_end, GestureHoldEndEvent);

    fn leave(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        serial: Serial,
        time: smithay::backend::input::InputTime,
    ) {
        match self {
            Focus::Surface(s) => PointerTarget::leave(s, seat, data, serial, time),
            Focus::Shell(f) => data.isle.shell.pointer_leave(f),
            Focus::Edge { .. } => {
                data.isle.cursor_status = smithay::input::pointer::CursorImageStatus::default_named();
                data.backend.update_cursor(&data.isle);
            }
        }
    }
}

impl KeyboardTarget<State> for Focus {
    fn enter(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            Focus::Surface(s) => KeyboardTarget::enter(s, seat, data, keys, serial),
            Focus::Shell(f) => data.isle.shell.keyboard_focus(f, true),
            Focus::Edge { .. } => {}
        }
    }

    fn leave(&self, seat: &Seat<State>, data: &mut State, serial: Serial) {
        match self {
            Focus::Surface(s) => KeyboardTarget::leave(s, seat, data, serial),
            Focus::Shell(f) => data.isle.shell.keyboard_focus(f, false),
            Focus::Edge { .. } => {}
        }
    }

    fn key(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: smithay::backend::input::InputTime,
    ) {
        match self {
            Focus::Surface(s) => KeyboardTarget::key(s, seat, data, key, state, serial, time),
            Focus::Shell(f) => {
                let keysym = key.modified_sym();
                let text = keysym.key_char().map(String::from);
                let evdev = key.raw_code().raw().saturating_sub(8);
                data.isle
                    .shell
                    .key(f, keysym, text, evdev, state == KeyState::Pressed);
            }
            Focus::Edge { .. } => {}
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<State>,
        data: &mut State,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self {
            Focus::Surface(s) => KeyboardTarget::modifiers(s, seat, data, modifiers, serial),
            Focus::Shell(_) => data.isle.shell.set_modifiers(&modifiers),
            Focus::Edge { .. } => {}
        }
    }
}

impl From<smithay::desktop::PopupKind> for Focus {
    fn from(popup: smithay::desktop::PopupKind) -> Self {
        Focus::Surface(popup.wl_surface().clone())
    }
}

impl DndFocus<State> for Focus {
    type OfferData<S>
        = WlOfferData<S>
    where
        S: Source;

    fn enter<S: Source>(
        &self,
        data: &mut State,
        dh: &DisplayHandle,
        source: Arc<S>,
        seat: &Seat<State>,
        location: Point<f64, Logical>,
        serial: &Serial,
    ) -> Option<WlOfferData<S>> {
        DndFocus::enter(self.surface()?, data, dh, source, seat, location, serial)
    }

    fn motion<S: Source>(
        &self,
        data: &mut State,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<State>,
        location: Point<f64, Logical>,
        time: smithay::backend::input::InputTime,
    ) {
        if let Some(surface) = self.surface() {
            DndFocus::motion(surface, data, offer, seat, location, time);
        }
    }

    fn leave<S: Source>(
        &self,
        data: &mut State,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<State>,
    ) {
        if let Some(surface) = self.surface() {
            DndFocus::leave(surface, data, offer, seat);
        }
    }

    fn drop<S: Source>(
        &self,
        data: &mut State,
        offer: Option<&mut WlOfferData<S>>,
        seat: &Seat<State>,
    ) {
        if let Some(surface) = self.surface() {
            DndFocus::drop(surface, data, offer, seat);
        }
    }
}

/// The pointer over a resize band, by the edges it resizes.
fn resize_cursor(edges: u32) -> smithay::input::pointer::CursorIcon {
    use crate::grabs::Edges;
    use smithay::input::pointer::CursorIcon as C;
    let (l, r, t, b) = (
        edges & Edges::LEFT != 0,
        edges & Edges::RIGHT != 0,
        edges & Edges::TOP != 0,
        edges & Edges::BOTTOM != 0,
    );
    match (l, r, t, b) {
        (true, _, true, _) => C::NwResize,
        (_, true, true, _) => C::NeResize,
        (true, _, _, true) => C::SwResize,
        (_, true, _, true) => C::SeResize,
        (true, _, _, _) | (_, true, _, _) => C::EwResize,
        _ => C::NsResize,
    }
}
