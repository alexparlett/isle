//! Icons view: a grid of icons with their names, always in the sort, filling the width as
//! Finder's does (`p1/surfaces/files.md` § Icons view).

use std::rc::Rc;

use freya::prelude::*;
use isle_ui::{colour, text, use_theme};

use crate::{
    list_view::{self, Held, Pick},
    tab::Tab,
    view::{self, Line},
};

#[derive(Clone, Copy, PartialEq)]
struct CellClick;

#[derive(PartialEq)]
pub struct IconsView {
    pub tab: State<Tab>,
    pub rev: u64,
    pub focused: bool,
    pub held: State<Held>,
    pub on_pick: EventHandler<Pick>,
    pub controller: ScrollController,
    pub interact: crate::interact::Interact,
}

/// One line of the grid: a group's heading or a row of cells, by the folder's indexes.
#[derive(Clone, Debug, PartialEq)]
enum GridLine {
    Heading(String),
    Cells(Vec<usize>),
}

#[derive(Clone, PartialEq)]
struct Grid {
    rev: u64,
    lines: Rc<Vec<GridLine>>,
    tab: State<Tab>,
    focused: bool,
    held: State<Held>,
    on_pick: EventHandler<Pick>,
    dark: bool,
    icon: f32,
    cell: f32,
    gap: f32,
    info: bool,
    interact: crate::interact::Interact,
}

/// Cuts the folder's lines into rows of `columns` cells, a group starting its own row.
fn grid(lines: Vec<Line>, columns: usize) -> Vec<GridLine> {
    let mut out = Vec::new();
    let mut row = Vec::with_capacity(columns);
    for line in lines {
        match line {
            Line::Heading(name) => {
                if !row.is_empty() {
                    out.push(GridLine::Cells(std::mem::take(&mut row)));
                }
                out.push(GridLine::Heading(name));
            }
            Line::Item { index, .. } => {
                row.push(index);
                if row.len() == columns {
                    out.push(GridLine::Cells(std::mem::take(&mut row)));
                }
            }
        }
    }
    if !row.is_empty() {
        out.push(GridLine::Cells(row));
    }
    out
}

impl Component for IconsView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut width = use_state(|| 0f32);
        let (shown, lines) = {
            let tab = self.tab.peek();
            let now = jiff::Timestamp::now().as_second();
            (tab.shown, view::lines(tab.folder.entries(), tab.shown.group, now))
        };
        let icon = shown.icon_size as f32;
        let cell = icon + t.size_icon_cell_pad;
        let inner = (*width.read() - 2. * t.space_l).max(cell);
        let columns = ((inner / cell).floor() as usize).max(1);
        // What is left over is shared between the gaps, so the grid fills the width.
        let gap = if columns > 1 { (inner - columns as f32 * cell) / (columns - 1) as f32 } else { 0. };
        let lines = Rc::new(grid(lines, columns));
        let (_, line_height, _) = text::metrics(&theme, text::Style::Body);
        let (_, info_height, _) = text::metrics(&theme, text::Style::Subheadline);
        let row_height = t.space_s + icon + t.space_xs + 2. * line_height + if shown.item_info { info_height } else { 0. } + t.space_s;
        let heading = t.size_row + t.space_s;
        let sizes = lines.clone();
        let data = Grid {
            rev: self.rev,
            lines: lines.clone(),
            tab: self.tab,
            focused: self.focused,
            held: self.held,
            on_pick: self.on_pick.clone(),
            dark: theme.appearance.mode == isle_tokens::Mode::Dark,
            icon,
            cell,
            gap,
            info: shown.item_info,
            interact: self.interact.clone(),
        };
        let on_pick = self.on_pick.clone();
        rect()
            .expanded()
            .background(colour(t.colour_view))
            .a11y_role(AccessibilityRole::Grid)
            .on_sized(move |e: Event<SizedEventData>| width.set_if_modified(e.area.width()))
            .on_mouse_down(move |_| on_pick.call(Pick::Nothing))
            .child(
                VirtualScrollView::new_with_data_controlled(data, grid_line, self.controller)
                    .length(lines.len())
                    .item_size(move |i: usize| match sizes.get(i) {
                        Some(GridLine::Heading(_)) => heading,
                        _ => row_height,
                    })
                    .expanded(),
            )
    }
}

fn grid_line(item: VirtualItem, grid: &Grid) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    match grid.lines.get(item.index) {
        Some(GridLine::Heading(name)) => rect()
            .width(Size::fill())
            .padding((t.space_s, 0., 0., 0.))
            .child(list_view::heading(&theme, name))
            .into_element(),
        Some(GridLine::Cells(cells)) => rect()
            .width(Size::fill())
            .padding((0., t.space_l))
            .horizontal()
            .spacing(grid.gap)
            .children(cells.iter().map(|&i| cell(&theme, grid, i)))
            .into_element(),
        None => rect().into_element(),
    }
}

fn cell(theme: &isle_tokens::Theme, grid: &Grid, index: usize) -> Element {
    let t = &theme.tokens;
    let tab = grid.tab.peek();
    let Some(entry) = tab.folder.entries().get(index) else {
        return rect().into_element();
    };
    let selected = tab.is_selected(&entry.path);
    let capsule = if grid.focused { t.colour_accent } else { t.colour_selection_unfocused };
    let ink = if selected && grid.focused { t.colour_on_accent } else { t.colour_text };
    let path = entry.path.clone();
    let interact = grid.interact.clone();
    let target = grid.interact.is_target(&entry.path);
    let body = text::metrics(theme, text::Style::Body);
    let (on_pick, held) = (grid.on_pick.clone(), grid.held);
    let info = if !grid.info {
        None
    } else if entry.is_dir() {
        Some("Folder".to_owned())
    } else {
        Some(crate::format::size(entry.size))
    };
    rect()
        .key(entry.path.to_string_lossy().into_owned())
        .width(Size::px(grid.cell))
        .padding((t.space_s, 0.))
        .cross_align(Alignment::Center)
        .spacing(t.space_xs)
        .opacity(if entry.hidden() || grid.interact.is_cut(&entry.path) { 0.5 } else { 1. })
        .a11y_role(AccessibilityRole::GridCell)
        .a11y_alt(entry.name.clone())
        .a11y_builder(move |node| node.set_selected(selected))
        .on_pointer_enter(grid.interact.target_on_enter(&entry.path, entry.is_dir()))
        .on_pointer_leave(grid.interact.target_on_leave(&entry.path))
        .on_mouse_down(move |e: Event<MouseEventData>| {
            if e.button != Some(MouseButton::Left) {
                return;
            }
            e.stop_propagation();
            interact.press(&path, e.global_location.x, e.global_location.y);
            let held = *held.peek();
            let double = EventsCombos::<CellClick>::pressed(e.global_location).is_double();
            on_pick.call(if double { Pick::Open(path.clone(), held) } else { Pick::Click(path.clone(), held) });
        })
        .child(
            rect()
                .padding(t.space_xs)
                .corner_radius(t.radius_card)
                .maybe(selected || target, |el| el.background(colour(t.colour_fill_strong)))
                .maybe(target, |el| el.border(crate::interact::drop_ring(theme)))
                .child(crate::thumbnail::Thumbnail {
                    entry: entry.clone(),
                    size: grid.icon,
                    dark: grid.dark,
                    remote: tab.remote,
                }),
        )
        .child(match grid.interact.renaming(&entry.path) {
            Some(rename) => crate::interact::rename_field(theme, rename, Size::px(grid.cell)),
            None => rect()
                .padding((1., t.space_xs))
                .corner_radius(t.radius_small)
                .maybe(selected, |el| el.background(colour(capsule)))
                .child(
                    label()
                        .max_width(Size::px(grid.cell - 4. * t.space_xs))
                        .font_size(body.0)
                        .line_height(body.1 / body.0)
                        .font_weight(body.2 as i32)
                        .color(colour(ink))
                        .max_lines(2)
                        .text_overflow(TextOverflow::Ellipsis)
                        .text(entry.name.clone()),
                )
                .into_element(),
        })
        .maybe_child(info.map(|info| text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), info)))
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_starts_its_own_row() {
        let lines = vec![
            Line::Heading("Folders".into()),
            Line::Item { index: 0, odd: false },
            Line::Heading("Documents".into()),
            Line::Item { index: 1, odd: false },
            Line::Item { index: 2, odd: true },
            Line::Item { index: 3, odd: false },
        ];
        assert_eq!(
            grid(lines, 2),
            [
                GridLine::Heading("Folders".into()),
                GridLine::Cells(vec![0]),
                GridLine::Heading("Documents".into()),
                GridLine::Cells(vec![1, 2]),
                GridLine::Cells(vec![3]),
            ]
        );
    }
}
