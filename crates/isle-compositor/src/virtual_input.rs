//! Clients' virtual keyboards and pointers (wlrctl, wtype, an agent driving the desktop).

use smithay::{
    backend::input::{AbsolutePositionEvent, Event, InputEvent, KeyState, KeyboardKeyEvent},
    input::keyboard::KeyboardSource,
    utils::{Logical, Point, Rectangle},
    wayland::{
        virtual_keyboard::{
            VirtualKeyboardBackend, VirtualKeyboardDevice, VirtualKeyboardHandler,
            VirtualKeyboardSpecialEvent,
        },
        virtual_pointer::{VirtualPointerBackend, VirtualPointerHandler},
    },
};

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

/// A virtual keyboard is a keyboard of Isle's, by its source; none reaches the lock.
impl VirtualKeyboardHandler for State {
    fn process_virtual_keyboard_event(&mut self, event: InputEvent<VirtualKeyboardBackend>) {
        match event {
            InputEvent::DeviceAdded { device } => {
                self.virtual_keyboard_added(&device_id(&device), device.keymap());
            }
            InputEvent::Special(VirtualKeyboardSpecialEvent::KeymapChanged { device }) => {
                self.virtual_keymap_changed(&device_id(&device), device.keymap());
            }
            InputEvent::Keyboard { event } => {
                let id = device_id(&event.device());
                // Its releases still come, so the keys it pressed before the lock let go.
                if self.isle.lock.locked && event.state() == KeyState::Pressed {
                    return;
                }
                let code = event.key_code().raw() - 8;
                self.on_virtual_key(&id, code, event.state(), event.time());
            }
            InputEvent::Special(VirtualKeyboardSpecialEvent::Modifiers {
                device,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            }) => {
                // The masks are in the device's keymap, so they hold only while it is the seat's.
                let spec = self.isle.keyboards.get(&device_id(&device)).map(|d| d.spec.clone());
                if spec.is_some() && self.isle.keyboards.active == spec {
                    let keyboard = self.isle.seat.get_keyboard().unwrap();
                    keyboard.with_xkb_state(self, |mut context| {
                        context.set_modifier_mask(mods_depressed, mods_latched, mods_locked, group)
                    });
                }
            }
            InputEvent::DeviceRemoved { device } => {
                self.virtual_keyboard_removed(&device_id(&device));
            }
            _ => {}
        }
    }
}

/// Unique among every client's keyboards, where the protocol object's id is not.
fn device_id(device: &VirtualKeyboardDevice) -> String {
    match device.source() {
        KeyboardSource::Auxiliary(n) => format!("virtual-{n}"),
        KeyboardSource::Physical => "virtual".to_owned(),
    }
}
