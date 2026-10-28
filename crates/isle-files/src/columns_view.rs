//! Columns view: a column per folder from the first column's down to the tab's, the
//! preview last (`p1/surfaces/files.md` § Columns view).

use std::path::{Path, PathBuf};

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
    pub on_pick: EventHandler<Pick>,
    /// Moves the tab to a folder, revealing an item in it.
    pub on_go: EventHandler<(PathBuf, Option<PathBuf>)>,
    pub interact: crate::interact::Interact,
}

/// A column before the last: a folder on the way from the first column to the tab's.
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

/// The folders from `root` down to `place`, both included, or None when `place` is not
/// under `root`.
fn chain(root: &Path, place: &Path) -> Option<Vec<PathBuf>> {
    let rest = place.strip_prefix(root).ok()?;
    let mut folders = vec![root.to_owned()];
    for part in rest.components() {
        let next = folders.last().unwrap().join(part);
        folders.push(next);
    }
    Some(folders)
}

impl Component for ColumnsView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let tab = self.tab;
        let place = tab.read().place.folder().map(PathBuf::from);
        let mut root = use_state(|| place.clone());
        let mut columns = use_state(Vec::<Column>::new);

        // The tab's place is the last column; the columns before it run down from the first
        // column's folder, which moves to the place when the place leaves it.
        let _rev = tab.read().rev;
        let folders = match (&*root.peek(), &place) {
            (Some(r), Some(p)) => chain(r, p),
            _ => None,
        };
        let folders = match folders {
            Some(folders) => folders,
            None => {
                root.set(place.clone());
                place.iter().cloned().collect()
            }
        };
        let before = &folders[..folders.len().saturating_sub(1)];
        let existing = columns.peek().clone();
        let mut chain_now = Vec::new();
        for (n, path) in before.iter().enumerate() {
            let column = match existing.iter().find(|c| c.path == *path) {
                Some(column) => column.clone(),
                None => {
                    let mut fresh = Tab::new(0, Place::Folder(path.clone()));
                    fresh.folder.set_show_hidden(tab.peek().folder.shows_hidden());
                    let column = Column { path: path.clone(), tab: State::create(fresh), _live: Live::default() };
                    tab::load(column.tab, &column._live, false);
                    column
                }
            };
            // A column shows the folder the next one lists as chosen.
            let chosen = folders[n + 1].clone();
            if column.tab.peek().selection != [chosen.clone()] {
                let mut listing = column.tab;
                listing.write().select_only(chosen);
            }
            chain_now.push(column);
        }
        if chain_now != existing {
            columns.set(chain_now.clone());
        }

        // The last column with a selection is the one the keys and the accent are in.
        let all: Vec<(State<Tab>, PathBuf)> = chain_now
            .iter()
            .map(|c| (c.tab, c.path.clone()))
            .chain(place.clone().map(|p| (tab, p)))
            .collect();
        let last = all.len().saturating_sub(1);
        let active = all.iter().rposition(|(t, _)| !t.read().selection.is_empty()).unwrap_or(last);
        let preview = {
            let t = tab.read();
            let items: Vec<_> = t.selection.iter().filter_map(|p| t.folder.get(p).cloned()).collect();
            (!items.is_empty() && items.iter().all(|e| !e.is_dir())).then_some(items)
        };

        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let column_views = all.iter().enumerate().map(|(n, (listing, folder))| {
            let (on_pick, on_go, listing, folder) = (self.on_pick.clone(), self.on_go.clone(), *listing, folder.clone());
            column(&theme, listing, n == active && self.focused, dark, &self.interact, move |pick: Pick| {
                let is_dir = |path: &Path| listing.peek().folder.get(path).is_some_and(|e| e.is_dir());
                if n == last {
                    match pick {
                        // A plain click on a folder opens it in the next column.
                        Pick::Click(path, held) if !held.cmd && !held.shift && is_dir(&path) => on_go.call((path, None)),
                        pick => on_pick.call(pick),
                    }
                    return;
                }
                match pick {
                    menu @ (Pick::Menu(..) | Pick::Header(_) | Pick::Shown(_)) => on_pick.call(menu),
                    Pick::Open(path, held) if is_dir(&path) && (held.cmd || held.option) => on_pick.call(Pick::Open(path, held)),
                    Pick::Click(path, _) | Pick::Open(path, _) if is_dir(&path) => on_go.call((path, None)),
                    Pick::Click(path, _) => on_go.call((folder.clone(), Some(path))),
                    Pick::Open(path, held) => {
                        on_go.call((folder.clone(), Some(path.clone())));
                        on_pick.call(Pick::Open(path, held));
                    }
                    Pick::Nothing => on_go.call((folder.clone(), None)),
                }
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
                        action: None,
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
            let dir = entry.is_dir();
            let target = interact.is_target(&entry.path) || interact.ringed.contains(&entry.path);
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
                            if let Some(menu) = Pick::menu_of(&e, Some(path.clone())) {
                                e.stop_propagation();
                                on_pick(menu);
                                return;
                            }
                            // A middle click opens a folder in a new tab.
                            if e.button == Some(MouseButton::Middle) && dir {
                                e.stop_propagation();
                                on_pick(Pick::Open(path.clone(), Held::NEW_TAB));
                                return;
                            }
                            if e.button != Some(MouseButton::Left) {
                                return;
                            }
                            e.stop_propagation();
                            pressing.press(&path, e.global_location.x, e.global_location.y);
                            let double = EventsCombos::<ColumnClick>::pressed(e.global_location).is_double();
                            let held = Held::of(e.modifiers);
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
        .on_mouse_down(move |e: Event<MouseEventData>| empty(Pick::menu_of(&e, None).unwrap_or(Pick::Nothing)))
        .child(
            isle_ui::scroll_area(theme)
                .width(Size::fill())
                .height(Size::fill())
                .child(rect().width(Size::fill()).padding((t.space_xs, 0.)).children(rows)),
        )
        .into_element()
}
