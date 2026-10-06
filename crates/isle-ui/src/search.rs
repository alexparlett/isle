//! The search field: Freya's input in the tokens' look, a magnifying glass before it
//! (`p1/surfaces/menus.md`, Help's search).

use freya::prelude::*;

use crate::{Glyph, colour, use_theme};

#[derive(Clone, PartialEq)]
pub struct SearchField {
    pub value: Writable<String>,
    pub placeholder: String,
    pub width: f32,
}

impl Component for SearchField {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::px(self.width))
            .height(Size::px(t.size_control))
            .corner_radius(t.radius_row)
            .background(colour(t.colour_fill))
            .horizontal()
            .cross_align(Alignment::Center)
            .padding((0., 0., 0., t.space_s))
            .spacing(t.space_xs)
            .child(Glyph::new(
                "magnifying-glass",
                t.size_glyph_small,
                colour(t.colour_text_2),
            ))
            .child(
                Input::new(self.value.clone())
                    .placeholder(self.placeholder.clone())
                    .auto_focus(true)
                    .width(Size::fill())
                    .background(Color::TRANSPARENT)
                    .hover_background(Color::TRANSPARENT)
                    .focus_background(Color::TRANSPARENT)
                    .border_fill(Color::TRANSPARENT)
                    .hover_border_fill(Color::TRANSPARENT)
                    .focus_border_fill(Color::TRANSPARENT)
                    .focus_ring_fill(Color::TRANSPARENT)
                    .color(colour(t.colour_text))
                    .placeholder_color(colour(t.colour_text_2)),
            )
    }
}
