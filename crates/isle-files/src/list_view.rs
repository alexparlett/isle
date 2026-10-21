//! List view: a table of the folder's items with Name, Date Modified, Size and Kind, drawn
//! only where it shows (`p1/surfaces/files.md` § List view; `conventions.md` § Lists and
//! tables).

use std::path::PathBuf;

use freya::prelude::*;
use isle_fs::{Entry, Order, SortBy};
use isle_ui::{AppIcon, Glyph, colour, text, use_theme};

use crate::{
    format, icons,
    tab::Tab,
    view::{self, Line},
};

/// The modifiers held as the pointer pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Held {
    pub cmd: bool,
    pub shift: bool,
    pub option: bool,
}

/// A press on an item, as the view's owner acts on it.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// A click: select it, or with ⌘ or ⇧ add to the selection.
    Click(PathBuf, Held),
    /// A double-click: open it.
    Open(PathBuf, Held),
    /// A click on no item.
    Nothing,
}

const DATE: f32 = 168.;
const SIZE: f32 = 80.;
const KIND: f32 = 140.;

#[derive(Clone, Copy, PartialEq)]
struct RowClick;

#[derive(PartialEq)]
pub struct ListView {
    pub tab: State<Tab>,
    pub rev: u64,
    pub focused: bool,
    pub held: State<Held>,
    pub on_pick: EventHandler<Pick>,
    pub controller: ScrollController,
    pub interact: crate::interact::Interact,
}

/// What a row's builder reads; equal while the tab has not moved.
#[derive(Clone, PartialEq)]
struct Rows {
    rev: u64,
    /// Group headings and items, when the view is grouped; the folder's order otherwise.
    lines: Option<std::rc::Rc<Vec<Line>>>,
    tab: State<Tab>,
    focused: bool,
    held: State<Held>,
    on_pick: EventHandler<Pick>,
    dark: bool,
    interact: crate::interact::Interact,
}

impl Component for ListView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut tab = self.tab;
        let (length, order, group) = {
            let t = tab.peek();
            (t.folder.len(), t.folder.order(), t.shown.group)
        };
        let lines = group.map(|by| {
            let now = jiff::Timestamp::now().as_second();
            std::rc::Rc::new(view::lines(tab.peek().folder.entries(), Some(by), now))
        });
        let length = lines.as_ref().map_or(length, |l| l.len());
        let cells = theme.clone();
        let header_cell = move |title: &'static str, by: SortBy, width: Size, right: bool| {
            let theme = &cells;
            let t = &theme.tokens;
            let sorted = order.by == by;
            let ink = if sorted { t.colour_text } else { t.colour_text_2 };
            rect()
                .width(width)
                .height(Size::fill())
                .horizontal()
                .cross_align(Alignment::Center)
                .main_align(if right { Alignment::End } else { Alignment::Start })
                .spacing(t.space_xs)
                .a11y_role(AccessibilityRole::ColumnHeader)
                .a11y_alt(title)
                .on_press(move |_| {
                    let next = if sorted {
                        Order { reverse: !order.reverse, ..order }
                    } else {
                        Order { by, reverse: false, ..order }
                    };
                    let mut t = tab.write();
                    t.folder.set_order(next);
                    t.shown.sort = next.by;
                    t.shown.reverse = next.reverse;
                    t.rev += 1;
                })
                .child(text::line(theme, text::Style::Callout, colour(ink), title))
                .maybe_child(sorted.then(|| {
                    Glyph::new(
                        if order.reverse { "caret-up" } else { "caret-down" },
                        t.size_glyph_small,
                        colour(t.colour_text),
                    )
                }))
        };
        let header = rect()
            .width(Size::fill())
            .height(Size::px(t.size_control))
            .padding((0., t.space_s + t.space_s))
            .horizontal()
            .content(Content::flex())
            .spacing(t.space_l)
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .child(header_cell("Name", SortBy::Name, Size::flex(1.), false))
            .child(header_cell("Date Modified", SortBy::Modified, Size::px(DATE), false))
            .child(header_cell("Size", SortBy::Size, Size::px(SIZE), true))
            .child(header_cell("Kind", SortBy::Kind, Size::px(KIND), false));
        let rows = Rows {
            rev: self.rev,
            lines,
            tab: self.tab,
            focused: self.focused,
            held: self.held,
            on_pick: self.on_pick.clone(),
            dark: theme.appearance.mode == isle_tokens::Mode::Dark,
            interact: self.interact.clone(),
        };
        let on_pick = self.on_pick.clone();
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_view))
            .a11y_role(AccessibilityRole::Table)
            .child(header)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .on_mouse_down(move |_| on_pick.call(Pick::Nothing))
                    .child(
                        VirtualScrollView::new_with_data_controlled(rows, row, self.controller)
                            .length(length)
                            .item_size(t.size_row)
                            .expanded(),
                    ),
            )
    }
}

/// A group's heading row (`surfaces/files.md` § Icons view, Group heading).
pub fn heading(theme: &isle_tokens::Theme, name: &str) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row))
        .padding((0., t.space_l))
        .main_align(Alignment::Center)
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                .alignment(BorderAlignment::Inner),
        )
        .child(text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), name.to_owned()).font_weight(600))
        .into_element()
}

/// The folder's index of a line, if it is an item, and whether it is an odd one.
fn item_at(rows: &Rows, line: usize) -> Option<(usize, bool)> {
    match &rows.lines {
        None => Some((line, line % 2 == 1)),
        Some(lines) => match lines.get(line)? {
            Line::Item { index, odd } => Some((*index, *odd)),
            Line::Heading(_) => None,
        },
    }
}

fn row(item: VirtualItem, rows: &Rows) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    if let Some(Line::Heading(name)) = rows.lines.as_ref().and_then(|l| l.get(item.index)) {
        return heading(&theme, name);
    }
    let tab = rows.tab.peek();
    let Some((index, odd)) = item_at(rows, item.index) else {
        return rect().into_element();
    };
    let Some(entry) = tab.folder.entries().get(index) else {
        return rect().into_element();
    };
    let selected_line = |line: Option<usize>| {
        line.and_then(|l| item_at(rows, l))
            .and_then(|(i, _)| tab.folder.entries().get(i))
            .is_some_and(|e| tab.is_selected(&e.path))
    };
    let selected = tab.is_selected(&entry.path);
    let joined_above = selected_line(item.index.checked_sub(1));
    let joined_below = selected_line(Some(item.index + 1));
    let fill = if selected {
        Some(if rows.focused { t.colour_accent } else { t.colour_selection_unfocused })
    } else if odd {
        Some(t.colour_fill)
    } else {
        None
    };
    let on_accent = selected && rows.focused;
    let ink = if on_accent { t.colour_on_accent } else { t.colour_text };
    let ink_2 = if on_accent { t.colour_on_accent } else { t.colour_text_2 };
    let dim = entry.hidden() || rows.interact.is_cut(&entry.path);
    let target = rows.interact.is_target(&entry.path);
    let interact = rows.interact.clone();
    let r = t.radius_row;
    // A run of selected rows joins into one shape, rounded only at its ends.
    let (top, bottom) = (
        if selected && joined_above { 0. } else { r },
        if selected && joined_below { 0. } else { r },
    );
    let radius = CornerRadius::new(top, top, bottom, bottom);
    let now = format::now();
    let path = entry.path.clone();
    let (on_pick, held) = (rows.on_pick.clone(), rows.held);
    rect()
        .key(entry.path.to_string_lossy().into_owned())
        .width(Size::fill())
        .height(Size::px(t.size_row))
        .padding((0., t.space_s))
        .child(
            rect()
                .expanded()
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_l)
                .corner_radius(radius)
                .map(fill, |el, fill| el.background(colour(fill)))
                .maybe(target, |el| el.border(crate::interact::drop_ring(&theme)))
                .a11y_role(AccessibilityRole::Row)
                .a11y_alt(entry.name.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_pointer_enter(rows.interact.target_on_enter(&entry.path, entry.is_dir()))
                .on_pointer_leave(rows.interact.target_on_leave(&entry.path))
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    if e.button != Some(MouseButton::Left) {
                        return;
                    }
                    e.stop_propagation();
                    interact.press(&path, e.global_location.x, e.global_location.y);
                    let held = *held.peek();
                    let double = EventsCombos::<RowClick>::pressed(e.global_location).is_double();
                    on_pick.call(if double {
                        Pick::Open(path.clone(), held)
                    } else {
                        Pick::Click(path.clone(), held)
                    });
                })
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .opacity(if dim { 0.5 } else { 1. })
                        .child(icon(entry, rows.dark, t.size_app_icon_small, ink_2))
                        .child(match rows.interact.renaming(&entry.path) {
                            Some(rename) => crate::interact::rename_field(&theme, rename, Size::flex(1.)),
                            None => text::line(&theme, text::Style::Body, colour(ink), entry.name.clone()).into_element(),
                        }),
                )
                .child(
                    rect().width(Size::px(DATE)).child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(ink_2),
                        format::date(entry.modified, &now),
                    )),
                )
                .child(
                    rect()
                        .width(Size::px(SIZE))
                        .main_align(Alignment::End)
                        .horizontal()
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(ink_2),
                            if entry.is_dir() { "--".into() } else { format::size(entry.size) },
                        )),
                )
                .child(rect().width(Size::px(KIND)).child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(ink_2),
                    kind(entry),
                ))),
        )
        .into_element()
}

/// The words Kind shows.
pub fn kind(entry: &Entry) -> String {
    match entry.mime.as_deref() {
        Some(isle_apps::mime::DIRECTORY) => "Folder".into(),
        Some(mime) => isle_apps::mime::types().comment(mime),
        None if matches!(entry.kind, isle_fs::Kind::Link { broken: true, .. }) => "Alias".into(),
        None => "Document".into(),
    }
}

/// An item's icon from the theme's cache, or a glyph until it is found.
pub fn icon(entry: &Entry, dark: bool, size: f32, ink: isle_tokens::Colour) -> Element {
    let found = entry
        .mime
        .as_deref()
        .and_then(|m| icons::cached(m, dark, if size > 32. { icons::LARGE } else { icons::SMALL }));
    match found {
        Some(path) => AppIcon::new(path, size).into_element(),
        None => Glyph::new(if entry.is_dir() { "folder-simple" } else { "file" }, size, colour(ink))
            .into_element(),
    }
}
