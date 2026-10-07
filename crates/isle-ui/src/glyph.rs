//! The shell's symbols: Phosphor's, and the few Isle draws on its grid (`p1/system/icons.md`).

use freya::prelude::*;

include!(concat!(env!("OUT_DIR"), "/glyphs.rs"));

/// The SVG for `name` in its Regular or Fill weight; Fill falls back to Regular.
pub fn glyph_svg(name: &str, fill: bool) -> Option<&'static [u8]> {
    let find = |n: &str| {
        GLYPHS
            .binary_search_by(|(k, _)| (*k).cmp(n))
            .ok()
            .map(|i| GLYPHS[i].1)
    };
    if fill && let Some(svg) = find(&format!("{name}-fill")) {
        return Some(svg);
    }
    find(name)
}

/// One symbol at `size` points in `colour`; nothing when the name is unknown.
#[derive(Clone, PartialEq)]
pub struct Glyph {
    name: String,
    fill: bool,
    size: f32,
    colour: Color,
    alt: Option<String>,
}

impl Glyph {
    pub fn new(name: impl Into<String>, size: f32, colour: Color) -> Self {
        Self {
            name: name.into(),
            fill: false,
            size,
            colour,
            alt: None,
        }
    }

    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Its accessible name; without one it is decoration.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
}

impl Component for Glyph {
    fn render(&self) -> impl IntoElement {
        let size = Size::px(self.size);
        match glyph_svg(&self.name, self.fill) {
            Some(svg) => SvgViewer::new(svg)
                .color(self.colour)
                .show_loader(false)
                .width(size.clone())
                .height(size)
                .map(self.alt.clone(), |el, alt| el.a11y_alt(alt))
                .into_element(),
            None => rect().width(size.clone()).height(size).into_element(),
        }
    }
}

/// An app's icon from its theme file: SVG through the vector viewer, PNG and the rest
/// through the image one, which cannot read SVG.
#[derive(Clone, PartialEq)]
pub struct AppIcon {
    pub path: std::path::PathBuf,
    pub size: f32,
}

impl AppIcon {
    pub fn new(path: impl Into<std::path::PathBuf>, size: f32) -> Self {
        Self {
            path: path.into(),
            size,
        }
    }
}

impl Component for AppIcon {
    fn render(&self) -> impl IntoElement {
        let size = Size::px(self.size);
        let svg = self
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        if svg {
            SvgViewer::new(self.path.clone())
                .show_loader(false)
                .width(size.clone())
                .height(size)
                .into_element()
        } else {
            ImageViewer::new(self.path.clone())
                .width(size.clone())
                .height(size)
                .into_element()
        }
    }
}

/// A level drawn in two layers: the whole glyph dimmed, the level's glyph over it
/// (`icons.md`, levels).
#[derive(Clone, PartialEq)]
pub struct LevelGlyph {
    pub whole: String,
    pub level: String,
    pub size: f32,
    pub colour: Color,
    pub dimmed: Color,
    pub alt: String,
}

impl Component for LevelGlyph {
    fn render(&self) -> impl IntoElement {
        let layer = |name: &str, colour: Color| {
            rect()
                .position(Position::new_absolute())
                .child(Glyph::new(name, self.size, colour))
        };
        rect()
            .width(Size::px(self.size))
            .height(Size::px(self.size))
            .a11y_alt(self.alt.clone())
            .child(layer(&self.whole, self.dimmed))
            .child(layer(&self.level, self.colour))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_found_by_name_and_weight() {
        assert!(glyph_svg("wifi-high", false).is_some());
        let regular = glyph_svg("moon", false).unwrap();
        let fill = glyph_svg("moon", true).unwrap();
        assert_ne!(regular, fill);
        assert_eq!(glyph_svg("isle-mark", true), glyph_svg("isle-mark", false));
        assert!(glyph_svg("isle-keyboard-slash", false).is_some());
        assert!(glyph_svg("no-such-glyph", false).is_none());
    }
}
