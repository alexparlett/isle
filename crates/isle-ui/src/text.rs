//! Text in the type styles of the tokens, scaled by the user's text size.

use freya::prelude::*;
use isle_tokens::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Body,
    Callout,
    Clock,
    /// A widget's one large figure: the time, a temperature.
    Figure,
    Footnote,
    Headline,
    Subheadline,
    Title,
    Title2,
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
        Style::Figure => (
            t.type_style_figure_size,
            t.type_style_figure_line,
            t.type_style_figure_weight,
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
        Style::Title2 => (
            t.type_style_title_2_size,
            t.type_style_title_2_line,
            t.type_style_title_2_weight,
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

/// `text` cut in the middle to at most `chars` characters, an ellipsis where the cut is:
/// a long file name keeps its start and its extension.
pub fn cut_middle(text: &str, chars: usize) -> String {
    let count = text.chars().count();
    if count <= chars || chars < 3 {
        return text.to_owned();
    }
    let kept = chars - 1;
    let tail = kept / 2;
    let head = kept - tail;
    let start: String = text.chars().take(head).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

/// A name on `lines` lines in `style`, cut in the middle when it would not fit the width
/// it is given, as Finder's names are. The width is measured; the cut is estimated from the
/// face's average glyph width.
#[derive(Clone, PartialEq)]
pub struct MiddleCut {
    text: String,
    style: Style,
    colour: Color,
    lines: usize,
    centred: bool,
    width: Size,
}

impl MiddleCut {
    pub fn new(text: impl Into<String>, style: Style, colour: Color) -> Self {
        Self { text: text.into(), style, colour, lines: 1, centred: false, width: Size::fill() }
    }

    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines.max(1);
        self
    }

    /// Its width in its parent: the rest of a row is `Size::flex(1.)`.
    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn centred(mut self) -> Self {
        self.centred = true;
        self
    }
}

impl Component for MiddleCut {
    fn render(&self) -> impl IntoElement {
        let theme = crate::use_theme();
        let mut room = use_state(|| 0f32);
        let (size, height, weight) = metrics(&theme, self.style);
        let width = *room.read();
        let shown = if width > 0. {
            let per_line = (width / (size * 0.56)).floor() as usize;
            cut_middle(&self.text, per_line * self.lines)
        } else {
            self.text.clone()
        };
        rect()
            .width(self.width.clone())
            .on_sized(move |e: Event<SizedEventData>| room.set_if_modified(e.area.width()))
            .child(
                label()
                    .width(Size::fill())
                    .font_size(size)
                    .line_height(height / size)
                    .font_weight(weight as i32)
                    .color(self.colour)
                    .max_lines(self.lines)
                    .text_overflow(TextOverflow::Ellipsis)
                    .maybe(self.centred, |l| l.text_align(TextAlign::Center))
                    .text(shown),
            )
    }
}

#[cfg(test)]
mod middle_tests {
    use super::*;

    #[test]
    fn a_long_name_keeps_its_start_and_its_end() {
        assert_eq!(cut_middle("Screenshot 2026-10-28 at 09.14.22.png", 15), "Screens….22.png");
        assert_eq!(cut_middle("short.txt", 15), "short.txt");
    }
}
