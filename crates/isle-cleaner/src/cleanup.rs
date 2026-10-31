//! Cleanup (`p1/surfaces/cleaner.md` Cleanup): Safe to clear, ticked; Worth a look, ticked one
//! at a time; and the System card, cleared by its own tools.

use freya::prelude::*;
use isle_scan::{
    Find,
    system::{Item as SystemItem, Share},
};
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, button::Kind as ButtonKind, colour, text};

use crate::{
    basket::Basket,
    model::size,
    ui::{self, kind_tile, mono},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    On,
    Off,
    Mixed,
}

/// A checkbox the row's height, its state drawn as `conventions.md`'s.
pub fn checkbox(theme: &Theme, tick: Tick, alt: String, on_toggle: EventHandler<()>) -> Element {
    let t = &theme.tokens;
    let on = tick != Tick::Off;
    rect()
        .height(Size::px(t.size_row))
        .center()
        .a11y_role(AccessibilityRole::CheckBox)
        .a11y_alt(alt)
        .a11y_builder(move |node| {
            node.set_toggled(match tick {
                Tick::On => accesskit::Toggled::True,
                Tick::Off => accesskit::Toggled::False,
                Tick::Mixed => accesskit::Toggled::Mixed,
            })
        })
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            on_toggle.call(());
        })
        .child(
            rect()
                .width(Size::px(14.))
                .height(Size::px(14.))
                .corner_radius(4.)
                .background(colour(if on {
                    t.colour_accent
                } else {
                    t.colour_control
                }))
                .border(
                    Border::new()
                        .fill(colour(t.colour_hairline_strong))
                        .width(t.stroke_hairline)
                        .alignment(BorderAlignment::Inner),
                )
                .center()
                .maybe_child(match tick {
                    Tick::On => {
                        Some(Glyph::new("check", 11., colour(t.colour_on_accent)).into_element())
                    }
                    Tick::Mixed => Some(
                        rect()
                            .width(Size::px(8.))
                            .height(Size::px(2.))
                            .background(colour(t.colour_on_accent))
                            .into_element(),
                    ),
                    Tick::Off => None,
                }),
        )
        .into_element()
}

pub struct Cleanup<'a> {
    pub finds: &'a [Find],
    pub system: &'a [Share],
    pub basket: &'a Basket,
    /// Which cards show every row: Safe to clear, Worth a look.
    pub open: (bool, bool),
    pub on_tick: EventHandler<(Find, bool)>,
    pub on_tick_all: EventHandler<bool>,
    pub on_more: EventHandler<(bool, bool)>,
    pub on_clear: EventHandler<SystemItem>,
}

fn row(theme: &Theme, find: &Find, ticked: bool, on_tick: EventHandler<(Find, bool)>) -> Element {
    let t = &theme.tokens;
    let second = if find.safe {
        ui::cut_path(&find.path, 72)
    } else {
        format!("{} · {}", find.why, ui::cut_path(&find.path, 48))
    };
    let (f, toggle) = (find.clone(), on_tick.clone());
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., t.space_m))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .on_press(move |_| toggle.call((f.clone(), !ticked)))
        .child(checkbox(
            theme,
            if ticked { Tick::On } else { Tick::Off },
            find.name.clone(),
            {
                let f = find.clone();
                EventHandler::new(move |_| on_tick.call((f.clone(), !ticked)))
            },
        ))
        .child(kind_tile(theme, find.kind))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    find.name.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    second,
                )),
        )
        .child(mono(theme, t.colour_text, size(find.bytes)))
        .into_element()
}

fn card(theme: &Theme, head: Element, rows: Vec<Element>) -> Element {
    let t = &theme.tokens;
    let mut children = vec![head];
    for r in rows {
        children.push(
            rect()
                .width(Size::fill())
                .padding((0., 0., 0., t.space_m))
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.stroke_hairline))
                        .background(colour(t.colour_hairline)),
                )
                .into_element(),
        );
        children.push(r);
    }
    rect()
        .width(Size::fill())
        .corner_radius(t.radius_card)
        .background(colour(t.colour_window_raised))
        .border(
            Border::new()
                .fill(colour(if theme.appearance.increase_contrast {
                    t.colour_hairline_strong
                } else {
                    t.colour_hairline
                }))
                .width(t.stroke_hairline)
                .alignment(BorderAlignment::Inner),
        )
        .overflow(Overflow::Clip)
        .children(children)
        .into_element()
}

fn head(
    theme: &Theme,
    leading: Option<Element>,
    name: &str,
    under: Option<&str>,
    total: Option<u64>,
) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .min_height(Size::px(44.))
        .padding(t.space_m)
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .maybe_child(leading)
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    name.to_owned(),
                ))
                .maybe_child(under.map(|u| {
                    text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        u.to_owned(),
                    )
                    .into_element()
                })),
        )
        .maybe_child(total.map(|b| mono(theme, t.colour_text_2, size(b))))
        .into_element()
}

fn more_row(theme: &Theme, label: String, on: EventHandler<()>) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row))
        .padding((0., t.space_m))
        .cross_align(Alignment::Center)
        .main_align(Alignment::Center)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(label.clone())
        .on_press(move |_| on.call(()))
        .child(text::line(
            theme,
            text::Style::Body,
            colour(t.colour_accent_text),
            label,
        ))
        .into_element()
}

pub fn view(theme: &Theme, c: Cleanup) -> Element {
    let t = &theme.tokens;
    let limit = t.limit_cleanup_rows as usize;
    let mut cards: Vec<Element> = Vec::new();
    for safe in [true, false] {
        let mut finds: Vec<&Find> = c.finds.iter().filter(|f| f.safe == safe).collect();
        finds.sort_by_key(|f| std::cmp::Reverse(f.bytes));
        let total: u64 = finds.iter().map(|f| f.bytes).sum();
        let open = if safe { c.open.0 } else { c.open.1 };
        let ticked = finds.iter().filter(|f| c.basket.contains(&f.path)).count();
        let leading = (safe && !finds.is_empty()).then(|| {
            let tick = match ticked {
                0 => Tick::Off,
                n if n == finds.len() => Tick::On,
                _ => Tick::Mixed,
            };
            let all = c.on_tick_all.clone();
            checkbox(
                theme,
                tick,
                "Safe to clear".into(),
                EventHandler::new(move |_| all.call(tick != Tick::On)),
            )
        });
        let name = if safe {
            "Safe to clear"
        } else {
            "Worth a look"
        };
        let mut rows: Vec<Element> = finds
            .iter()
            .take(if open { usize::MAX } else { limit })
            .map(|f| row(theme, f, c.basket.contains(&f.path), c.on_tick.clone()))
            .collect();
        if finds.is_empty() {
            rows.push(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row_large))
                    .padding((0., t.space_m))
                    .cross_align(Alignment::Center)
                    .main_align(Alignment::Center)
                    .child(text::line(
                        theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        if safe {
                            "Nothing safe to clear"
                        } else {
                            "Nothing to look at"
                        },
                    ))
                    .into_element(),
            );
        } else if finds.len() > limit {
            let on = c.on_more.clone();
            let (a, b) = c.open;
            let flipped = if safe { (!a, b) } else { (a, !b) };
            let label = if open {
                "Show less".to_owned()
            } else {
                format!("{} more", finds.len() - limit)
            };
            rows.push(more_row(
                theme,
                label,
                EventHandler::new(move |_| on.call(flipped)),
            ));
        }
        cards.push(card(
            theme,
            head(theme, leading, name, None, Some(total)),
            rows,
        ));
    }
    let system_rows: Vec<Element> = c
        .system
        .iter()
        .map(|s| {
            let (item, clear) = (s.item, c.on_clear.clone());
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row_large))
                .padding((0., t.space_m))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(text::line(
                            theme,
                            text::Style::Body,
                            colour(t.colour_text),
                            s.item.name(),
                        ))
                        .child(text::line(
                            theme,
                            text::Style::Subheadline,
                            colour(t.colour_text_2),
                            text::cut_middle(&s.note, 80),
                        )),
                )
                .child(mono(theme, t.colour_text_2, size(s.bytes)))
                .child(
                    Button::new(s.item.action())
                        .kind(ButtonKind::Destructive)
                        .enabled(s.bytes > 0)
                        .on_press(move |_| clear.call(item)),
                )
                .into_element()
        })
        .collect();
    let system_total = c.system.iter().map(|s| s.bytes).sum();
    cards.push(card(
        theme,
        head(
            theme,
            None,
            "System",
            Some("These can't be put back."),
            Some(system_total),
        ),
        system_rows,
    ));
    rect()
        .width(Size::fill())
        .spacing(t.space_xl)
        .children(cards)
        .into_element()
}
