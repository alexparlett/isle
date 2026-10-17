//! An app's toolbar as a title bar: a press on empty toolbar moves the window, and a double
//! or secondary click goes to the compositor, which does what the user set
//! (`settings-2026.md` §4). The lights over it are the compositor's, at the app's inset.

use freya::prelude::*;

use crate::{Chrome, wayland::Gesture};

#[derive(Clone, Copy, PartialEq)]
struct ToolbarGesture;

pub trait ToolbarExt {
    /// Interactive children stop the pointer event propagating so they do not move the window.
    fn toolbar(self) -> Self;
}

impl ToolbarExt for Rect {
    fn toolbar(self) -> Self {
        let chrome = use_consume::<Chrome>();
        let hand = move |gesture| {
            if let Some(extras) = chrome.0.borrow().as_ref() {
                extras.gesture(gesture);
            }
        };
        self.on_pointer_down(move |e: Event<PointerEventData>| match e.button() {
            Some(MouseButton::Right) => hand(Gesture::RightClick),
            Some(MouseButton::Middle) => hand(Gesture::MiddleClick),
            Some(MouseButton::Left) => {
                if EventsCombos::<ToolbarGesture>::pressed(e.global_location()).is_double() {
                    hand(Gesture::DoubleClick);
                }
            }
            _ => {}
        })
        .on_global_pointer_move(|e: Event<PointerEventData>| {
            if EventsCombos::<ToolbarGesture>::moved(e.global_location()) {
                Platform::get().with_window(Platform::window_id(), |window| {
                    let _ = window.drag_window();
                });
            }
        })
        .on_global_pointer_up(|_: Event<PointerEventData>| {
            EventsCombos::<ToolbarGesture>::released();
        })
    }
}
