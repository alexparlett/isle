//! Text in the type styles of the tokens, scaled by the user's text size.

use freya::prelude::*;
use isle_tokens::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Body,
    Callout,
    Clock,
    Footnote,
    Headline,
    Subheadline,
    Title,
    Title3,
}

/// Size, line height and weight of `style`, scaled.
pub fn metrics(theme: &Theme, style: Style) -> (f32, f32, f32) {
    let t = &theme.tokens;
    let (size, line, weight) = match style {
        Style::Body => (
            t.type_style_body_size,
            t.type_style_body_line,
            t.type_style_body_weight,
        ),
        Style::Callout => (
            t.type_style_callout_size,
            t.type_style_callout_line,
            t.type_style_callout_weight,
        ),
        Style::Clock => (
            t.type_style_clock_size,
            t.type_style_clock_line,
            t.type_style_clock_weight,
        ),
        Style::Footnote => (
            t.type_style_footnote_size,
            t.type_style_footnote_line,
            t.type_style_footnote_weight,
        ),
        Style::Headline => (
            t.type_style_headline_size,
            t.type_style_headline_line,
            t.type_style_headline_weight,
        ),
        Style::Subheadline => (
            t.type_style_subheadline_size,
            t.type_style_subheadline_line,
            t.type_style_subheadline_weight,
        ),
        Style::Title => (
            t.type_style_title_size,
            t.type_style_title_line,
            t.type_style_title_weight,
        ),
        Style::Title3 => (
            t.type_style_title_3_size,
            t.type_style_title_3_line,
            t.type_style_title_3_weight,
        ),
    };
    let scale = theme.appearance.text_scale.min(t.type_scale_max);
    (size * scale, line * scale, weight)
}

/// One line of `text` in `style` and `colour`, cut with an ellipsis where it does not fit.
pub fn line(theme: &Theme, style: Style, colour: Color, text: impl Into<String>) -> Label {
    let (size, height, weight) = metrics(theme, style);
    label()
        .font_size(size)
        .line_height(height / size)
        .font_weight(weight as i32)
        .color(colour)
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis)
        .text(text.into())
}
