//! The list: a locked collection's row at its head, then two-line rows in a virtual list
//! (`p1/surfaces/keychain.md` The list). No row holds a secret.

use std::rc::Rc;

use freya::prelude::*;
use isle_secrets::kinds::Kind;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Button, Glyph, colour, text};

use crate::model::{Detail, Item, ItemId, Row};

/// What the list asks of the window.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Select(ItemId),
    Menu(ItemId, (f32, f32)),
    Unlock(String),
    NewKey,
}

/// The glyph a kind's tile carries.
pub fn kind_glyph(kind: Kind) -> &'static str {
    match kind {
        Kind::Login => "globe",
        Kind::WiFi => "wifi-high",
        Kind::SshKey => "terminal-window",
        Kind::App => "app-window",
    }
}

/// An item's tile at `size`: a login's monogram, an app's icon, or its kind's glyph.
pub fn item_tile(theme: &Theme, item: &Item, size: f32, radius: f32) -> Element {
    let t = &theme.tokens;
    if let Detail::Secret {
        icon: Some(icon), ..
    } = &item.detail
        && item.kind == Kind::App
    {
        return AppIcon::new(icon.clone(), size).into_element();
    }
    let well = rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(radius)
        .background(colour(t.colour_fill_strong))
        .maybe(theme.appearance.increase_contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .center();
    if item.kind == Kind::Login {
        let letter = item
            .title
            .chars()
            .find(|c| c.is_alphanumeric())
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "?".into());
        let style = if size > 40. {
            text::Style::Title
        } else {
            text::Style::Title3
        };
        return well
            .child(text::line(theme, style, colour(t.colour_text), letter))
            .into_element();
    }
    well.child(Glyph::new(
        kind_glyph(item.kind),
        size * 0.625,
        colour(t.colour_text_2),
    ))
    .into_element()
}

#[derive(Clone)]
pub struct List {
    pub items: Rc<Vec<Item>>,
    pub selected: Option<ItemId>,
    /// Locked collections the list spans, by path and label.
    pub locked: Vec<(String, String)>,
    /// The keyring's missing or failed line, when the list spans it.
    pub keyring_note: Option<String>,
    pub row: Row,
    pub spans_sources: bool,
    pub query: String,
    pub active: bool,
    pub loaded: bool,
    pub on_act: EventHandler<Act>,
}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
            && self.selected == other.selected
            && self.locked == other.locked
            && self.keyring_note == other.keyring_note
            && self.row == other.row
            && self.query == other.query
            && self.active == other.active
            && self.loaded == other.loaded
    }
}

struct Rows {
    items: Rc<Vec<Item>>,
    selected: Option<ItemId>,
    spans_sources: bool,
    active: bool,
    on_act: EventHandler<Act>,
}

impl PartialEq for Rows {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
            && self.selected == other.selected
            && self.active == other.active
    }
}

fn row(item: VirtualItem, rows: &Rows) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let Some(it) = rows.items.get(item.index) else {
        return rect().into_element();
    };
    let selected = rows.selected.as_ref() == Some(&it.id);
    let (fill, ink, ink_2) = match (selected, rows.active) {
        (true, true) => (
            Some(t.colour_accent),
            t.colour_on_accent,
            t.colour_on_accent,
        ),
        (true, false) => (Some(t.colour_fill_strong), t.colour_text, t.colour_text_2),
        _ => (None, t.colour_text, t.colour_text_2),
    };
    let second = match (it.second.is_empty(), rows.spans_sources) {
        (true, true) => it.source.clone(),
        (false, true) => format!("{} · {}", it.second, it.source),
        _ => it.second.clone(),
    };
    let (id, menu_id) = (it.id.clone(), it.id.clone());
    let (on_press, on_menu) = (rows.on_act.clone(), rows.on_act.clone());
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., t.space_s))
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .corner_radius(t.radius_row)
                .map(fill, |el, f| el.background(colour(f)))
                .maybe(theme.appearance.increase_contrast, |el| {
                    el.border(
                        Border::new()
                            .fill(colour(t.colour_hairline_strong))
                            .width(t.stroke_hairline)
                            .alignment(BorderAlignment::Inner),
                    )
                })
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(it.title.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    if e.button == Some(MouseButton::Right) {
                        e.stop_propagation();
                        on_menu.call(Act::Menu(
                            menu_id.clone(),
                            (e.global_location.x as f32, e.global_location.y as f32),
                        ));
                    }
                })
                .on_press(move |_| on_press.call(Act::Select(id.clone())))
                .child(item_tile(&theme, it, t.size_pane_tile_large, 8.))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(ink),
                            it.title.clone(),
                        ))
                        .maybe_child((!second.is_empty()).then(|| {
                            text::line(&theme, text::Style::Subheadline, colour(ink_2), second)
                                .into_element()
                        })),
                ),
        )
        .into_element()
}

/// A line in the list's place: empty, no results, or the keyring missing.
fn line(theme: &Theme, words: String) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding(t.space_xl)
        .center()
        .child(
            label()
                .text(words)
                .font_size(t.type_style_body_size)
                .color(colour(t.colour_text_2))
                .text_align(TextAlign::Center),
        )
        .into_element()
}

/// A row heading the list: a locked collection with Unlock…, or the keyring missing.
fn heading_row(theme: &Theme, words: String, unlock: Option<EventHandler<()>>) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding((0., t.space_s))
        .child(
            rect()
                .width(Size::fill())
                .min_height(Size::px(t.size_row_large))
                .padding((t.space_xs, t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(
                    rect()
                        .width(Size::px(t.size_pane_tile_large))
                        .height(Size::px(t.size_pane_tile_large))
                        .corner_radius(8.)
                        .background(colour(t.colour_fill_strong))
                        .center()
                        .child(Glyph::new("lock", 20., colour(t.colour_text_2))),
                )
                .child(
                    rect().width(Size::flex(1.)).child(
                        label()
                            .text(words)
                            .font_size(t.type_style_body_size)
                            .color(colour(t.colour_text)),
                    ),
                )
                .maybe_child(unlock.map(|on| Button::new("Unlock…").on_press(on).into_element())),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(t.stroke_hairline))
                .background(colour(t.colour_hairline)),
        )
        .into_element()
}

impl Component for List {
    fn render(&self) -> impl IntoElement {
        let theme = isle_ui::use_theme();
        let t = &theme.tokens;
        let mut heads: Vec<Element> = Vec::new();
        for (path, label) in &self.locked {
            let on_act = self.on_act.clone();
            let path = path.clone();
            heads.push(heading_row(
                &theme,
                format!("{label} is locked"),
                Some(EventHandler::new(move |_| {
                    on_act.call(Act::Unlock(path.clone()))
                })),
            ));
        }
        let only_keyring = matches!(self.row, Row::Logins | Row::Apps | Row::Source(_));
        if let Some(note) = &self.keyring_note {
            if only_keyring {
                return rect()
                    .expanded()
                    .child(line(&theme, note.clone()))
                    .into_element();
            }
            heads.push(heading_row(&theme, note.clone(), None));
        }
        let body = if self.items.is_empty() && self.loaded {
            let words = if self.query.trim().is_empty() {
                self.row.empty_line().to_owned()
            } else {
                format!("No results for “{}”", self.query.trim())
            };
            let new_key = (self.row == Row::SshKeys && self.query.trim().is_empty()).then(|| {
                let on_act = self.on_act.clone();
                Button::new("New SSH key…")
                    .on_press(move |_| on_act.call(Act::NewKey))
                    .into_element()
            });
            rect()
                .width(Size::fill())
                .height(Size::flex(1.))
                .center()
                .spacing(t.space_m)
                .child(line(&theme, words))
                .maybe_child(new_key)
                .into_element()
        } else {
            let rows = Rows {
                items: self.items.clone(),
                selected: self.selected.clone(),
                spans_sources: self.spans_sources,
                active: self.active,
                on_act: self.on_act.clone(),
            };
            let count = self.items.len();
            rect()
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(
                    VirtualScrollView::new_with_data(rows, row)
                        .length(count)
                        .item_size(t.size_row_large)
                        .expanded(),
                )
                .into_element()
        };
        rect()
            .expanded()
            .content(Content::flex())
            .padding((t.space_s, 0., 0., 0.))
            .a11y_role(AccessibilityRole::List)
            .children(heads)
            .child(body)
            .into_element()
    }
}
