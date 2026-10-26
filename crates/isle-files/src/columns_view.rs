//! Columns view: a column per level from the folder shown, each opened by the folder chosen
//! in the one before, the preview last (`p1/surfaces/files.md` § Columns view).

use std::path::PathBuf;

use freya::prelude::*;
use isle_ui::{Glyph, colour, text, use_theme};

use crate::{
    list_view::{self, Held, Pick},
    places::Place,
    preview::Preview,
    tab::{self, Live, Tab},
};

#[derive(Clone, Copy, PartialEq)]
struct ColumnClick;

#[derive(PartialEq)]
pub struct ColumnsView {
    pub tab: State<Tab>,
    pub rev: u64,
    pub focused: bool,
    pub held: State<Held>,
    pub on_pick: EventHandler<Pick>,
    pub interact: crate::interact::Interact,
}

/// A column after the first: a folder chosen in the column before it.
#[derive(Clone)]
struct Column {
    path: PathBuf,
    tab: State<Tab>,
    _live: Live,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.tab == other.tab
    }
}

/// The one folder selected in a column, which the next column shows.
fn chosen_folder(tab: &Tab) -> Option<PathBuf> {
    match tab.selection.as_slice() {
        [one] => tab.folder.get(one).filter(|e| e.is_dir()).map(|e| e.path.clone()),
        _ => None,
    }
}

impl Component for ColumnsView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut columns = use_state(Vec::<Column>::new);
        let root = self.tab;

        // The chain follows each column's chosen folder; a column whose folder is still
        // chosen keeps its listing.
        let _rev = root.read().rev;
        let mut chain = Vec::new();
        let mut next = chosen_folder(&root.read());
        let existing = columns.read().clone();
        for column in &existing {
            match &next {
                Some(path) if *path == column.path => {
                    chain.push(column.clone());
                    next = chosen_folder(&column.tab.read());
                }
                _ => break,
            }
        }
        let mut grown = false;
        while let Some(path) = next.take() {
            let mut fresh = Tab::new(0, Place::Folder(path.clone()));
            fresh.folder.set_show_hidden(root.peek().folder.shows_hidden());
            let column = Column {
                path,
                tab: State::create(fresh),
                _live: Live::default(),
            };
            tab::load(column.tab, &column._live, false);
            chain.push(column);
            grown = true;
        }
        if grown || chain.len() != existing.len() {
            columns.set(chain.clone());
        }

        // The deepest column with a selection is the one the keys and the accent are in.
        let all: Vec<State<Tab>> = std::iter::once(root).chain(chain.iter().map(|c| c.tab)).collect();
        let active = all
            .iter()
            .rposition(|tab| !tab.read().selection.is_empty())
            .unwrap_or(0);
        let last = all.last().copied().unwrap_or(root);
        let preview = {
            let tab = last.read();
            let items: Vec<_> = tab.selection.iter().filter_map(|p| tab.folder.get(p).cloned()).collect();
            (!items.is_empty() && items.iter().all(|e| !e.is_dir())).then_some(items)
        };

        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let column_views = all.iter().enumerate().map(|(n, &tab)| {
            let on_pick = self.on_pick.clone();
            let held = self.held;
            column(&theme, tab, n == active && self.focused, dark, &self.interact, move |pick: Pick| {
                if n == 0 {
                    on_pick.call(pick);
                    return;
                }
                let mut tab = tab;
                match pick {
                    Pick::Click(path, held) if held.cmd => tab.write().toggle(path),
                    Pick::Click(path, _) => tab.write().select_only(path),
                    Pick::Open(path, held) => {
                        tab.write().select_only(path.clone());
                        on_pick.call(Pick::Open(path, held));
                    }
                    Pick::Nothing => tab.write().deselect(),
                }
                let _ = held;
            })
        });
        ScrollView::new()
            .direction(Direction::Horizontal)
            .expanded()
            .child(
                rect()
                    .height(Size::fill())
                    .horizontal()
                    .background(colour(t.colour_view))
                    .children(column_views)
                    .maybe_child(preview.map(|items| Preview {
                        items,
                        folder: None,
                        width: t.size_inspector,
                        pane: true,
                    })),
            )
    }
}

fn column(
    theme: &isle_tokens::Theme,
    tab: State<Tab>,
    active: bool,
    dark: bool,
    interact: &crate::interact::Interact,
    on_pick: impl Fn(Pick) + Clone + 'static,
) -> Element {
    let t = &theme.tokens;
    let snapshot = tab.read();
    let rows: Vec<Element> = snapshot
        .folder
        .entries()
        .iter()
        .map(|entry| {
            let selected = snapshot.is_selected(&entry.path);
            let fill = selected.then(|| if active { t.colour_accent } else { t.colour_selection_unfocused });
            let ink = if selected && active { t.colour_on_accent } else { t.colour_text };
            let path = entry.path.clone();
            let on_pick = on_pick.clone();
            let pressing = interact.clone();
            let target = interact.is_target(&entry.path);
            rect()
                .key(entry.path.to_string_lossy().into_owned())
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_xs))
                .child(
                    rect()
                        .expanded()
                        .padding((0., t.space_s))
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .corner_radius(t.radius_row)
                        .opacity(if interact.is_dimmed(entry) { 0.5 } else { 1. })
                        .map(fill, |el, fill| el.background(colour(fill)))
                        .maybe(target, |el| el.border(crate::interact::drop_ring(theme)))
                        .on_pointer_enter(interact.target_on_enter(&entry.path, entry.is_dir()))
                        .on_pointer_leave(interact.target_on_leave(&entry.path))
                        .a11y_role(AccessibilityRole::ListItem)
                        .a11y_alt(entry.name.clone())
                        .a11y_builder(move |node| node.set_selected(selected))
                        .on_mouse_down(move |e: Event<MouseEventData>| {
                            if e.button != Some(MouseButton::Left) {
                                return;
                            }
                            e.stop_propagation();
                            pressing.press(&path, e.global_location.x, e.global_location.y);
                            let double = EventsCombos::<ColumnClick>::pressed(e.global_location).is_double();
                            let held = Held::default();
                            on_pick(if double { Pick::Open(path.clone(), held) } else { Pick::Click(path.clone(), held) });
                        })
                        .child(list_view::icon(entry, dark, t.size_app_icon_small, t.colour_text_2))
                        .child(rect().width(Size::flex(1.)).child(match interact.renaming(&entry.path) {
                            Some(rename) => crate::interact::rename_field(theme, rename, Size::fill()),
                            None => text::line(theme, text::Style::Body, colour(ink), crate::prefs::display_name(entry).to_owned()).into_element(),
                        }))
                        .maybe_child(entry.is_dir().then(|| {
                            Glyph::new(
                                "caret-right",
                                t.size_glyph_small,
                                colour(if selected && active { t.colour_on_accent } else { t.colour_text_2 }),
                            )
                        })),
                )
                .into_element()
        })
        .collect();
    drop(snapshot);
    let empty = on_pick.clone();
    rect()
        .width(Size::px(t.size_column))
        .height(Size::fill())
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth { top: 0., right: 1., bottom: 0., left: 0. })
                .alignment(BorderAlignment::Inner),
        )
        .on_mouse_down(move |_| empty(Pick::Nothing))
        .child(
            isle_ui::scroll_area(theme)
                .width(Size::fill())
                .height(Size::fill())
                .child(rect().width(Size::fill()).padding((t.space_xs, 0.)).children(rows)),
        )
        .into_element()
}
