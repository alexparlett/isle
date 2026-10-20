//! Clients' virtual keyboards and pointers (wlrctl, wtype, an agent driving the desktop).

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Event, InputEvent, KeyboardKeyEvent,
    },
    input::keyboard::FilterResult,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::{
        virtual_keyboard::{VirtualKeyboardBackend, VirtualKeyboardHandler, VirtualKeyboardSpecialEvent},
        virtual_pointer::{VirtualPointerBackend, VirtualPointerHandler},
    },
};
use tracing::warn;

use crate::state::State;

impl VirtualPointerHandler for State {
    fn process_virtual_pointer_event(&mut self, event: InputEvent<VirtualPointerBackend>) {
        match event {
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(area) = self.virtual_pointer_area(event.device().output().as_ref()) else {
                    return;
                };
                let offset: Point<f64, Logical> =
                    (event.x_transformed(area.size.w), event.y_transformed(area.size.h)).into();
                let location = self.isle.clamp_to_outputs(area.loc.to_f64() + offset);
                self.on_pointer_to(location, event.time());
            }
            event => self.process_input_event(event),
        }
    }
}

impl State {
    /// The area an absolute position spans: the pointer's output, or every output.
    fn virtual_pointer_area(
        &self,
        output: Option<&smithay::output::Output>,
    ) -> Option<Rectangle<i32, Logical>> {
        let space = &self.isle.space;
        if let Some(area) = output.and_then(|o| space.output_geometry(o)) {
            return Some(area);
        }
        space
            .outputs()
            .filter_map(|o| space.output_geometry(o))
            .reduce(|a, b| a.merge(b))
    }
}

/// A virtual keyboard's keys are in its own keymap and reach the focus untranslated, past
/// Isle's bindings; none reach the lock.
impl VirtualKeyboardHandler for State {
    fn process_virtual_keyboard_event(&mut self, event: InputEvent<VirtualKeyboardBackend>) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        match event {
            InputEvent::Keyboard { event } => {
                if self.isle.lock.locked {
                    return;
                }
                self.idle_activity();
                let device = event.device();
                // The keymap changes only with nothing held; a physical key brings Isle's back.
                if let Some(keymap) = device.keymap()
                    && keyboard.pressed_keys().is_empty()
                {
                    if let Err(err) = keyboard.set_keymap_from_string(self, keymap.to_string()) {
                        warn!(?err, "a virtual keyboard's keymap was refused");
                        return;
                    }
                    self.isle.keyboards.active = None;
                }
                keyboard.input_from_source::<(), _>(
                    device.source(),
                    self,
                    event.key_code(),
                    event.state(),
                    SERIAL_COUNTER.next_serial(),
                    event.time(),
                    |_, _, _| FilterResult::Forward,
                );
            }
            InputEvent::Special(VirtualKeyboardSpecialEvent::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                device,
            }) => {
                // The masks are in the device's keymap, or the seat's when it sent none.
                if device.keymap().is_none() || self.isle.keyboards.active.is_none() {
                    keyboard.with_xkb_state(self, |mut context| {
                        context.set_modifier_mask(mods_depressed, mods_latched, mods_locked, group)
                    });
                }
            }
            InputEvent::DeviceRemoved { .. } => {
                if self.isle.keyboards.active.is_none() && keyboard.pressed_keys().is_empty() {
                    self.restore_seat_keymap();
                }
            }
            _ => {}
        }
    }
}
