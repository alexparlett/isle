//! A table of rows and columns (`p1/surfaces/conventions.md` § Lists and tables): sortable,
//! resizable and reorderable columns, figures in the mono face, selection with ⌘ and ⇧,
//! type-select, disclosure, and an order held while the pointer is over it, a key is fresh
//! or the owner says so. Rows are drawn only where they show. The owner sorts; the table
//! keeps what it shows.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use freya::{animation::*, prelude::*};

use crate::{Glyph, colour, text, use_theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Leading,
    Trailing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub id: &'static str,
    pub title: String,
    /// Points; the first column flexes from this as its least width.
    pub width: f32,
    pub align: Align,
    /// Its cells are figures: mono, tabular, trailing.
    pub figures: bool,
}

impl Column {
    pub fn text(id: &'static str, title: impl Into<String>, width: f32) -> Self {
        Self { id, title: title.into(), width, align: Align::Leading, figures: false }
    }

    pub fn figures(id: &'static str, title: impl Into<String>, width: f32) -> Self {
        Self { id, title: title.into(), width, align: Align::Trailing, figures: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub text: String,
    /// A subtree's total, shown after the row's own figure in the second ink.
    pub total: Option<String>,
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self { text, total: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub key: u64,
    /// Levels under the top, for the hierarchy.
    pub depth: u16,
    /// Open or closed where the row has children; none for a leaf.
    pub disclosure: Option<bool>,
    /// An icon file for the first column, else its room is kept empty.
    pub icon: Option<std::path::PathBuf>,
    /// The first column's text, which type-select matches.
    pub name: String,
    /// A note after the name in the second ink: "Stopped".
    pub note: Option<String>,
    /// The other columns' cells, in the columns' order.
    pub cells: Vec<Cell>,
    /// A section's heading over the rows under it: its name alone, not selectable.
    pub heading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sort {
    pub column: &'static str,
    pub descending: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableEvent {
    /// The selection, as a click or a key left it.
    Selected(Vec<u64>),
    /// A double-click or Return on a row.
    Open(u64),
    /// A secondary click on a row or the empty table, at a point in the window.
    Menu(Option<u64>, (f32, f32)),
    /// A secondary click on the header.
    HeaderMenu((f32, f32)),
    Sort(Sort),
    /// A column's new width.
    Resize(&'static str, f32),
    /// A column dragged to a new place among the columns after the first.
    Move(&'static str, usize),
    Disclose(u64, bool),
}

/// The order to show `next` in while the order is held: `prev`'s, without the rows gone, and
/// each new row after the row it follows in `next`, so a new child lands under its parent.
pub fn held_order(prev: &[u64], next: &[u64]) -> Vec<u64> {
    let present: std::collections::HashSet<u64> = next.iter().copied().collect();
    let mut out: Vec<u64> = prev.iter().copied().filter(|k| present.contains(k)).collect();
    let mut known: std::collections::HashSet<u64> = out.iter().copied().collect();
    for (i, key) in next.iter().enumerate() {
        if known.contains(key) {
            continue;
        }
        let at = match i.checked_sub(1).map(|p| next[p]) {
            Some(before) => out.iter().position(|k| *k == before).map_or(out.len(), |p| p + 1),
            None => 0,
        };
        out.insert(at, *key);
        known.insert(*key);
    }
    out
}

/// A click's selection: alone, toggled with ⌘, or the run from `anchor` with ⇧.
pub fn click(order: &[u64], selected: &[u64], anchor: Option<u64>, key: u64, cmd: bool, shift: bool) -> Vec<u64> {
    if shift && let Some(anchor) = anchor.filter(|a| order.contains(a)) {
        let (a, b) = (position(order, anchor), position(order, key));
        let (from, to) = (a.min(b), a.max(b));
        let mut run: Vec<u64> = order[from..=to].to_vec();
        if cmd {
            let others: Vec<u64> = selected.iter().copied().filter(|k| !run.contains(k)).collect();
            run.extend(others);
        }
        return run;
    }
    if cmd {
        return match selected.contains(&key) {
            true => selected.iter().copied().filter(|k| *k != key).collect(),
            false => selected.iter().copied().chain([key]).collect(),
        };
    }
    vec![key]
}

/// The row a step of `by` from the cursor lands on, kept inside the table; the first or the
/// last when nothing is selected.
pub fn step(order: &[u64], cursor: Option<u64>, by: isize) -> Option<u64> {
    if order.is_empty() {
        return None;
    }
    let Some(at) = cursor.filter(|c| order.contains(c)).map(|c| position(order, c)) else {
        return Some(if by < 0 { order[order.len() - 1] } else { order[0] });
    };
    let to = (at as isize + by).clamp(0, order.len() as isize - 1) as usize;
    Some(order[to])
}

fn position(order: &[u64], key: u64) -> usize {
    order.iter().position(|k| *k == key).unwrap_or(0)
}

/// What has been typed to select by, reset after `timing-type-select-ms` without a key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Typed {
    text: String,
    last: Option<Instant>,
}

impl Typed {
    pub fn push(&mut self, c: &str, now: Instant, reset: Duration) -> String {
        if self.last.is_none_or(|t| now.duration_since(t) > reset) {
            self.text.clear();
        }
        self.text.push_str(&c.to_lowercase());
        self.last = Some(now);
        self.text.clone()
    }

    /// A key came within `reset`, which holds the order.
    pub fn fresh(&self, now: Instant, reset: Duration) -> bool {
        self.last.is_some_and(|t| now.duration_since(t) <= reset)
    }
}

/// The row `typed` selects: the first whose name starts with it, else the first after it,
/// else the last.
pub fn by_name<'a>(names: impl Iterator<Item = &'a str>, typed: &str) -> Option<usize> {
    let mut after = None;
    let mut count = 0;
    for (i, name) in names.enumerate() {
        count = i + 1;
        let name = name.to_lowercase();
        if name.starts_with(typed) {
            return Some(i);
        }
        if after.is_none() && name.as_str() > typed {
            after = Some(i);
        }
    }
    after.or(count.checked_sub(1))
}

#[derive(Clone, Copy, PartialEq)]
struct RowClick;

/// The band over a column's leading edge that drags its width.
const DIVIDER: f32 = 8.;

#[derive(PartialEq)]
pub struct Table {
    pub columns: Rc<Vec<Column>>,
    /// In the owner's order.
    pub rows: Rc<Vec<Row>>,
    pub sort: Sort,
    pub selected: Rc<Vec<u64>>,
    pub focused: bool,
    /// The owner holds the order too: a menu or an alert is open.
    pub hold: bool,
    pub on_event: EventHandler<TableEvent>,
    pub controller: ScrollController,
}

/// What a row's builder reads.
#[derive(Clone, PartialEq)]
struct Rows {
    columns: Rc<Vec<Column>>,
    rows: Rc<Vec<Row>>,
    order: Rc<Vec<u64>>,
    selected: Rc<Vec<u64>>,
    focused: bool,
    on_event: EventHandler<TableEvent>,
    anchor: State<Option<u64>>,
    /// Each moving row's distance from its old place, and how much of it is left, 1 to 0.
    moves: Rc<std::collections::HashMap<u64, f32>>,
    slide: f32,
    contrast: bool,
}

impl Component for Table {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let reset = Duration::from_millis(t.timing_type_select_ms as u64);
        let mut over = use_state(|| false);
        let mut typed = use_state(Typed::default);
        let mut shown = use_state(Vec::<u64>::new);
        let anchor = use_state(|| None::<u64>);
        let next: Vec<u64> = self.rows.iter().map(|r| r.key).collect();
        // A sort the user asked for takes effect at once; only a sample's re-sort waits.
        let mut sorted_by = use_state(|| self.sort);
        let resorted = *sorted_by.peek() != self.sort;
        if resorted {
            sorted_by.set(self.sort);
        }
        let held = !resorted && (self.hold || *over.peek() || typed.peek().fresh(Instant::now(), reset));
        let order = if held { held_order(&shown.peek(), &next) } else { next };
        if *shown.peek() != order {
            shown.set(order.clone());
        }
        // A re-sort slides each row from its old place to its new one; held, or under
        // Reduce Motion, rows take their places at once.
        let reduced = theme.appearance.reduce_motion;
        let mut last = use_state(Vec::<u64>::new);
        let mut moves = use_state(|| Rc::new(std::collections::HashMap::<u64, f32>::new()));
        let mut slide = use_animation(|_| AnimNum::new(1.0, 0.0).time(240).function(Function::Cubic));
        let row_height = t.size_row;
        use_side_effect_with_deps(&order, move |order: &Vec<u64>| {
            let old = last.peek().clone();
            last.set(order.clone());
            if reduced || old.is_empty() {
                return;
            }
            let at: std::collections::HashMap<u64, usize> = old.iter().enumerate().map(|(i, k)| (*k, i)).collect();
            let from: std::collections::HashMap<u64, f32> = order
                .iter()
                .enumerate()
                .filter_map(|(i, k)| Some((*k, (*at.get(k)? as f32 - i as f32) * row_height)))
                .filter(|(_, d)| *d != 0.0)
                .collect();
            if !from.is_empty() {
                moves.set(Rc::new(from));
                slide.start();
            }
        });
        let order = Rc::new(order);

        let on_event = self.on_event.clone();
        let sort = self.sort;
        let mut resizing = use_state(|| None::<(&'static str, f32, f64, f32)>);
        // A title dragged to a new place: its column, where the press began, whether it moved.
        let mut dragging = use_state(|| None::<(&'static str, f64, bool)>);
        let mut areas = use_state(Vec::<(&'static str, f32, f32)>::new);
        let gap = t.space_l;
        let header_cells = self.columns.iter().enumerate().map(|(i, column)| {
            let sorted = column.id == sort.column;
            let ink = if sorted { t.colour_text } else { t.colour_text_2 };
            let width = match *resizing.read() {
                Some((id, w, _, _)) if id == column.id => w,
                _ => column.width,
            };
            let (id, on_sort) = (column.id, on_event.clone());
            let lifted = matches!(*dragging.read(), Some((d, _, true)) if d == id);
            let divider = (i > 0).then(|| {
                rect()
                    .width(Size::px(DIVIDER))
                    .height(Size::fill())
                    .margin((0., -DIVIDER + (gap + DIVIDER) / 2., 0., -(gap + DIVIDER) / 2.))
                    .a11y_role(AccessibilityRole::Splitter)
                    .a11y_alt(format!("{} width", column.title))
                    .on_mouse_down(move |e: Event<MouseEventData>| {
                        if e.button == Some(MouseButton::Left) {
                            e.stop_propagation();
                            resizing.set(Some((id, width, e.global_location.x, width)));
                        }
                    })
                    .into_element()
            });
            let trailing = column.align == Align::Trailing;
            rect()
                .maybe(i == 0, |el| el.width(Size::flex(1.)).min_width(Size::px(column.width)))
                .maybe(i > 0, |el| el.width(Size::px(width)))
                .height(Size::fill())
                .horizontal()
                .content(Content::flex())
                .opacity(if lifted { 0.5 } else { 1. })
                .on_sized(move |e: Event<SizedEventData>| {
                    let (lo, hi) = (e.area.min_x(), e.area.max_x());
                    let mut a = areas.write();
                    match a.iter_mut().find(|(c, _, _)| *c == id) {
                        Some(found) => *found = (id, lo, hi),
                        None => a.push((id, lo, hi)),
                    }
                })
                .maybe(i > 0, |el| {
                    el.on_mouse_down(move |e: Event<MouseEventData>| {
                        if e.button == Some(MouseButton::Left) {
                            dragging.set(Some((id, e.global_location.x, false)));
                        }
                    })
                })
                .maybe_child(divider)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .main_align(if trailing { Alignment::End } else { Alignment::Start })
                        .spacing(t.space_xs)
                        .a11y_role(AccessibilityRole::ColumnHeader)
                        .a11y_alt(column.title.clone())
                        .on_press(move |_| {
                            // A drag that moved is a reorder, not a click.
                            if matches!(*dragging.peek(), Some((_, _, true))) {
                                return;
                            }
                            let descending = if sorted { !sort.descending } else { true };
                            on_sort.call(TableEvent::Sort(Sort { column: id, descending }));
                        })
                        .child(text::line(&theme, text::Style::Callout, colour(ink), column.title.clone()))
                        .maybe_child(sorted.then(|| {
                            Glyph::new(
                                if sort.descending { "caret-down" } else { "caret-up" },
                                t.size_glyph_small,
                                colour(t.colour_text),
                            )
                        })),
                )
                .into_element()
        });
        let header = rect()
            .width(Size::fill())
            .height(Size::px(t.size_control))
            .overflow(Overflow::Clip)
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
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                let held = *dragging.peek();
                if let Some((id, x0, moved)) = held
                    && !moved
                    && (e.global_location().x - x0).abs() > 4.
                {
                    dragging.set(Some((id, x0, true)));
                }
                let Some((id, _, x0, start)) = *resizing.peek() else { return };
                // The leading edge moved right narrows the column.
                let width = (start - (e.global_location().x - x0) as f32).max(48.);
                resizing.set(Some((id, width, x0, start)));
            })
            .on_global_pointer_up({
                let on_event = on_event.clone();
                move |e: Event<PointerEventData>| {
                    let held = *dragging.peek();
                    if let Some((id, _, moved)) = held {
                        dragging.set(None);
                        let x = e.global_location().x as f32;
                        let rest: Vec<(&'static str, f32, f32)> = areas.peek().iter().skip(1).copied().collect();
                        if moved && let Some(to) = rest.iter().position(|(_, lo, hi)| x >= *lo && x < *hi) {
                            on_event.call(TableEvent::Move(id, to));
                        }
                    }
                    let Some((id, width, _, _)) = *resizing.peek() else { return };
                    resizing.set(None);
                    on_event.call(TableEvent::Resize(id, width));
                }
            })
            .on_mouse_down({
                let on_event = on_event.clone();
                move |e: Event<MouseEventData>| {
                    if e.button == Some(MouseButton::Right) {
                        e.stop_propagation();
                        on_event.call(TableEvent::HeaderMenu((e.global_location.x as f32, e.global_location.y as f32)));
                    }
                }
            })
            .children(header_cells);

        let on_key = {
            let (order, rows, selected, on_event, focused) =
                (order.clone(), self.rows.clone(), self.selected.clone(), on_event.clone(), self.focused);
            let (mut anchor, mut controller, row_height) = (anchor, self.controller, t.size_row);
            move |e: Event<KeyboardEventData>| {
                if !focused || e.modifiers.contains(Modifiers::META) || e.modifiers.contains(Modifiers::CONTROL) {
                    return;
                }
                let cursor = selected.last().copied();
                let shift = e.modifiers.contains(Modifiers::SHIFT);
                let to = match &e.key {
                    Key::Named(NamedKey::ArrowUp) => step(&order, cursor, -1),
                    Key::Named(NamedKey::ArrowDown) => step(&order, cursor, 1),
                    Key::Named(NamedKey::Home) => order.first().copied(),
                    Key::Named(NamedKey::End) => order.last().copied(),
                    Key::Named(NamedKey::Enter) => {
                        if let Some(key) = cursor {
                            on_event.call(TableEvent::Open(key));
                        }
                        return;
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowLeft) => {
                        let open = matches!(e.key, Key::Named(NamedKey::ArrowRight));
                        if let Some(row) = cursor.and_then(|c| rows.iter().find(|r| r.key == c))
                            && row.disclosure.is_some_and(|o| o != open)
                        {
                            on_event.call(TableEvent::Disclose(row.key, open));
                        }
                        return;
                    }
                    Key::Character(c) if !c.trim().is_empty() => {
                        let typed = typed.write().push(c, Instant::now(), reset);
                        let names = order.iter().filter_map(|k| rows.iter().find(|r| r.key == *k)).map(|r| r.name.as_str());
                        by_name(names, &typed).map(|i| order[i])
                    }
                    _ => return,
                };
                let Some(to) = to else { return };
                e.stop_propagation();
                let next = if shift { click(&order, &selected, *anchor.peek(), to, false, true) } else { vec![to] };
                if !shift {
                    anchor.set(Some(to));
                }
                let line = position(&order, to);
                controller.scroll_to_offset(line as f32 * row_height, row_height, Direction::Vertical);
                on_event.call(TableEvent::Selected(next));
            }
        };

        let rows = Rows {
            columns: self.columns.clone(),
            rows: self.rows.clone(),
            order: order.clone(),
            selected: self.selected.clone(),
            focused: self.focused,
            on_event: on_event.clone(),
            anchor,
            moves: moves.read().clone(),
            slide: if reduced { 0.0 } else { slide.get().value() },
            contrast: theme.appearance.increase_contrast,
        };
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_view))
            .a11y_role(AccessibilityRole::Table)
            .on_pointer_enter(move |_| over.set(true))
            .on_pointer_leave(move |_| over.set(false))
            .on_global_key_down(on_key)
            .child(header)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .on_mouse_down({
                        let on_event = on_event.clone();
                        move |e: Event<MouseEventData>| {
                            if e.button == Some(MouseButton::Right) {
                                on_event.call(TableEvent::Menu(None, (e.global_location.x as f32, e.global_location.y as f32)));
                            } else if e.button == Some(MouseButton::Left) {
                                on_event.call(TableEvent::Selected(Vec::new()));
                            }
                        }
                    })
                    .child(
                        VirtualScrollView::new_with_data_controlled(rows, row, self.controller)
                            .length(order.len())
                            .item_size(t.size_row)
                            .expanded(),
                    ),
            )
    }
}

fn row(item: VirtualItem, rows: &Rows) -> Element {
    let theme = crate::theme();
    let t = &theme.tokens;
    let Some(key) = rows.order.get(item.index).copied() else {
        return rect().into_element();
    };
    let Some(row) = rows.rows.iter().find(|r| r.key == key) else {
        return rect().into_element();
    };
    if row.heading {
        return rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_s + t.space_s))
            .main_align(Alignment::End)
            .a11y_role(AccessibilityRole::RowHeader)
            .a11y_alt(row.name.clone())
            .child(text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), row.name.clone()).font_weight(600))
            .into_element();
    }
    let is_selected = |line: Option<usize>| line.and_then(|l| rows.order.get(l)).is_some_and(|k| rows.selected.contains(k));
    let selected = rows.selected.contains(&key);
    let (above, below) = (is_selected(item.index.checked_sub(1)), is_selected(Some(item.index + 1)));
    let fill = if selected {
        Some(if rows.focused { t.colour_accent } else { t.colour_selection_unfocused })
    } else if item.index % 2 == 1 {
        Some(t.colour_fill)
    } else {
        None
    };
    let on_accent = selected && rows.focused;
    let ink = if on_accent { t.colour_on_accent } else { t.colour_text };
    let ink_2 = if on_accent { t.colour_on_accent } else { t.colour_text_2 };
    let r = t.radius_row;
    // A run of selected rows joins into one shape, rounded only at its ends.
    let (top, bottom) = (if selected && above { 0. } else { r }, if selected && below { 0. } else { r });
    let (order, selected_now, on_event, mut anchor) = (rows.order.clone(), rows.selected.clone(), rows.on_event.clone(), rows.anchor);
    let disclose = row.disclosure.map(|open| {
        let on_event = rows.on_event.clone();
        rect()
            .width(Size::px(16.))
            .height(Size::px(16.))
            .center()
            .on_mouse_down(move |e: Event<MouseEventData>| {
                if e.button == Some(MouseButton::Left) {
                    e.stop_propagation();
                    on_event.call(TableEvent::Disclose(key, !open));
                }
            })
            .child(Glyph::new(if open { "caret-down" } else { "caret-right" }, t.size_glyph_small, colour(ink_2)))
            .into_element()
    });
    let any_disclosure = rows.rows.iter().any(|r| r.disclosure.is_some() || r.depth > 0);
    let first = rows.columns.first().cloned();
    let name_cell = rect()
        .width(Size::flex(1.))
        .min_width(Size::px(first.as_ref().map_or(240., |c| c.width)))
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .padding((0., 0., 0., row.depth as f32 * t.space_l))
        .maybe_child(any_disclosure.then(|| disclose.unwrap_or_else(|| rect().width(Size::px(16.)).into_element())))
        .child(match &row.icon {
            Some(path) => crate::AppIcon::new(path.clone(), t.size_app_icon_small).into_element(),
            None => rect().width(Size::px(t.size_app_icon_small)).into_element(),
        })
        .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Body, colour(ink), row.name.clone())))
        .maybe_child(row.note.clone().map(|n| text::line(&theme, text::Style::Callout, colour(ink_2), n)));
    let cells = rows.columns.iter().skip(1).zip(row.cells.iter()).map(|(column, cell)| {
        let trailing = column.align == Align::Trailing;
        let figure = |s: String, c| {
            if column.figures {
                label().font_family("JetBrains Mono").font_size(t.type_style_body_size).color(colour(c)).max_lines(1).text(s).into_element()
            } else {
                text::line(&theme, text::Style::Body, colour(c), s).into_element()
            }
        };
        rect()
            .width(Size::px(column.width))
            .height(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .main_align(if trailing { Alignment::End } else { Alignment::Start })
            // Increase Contrast draws the columns' dividers through the table.
            .maybe(rows.contrast, |el| {
                el.border(
                    Border::new()
                        .fill(colour(t.colour_hairline_strong))
                        .width(BorderWidth { top: 0., right: 0., bottom: 0., left: 1. })
                        .alignment(BorderAlignment::Outer),
                )
            })
            .spacing(t.space_s)
            .overflow(Overflow::Clip)
            .child(figure(cell.text.clone(), ink))
            .maybe_child(cell.total.clone().map(|total| figure(total, ink_2)))
            .into_element()
    });
    let shift = rows.moves.get(&key).copied().unwrap_or(0.0) * rows.slide;
    // Unkeyed: Freya loses a component's scope inside a keyed element that moves far.
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row))
        .padding((0., t.space_s))
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .margin((shift, 0., -shift, 0.))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_l)
                .corner_radius(CornerRadius::new(top, top, bottom, bottom))
                .map(fill, |el, fill| el.background(colour(fill)))
                .a11y_role(AccessibilityRole::Row)
                .a11y_alt(row.name.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    let at = (e.global_location.x as f32, e.global_location.y as f32);
                    let secondary = e.button == Some(MouseButton::Right)
                        || (e.button == Some(MouseButton::Left) && e.modifiers.contains(Modifiers::CONTROL));
                    if secondary {
                        e.stop_propagation();
                        if !selected_now.contains(&key) {
                            on_event.call(TableEvent::Selected(vec![key]));
                        }
                        on_event.call(TableEvent::Menu(Some(key), at));
                        return;
                    }
                    if e.button != Some(MouseButton::Left) {
                        return;
                    }
                    e.stop_propagation();
                    if EventsCombos::<RowClick>::pressed(e.global_location).is_double() {
                        on_event.call(TableEvent::Open(key));
                        return;
                    }
                    let (cmd, shift) = (e.modifiers.contains(Modifiers::META), e.modifiers.contains(Modifiers::SHIFT));
                    let next = click(&order, &selected_now, *anchor.peek(), key, cmd, shift);
                    if !shift {
                        anchor.set(Some(key));
                    }
                    on_event.call(TableEvent::Selected(next));
                })
                .child(name_cell)
                .children(cells),
        )
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_order_keeps_places_drops_the_gone_and_puts_the_new_after_their_neighbour() {
        assert_eq!(held_order(&[1, 2, 3, 4], &[4, 3, 2, 1]), [1, 2, 3, 4], "a re-sort waits");
        assert_eq!(held_order(&[1, 2, 3], &[3, 1]), [1, 3], "a row that went leaves");
        // 5 is 2's new child: it follows 2 in the owner's order, so it lands under 2.
        assert_eq!(held_order(&[1, 2, 3], &[3, 2, 5, 1]), [1, 2, 5, 3]);
        assert_eq!(held_order(&[1, 2], &[9, 2, 1]), [9, 1, 2], "first in the owner's order, first here");
        assert_eq!(held_order(&[], &[2, 1]), [2, 1]);
    }

    #[test]
    fn clicks_select_alone_toggle_with_cmd_and_run_with_shift() {
        let order = [10, 20, 30, 40, 50];
        assert_eq!(click(&order, &[10], Some(10), 30, false, false), [30]);
        assert_eq!(click(&order, &[10], Some(10), 30, true, false), [10, 30]);
        assert_eq!(click(&order, &[10, 30], Some(10), 30, true, false), [10]);
        assert_eq!(click(&order, &[20], Some(20), 40, false, true), [20, 30, 40]);
        assert_eq!(click(&order, &[40], Some(40), 20, false, true), [20, 30, 40], "upward too");
        assert_eq!(click(&order, &[50], Some(20), 30, true, true), [20, 30, 50], "⌘⇧ adds the run");
    }

    #[test]
    fn keys_step_inside_the_table() {
        let order = [1, 2, 3];
        assert_eq!(step(&order, None, 1), Some(1));
        assert_eq!(step(&order, None, -1), Some(3));
        assert_eq!(step(&order, Some(2), 1), Some(3));
        assert_eq!(step(&order, Some(3), 1), Some(3), "no wrap");
        assert_eq!(step(&[], Some(3), 1), None);
    }

    #[test]
    fn typing_selects_by_name_and_resets_after_a_pause() {
        let names = ["cargo", "firefox", "isle-compositor", "steam"];
        assert_eq!(by_name(names.iter().copied(), "is"), Some(2));
        assert_eq!(by_name(names.iter().copied(), "g"), Some(2), "the first after it");
        assert_eq!(by_name(names.iter().copied(), "z"), Some(3), "else the last");
        let mut typed = Typed::default();
        let t0 = Instant::now();
        let second = Duration::from_secs(1);
        assert_eq!(typed.push("S", t0, second), "s");
        assert_eq!(typed.push("t", t0 + Duration::from_millis(300), second), "st");
        assert!(typed.fresh(t0 + Duration::from_millis(900), second));
        assert_eq!(typed.push("f", t0 + Duration::from_secs(3), second), "f");
    }
}
