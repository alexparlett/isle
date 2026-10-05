//! Moving and resizing a window, by any route: Isle's bar, the client's own controls, X11's
//! `_NET_WM_MOVERESIZE` and the modifier drag.

use std::cell::RefCell;

use smithay::{
    desktop::Window,
    input::pointer::{
        AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
        GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
        GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab,
        PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, Size},
    wayland::{compositor, shell::xdg::SurfaceCachedState},
};

use crate::state::State;

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;

/// Forwards the pointer events a grab does not change.
macro_rules! forward_rest {
    () => {
        fn relative_motion(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            focus: Option<(crate::focus::Focus, Point<f64, Logical>)>,
            event: &RelativeMotionEvent,
        ) {
            handle.relative_motion(data, focus, event);
        }
        fn axis(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            details: AxisFrame,
        ) {
            handle.axis(data, details)
        }
        fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
            handle.frame(data);
        }
        fn gesture_swipe_begin(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GestureSwipeBeginEvent,
        ) {
            handle.gesture_swipe_begin(data, event)
        }
        fn gesture_swipe_update(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GestureSwipeUpdateEvent,
        ) {
            handle.gesture_swipe_update(data, event)
        }
        fn gesture_swipe_end(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GestureSwipeEndEvent,
        ) {
            handle.gesture_swipe_end(data, event)
        }
        fn gesture_pinch_begin(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GesturePinchBeginEvent,
        ) {
            handle.gesture_pinch_begin(data, event)
        }
        fn gesture_pinch_update(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GesturePinchUpdateEvent,
        ) {
            handle.gesture_pinch_update(data, event)
        }
        fn gesture_pinch_end(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GesturePinchEndEvent,
        ) {
            handle.gesture_pinch_end(data, event)
        }
        fn gesture_hold_begin(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GestureHoldBeginEvent,
        ) {
            handle.gesture_hold_begin(data, event)
        }
        fn gesture_hold_end(
            &mut self,
            data: &mut State,
            handle: &mut PointerInnerHandle<'_, State>,
            event: &GestureHoldEndEvent,
        ) {
            handle.gesture_hold_end(data, event)
        }
        fn start_data(&self) -> &GrabStartData<State> {
            &self.start_data
        }
        fn unset(&mut self, _data: &mut State) {}
    };
}

pub struct MoveGrab {
    pub start_data: GrabStartData<State>,
    pub window: Window,
    /// The frame's place when the pointer was at the grab's start.
    pub initial: Point<i32, Logical>,
    /// The box a tiled or filled window returns to once the drag gets going.
    pub unsnap: Option<Rectangle<i32, Logical>>,
}

impl MoveGrab {
    pub fn new(data: &State, start_data: GrabStartData<State>, window: Window) -> Option<Self> {
        let id = crate::wm::window_id(&window);
        let initial = data.isle.frame(id)?.loc;
        let unsnap = match data.isle.wm.get(id)?.mode {
            crate::wm::Mode::Tiled { restore, .. } | crate::wm::Mode::Filled { restore } => {
                Some(restore)
            }
            crate::wm::Mode::Floating => None,
        };
        Some(Self {
            start_data,
            window,
            initial,
            unsnap,
        })
    }
}

/// How far the pointer moves before a drag unsnaps a tiled window.
const UNSNAP_DISTANCE: f64 = 4.0;

impl PointerGrab<State> for MoveGrab {
    fn motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        _focus: Option<(crate::focus::Focus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        let id = crate::wm::window_id(&self.window);
        let delta = event.location - self.start_data.location;
        if let Some(restore) = self.unsnap
            && (delta.x.abs() > UNSNAP_DISTANCE || delta.y.abs() > UNSNAP_DISTANCE)
        {
            self.unsnap = None;
            let Some(frame) = data.isle.frame(id) else {
                return;
            };
            let at = crate::wm::geometry::unsnapped(frame, restore.size, event.location);
            self.initial = at - delta.to_i32_round();
            data.float(id);
            data.set_frame(id, Rectangle::new(at, restore.size));
            return;
        }
        if self.unsnap.is_some() {
            return;
        }
        let Some(frame) = data.isle.frame(id) else {
            return;
        };
        let wanted = Rectangle::new(self.initial + delta.to_i32_round(), frame.size);
        let at = data.isle.snapped(id, wanted, event.location);
        let bar = data.isle.bar_height(id);
        data.move_window(id, Point::from((at.x, at.y + bar)));
        data.isle.update_zone_preview(id, event.location);
    }

    fn button(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().contains(&BTN_LEFT)
            || handle.current_pressed().contains(&BTN_RIGHT)
        {
            return;
        }
        handle.unset_grab(self, data, event.serial, event.time, true);
        let id = crate::wm::window_id(&self.window);
        match data.isle.zone_preview.take() {
            Some(preview) => {
                data.isle.queue_redraw(&preview.output);
                data.tile_on(id, preview.zone, &preview.output);
            }
            None => data.isle.record_place(id),
        }
    }

    forward_rest!();
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges(u32);

impl Edges {
    pub const TOP: u32 = 1;
    pub const BOTTOM: u32 = 2;
    pub const LEFT: u32 = 4;
    pub const RIGHT: u32 = 8;

    pub fn new(edges: u32) -> Self {
        Edges(edges)
    }

    fn has(self, edge: u32) -> bool {
        self.0 & edge != 0
    }
}

impl From<xdg_toplevel::ResizeEdge> for Edges {
    fn from(edge: xdg_toplevel::ResizeEdge) -> Self {
        Edges(edge as u32)
    }
}

pub struct ResizeGrab {
    start_data: GrabStartData<State>,
    window: Window,
    edges: Edges,
    initial: Rectangle<i32, Logical>,
    last_size: Size<i32, Logical>,
}

impl ResizeGrab {
    pub fn start(
        start_data: GrabStartData<State>,
        window: Window,
        edges: Edges,
        initial: Rectangle<i32, Logical>,
    ) -> Self {
        if let Some(toplevel) = window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Resizing);
            });
            toplevel.send_pending_configure();
        }
        ResizeState::with(window.toplevel().unwrap().wl_surface(), |state| {
            *state = ResizeState::Resizing { edges, initial };
        });
        Self {
            start_data,
            window,
            edges,
            initial,
            last_size: initial.size,
        }
    }
}

impl PointerGrab<State> for ResizeGrab {
    fn motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        _focus: Option<(crate::focus::Focus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        let mut delta = event.location - self.start_data.location;
        let mut size = self.initial.size;
        if self.edges.has(Edges::LEFT | Edges::RIGHT) {
            if self.edges.has(Edges::LEFT) {
                delta.x = -delta.x;
            }
            size.w = (self.initial.size.w as f64 + delta.x) as i32;
        }
        if self.edges.has(Edges::TOP | Edges::BOTTOM) {
            if self.edges.has(Edges::TOP) {
                delta.y = -delta.y;
            }
            size.h = (self.initial.size.h as f64 + delta.y) as i32;
        }
        let toplevel = self.window.toplevel().unwrap();
        let (min, max) = compositor::with_states(toplevel.wl_surface(), |states| {
            let mut cached = states.cached_state.get::<SurfaceCachedState>();
            let current = cached.current();
            (current.min_size, current.max_size)
        });
        let max_w = if max.w == 0 { i32::MAX } else { max.w };
        let max_h = if max.h == 0 { i32::MAX } else { max.h };
        self.last_size = Size::from((
            size.w.clamp(min.w.max(1), max_w),
            size.h.clamp(min.h.max(1), max_h),
        ));
        toplevel.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
            state.size = Some(self.last_size);
        });
        toplevel.send_pending_configure();
    }

    fn button(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().contains(&BTN_LEFT)
            || handle.current_pressed().contains(&BTN_RIGHT)
        {
            return;
        }
        handle.unset_grab(self, data, event.serial, event.time, true);
        let toplevel = self.window.toplevel().unwrap();
        toplevel.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Resizing);
            state.size = Some(self.last_size);
        });
        let serial = toplevel.send_configure();
        ResizeState::with(toplevel.wl_surface(), |state| {
            *state = ResizeState::WaitingForLastCommit {
                edges: self.edges,
                initial: self.initial,
                serial,
            };
        });
    }

    forward_rest!();
}

/// Kept on the surface, so a commit after the grab ends can still move the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ResizeState {
    #[default]
    Idle,
    Resizing {
        edges: Edges,
        initial: Rectangle<i32, Logical>,
    },
    /// The release's configure, whose commit ends the resize.
    WaitingForLastCommit {
        edges: Edges,
        initial: Rectangle<i32, Logical>,
        serial: smithay::utils::Serial,
    },
}

impl ResizeState {
    fn with<T>(surface: &WlSurface, f: impl FnOnce(&mut Self) -> T) -> T {
        compositor::with_states(surface, |states| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);
            f(&mut states.data_map.get::<RefCell<Self>>().unwrap().borrow_mut())
        })
    }

    /// The resize under way, and whether this commit, of the configure `acked`, ends it.
    fn commit(
        &mut self,
        acked: Option<smithay::utils::Serial>,
    ) -> Option<(Edges, Rectangle<i32, Logical>, bool)> {
        match *self {
            Self::Resizing { edges, initial } => Some((edges, initial, false)),
            Self::WaitingForLastCommit {
                edges,
                initial,
                serial,
            } => {
                let done = acked.is_some_and(|a| a.is_no_older_than(&serial));
                if done {
                    *self = Self::Idle;
                }
                Some((edges, initial, done))
            }
            Self::Idle => None,
        }
    }
}

impl State {
    /// A window resized from its top or left edge keeps its opposite edge still; the
    /// resize's last commit records its place.
    pub fn handle_resize_commit(&mut self, surface: &WlSurface) {
        let Some(window) = self.isle.window_for_surface(surface) else {
            return;
        };
        let Some(mut location) = self.isle.space.element_location(&window) else {
            return;
        };
        let size = window.geometry().size;
        let acked = compositor::with_states(surface, |states| {
            states
                .cached_state
                .get::<smithay::wayland::shell::xdg::ToplevelCachedState>()
                .current()
                .last_acked
                .as_ref()
                .map(|c| c.serial)
        });
        let Some((edges, initial, done)) = ResizeState::with(surface, |state| state.commit(acked))
        else {
            return;
        };
        if edges.has(Edges::LEFT) {
            location.x = initial.loc.x + (initial.size.w - size.w);
        }
        if edges.has(Edges::TOP) {
            location.y = initial.loc.y + (initial.size.h - size.h);
        }
        let id = crate::wm::window_id(&window);
        self.move_window(id, location);
        if done {
            self.isle.record_place(id);
        }
    }
}
