//! The switch, the spinner and the scroll area (DESIGN.md, Components).

use freya::{animation::*, prelude::*};

use crate::{colour, focus_ring, shadows, use_theme};

const SWITCH_WIDTH: f32 = 32.0;
const SWITCH_HEIGHT: f32 = 18.0;
const SWITCH_KNOB: f32 = 14.0;

/// On or off, toggled by a click, Space or Return.
#[derive(Clone, PartialEq)]
pub struct Switch {
    on: bool,
    alt: String,
    enabled: bool,
    on_toggle: EventHandler<bool>,
}

impl Switch {
    /// `alt` is its accessible name; `on_toggle` gets the state asked for.
    pub fn new(on: bool, alt: impl Into<String>, on_toggle: impl Into<EventHandler<bool>>) -> Self {
        Self {
            on,
            alt: alt.into(),
            enabled: true,
            on_toggle: on_toggle.into(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for Switch {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let (on, enabled) = (self.on, self.enabled);
        let on_toggle = self.on_toggle.clone();
        let inset = (SWITCH_HEIGHT - SWITCH_KNOB) / 2.0;
        let knob = shadows(&t.shadow_control).into_iter().fold(
            rect()
                .position(Position::new_absolute().top(inset).left(if on {
                    SWITCH_WIDTH - SWITCH_KNOB - inset
                } else {
                    inset
                }))
                .width(Size::px(SWITCH_KNOB))
                .height(Size::px(SWITCH_KNOB))
                .corner_radius(SWITCH_KNOB / 2.0)
                .background(Color::WHITE),
            |el, s| el.shadow(s),
        );
        rect()
            .width(Size::px(SWITCH_WIDTH))
            .height(Size::px(SWITCH_HEIGHT))
            .corner_radius(SWITCH_HEIGHT / 2.0)
            .background(colour(if on {
                t.colour_accent
            } else {
                t.colour_fill_strong
            }))
            .opacity(if enabled { 1.0 } else { 0.45 })
            .maybe(theme.appearance.increase_contrast, |el| {
                el.border(
                    Border::new()
                        .fill(colour(t.colour_hairline_strong))
                        .width(t.stroke_hairline)
                        .alignment(BorderAlignment::Inner),
                )
            })
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::Switch)
            .a11y_alt(self.alt.clone())
            .a11y_builder(move |node| node.set_toggled(accesskit::Toggled::from(on)))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                if enabled {
                    on_toggle.call(!on);
                }
            })
            .child(knob)
    }
}

/// Work under way with no measure of how much is left: a ring with a turning arc.
#[derive(Clone, PartialEq)]
pub struct Spinner {
    size: f32,
    alt: String,
}

impl Spinner {
    pub fn new(size: f32, alt: impl Into<String>) -> Self {
        Self {
            size,
            alt: alt.into(),
        }
    }
}

impl Component for Spinner {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let turn = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0.0, 360.0).time(800)
        });
        let size = self.size;
        let ring = (size / 8.0).max(1.5);
        rect()
            .width(Size::px(size))
            .height(Size::px(size))
            .a11y_role(AccessibilityRole::ProgressIndicator)
            .a11y_alt(self.alt.clone())
            .child(
                rect()
                    .position(Position::new_absolute())
                    .width(Size::px(size))
                    .height(Size::px(size))
                    .corner_radius(size / 2.0)
                    .border(
                        Border::new()
                            .fill(colour(t.colour_fill_strong))
                            .width(ring)
                            .alignment(BorderAlignment::Inner),
                    ),
            )
            .child(
                SvgViewer::new(ARC.as_bytes())
                    .show_loader(false)
                    .width(Size::px(size))
                    .height(Size::px(size))
                    .stroke(colour(t.colour_text_2))
                    .rotation(turn.get().value()),
            )
    }
}

/// A quarter of the ring, drawn on a 16-point grid with a 2-point stroke.
const ARC: &str = r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg"><path d="M8 1 A7 7 0 0 1 15 8" fill="none" stroke="currentColor" stroke-width="2"/></svg>"#;

/// A scroll view in the shell's way: no drag scrolling, so a drag can move a slider inside
/// it, and a thin overlay bar that shows while scrolling.
pub fn scroll_area(theme: &isle_tokens::Theme) -> ScrollView {
    let t = &theme.tokens;
    let (rest, held) = (colour(t.colour_text_2), colour(t.colour_text));
    ScrollView::new()
        .drag_scrolling(false)
        .scrollbar(move |context: ScrollBarContext| {
            ScrollBar::new(context)
                .theme(
                    ScrollBarThemePartial::new()
                        .background(Color::TRANSPARENT)
                        .thumb_background(rest)
                        .hover_thumb_background(held)
                        .active_thumb_background(held),
                )
                .into_element()
        })
}
