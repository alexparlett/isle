//! Input routed to the seat. Backend events and the Test interface's injected input take
//! the same handlers, so a test drives exactly what a device would.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        InputTime, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, Keycode},
        pointer::{AxisFrame, ButtonEvent, CursorImageStatus, MotionEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::{binds::Action, state::State};

impl State {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                self.on_key(event.key_code(), event.state(), Event::time(&event))
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(output) = self.isle.space.outputs().next().cloned() else {
                    return;
                };
                let geometry = self.isle.space.output_geometry(&output).unwrap();
                let location = event.position_transformed(geometry.size) + geometry.loc.to_f64();
                self.on_pointer_to(location, event.time());
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
                let pointer = self.isle.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }

    pub fn on_key(&mut self, code: Keycode, key_state: KeyState, time: InputTime) {
        let serial = SERIAL_COUNTER.next_serial();
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let pressed = key_state == KeyState::Pressed;
        let action = keyboard.input::<Option<Action>, _>(
            self,
            code,
            key_state,
            serial,
            time,
            |state, modifiers, handle| {
                if !pressed {
                    if state.isle.suppressed_keys.remove(&code) {
                        return FilterResult::Intercept(None);
                    }
                    return FilterResult::Forward;
                }
                let action = handle
                    .raw_syms()
                    .iter()
                    .find_map(|sym| state.bound_action(modifiers, *sym));
                match action {
                    Some(action) => {
                        state.isle.suppressed_keys.insert(code);
                        FilterResult::Intercept(Some(action))
                    }
                    None => FilterResult::Forward,
                }
            },
        );
        if let Some(Some(action)) = action {
            self.run_action(action);
        }
    }

    /// Moves the pointer to `location` in the global space.
    pub fn on_pointer_to(&mut self, location: Point<f64, Logical>, time: InputTime) {
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
        if matches!(self.isle.cursor_status, CursorImageStatus::Surface(_)) {
            self.isle.queue_redraw_all();
        }
    }

    /// A press focuses and raises the window under the pointer, then reaches it.
    pub fn on_button(&mut self, button: u32, button_state: ButtonState, time: InputTime) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        if button_state == ButtonState::Pressed && !pointer.is_grabbed() {
            let window = self
                .isle
                .space
                .element_under(pointer.current_location())
                .map(|(w, _)| w.clone());
            self.set_focus(window.as_ref());
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
    }
}
