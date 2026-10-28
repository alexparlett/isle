//! List view: a table of the folder's items with Name, Date Modified, Size and Kind, drawn
//! only where it shows (`p1/surfaces/files.md` § List view; `conventions.md` § Lists and
//! tables).

use std::path::PathBuf;

use freya::prelude::*;
use isle_fs::{Entry, SortBy};
use isle_ui::{AppIcon, Glyph, colour, text, use_theme};

use crate::{
    format, icons,
    menus::Column,
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

impl Held {
    /// As ⌘ held: a folder opens in a new tab.
    pub const NEW_TAB: Held = Held { cmd: true, shift: false, option: false };

    pub fn of(m: Modifiers) -> Self {
        Self { cmd: m.contains(Modifiers::META), shift: m.contains(Modifiers::SHIFT), option: m.contains(Modifiers::ALT) }
    }
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
    /// A secondary click or ⌃-click, on an item or the background, at a point in the window.
    Menu(Option<PathBuf>, (f32, f32)),
    /// A secondary click on List's column header.
    Header((f32, f32)),
    /// The folder's view changed from the view itself: a sort, a column's width.
    Shown(crate::view::Shown),
    /// A folder's disclosure: opened or closed in place, with every folder under it with ⌥.
    Disclose(PathBuf, bool),
}

impl Pick {
    /// The menu a press asks for: a secondary click, or ⌃ with the primary.
    pub fn menu_of(e: &MouseEventData, path: Option<PathBuf>) -> Option<Pick> {
        let secondary = e.button == Some(MouseButton::Right)
            || (e.button == Some(MouseButton::Left) && e.modifiers.contains(Modifiers::CONTROL));
        secondary.then(|| Pick::Menu(path, (e.global_location.x as f32, e.global_location.y as f32)))
    }
}

/// The Name column's least width.
const NAME_MIN: f32 = 240.;

/// The hit band over a column's leading edge that drags its width.
const DIVIDER: f32 = 8.;

#[derive(Clone, Copy, PartialEq)]
struct RowClick;

#[derive(Clone, Copy, PartialEq)]
struct Divider;

#[derive(PartialEq)]
pub struct ListView {
    pub tab: State<Tab>,
    pub rev: u64,
    pub focused: bool,
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
    /// Folders open in place: each row's item and depth, in place of the folder's order.
    tree: Option<std::rc::Rc<Vec<(Entry, usize)>>>,
    tab: State<Tab>,
    focused: bool,
    on_pick: EventHandler<Pick>,
    dark: bool,
    interact: crate::interact::Interact,
    /// The columns after Name, and their widths.
    columns: std::rc::Rc<Vec<Column>>,
    widths: crate::view::Hidden,
}

impl Component for ListView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let tab = self.tab;
        let (length, order, group, shown, place) = {
            let t = tab.peek();
            (t.folder.len(), t.folder.order(), t.shown.group, t.shown, t.place.clone())
        };
        let lines = group.map(|by| {
            let now = jiff::Timestamp::now().as_second();
            std::rc::Rc::new(view::lines(tab.peek().folder.entries(), Some(by), now))
        });
        // Folders open in place: every row with its depth.
        let tree = tab.peek().tree().map(std::rc::Rc::new);
        let length = match (&tree, &lines) {
            (Some(tree), _) => tree.len(),
            (None, Some(lines)) => lines.len(),
            (None, None) => length,
        };
        // A divider being dragged: its column and width now, shown before it is kept.
        let mut resizing = use_state(|| None::<(Column, f32, f64, f32)>);
        let mut widths = shown.hidden;
        if let Some((column, width, _, _)) = *resizing.read() {
            widths.widths[column.index()] = width as u16;
        }
        let columns = shown.hidden.columns(&place);
        let on_pick_sort = self.on_pick.clone();
        let cells = theme.clone();
        let header_cell = move |title: &'static str, sort: Option<SortBy>, width: Size, right: bool, divider: Option<Element>| {
            let theme = &cells;
            let t = &theme.tokens;
            let sorted = sort == Some(order.by);
            let ink = if sorted { t.colour_text } else { t.colour_text_2 };
            let on_pick = on_pick_sort.clone();
            let name = divider.is_none();
            rect()
                .width(width)
                .maybe(name, |el| el.min_width(Size::px(NAME_MIN)))
                .height(Size::fill())
                .horizontal()
                .content(Content::flex())
                .maybe_child(divider)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .main_align(if right { Alignment::End } else { Alignment::Start })
                        .spacing(t.space_xs)
                        .a11y_role(AccessibilityRole::ColumnHeader)
                        .a11y_alt(title)
                        .on_press(move |_| {
                            let Some(by) = sort else { return };
                            let reverse = sorted && !order.reverse;
                            on_pick.call(Pick::Shown(crate::view::Shown { sort: by, reverse, ..shown }));
                        })
                        .child(text::line(theme, text::Style::Callout, colour(ink), title))
                        .maybe_child(sorted.then(|| {
                            Glyph::new(
                                if order.reverse { "caret-up" } else { "caret-down" },
                                t.size_glyph_small,
                                colour(t.colour_text),
                            )
                        })),
                )
                .into_element()
        };
        // The divider at a column's leading edge, in the gap before it and taking no room:
        // dragged, the column's width; double-clicked, fitted to its values.
        let gap = t.space_l;
        let divider = {
            let on_pick = self.on_pick.clone();
            move |column: Column, width: f32| {
                let on_pick = on_pick.clone();
                rect()
                    .width(Size::px(DIVIDER))
                    .height(Size::fill())
                    .margin((0., -DIVIDER + (gap + DIVIDER) / 2., 0., -(gap + DIVIDER) / 2.))
                    .a11y_role(AccessibilityRole::Splitter)
                    .a11y_alt(format!("{} width", column.name()))
                    .on_mouse_down(move |e: Event<MouseEventData>| {
                        if e.button != Some(MouseButton::Left) {
                            return;
                        }
                        e.stop_propagation();
                        if EventsCombos::<Divider>::pressed(e.global_location).is_double() {
                            let fitted = right_size(column, &tab.peek());
                            let mut next = shown;
                            next.hidden.widths[column.index()] = fitted as u16;
                            on_pick.call(Pick::Shown(next));
                            return;
                        }
                        resizing.set(Some((column, width, e.global_location.x, width)));
                    })
                    .into_element()
            }
        };
        let mut header_cells = vec![header_cell("Name", Some(SortBy::Name), Size::flex(1.), false, None)];
        for column in &columns {
            let width = widths.width(*column);
            header_cells.push(header_cell(column.name(), column.sort(), Size::px(width), *column == Column::Size, Some(divider(*column, width))));
        }
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
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                let Some((column, _, x0, start)) = *resizing.peek() else { return };
                // The leading edge moved right narrows the column.
                let width = (start - (e.global_location().x - x0) as f32).max(48.);
                resizing.set(Some((column, width, x0, start)));
            })
            .on_global_pointer_up({
                let on_pick = self.on_pick.clone();
                move |_: Event<PointerEventData>| {
                    let Some((column, width, _, _)) = *resizing.peek() else { return };
                    resizing.set(None);
                    let mut next = shown;
                    next.hidden.widths[column.index()] = width as u16;
                    on_pick.call(Pick::Shown(next));
                }
            })
            .on_mouse_down({
                let on_pick = self.on_pick.clone();
                move |e: Event<MouseEventData>| {
                    if e.button == Some(MouseButton::Right) {
                        e.stop_propagation();
                        on_pick.call(Pick::Header((e.global_location.x as f32, e.global_location.y as f32)));
                    }
                }
            })
            .children(header_cells);
        let bander = crate::band::use_band(self.controller);
        // The cursor moved by a key stays in view.
        let cursor = tab.read().cursor.clone();
        use_side_effect_with_deps(&cursor, {
            let (lines, mut controller, row_height, tree) = (lines.clone(), self.controller, t.size_row, tree.clone());
            move |cursor: &Option<PathBuf>| {
                if let (Some(tree), Some(cursor)) = (&tree, cursor) {
                    if let Some(line) = tree.iter().position(|(e, _)| e.path == *cursor) {
                        controller.scroll_to_offset(line as f32 * row_height, row_height, Direction::Vertical);
                    }
                    return;
                }
                let t = tab.peek();
                let Some(at) = cursor.as_deref().and_then(|c| t.folder.position(c)) else { return };
                let line = match &lines {
                    None => Some(at),
                    Some(lines) => lines.iter().position(|l| matches!(l, Line::Item { index, .. } if *index == at)),
                };
                if let Some(line) = line {
                    controller.scroll_to_offset(line as f32 * row_height, row_height, Direction::Vertical);
                }
            }
        });
        let mut layout = self.interact.layout;
        let row_height = t.size_row;
        let hits = {
            let (lines, row_height, tree) = (lines.clone(), t.size_row, tree.clone());
            move |(_, top, _, bottom): (f32, f32, f32, f32)| -> Vec<PathBuf> {
                let t = tab.peek();
                let first = (top / row_height).floor().max(0.) as usize;
                let last = ((bottom / row_height).floor().max(0.) as usize).min(length.saturating_sub(1));
                if let Some(tree) = &tree {
                    return (first..=last).filter(|_| length > 0).filter_map(|l| tree.get(l).map(|(e, _)| e.path.clone())).collect();
                }
                (first..=last)
                    .filter(|_| length > 0)
                    .filter_map(|line| match &lines {
                        None => Some(line),
                        Some(lines) => match lines.get(line)? {
                            Line::Item { index, .. } => Some(*index),
                            Line::Heading(_) => None,
                        },
                    })
                    .filter_map(|i| t.folder.entries().get(i).map(|e| e.path.clone()))
                    .collect()
            }
        };
        let rows = Rows {
            rev: self.rev,
            lines,
            tree: tree.clone(),
            tab: self.tab,
            focused: self.focused,
            on_pick: self.on_pick.clone(),
            dark: theme.appearance.mode == isle_tokens::Mode::Dark,
            interact: self.interact.clone(),
            columns: std::rc::Rc::new(columns),
            widths,
        };
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
                    .on_sized(move |e: Event<SizedEventData>| {
                        bander.sized(e.area);
                        let page = (e.area.height() / row_height).floor().max(1.) as usize;
                        layout.set_if_modified(crate::keys::Layout { columns: 1, page });
                    })
                    .on_mouse_down({
                        let on_pick = self.on_pick.clone();
                        move |e: Event<MouseEventData>| {
                            if let Some(menu) = Pick::menu_of(&e, None) {
                                on_pick.call(menu);
                            } else if e.button == Some(MouseButton::Left) {
                                bander.begin(tab, &e);
                            }
                        }
                    })
                    .on_global_pointer_move(move |e: Event<PointerEventData>| {
                        if bander.dragging() {
                            let at = e.global_location();
                            bander.drag(tab, (at.x, at.y), &hits);
                        }
                    })
                    .on_global_pointer_up(move |_| bander.end())
                    .maybe_child(bander.overlay(&theme))
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
    let (entry, depth, odd) = match &rows.tree {
        Some(tree) => match tree.get(item.index) {
            Some((entry, depth)) => (entry, *depth, item.index % 2 == 1),
            None => return rect().into_element(),
        },
        None => {
            let Some((index, odd)) = item_at(rows, item.index) else {
                return rect().into_element();
            };
            let Some(entry) = tab.folder.entries().get(index) else {
                return rect().into_element();
            };
            (entry, 0, odd)
        }
    };
    let selected_line = |line: Option<usize>| match &rows.tree {
        Some(tree) => line.and_then(|l| tree.get(l)).is_some_and(|(e, _)| tab.is_selected(&e.path)),
        None => line
            .and_then(|l| item_at(rows, l))
            .and_then(|(i, _)| tab.folder.entries().get(i))
            .is_some_and(|e| tab.is_selected(&e.path)),
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
    let dim = rows.interact.is_dimmed(entry);
    let target = rows.interact.is_target(&entry.path) || rows.interact.ringed.contains(&entry.path);
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
    let on_pick = rows.on_pick.clone();
    let dir = entry.is_dir();
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
                .maybe(tab.cursor.as_deref() == Some(entry.path.as_path()), |el| el.on_sized(rows.interact.cursor_sized()))
                .a11y_alt(entry.name.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_pointer_enter(rows.interact.target_on_enter(&entry.path, entry.is_dir()))
                .on_pointer_leave(rows.interact.target_on_leave(&entry.path))
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    if let Some(menu) = Pick::menu_of(&e, Some(path.clone())) {
                        e.stop_propagation();
                        on_pick.call(menu);
                        return;
                    }
                    // A middle click opens a folder in a new tab.
                    if e.button == Some(MouseButton::Middle) && dir {
                        e.stop_propagation();
                        on_pick.call(Pick::Open(path.clone(), Held::NEW_TAB));
                        return;
                    }
                    if e.button != Some(MouseButton::Left) {
                        return;
                    }
                    e.stop_propagation();
                    interact.press(&path, e.global_location.x, e.global_location.y);
                    let held = Held::of(e.modifiers);
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
                        .min_width(Size::px(NAME_MIN))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .padding((0., 0., 0., depth as f32 * t.space_l))
                        .child(disclosure(&theme, entry, tab.expanded.contains(&entry.path), rows.on_pick.clone(), ink_2))
                        .opacity(if dim { 0.5 } else { 1. })
                        .child(icon(entry, rows.dark, t.size_app_icon_small, ink_2))
                        .child(match rows.interact.renaming(&entry.path) {
                            Some(rename) => crate::interact::rename_field(&theme, rename, Size::flex(1.)),
                            None => text::line(&theme, text::Style::Body, colour(ink), crate::prefs::display_name(entry).to_owned()).into_element(),
                        }),
                )
                .children(rows.columns.iter().map(|column| {
                    let width = rows.widths.width(*column);
                    if entry.writing && *column == Column::Modified {
                        return rect().width(Size::px(width)).center().child(writing_bar(&theme, 96., rows.interact.writing)).into_element();
                    }
                    let counted = tab.sizes.get(&entry.path).copied();
                    cell(&theme, *column, entry, counted, width, ink_2, &now)
                })),
        )
        .into_element()
}

/// A folder's disclosure triangle, or its room for anything else; a press opens or closes
/// the folder in place.
fn disclosure(theme: &isle_tokens::Theme, entry: &Entry, open: bool, on_pick: EventHandler<Pick>, ink: isle_tokens::Colour) -> Element {
    let t = &theme.tokens;
    let path = entry.path.clone();
    rect()
        .width(Size::px(16.))
        .height(Size::px(16.))
        .center()
        .margin((0., -t.space_s + t.space_xxs, 0., 0.))
        .maybe(entry.is_dir(), |el| {
            el.a11y_role(AccessibilityRole::Button)
                .a11y_alt(if open { "Collapse" } else { "Expand" })
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    if e.button == Some(MouseButton::Left) {
                        e.stop_propagation();
                        on_pick.call(Pick::Disclose(path.clone(), e.modifiers.contains(Modifiers::ALT)));
                    }
                })
                .child(Glyph::new(if open { "caret-down" } else { "caret-right" }, 12., colour(ink)))
        })
        .into_element()
}

/// One column's cell for `entry`.
/// `counted` is a folder's size as Calculate all sizes found it.
fn cell(theme: &isle_tokens::Theme, column: Column, entry: &Entry, counted: Option<u64>, width: f32, ink: isle_tokens::Colour, now: &jiff::Zoned) -> Element {
    let t = &theme.tokens;
    let words = |s: String| text::line(theme, text::Style::Body, colour(ink), s).max_lines(1).text_overflow(TextOverflow::Ellipsis);
    let deleted = || entry.trashed.as_deref().and_then(|t| deleted_at(&t.deleted));
    let folder = |p: Option<&std::path::Path>| {
        rect()
            .width(Size::px(width))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(Glyph::new("folder-simple", t.size_glyph_small, colour(ink)))
            .child(words(p.map(crate::places::folder_name).unwrap_or_default()))
            .into_element()
    };
    match column {
        Column::Where => folder(entry.path.parent()),
        Column::Original => folder(entry.trashed.as_deref().and_then(|t| t.original.parent())),
        Column::Size => rect()
            .width(Size::px(width))
            .main_align(Alignment::End)
            .horizontal()
            .child(words(match (entry.is_dir(), counted) {
                (true, Some(bytes)) => format::size(bytes),
                (true, None) => "--".into(),
                (false, _) => format::size(entry.size),
            }))
            .into_element(),
        other => {
            let value = match other {
                Column::Modified => format::date(entry.modified, now),
                Column::Created => entry.created.map_or_else(|| "--".into(), |c| format::date(c, now)),
                Column::Opened => format::date(entry.accessed, now),
                Column::Deleted => deleted().map_or_else(|| "--".into(), |d| format::date(d, now)),
                _ => kind(entry),
            };
            rect().width(Size::px(width)).child(words(value)).into_element()
        }
    }
}

/// A trash's deletion time, `YYYY-MM-DDThh:mm:ss` in local time, as seconds.
fn deleted_at(text: &str) -> Option<i64> {
    let civil: jiff::civil::DateTime = text.parse().ok()?;
    Some(civil.to_zoned(jiff::tz::TimeZone::system()).ok()?.timestamp().as_second())
}

/// The width that fits `column`'s widest value in `tab`, as Right Size gives it.
fn right_size(column: Column, tab: &Tab) -> f32 {
    let now = format::now();
    let widest = tab
        .folder
        .entries()
        .iter()
        .map(|e| match column {
            Column::Modified => format::date(e.modified, &now),
            Column::Created => e.created.map_or_else(|| "--".into(), |c| format::date(c, &now)),
            Column::Opened => format::date(e.accessed, &now),
            Column::Size => format::size(e.size),
            Column::Kind => kind(e),
            Column::Deleted => e.trashed.as_deref().and_then(|t| deleted_at(&t.deleted)).map_or_else(String::new, |d| format::date(d, &now)),
            Column::Original => e.trashed.as_deref().and_then(|t| t.original.parent().map(crate::places::folder_name)).unwrap_or_default(),
            Column::Where => e.path.parent().map(crate::places::folder_name).unwrap_or_default(),
        })
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(4);
    (widest.max(column.name().chars().count()) as f32 * 7.2 + 24.).clamp(48., 480.)
}

/// The words Kind shows.
pub fn kind(entry: &Entry) -> String {
    if entry.name.to_lowercase().ends_with(".appimage") {
        return "Application (AppImage)".into();
    }
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
    let image = match found {
        Some(path) => AppIcon::new(path, size).into_element(),
        None => Glyph::new(if entry.is_dir() { "folder-simple" } else { "file" }, size, colour(ink))
            .into_element(),
    };
    badged(entry, image, size)
}

/// `image`, an item's icon at `size`, with an alias's badge on its bottom leading corner; a
/// broken alias at half strength.
pub fn badged(entry: &Entry, image: Element, size: f32) -> Element {
    let isle_fs::Kind::Link { broken, .. } = entry.kind else { return image };
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let badge = (size / 4.).max(10.);
    rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .opacity(if broken { 0.5 } else { 1. })
        .child(image)
        .child(
            rect()
                .position(Position::new_absolute().left(0.).top(size - badge))
                .width(Size::px(badge))
                .height(Size::px(badge))
                .center()
                .corner_radius(t.radius_small)
                .background(colour(t.colour_window_raised))
                .maybe(true, |el| isle_ui::shadows(&t.shadow_control).into_iter().fold(el, |el, s| el.shadow(s)))
                .child(Glyph::new("arrow-bend-up-right", badge * 0.75, colour(t.colour_text))),
        )
        .into_element()
}

/// The bar under an item being written: `share` of it done.
pub fn writing_bar(theme: &isle_tokens::Theme, width: f32, share: f32) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(width))
        .height(Size::px(t.size_item_progress))
        .corner_radius(2.)
        .background(colour(t.colour_fill_strong))
        .child(
            rect()
                .width(Size::percent(share.clamp(0., 1.) * 100.))
                .height(Size::fill())
                .corner_radius(2.)
                .background(colour(t.colour_accent)),
        )
        .into_element()
}
