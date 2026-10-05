//! Input routed to the seat. Backend events and the Test interface's injected input take
//! the same handlers, so a test drives exactly what a device would.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        InputTime, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        PointerMotionEvent,
    },
    input::{
        keyboard::{FilterResult, Keycode},
        pointer::{AxisFrame, ButtonEvent, CursorImageStatus, MotionEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use tracing::debug;

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
            InputEvent::PointerMotion { event } => {
                let pointer = self.isle.seat.get_pointer().unwrap();
                let target = self
                    .isle
                    .clamp_to_outputs(pointer.current_location() + event.delta());
                self.on_pointer_to(target, event.time());
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
                debug!(
                    key = ?handle.modified_sym(),
                    pressed,
                    logo = modifiers.logo,
                    ctrl = modifiers.ctrl,
                    alt = modifiers.alt,
                    shift = modifiers.shift,
                    "key"
                );
                if !pressed {
                    if state.isle.suppressed_keys.remove(&code) {
                        return FilterResult::Intercept(None);
                    }
                    return FilterResult::Forward;
                }
                let action = State::vt_for(handle.modified_sym()).or_else(|| {
                    handle
                        .raw_syms()
                        .iter()
                        .find_map(|sym| state.bound_action(modifiers, *sym))
                });
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

    pub fn on_axis(&mut self, frame: AxisFrame) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        pointer.axis(self, frame);
        pointer.frame(self);
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
        // A nested backend's host draws named cursors; everywhere else the cursor is
        // Isle's to draw, so it moves only with a redraw.
        let host_draws = matches!(self.backend, crate::backend::Backend::Winit(_))
            && !matches!(self.isle.cursor_status, CursorImageStatus::Surface(_));
        if !host_draws {
            self.isle.queue_redraw_all();
        }
    }

    /// A press focuses and raises the window under the pointer, then reaches it.
    pub fn on_button(&mut self, button: u32, button_state: ButtonState, time: InputTime) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let serial = SERIAL_COUNTER.next_serial();
        if button_state == ButtonState::Pressed
            && !pointer.is_grabbed()
            && self.modifier_drag(button, serial, time)
        {
            return;
        }
        if button_state == ButtonState::Pressed && !pointer.is_grabbed() {
            let location = pointer.current_location();
            let on_shell = matches!(pointer.current_focus(), Some(crate::focus::Focus::Shell(_)));
            match self.isle.layer_taking_focus_at(location) {
                _ if on_shell => {}
                Some(surface) => {
                    let keyboard = self.isle.seat.get_keyboard().unwrap();
                    keyboard.set_focus(self, Some(surface.into()), serial);
                }
                None => {
                    let window = self
                        .isle
                        .space
                        .element_under(location)
                        .map(|(w, _)| w.clone());
                    self.set_focus(window.as_ref());
                }
            }
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

impl State {
    /// Ctrl+Cmd and a press anywhere on a window: the left button moves it, the right resizes
    /// it from the nearest corner, without raising it (macOS's drag gesture).
    fn modifier_drag(&mut self, button: u32, serial: smithay::utils::Serial, time: InputTime) -> bool {
        const LEFT: u32 = 0x110;
        const RIGHT: u32 = 0x111;
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let mods = keyboard.modifier_state();
        if !(mods.logo && mods.ctrl) || (button != LEFT && button != RIGHT) {
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
