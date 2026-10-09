//! Spotlight: the field, its results and the Apps view (`p1/structure/spotlight.md`,
//! `p1/surfaces/spotlight.md`).

pub mod model;
pub mod surface;

pub use surface::{Frame, SpotlightAction, SpotlightSurface};

/// The places in the emoji table of the glyphs the colour emoji font can draw; symbols are
/// kept, as any text font draws them. Every place when there is no colour emoji font. Run on
/// the loop: it asks the font manager the shell already holds, and fontconfig from the worker
/// would hold the loop's text layout up behind its lock.
pub fn drawable_emoji() -> Vec<usize> {
    use freya::engine::prelude::{FontMgr, FontStyle};
    let table = isle_search::sources::emoji::table();
    let Some(typeface) = FontMgr::default().match_family_style("Noto Color Emoji", FontStyle::normal()) else {
        return (0..table.len()).collect();
    };
    table
        .iter()
        .enumerate()
        .filter(|(_, g)| {
            let symbol = g.sub.is_empty();
            let first = g.text.chars().next().map_or(0, |c| c as i32);
            symbol || typeface.unichar_to_glyph(first) != 0
        })
        .map(|(i, _)| i)
        .collect()
}
