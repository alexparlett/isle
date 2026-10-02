//! Pieces shared by the island and the launcher.

use std::time::Duration;

use freya::prelude::*;

pub const PRIMARY: Color = Color::from_argb(235, 255, 255, 255);
pub const SECONDARY: Color = Color::from_argb(153, 255, 255, 255);
const FIELD: Color = Color::from_argb(20, 255, 255, 255);

/// A one-line field: focus by click, type, Backspace, caret blinking every 500 ms.
#[derive(PartialEq)]
pub struct TextField {
    pub text: State<String>,
    pub id: AccessibilityId,
    pub placeholder: &'static str,
    pub height: f32,
    pub radius: f32,
    pub padding: f32,
    pub font_size: f32,
}

impl Component for TextField {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        let mut text = self.text;
        let focused = use_focus(id)().is_focused();
        let mut caret = use_state(|| true);

        use_hook(move || {
            spawn(async move {
                loop {
                    timer(Duration::from_millis(500)).await;
                    if *Platform::get().focused_accessibility_id.peek() == id {
                        let shown = *caret.peek();
                        caret.set(!shown);
                    }
                }
            });
        });

        let on_key_down = move |e: Event<KeyboardEventData>| {
            match &e.key {
                Key::Named(NamedKey::Backspace) => {
                    text.write().pop();
                }
                Key::Character(c) => text.write().push_str(c),
                _ => return,
            }
            caret.set_if_modified(true);
        };
        let value = text.read().clone();
        let caret_color = if focused && caret() {
            PRIMARY
        } else {
            Color::TRANSPARENT
        };

        rect()
            .a11y_id(id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::TextInput)
            .on_mouse_down(move |_| {
                id.request_focus();
                caret.set_if_modified(true);
            })
            .on_key_down(on_key_down)
            .horizontal()
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(self.height))
            .padding((0., self.padding))
            .corner_radius(self.radius)
            .background(FIELD)
            .font_size(self.font_size)
            .maybe(!value.is_empty(), |el| {
                el.child(label().text(value.clone()))
            })
            .child(
                rect()
                    .width(Size::px(1.5))
                    .height(Size::px(self.font_size * 1.25))
                    .background(caret_color),
            )
            .maybe(value.is_empty(), |el| {
                el.child(label().text(self.placeholder).color(SECONDARY))
            })
    }
}
