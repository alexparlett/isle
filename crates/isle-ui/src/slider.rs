//! The slider and the scrubber (DESIGN.md, Components; `surfaces/control-centre.md`
//! Sliders).

use freya::prelude::*;

use crate::{colour, focus_ring, shadows, use_theme};

const TRACK: f32 = 4.0;
const KNOB: f32 = 18.0;

/// A level from 0 to 1 set by dragging, clicking the track or the arrow keys. The wheel never
/// moves it, so scrolling a panel cannot change a volume.
#[derive(Clone, PartialEq)]
pub struct Slider {
    value: f32,
    on_change: EventHandler<f32>,
    alt: String,
    glass: bool,
    muted: bool,
    enabled: bool,
    scrubber: bool,
    step: f32,
}

impl Slider {
    /// `alt` is its accessible name.
    pub fn new(
        value: f32,
        alt: impl Into<String>,
        on_change: impl Into<EventHandler<f32>>,
    ) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            on_change: on_change.into(),
            alt: alt.into(),
            glass: false,
            muted: false,
            enabled: true,
            scrubber: false,
            step: 1.0 / 16.0,
        }
    }

    /// On glass the fill is the text colour, as the island's level bar; in windows the accent.
    pub fn glass(mut self, glass: bool) -> Self {
        self.glass = glass;
        self
    }

    /// Muted or unset: the fill keeps its length, greyed.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A media scrubber: no knob at rest, a small one on hover, focus or drag.
    pub fn scrubber(mut self) -> Self {
        self.scrubber = true;
        self
    }

    /// How far an arrow key moves it; Shift moves four steps.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }
}

/// Where along a track of `width` a pointer at `x` puts the value, for a knob `knob` wide
/// whose centre runs from `knob / 2` to `width - knob / 2`.
pub fn value_at(x: f32, width: f32, knob: f32) -> f32 {
    let run = (width - knob).max(1.0);
    ((x - knob / 2.0) / run).clamp(0.0, 1.0)
}

impl Component for Slider {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut area = use_state(Area::default);
        let mut dragging = use_state(|| false);
        let mut hover = use_state(|| false);
        let (value, step, enabled) = (self.value, self.step, self.enabled);
        let knob_shown =
            !self.scrubber || *hover.read() || *dragging.read() || focus() == Focus::Keyboard;
        let knob = if self.scrubber {
            t.size_scrubber_knob
        } else {
            KNOB
        };

        let fill = if !enabled || self.muted {
            colour(t.colour_text_disabled)
        } else if self.glass {
            colour(t.colour_text)
        } else {
            colour(t.colour_accent)
        };
        let width = area.read().width();
        let left = value * (width - knob).max(0.0);

        let keys = {
            let on_change = self.on_change.clone();
            move |e: Event<KeyboardEventData>| {
                let by = if e.modifiers.contains(Modifiers::SHIFT) {
                    step * 4.0
                } else {
                    step
                };
                let to = match e.key {
                    Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => value - by,
                    Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => value + by,
                    Key::Named(NamedKey::Home) => 0.0,
                    Key::Named(NamedKey::End) => 1.0,
                    _ => return,
                };
                e.stop_propagation();
                on_change.call(to.clamp(0.0, 1.0));
            }
        };
        let down = {
            let on_change = self.on_change.clone();
            move |e: Event<PointerEventData>| {
                if !e.data().is_primary() {
                    return;
                }
                e.stop_propagation();
                a11y.request_focus();
                dragging.set(true);
                let x = e.element_location().x as f32;
                on_change.call(value_at(x, area.read().width(), knob));
            }
        };
        let moved = {
            let on_change = self.on_change.clone();
            move |e: Event<PointerEventData>| {
                if *dragging.peek() {
                    e.prevent_default();
                    let x = e.global_location().x as f32 - area.read().min_x();
                    on_change.call(value_at(x, area.read().width(), knob));
                }
            }
        };

        rect()
            .width(Size::fill())
            .height(Size::px(t.size_control))
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::Slider)
            .a11y_alt(self.alt.clone())
            .a11y_builder(move |node| {
                node.set_numeric_value((value * 100.0).round() as f64);
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(100.0);
            })
            .on_sized(move |e: Event<SizedEventData>| area.set_if_modified(e.area))
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .maybe(enabled, |el| {
                el.on_key_down(keys)
                    .on_pointer_down(down)
                    .on_capture_global_pointer_move(moved)
                    .on_global_pointer_up(move |_| dragging.set_if_modified(false))
            })
            .main_align(Alignment::Center)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(TRACK))
                    .corner_radius(TRACK / 2.0)
                    .background(colour(t.colour_fill_strong))
                    .child(
                        rect()
                            .width(Size::px(left + knob / 2.0))
                            .height(Size::fill())
                            .corner_radius(TRACK / 2.0)
                            .background(fill),
                    ),
            )
            .maybe(knob_shown, |el| {
                let shadow = shadows(&t.shadow_control);
                el.child(
                    shadow.into_iter().fold(
                        // Over the fill, which is a level deeper: a layer is its depth plus
                        // this, and within one layer Freya draws in no set order.
                        rect()
                            .layer(2)
                            .position(
                                Position::new_absolute()
                                    .left(left)
                                    .top((t.size_control - knob) / 2.0),
                            )
                            .width(Size::px(knob))
                            .height(Size::px(knob))
                            .corner_radius(knob / 2.0)
                            .background(Color::WHITE)
                            .maybe(focus() == Focus::Keyboard, |el| {
                                el.border(focus_ring(&theme))
                            }),
                        |el, s| el.shadow(s),
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::value_at;

    #[test]
    fn the_knobs_centre_runs_inside_the_track() {
        assert_eq!(value_at(9.0, 218.0, 18.0), 0.0);
        assert_eq!(value_at(0.0, 218.0, 18.0), 0.0);
        assert_eq!(value_at(209.0, 218.0, 18.0), 1.0);
        assert_eq!(value_at(500.0, 218.0, 18.0), 1.0);
        assert_eq!(value_at(109.0, 218.0, 18.0), 0.5);
    }
}
