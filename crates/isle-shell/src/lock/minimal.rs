//! The minimal lock: what stands in once the face has panicked twice in a minute. No picture,
//! plates or glass; window roles throughout, and nothing on it moves.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_ui::{Glyph, colour, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::Frame;
use crate::surface::{Here, Layer, Scope, SurfaceDef};

#[derive(Default)]
pub struct MinimalSurface {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl MinimalSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, output: &str, frame: Frame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
    }

    /// Puts `frame` in force on `output`; call it in that instance's root scope.
    pub fn set(&self, output: &str, frame: Frame) {
        self.frames
            .borrow_mut()
            .insert(output.to_owned(), frame.clone());
        if let Some(mut state) = self.states.borrow().get(output).copied() {
            state.set_if_modified(frame);
        }
    }
}

impl SurfaceDef for MinimalSurface {
    fn name(&self) -> &'static str {
        "lock-minimal"
    }

    fn layer(&self) -> Layer {
        Layer::Lock
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(MinimalApp {
            frames: self.frames.clone(),
            states: self.states.clone(),
        })
    }

    fn transitions(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct MinimalApp {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl App for MinimalApp {
    fn render(&self) -> impl IntoElement {
        let here = use_consume::<Here>().0;
        use_drop({
            let (states, here) = (self.states.clone(), here.clone());
            move || {
                states.borrow_mut().remove(&here);
            }
        });
        let state = use_hook(|| {
            let frame = self.frames.borrow().get(&here).cloned().unwrap_or_default();
            let state = State::create(frame);
            self.states.borrow_mut().insert(here.clone(), state);
            state
        });
        Minimal { frame: state }
    }
}

#[derive(PartialEq)]
struct Minimal {
    frame: State<Frame>,
}

impl Component for Minimal {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let face = &frame.face;
        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);
        let time = label()
            .font_size(t.type_style_display_size)
            .line_height(t.type_style_display_line / t.type_style_display_size)
            .font_weight(t.type_style_display_weight as i32)
            .color(ink)
            .text(face.time.clone());
        let mut root = rect()
            .expanded()
            .background(colour(t.colour_window_sunken))
            .child(
                rect()
                    .position(
                        Position::new_absolute()
                            .top(t.size_strip + t.space_xxxl)
                            .left(0.),
                    )
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .child(time),
            );
        if !frame.here {
            return root.into_element();
        }
        let field_h = t.size_control_large;
        let field_top = frame.height * 0.8 - field_h / 2.0;
        let message_line = t.type_style_callout_line;
        let dots = rect()
            .horizontal()
            .spacing(t.space_xs)
            .children((0..face.dots.min(24)).map(|_| {
                rect()
                    .width(Size::px(6.))
                    .height(Size::px(6.))
                    .corner_radius(3.)
                    .background(ink)
                    .into_element()
            }));
        let field = rect()
            .a11y_focusable(true)
            .a11y_auto_focus(true)
            .a11y_role(AccessibilityRole::PasswordInput)
            .a11y_alt("Password")
            .width(Size::px(t.size_lock_field))
            .height(Size::px(field_h))
            .padding((0., t.space_m))
            .corner_radius(t.radius_control)
            .background(colour(t.colour_window_raised))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(1.0)
                    .alignment(BorderAlignment::Inner),
            )
            .horizontal()
            .cross_align(Alignment::Center)
            .child(dots);
        let status = match face.line.text() {
            Some((glyph, words)) => {
                let tint = if face.line.acts() { ink } else { ink_2 };
                rect()
                    .height(Size::px(16.))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .maybe_child(glyph.map(|g| Glyph::new(g, t.size_glyph_small, tint)))
                    .child(text::line(&theme, text::Style::Callout, tint, words))
            }
            None => rect().height(Size::px(16.)),
        };
        root = root.child(
            rect()
                .position(
                    Position::new_absolute()
                        .top(field_top - t.space_m - message_line)
                        .left(0.),
                )
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .child(text::line(
                    &theme,
                    text::Style::Callout,
                    ink_2,
                    "The lock screen had a problem. Enter your password to unlock.",
                ))
                .child(rect().height(Size::px(t.space_m)))
                .child(field)
                .child(rect().height(Size::px(t.space_s)))
                .child(status),
        );
        root.into_element()
    }
}
