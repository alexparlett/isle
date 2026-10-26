//! Input routed to the seat. Backend events and the Test interface's injected input take
//! the same handlers, so a test drives exactly what a device would.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        InputTime, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        PointerMotionEvent,
    },
    input::pointer::{AxisFrame, ButtonEvent, CursorImageStatus, MotionEvent},
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::state::State;

impl State {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                use smithay::backend::input::Device;
                let device = event.device();
                let code = event.key_code().raw() - 8;
                self.on_device_key(
                    &device.id(),
                    &device.name(),
                    code,
                    event.state(),
                    Event::time(&event),
                )
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(output) = self.isle.space.outputs().next().cloned() else {
                    return;
                };
                let geometry = self.isle.space.output_geometry(&output).unwrap();
                let location = event.position_transformed(geometry.size) + geometry.loc.to_f64();
                self.on_pointer_to(location, event.time());
            }
            InputEvent::PointerMotion { event } => {
                self.on_pointer_by(event.delta(), event.delta_unaccel(), event.time())
            }
            InputEvent::PointerButton { event } => {
                self.on_button(event.button_code(), event.state(), event.time())
            }
            InputEvent::PointerAxis { event } => {
                let source = event.source();
                let mut frame = AxisFrame::new(event.time()).source(source);
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    let v120 = event.amount_v120(axis);
                    let amount = event
                        .amount(axis)
                        .unwrap_or_else(|| v120.unwrap_or(0.0) * 15.0 / 120.0);
                    if amount != 0.0 {
                        frame = frame.value(axis, amount);
                        if let Some(v120) = v120 {
                            frame = frame.v120(axis, v120 as i32);
                        }
                    }
                    if source == AxisSource::Finger && event.amount(axis) == Some(0.0) {
                        frame = frame.stop(axis);
                    }
                }
                self.on_axis(frame);
            }
            _ => {}
        }
    }

    pub fn on_axis(&mut self, frame: AxisFrame) {
        if self.idle_activity() {
            return;
        }
        if self.mission_reads_input() {
            let (x, y) = frame.axis;
            self.mission_axis(if x.abs() > y.abs() { x } else { y });
            return;
        }
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.axis(self, frame);
        pointer.frame(self);
    }

    /// Moves the pointer by `delta`, held to the outputs and to a constraint; the top edge
    /// sees the travel the edge stopped, and the surface under it the device's own motion.
    pub fn on_pointer_by(
        &mut self,
        delta: Point<f64, Logical>,
        delta_unaccel: Point<f64, Logical>,
        time: InputTime,
    ) {
        self.idle_activity();
        let pointer = self.isle.seat.get_pointer().unwrap();
        let current = pointer.current_location();
        let target = self.isle.clamp_to_outputs(current + delta);
        let target = if self.picking() {
            target
        } else {
            self.constrained_motion(current, target)
        };
        if target != current || !self.pointer_constrained() {
            self.move_pointer(target, time);
        }
        if !self.isle.lock.locked && !self.picking() {
            self.send_relative_motion(delta, delta_unaccel, time);
        }
        if !self.isle.lock.locked && !self.pointer_constrained() {
            self.top_edge_motion(delta);
            self.hot_corner_motion();
        }
    }

    /// Moves the pointer to `location` in the global space.
    pub fn on_pointer_to(&mut self, location: Point<f64, Logical>, time: InputTime) {
        self.idle_activity();
        let from = self.isle.seat.get_pointer().unwrap().current_location();
        self.move_pointer(location, time);
        if !self.isle.lock.locked {
            self.top_edge_motion(location - from);
            self.hot_corner_motion();
        }
    }

    fn move_pointer(&mut self, location: Point<f64, Logical>, time: InputTime) {
        if self.picking() {
            self.pick_pointer_to(location);
            return;
        }
        if self.mission_reads_input() {
            self.mission_pointer_to(location);
            self.isle.queue_pointer_redraw_all();
            return;
        }
        let under = self.isle.surface_under(location);
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time,
            },
        );
        pointer.frame(self);
        self.refresh_constraint();
        if self.isle.lock.locked {
            self.lock_pointer_moved();
        } else {
            self.refresh_pointer_cursor();
            self.update_bar_option();
            self.check_zoom_dwell();
            self.spotlight_drag_check(location);
            self.thumbnail_pointer(location);
            self.toolbar_pointer(location);
        }
        // A nested backend's host draws named cursors; everywhere else the cursor is
        // Isle's to draw, so it moves only with a redraw.
        let host_draws = matches!(self.backend, crate::backend::Backend::Winit(_))
            && !matches!(self.isle.cursor_status, CursorImageStatus::Surface(_));
        if host_draws {
            return;
        }
        // A grab or a drag moves more than the pointer.
        if self.isle.seat.get_pointer().is_some_and(|p| p.is_grabbed()) || self.isle.dnd_icon.is_some() {
            self.isle.queue_redraw_all();
        } else {
            self.isle.queue_pointer_redraw_all();
        }
    }

    /// A press focuses and raises the window under the pointer, then reaches it.
    pub fn on_button(&mut self, button: u32, button_state: ButtonState, time: InputTime) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        // A press that wakes dark screens lands on nothing, and neither does its release.
        if button_state == ButtonState::Pressed && self.idle_activity() {
            self.isle.taken_buttons.insert(button);
            return;
        }
        match button_state {
            ButtonState::Pressed => self.isle.buttons_down.insert(button),
            ButtonState::Released => self.isle.buttons_down.remove(&button),
        };
        if button_state == ButtonState::Pressed {
            self.mark_click();
        }
        // The pick's presses and their releases are its own, even once it has closed.
        if self.picking() {
            if button_state == ButtonState::Pressed {
                self.isle.taken_buttons.insert(button);
            } else {
                self.isle.taken_buttons.remove(&button);
            }
            self.pick_button(button, button_state);
            return;
        }
        if !self.isle.lock.locked && self.thumbnail_button(button, button_state) {
            return;
        }
        // A button bound as a shortcut runs it, and the window never sees the press.
        if button_state == ButtonState::Pressed
            && !self.isle.lock.locked
            && let Some(bound) = self.isle.bindings.lookup_button(self.held_mods(), button)
        {
            self.isle.shortcut_buttons.insert(button);
            self.run_action(bound.action);
            return;
        }
        if button_state == ButtonState::Released && self.isle.shortcut_buttons.remove(&button) {
            return;
        }
        // Mission Control's presses and their releases are its own.
        if !self.isle.lock.locked
            && (self.mission_reads_input()
                || (button_state == ButtonState::Released && self.isle.mission_buttons.contains(&button)))
        {
            if button_state == ButtonState::Pressed {
                self.isle.mission_buttons.insert(button);
            } else {
                self.isle.mission_buttons.remove(&button);
            }
            if self.mission_reads_input() {
                self.mission_button(button, button_state == ButtonState::Pressed);
            }
            return;
        }
        if self.isle.toolbar.is_some() && self.toolbar_button(button, button_state) {
            if button_state == ButtonState::Pressed {
                self.isle.taken_buttons.insert(button);
            } else {
                self.isle.taken_buttons.remove(&button);
            }
            return;
        }
        // A press taken to close a menu is kept from the window, and so is its release.
        if button_state == ButtonState::Pressed && !self.isle.lock.locked && self.top_edge_press() {
            self.isle.taken_buttons.insert(button);
            return;
        }
        if button_state == ButtonState::Released && self.isle.taken_buttons.remove(&button) {
            return;
        }
        // While locked a button goes to the lock's surface under it, and moves no focus.
        if self.isle.lock.locked {
            pointer.button(
                self,
                &ButtonEvent {
                    button,
                    state: button_state,
                    serial,
                    time,
                },
            );
            pointer.frame(self);
            return;
        }
        if button_state == ButtonState::Pressed
            && !pointer.is_grabbed()
            && self.modifier_drag(button, serial, time)
        {
            return;
        }
        if button_state == ButtonState::Pressed
            && self.isle.zoom.menu_for.is_some()
            && !matches!(
                pointer.current_focus(),
                Some(crate::focus::Focus::Shell(f)) if self.isle.shell.name_of(&f) == Some("zoom-menu")
            )
        {
            self.close_zoom_menu();
        }
        if button_state == ButtonState::Pressed
            && !pointer.is_grabbed()
            && let Some(crate::focus::Focus::Edge { window, edges }) = pointer.current_focus()
        {
            let target = self.isle.window_by_id(window);
            self.set_focus(target.as_ref());
            self.begin_edge_resize(window, edges, button);
            return;
        }
        if button_state == ButtonState::Pressed && !pointer.is_grabbed() {
            let location = pointer.current_location();
            let on_shell = matches!(pointer.current_focus(), Some(crate::focus::Focus::Shell(_)));
            match self.isle.layer_taking_focus_at(location) {
                _ if on_shell => {}
                Some(surface) => self.focus_keyboard(Some(surface.into()), serial),
                None => {
                    let window = self
                        .isle
                        .space
                        .element_under(location)
                        .map(|(w, _)| w.clone());
                    let window = self.isle.click_focus(window, location);
                    if let Some(window) = &window {
                        self.desktop_touched(crate::wm::window_id(window));
                    }
                    self.set_focus(window.as_ref());
                    if let Some(window) = window {
                        self.ping(crate::wm::window_id(&window));
                    }
                }
            }
        }
        // Withheld Cmd and Option go with a click, as Ctrl and Alt, after focus has moved.
        let first = button_state == ButtonState::Pressed && self.isle.buttons_down.len() == 1;
        if first {
            self.keyboard_buttons(true);
        }
        pointer.button(
            self,
            &ButtonEvent {
                button,
                state: button_state,
                serial,
                time,
            },
        );
        pointer.frame(self);
        if button_state == ButtonState::Released && self.isle.buttons_down.is_empty() {
            self.keyboard_buttons(false);
        }
    }
}

impl State {
    /// The drag modifiers, Ctrl+Cmd, and a press anywhere on a window: the left button moves
    /// it, the right resizes it from the nearest corner, without raising it (macOS's drag
    /// gesture).
    fn modifier_drag(&mut self, button: u32, serial: smithay::utils::Serial, time: InputTime) -> bool {
        const LEFT: u32 = 0x110;
        const RIGHT: u32 = 0x111;
        let drag = self.isle.bindings.drag;
        if drag.is_empty() || !self.held_mods().contains(drag) || (button != LEFT && button != RIGHT) {
            return false;
        }
        let pointer = self.isle.seat.get_pointer().unwrap();
        let location = pointer.current_location();
        if matches!(pointer.current_focus(), Some(crate::focus::Focus::Shell(_))) {
            return false;
        }
        let Some((window, _)) = self.isle.space.element_under(location).map(|(w, p)| (w.clone(), p))
        else {
            return false;
        };
        let start_data = smithay::input::pointer::GrabStartData {
            focus: None,
            button,
            location,
        };
        let _ = time;
        if button == LEFT {
            let Some(grab) = crate::grabs::MoveGrab::new(self, start_data, window) else {
                return false;
            };
            pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Clear);
        } else {
            let id = crate::wm::window_id(&window);
            let Some(rect) = self.isle.space.element_geometry(&window) else {
                return false;
            };
            use crate::grabs::Edges;
            let centre = rect.loc.to_f64() + rect.size.to_f64().downscale(2.0).to_point();
            let x = if location.x < centre.x { Edges::LEFT } else { Edges::RIGHT };
            let y = if location.y < centre.y { Edges::TOP } else { Edges::BOTTOM };
            self.float(id);
            let grab = crate::grabs::ResizeGrab::start(start_data, window, Edges::new(x | y), rect);
            pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Clear);
        }
        true
    }
}
