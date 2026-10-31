//! The kinds' marks and the small parts every view shares (`p1/surfaces/cleaner.md` Kinds).

use freya::prelude::*;
use isle_scan::Kind;
use isle_tokens::{Colour, Theme};
use isle_ui::{Glyph, colour, text};

pub fn kind_colour(theme: &Theme, kind: Kind) -> Colour {
    let t = &theme.tokens;
    match kind {
        Kind::Games => t.kind_games,
        Kind::Video => t.kind_video,
        Kind::Images => t.kind_images,
        Kind::Audio => t.kind_audio,
        Kind::Documents => t.kind_documents,
        Kind::Archives => t.kind_archives,
        Kind::Developer => t.kind_developer,
        Kind::System => t.kind_apps,
        Kind::Caches => t.kind_caches,
        Kind::Other => t.kind_other,
    }
}

pub fn kind_glyph(kind: Kind) -> &'static str {
    match kind {
        Kind::Games => "game-controller",
        Kind::Video => "film-strip",
        Kind::Images => "image",
        Kind::Audio => "music-notes",
        Kind::Documents => "file-text",
        Kind::Archives => "file-zip",
        Kind::Developer => "code",
        Kind::System => "package",
        Kind::Caches => "stack",
        Kind::Other => "shapes",
    }
}

/// The kind's tile: its colour filled, its symbol in the view's colour; ringed under Increase
/// Contrast.
pub fn kind_tile(theme: &Theme, kind: Kind) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(t.size_pane_tile))
        .height(Size::px(t.size_pane_tile))
        .corner_radius(t.radius_row)
        .background(colour(kind_colour(theme, kind)))
        .maybe(theme.appearance.increase_contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .center()
        .a11y_alt(kind.name())
        .child(Glyph::new(
            format!("{}-fill", kind_glyph(kind)),
            16.,
            colour(t.colour_view),
        ))
        .into_element()
}

/// A tile like the kind's for something that is no kind: a glyph on a fill.
pub fn plain_tile(theme: &Theme, glyph: &str, fill: Colour, ink: Colour) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(t.size_pane_tile))
        .height(Size::px(t.size_pane_tile))
        .corner_radius(t.radius_row)
        .background(colour(fill))
        .center()
        .child(Glyph::new(glyph.to_owned(), 16., colour(ink)))
        .into_element()
}

/// Figures in the monospaced face, tabular.
pub fn mono(theme: &Theme, ink: Colour, value: impl Into<String>) -> Element {
    let t = &theme.tokens;
    label()
        .font_family("JetBrains Mono")
        .font_size(t.type_style_body_size)
        .color(colour(ink))
        .max_lines(1)
        .text(value.into())
        .into_element()
}

/// `conventions.md`'s determinate bar: a track and the share filled.
pub fn progress_bar(theme: &Theme, share: f32, width: Size, ink: Colour) -> Element {
    let t = &theme.tokens;
    rect()
        .width(width)
        .height(Size::px(6.))
        .corner_radius(3.)
        .background(colour(t.colour_fill_strong))
        .overflow(Overflow::Clip)
        .child(
            rect()
                .width(Size::percent(share.clamp(0., 1.) * 100.))
                .height(Size::fill())
                .corner_radius(3.)
                .background(colour(ink)),
        )
        .into_element()
}

/// A path for a second line, cut in the middle to `chars`.
pub fn cut_path(path: &std::path::Path, chars: usize) -> String {
    text::cut_middle(&crate::model::shown_path(path), chars)
}
