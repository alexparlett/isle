//! The sidebar: the kinds, then the keyring's collections, each with its tile and count, a
//! locked collection with its lock (`p1/surfaces/keychain.md` The sidebar).

use freya::prelude::*;
use isle_tokens::{Colour, Theme};
use isle_ui::{Glyph, colour, text};

use crate::model::{self, Keyring, Row, Snapshot};

pub fn pane_colour(theme: &Theme, name: &str) -> Colour {
    let t = &theme.tokens;
    match name {
        "blue" => t.pane_blue,
        "indigo" => t.pane_indigo,
        "teal" => t.pane_teal,
        "graphite" => t.pane_graphite,
        "purple" => t.pane_purple,
        _ => t.pane_grey,
    }
}

/// A sidebar row's tile: the glyph's fill, white, on the row's hue.
pub fn row_tile(theme: &Theme, row: &Row) -> Element {
    let t = &theme.tokens;
    let (glyph, hue) = row.tile();
    rect()
        .width(Size::px(t.size_pane_tile))
        .height(Size::px(t.size_pane_tile))
        .corner_radius(t.radius_row)
        .background(colour(pane_colour(theme, hue)))
        .maybe(theme.appearance.increase_contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .center()
        .child(Glyph::new(format!("{glyph}-fill"), 16., Color::WHITE))
        .into_element()
}

enum Trailing {
    Count(usize),
    Lock,
}

pub struct Sidebar<'a> {
    pub snapshot: &'a Snapshot,
    pub selected: &'a Row,
    pub active: bool,
    pub on_select: EventHandler<Row>,
    pub on_menu: EventHandler<(Row, (f32, f32))>,
}

pub fn view(theme: &Theme, s: Sidebar<'_>) -> Element {
    let t = &theme.tokens;
    let kinds: Vec<Element> = Row::KINDS
        .iter()
        .map(|row| {
            let title = row.title(s.snapshot);
            let count = model::count(s.snapshot, row);
            row_element(
                theme,
                row.clone(),
                title,
                Trailing::Count(count),
                s.selected == row,
                s.active,
                s.on_select.clone(),
                None,
            )
        })
        .collect();
    let sources: Vec<Element> = if s.snapshot.keyring == Keyring::Ready {
        s.snapshot
            .sources
            .iter()
            .map(|source| {
                let row = Row::Source(source.path.clone());
                let trailing = if source.locked {
                    Trailing::Lock
                } else {
                    Trailing::Count(model::count(s.snapshot, &row))
                };
                let selected = *s.selected == row;
                row_element(
                    theme,
                    row,
                    source.label.clone(),
                    trailing,
                    selected,
                    s.active,
                    s.on_select.clone(),
                    Some(s.on_menu.clone()),
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    rect()
        .width(Size::px(t.size_sidebar))
        .height(Size::fill())
        .background(colour(t.colour_sidebar))
        .padding(t.space_s)
        .spacing(t.space_m)
        .a11y_role(AccessibilityRole::List)
        .child(
            rect()
                .width(Size::fill())
                .spacing(t.space_xxs)
                .children(kinds),
        )
        .maybe_child((!sources.is_empty()).then(|| {
            rect()
                .width(Size::fill())
                .spacing(t.space_xxs)
                .children(sources)
                .into_element()
        }))
        .into_element()
}

#[allow(clippy::too_many_arguments)]
fn row_element(
    theme: &Theme,
    row: Row,
    title: String,
    trailing: Trailing,
    selected: bool,
    active: bool,
    on_select: EventHandler<Row>,
    on_menu: Option<EventHandler<(Row, (f32, f32))>>,
) -> Element {
    let t = &theme.tokens;
    let (fill, ink, ink_2) = match (selected, active) {
        (true, true) => (
            Some(t.colour_accent),
            t.colour_on_accent,
            t.colour_on_accent,
        ),
        (true, false) => (Some(t.colour_fill_strong), t.colour_text, t.colour_text_2),
        _ => (None, t.colour_text, t.colour_text_2),
    };
    let tile = row_tile(theme, &row);
    let menu_row = row.clone();
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row))
        .padding((0., t.space_s))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .corner_radius(t.radius_row)
        .map(fill, |el, f| el.background(colour(f)))
        .a11y_role(AccessibilityRole::ListItem)
        .a11y_alt(title.clone())
        .a11y_builder(move |node| node.set_selected(selected))
        .on_mouse_down(move |e: Event<MouseEventData>| {
            if e.button == Some(MouseButton::Right)
                && let Some(on_menu) = &on_menu
            {
                e.stop_propagation();
                on_menu.call((
                    menu_row.clone(),
                    (e.global_location.x as f32, e.global_location.y as f32),
                ));
            }
        })
        .on_press(move |_| on_select.call(row.clone()))
        .child(tile)
        .child(rect().width(Size::flex(1.)).child(text::line(
            theme,
            text::Style::Body,
            colour(ink),
            title,
        )))
        .child(match trailing {
            Trailing::Count(n) => text::line(
                theme,
                text::Style::Subheadline,
                colour(ink_2),
                n.to_string(),
            )
            .into_element(),
            Trailing::Lock => {
                Glyph::new("lock-simple-fill", t.size_glyph_small, colour(ink_2)).into_element()
            }
        })
        .into_element()
}
