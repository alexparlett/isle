//! The basket as the inspector (`p1/surfaces/cleaner.md` The basket): its items and what they
//! free, moving, and moved with Undo, Empty Trash… and any failures.

use std::path::PathBuf;

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, colour, scroll_area, text};

use crate::{
    basket::{Basket, Item},
    model::{disk_name, size},
    ui::{kind_tile, mono, progress_bar},
};

/// A move that ended: how many went, what failed and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub count: usize,
    pub failed: Vec<(Item, String)>,
    /// Its steps back, where it can be undone.
    pub undo: Option<Vec<isle_fs::jobs::Step>>,
    /// The items it moved, which Undo puts back in the basket.
    pub items: Vec<Item>,
}

/// What the basket frees on each disk, and whether any of it is shared with other files.
pub fn frees(items: &[Item]) -> (Vec<(String, u64)>, bool) {
    let mounts = isle_fs::mounts::read();
    let mut out: Vec<(String, u64)> = Vec::new();
    let mut shared = false;
    for i in items {
        let disk = isle_fs::mounts::of(&mounts, &i.path)
            .map_or_else(|| "this disk".to_owned(), |m| disk_name(&m.point));
        let (bytes, some_shared) = crate::mover::frees(i);
        shared |= some_shared;
        match out.iter_mut().find(|(d, _)| *d == disk) {
            Some((_, b)) => *b += bytes,
            None => out.push((disk, bytes)),
        }
    }
    (out, shared)
}

pub struct Panel<'a> {
    pub basket: &'a Basket,
    pub frees: &'a (Vec<(String, u64)>, bool),
    pub moving: Option<f64>,
    pub moved: Option<&'a Moved>,
    pub on_remove: EventHandler<PathBuf>,
    pub on_move: EventHandler<()>,
    pub on_undo: EventHandler<()>,
    pub on_empty: EventHandler<()>,
}

fn item_row(
    theme: &Theme,
    i: &Item,
    reason: Option<&str>,
    on_remove: Option<EventHandler<PathBuf>>,
) -> Element {
    let t = &theme.tokens;
    let path = i.path.clone();
    rect()
        .width(Size::fill())
        .height(Size::px(if reason.is_some() {
            t.size_row_large
        } else {
            t.size_row
        }))
        .padding((0., t.space_s))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .child(kind_tile(theme, i.kind))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    text::cut_middle(&i.name, 26),
                ))
                .maybe_child(reason.map(|r| {
                    text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        r.to_owned(),
                    )
                    .into_element()
                })),
        )
        .child(mono(theme, t.colour_text_2, size(i.bytes)))
        .maybe_child(on_remove.map(|on| {
            rect()
                .width(Size::px(20.))
                .height(Size::px(20.))
                .center()
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(format!("Remove {} from basket", i.name))
                .on_press(move |_| on.call(path.clone()))
                .child(Glyph::new("x-circle-fill", 16., colour(t.colour_text_2)))
                .into_element()
        }))
        .into_element()
}

pub fn view(theme: &Theme, p: Panel) -> Element {
    let t = &theme.tokens;
    let hairline = || {
        rect()
            .width(Size::fill())
            .height(Size::px(t.stroke_hairline))
            .background(colour(t.colour_hairline))
            .into_element()
    };
    let (head, items, foot): (Element, Element, Element) = match p.moved {
        Some(m) if p.basket.is_empty() => {
            let undo = p.on_undo.clone();
            let failed: Vec<Element> = m
                .failed
                .iter()
                .map(|(i, why)| item_row(theme, i, Some(why), None))
                .collect();
            let head = rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_s)
                .child(
                    text::line(
                        theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        format!("Moved {} items to the Trash", m.count),
                    )
                    .max_lines(2)
                    .width(Size::fill()),
                )
                .maybe_child(m.undo.is_some().then(|| {
                    Button::new("Undo")
                        .on_press(move |_| undo.call(()))
                        .into_element()
                }))
                .maybe_child((!failed.is_empty()).then(|| {
                    rect()
                        .width(Size::fill())
                        .padding((t.space_m, 0., 0., 0.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_xs)
                        .child(Glyph::new(
                            "warning-circle-fill",
                            12.,
                            colour(t.colour_danger_text),
                        ))
                        .child(text::line(
                            theme,
                            text::Style::Callout,
                            colour(t.colour_text),
                            format!("Couldn't move {} items", failed.len()),
                        ))
                        .into_element()
                }))
                .into_element();
            let items = rect()
                .width(Size::fill())
                .padding((0., t.space_s))
                .children(failed)
                .into_element();
            let empty = p.on_empty.clone();
            let foot = rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_m)
                .child(
                    text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        "Nothing is freed until the Trash is emptied",
                    )
                    .max_lines(3)
                    .width(Size::fill()),
                )
                .child(
                    Button::new("Empty Trash…")
                        .wide()
                        .on_press(move |_| empty.call(())),
                )
                .into_element();
            (head, items, foot)
        }
        _ => {
            let head = rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_xs)
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    "Basket",
                ))
                .child(text::line(
                    theme,
                    text::Style::Title2,
                    colour(t.colour_text),
                    size(p.basket.total()),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    format!("{} items", p.basket.items().len()),
                ))
                .into_element();
            let rows: Vec<Element> = p
                .basket
                .items()
                .iter()
                .map(|i| {
                    item_row(
                        theme,
                        i,
                        None,
                        (p.moving.is_none()).then(|| p.on_remove.clone()),
                    )
                })
                .collect();
            let items = rect()
                .width(Size::fill())
                .padding((0., t.space_s))
                .children(rows)
                .into_element();
            let (by_disk, shared) = p.frees;
            let mut lines: Vec<Element> = by_disk
                .iter()
                .map(|(disk, bytes)| {
                    text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        format!("Frees {} on {disk} once the Trash is emptied", size(*bytes)),
                    )
                    .max_lines(3)
                    .width(Size::fill())
                    .into_element()
                })
                .collect();
            if *shared {
                lines.push(
                    text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        "Some of this is shared with other files.",
                    )
                    .max_lines(2)
                    .width(Size::fill())
                    .into_element(),
                );
            }
            let on_move = p.on_move.clone();
            let action = match p.moving {
                Some(share) => progress_bar(theme, share as f32, Size::fill(), t.colour_accent),
                None => Button::new("Move to Trash")
                    .wide()
                    .on_press(move |_| on_move.call(()))
                    .into_element(),
            };
            let foot = rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_m)
                .children(lines)
                .child(action)
                .into_element();
            (head, items, foot)
        }
    };
    rect()
        .width(Size::px(260.))
        .height(Size::fill())
        .content(Content::flex())
        .background(colour(t.colour_window))
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: 0.,
                    left: t.stroke_hairline,
                })
                .alignment(BorderAlignment::Inner),
        )
        .child(head)
        .child(
            scroll_area(theme)
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(items),
        )
        .child(hairline())
        .child(foot)
        .into_element()
}
