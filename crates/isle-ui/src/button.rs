//! Buttons, in the component sheet's kinds and sizes (DESIGN.md, Components).

use freya::prelude::*;

use crate::{Glyph, colour, focus_ring, shadows, text, use_theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Regular,
    /// The accent fill: the one action Return takes.
    Default,
    /// No fill until hovered.
    Plain,
}

/// A button with a label or a glyph.
#[derive(Clone, PartialEq)]
pub struct Button {
    label: Option<String>,
    glyph: Option<String>,
    alt: Option<String>,
    kind: Kind,
    small: bool,
    enabled: bool,
    on_press: Option<EventHandler<()>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            glyph: None,
            alt: None,
            kind: Kind::Regular,
            small: false,
            enabled: true,
            on_press: None,
        }
    }

    /// A glyph alone; `alt` is its accessible name.
    pub fn icon(glyph: impl Into<String>, alt: impl Into<String>) -> Self {
        Self {
            label: None,
            glyph: Some(glyph.into()),
            alt: Some(alt.into()),
            ..Self::new("")
        }
    }

    pub fn kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// A different accessible name from its label.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    pub fn on_press(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_press = Some(f.into());
        self
    }
}

impl Component for Button {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let mut pressed = use_state(|| false);
        let enabled = self.enabled;
        let (height, style, pad) = if self.small {
            (t.size_control_small, text::Style::Callout, 9.)
        } else {
            (t.size_control, text::Style::Body, 12.)
        };
        let accent = colour(t.colour_accent);
        let inverse = {
            let c = t.colour_on_accent;
            Color::from_rgb(255 - c.r, 255 - c.g, 255 - c.b)
        };
        let fill = match (self.kind, *pressed.read(), *hover.read()) {
            _ if !enabled && self.kind == Kind::Plain => Color::TRANSPARENT,
            (Kind::Default, true, _) => mix(accent, inverse, t.accent_pressed_mix),
            (Kind::Default, false, true) => mix(accent, inverse, t.accent_hover_mix),
            (Kind::Default, false, false) => accent,
            (_, true, _) => colour(t.colour_control_pressed),
            (_, false, true) => colour(t.colour_control_hover),
            (Kind::Plain, false, false) => Color::TRANSPARENT,
            (Kind::Regular, false, false) => colour(t.colour_control),
        };
        let ink = if !enabled {
            colour(t.colour_text_disabled)
        } else if self.kind == Kind::Default {
            colour(t.colour_on_accent)
        } else {
            colour(t.colour_text)
        };
        let on_press = self.on_press.clone();
        rect()
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::Button)
            .map(self.alt.clone().or(self.label.clone()), |el, alt| {
                el.a11y_alt(alt)
            })
            .min_height(Size::px(height))
            .height(Size::px(height))
            .maybe(self.glyph.is_some(), |el| el.width(Size::px(height)))
            .maybe(self.label.is_some(), |el| el.padding((0., pad)))
            .horizontal()
            .center()
            .corner_radius(t.radius_control)
            .background(fill)
            .maybe(self.kind == Kind::Regular && enabled, |el| {
                shadows(&t.shadow_control)
                    .into_iter()
                    .fold(el, |el, s| el.shadow(s))
            })
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| {
                hover.set_if_modified(false);
                pressed.set_if_modified(false);
            })
            .on_pointer_down(move |_| pressed.set_if_modified(enabled))
            .on_mouse_up(move |_| pressed.set_if_modified(false))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                if enabled && let Some(f) = &on_press {
                    f.call(());
                }
            })
            .map(self.glyph.clone(), |el, glyph| {
                el.child(Glyph::new(glyph, t.size_glyph, ink))
            })
            .map(self.label.clone(), |el, label| {
                el.child(text::line(&theme, style, ink, label))
            })
    }
}

/// `a` moved `t` of the way toward `b`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_argb(
        a.a(),
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
    )
}

/// A level from 0 to 1 as a continuous bar (`surfaces/island.md`, Level bar).
#[derive(Clone, PartialEq)]
pub struct LevelBar {
    pub level: f32,
    pub track: Color,
    pub fill: Color,
}

impl Component for LevelBar {
    fn render(&self) -> impl IntoElement {
        let level = self.level.clamp(0.0, 1.0);
        rect()
            .width(Size::fill())
            .height(Size::px(6.))
            .corner_radius(3.)
            .background(self.track)
            .a11y_role(AccessibilityRole::ProgressIndicator)
            .a11y_builder(move |node| node.set_numeric_value((level * 100.0).round() as f64))
            .child(
                rect()
                    .width(Size::percent(level * 100.0))
                    .height(Size::fill())
                    .corner_radius(3.)
                    .background(self.fill),
            )
    }
}
