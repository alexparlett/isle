//! The sidebar of places (`p1/surfaces/files.md` § Sidebar).

use std::path::PathBuf;

use freya::prelude::*;
use isle_app::ToolbarExt;
use isle_ui::{Glyph, colour, text, use_theme};

use crate::{
    places::{Place, Places, Row},
    search::Search,
};


#[derive(PartialEq)]
pub(crate) struct Sidebar {
    pub places: Places,
    pub prefs: crate::prefs::Prefs,
    pub saved: Vec<Search>,
    pub shown: Place,
    pub volumes: Vec<isle_disks::Volume>,
    pub shares: Vec<PathBuf>,
    pub space: std::collections::HashMap<PathBuf, (u64, u64)>,
    /// Volumes being mounted, by object path.
    pub mounting: Vec<String>,
    pub dragging: bool,
    pub drop_target: State<Option<PathBuf>>,
    /// The Favourite a drop would add a bookmark before, past the last for the end.
    pub bookmark_at: State<Option<usize>>,
    /// Groups collapsed, by heading, and a heading clicked.
    pub collapsed: Vec<String>,
    pub on_heading: EventHandler<String>,
    /// A Favourite dragged from one place to a gap: indexes into the Favourites.
    pub on_reorder: Option<EventHandler<(usize, usize)>>,
    pub on_go: EventHandler<Place>,
    pub on_volume: EventHandler<isle_disks::Volume>,
    pub on_eject: EventHandler<isle_disks::Volume>,
    /// A network share's Eject.
    pub on_unmount: EventHandler<PathBuf>,
    /// A secondary click on a row, at a point in the window.
    pub on_menu: EventHandler<(crate::menus::Side, (f32, f32))>,
    pub width: f32,
    /// Leaves the window's toolbar band clear at its top.
    pub under_toolbar: bool,
    /// The toolbar's sidebar toggle, over the sidebar's top trailing corner.
    pub on_hide: Option<EventHandler<()>>,
}

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dragging = self.dragging;
        let drop_target = self.drop_target;
        // A row: its glyph and label, selected while its place is shown, and under a drag the
        // folder (or the Trash) a drop goes into.
        let row = |key: String, glyph: &'static str, label: String, selected: bool, dimmed: bool, target: Option<PathBuf>, on_press: Box<dyn Fn()>, trailing: Option<Element>, side: crate::menus::Side, description: Option<String>| {
            let on_menu = self.on_menu.clone();
            let ink = if selected { t.colour_on_accent } else if dimmed { t.colour_text_3 } else { t.colour_text };
            let glyph_ink = if selected { t.colour_on_accent } else if dimmed { t.colour_text_3 } else { t.colour_accent_text };
            let ringed = dragging && target.is_some() && drop_target.peek().as_deref() == target.as_deref();
            let (enter, leave) = (target.clone(), target.clone());
            rect()
                .key(key)
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .corner_radius(t.radius_row)
                .maybe(selected, |el| el.background(colour(t.colour_accent)))
                .maybe(ringed, |el| el.border(crate::interact::drop_ring(&theme)))
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(label.clone())
                .a11y_builder(move |node| {
                    node.set_selected(selected);
                    if let Some(description) = description.clone() {
                        node.set_description(description);
                    }
                })
                .on_pointer_enter(move |_| {
                    if dragging && enter.is_some() {
                        let mut drop_target = drop_target;
                        drop_target.set_if_modified(enter.clone());
                    }
                })
                .on_pointer_leave(move |_| {
                    if leave.is_some() && *drop_target.peek() == leave {
                        let mut drop_target = drop_target;
                        drop_target.set(None);
                    }
                })
                .on_press(move |_| on_press())
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    let secondary = e.button == Some(MouseButton::Right)
                        || (e.button == Some(MouseButton::Left) && e.modifiers.contains(Modifiers::CONTROL));
                    if secondary {
                        e.stop_propagation();
                        on_menu.call((side.clone(), (e.global_location.x as f32, e.global_location.y as f32)));
                    }
                })
                .child(Glyph::new(glyph, t.size_glyph, colour(glyph_ink)))
                .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Body, colour(ink), label)))
                .maybe_child(trailing)
                .into_element()
        };
        let place_row = |r: &Row| {
            let on_go = self.on_go.clone();
            let place = r.place.clone();
            let target = match &r.place {
                Place::Folder(p) => Some(p.clone()),
                Place::Trash => Some(PathBuf::from(crate::interact::TRASH)),
                _ => None,
            };
            let side = crate::menus::Side::Place(r.place.clone(), r.label.clone());
            row(format!("{:?}", r.place), r.glyph, r.label.clone(), r.place == self.shown, false, target, Box::new(move || on_go.call(place.clone())), None, side, None)
        };
        // A Favourite pressed: where from, the pointer's height then, and once it moves the gap
        // it would go to.
        let mut reorder = use_state(|| None::<(usize, f64, Option<usize>)>);
        // Each Favourite's top and bottom in the window, by index.
        let areas = use_state(std::collections::BTreeMap::<usize, (f32, f32)>::new);
        let can_reorder = self.on_reorder.is_some();
        // A heading collapses its group on a click; its caret shows under hover.
        let mut hovered = use_state(|| None::<String>);
        let header = |title: &'static str| {
            let open = !self.collapsed.iter().any(|c| c == title);
            let on_heading = self.on_heading.clone();
            rect()
                .width(Size::fill())
                .height(Size::px(24.))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::End)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(title)
                .a11y_builder(move |node| node.set_expanded(open))
                .on_pointer_enter(move |_| hovered.set(Some(title.to_owned())))
                .on_pointer_leave(move |_| hovered.set(None))
                .on_press(move |_| on_heading.call(title.to_owned()))
                .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), title.to_owned()).font_weight(600)))
                .maybe_child((hovered.read().as_deref() == Some(title)).then(|| {
                    rect()
                        .width(Size::px(20.))
                        .height(Size::px(20.))
                        .center()
                        .child(Glyph::new(if open { "caret-down" } else { "caret-right" }, 12., colour(t.colour_text_2)))
                }))
                .into_element()
        };
        let open = |title: &str| !self.collapsed.iter().any(|c| c == title);
        let recents = Row {
            place: Place::Recents,
            label: "Recents".into(),
            glyph: "clock-counter-clockwise",
        };
        let volume_rows = self.volumes.iter().map(|v| {
            let mounted = v.mount_point.as_ref().map(PathBuf::from);
            let shown = mounted.as_ref().is_some_and(|m| Place::Folder(m.clone()) == self.shown);
            let (on_volume, on_eject) = (self.on_volume.clone(), self.on_eject.clone());
            let (pressed, ejecting) = (v.clone(), v.clone());
            let glyph = if v.removable || v.loop_device { "usb" } else { "hard-drive" };
            let space = mounted.as_ref().and_then(|m| self.space.get(m));
            let full = crate::volumes::nearly_full(space);
            let mut trailing: Vec<Element> = Vec::new();
            if let Some(used) = full {
                trailing.push(
                    rect()
                        .width(Size::px(t.size_capacity_mark))
                        .height(Size::px(4.))
                        .corner_radius(2.)
                        .background(colour(t.colour_fill_strong))
                        .a11y_alt(format!("{} available", crate::format::size(space.map_or(0, |s| s.0))))
                        .child(rect().width(Size::percent(used * 100.)).height(Size::fill()).corner_radius(2.).background(colour(t.colour_warn)))
                        .into_element(),
                );
            }
            if self.mounting.contains(&v.path) {
                trailing.push(isle_ui::Spinner::new(16., format!("Mounting “{}”", v.label)).into_element());
            } else if mounted.is_some() && (v.removable || v.loop_device) {
                trailing.push(
                    rect()
                        .width(Size::px(t.size_control_small))
                        .height(Size::px(t.size_control_small))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt(format!("Eject “{}”", v.label))
                        .on_press(move |e: Event<PressEventData>| {
                            e.stop_propagation();
                            on_eject.call(ejecting.clone());
                        })
                        .child(Glyph::new("eject", t.size_glyph, colour(if shown { t.colour_on_accent } else { t.colour_text_2 })))
                        .into_element(),
                );
            }
            let trailing = (!trailing.is_empty()).then(|| rect().horizontal().cross_align(Alignment::Center).spacing(t.space_s).children(trailing).into_element());
            let free = free_space(space);
            let el = row(
                v.path.clone(),
                glyph,
                v.label.clone(),
                shown,
                mounted.is_none(),
                mounted.clone(),
                Box::new(move || on_volume.call(pressed.clone())),
                trailing,
                crate::menus::Side::Volume(v.clone()),
                free.clone(),
            );
            with_tooltip(el, free)
        });
        let share_rows = self.shares.iter().map(|p| {
            let on_go = self.on_go.clone();
            let place = Place::Folder(p.clone());
            let side = crate::menus::Side::Place(place.clone(), crate::volumes::share_name(p));
            let free = free_space(self.space.get(p));
            let shown = place == self.shown;
            let (on_unmount, share) = (self.on_unmount.clone(), p.clone());
            let trailing = Some(
                rect()
                    .width(Size::px(t.size_control_small))
                    .height(Size::px(t.size_control_small))
                    .center()
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(format!("Eject “{}”", crate::volumes::share_name(p)))
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        on_unmount.call(share.clone());
                    })
                    .child(Glyph::new("eject", t.size_glyph, colour(if shown { t.colour_on_accent } else { t.colour_text_2 })))
                    .into_element(),
            );
            let el = row(p.display().to_string(), "network", crate::volumes::share_name(p), place == self.shown, false, Some(p.clone()), Box::new(move || on_go.call(place.clone())), trailing, side, free.clone());
            with_tooltip(el, free)
        });
        let (computer, trash) = (self.places.locations.first(), self.places.locations.get(1));
        let on_reorder = self.on_reorder.clone();
        rect()
            .width(Size::px(self.width))
            .height(Size::fill())
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                let Some((from, y0, to)) = *reorder.peek() else { return };
                let y = e.global_location().y;
                if to.is_none() && (y - y0).abs() <= 4. {
                    return;
                }
                // The gap: above the row under the pointer, or below it in its lower half.
                let areas = areas.peek();
                let gap = areas
                    .iter()
                    .find(|(_, (top, bottom))| y >= *top as f64 && y < *bottom as f64)
                    .map(|(at, (top, bottom))| if y < (*top as f64 + *bottom as f64) / 2. { *at } else { at + 1 })
                    .or(to)
                    .unwrap_or(from);
                reorder.set_if_modified(Some((from, y0, Some(gap))));
            })
            .on_global_pointer_up(move |_: Event<PointerEventData>| {
                let Some((from, _, to)) = *reorder.peek() else { return };
                reorder.set(None);
                if let (Some(to), Some(on_reorder)) = (to, on_reorder.as_ref())
                    && to != from
                    && to != from + 1
                {
                    on_reorder.call((from, to));
                }
            })
            .background(colour(t.colour_sidebar))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 1., bottom: 0., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .maybe_child(self.under_toolbar.then(|| {
                let on_hide = self.on_hide.clone();
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_toolbar))
                    .padding((0., t.space_s))
                    .horizontal()
                    .main_align(Alignment::End)
                    .cross_align(Alignment::Center)
                    .toolbar()
                    .maybe_child(on_hide.map(|on_hide| {
                        crate::toolbar::icon_button(&theme, "sidebar-simple", "Hide Sidebar", true, false, move || on_hide.call(()))
                    }))
            }))
            .child(
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((t.space_s, t.space_s))
                            .a11y_role(AccessibilityRole::List)
                            .maybe_child(self.prefs.shows_row("recents").then(|| place_row(&recents)))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Favourites"))
                            .children({
                                let shown: Vec<&Row> = self
                                    .places
                                    .favourites
                                    .iter()
                                    .filter(|r| open("Favourites") && self.prefs.shows_row(&r.label.to_lowercase()))
                                    .collect();
                                let count = shown.len();
                                shown
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, r)| {
                                        // The index in all Favourites, hidden ones included.
                                        let at = self.places.favourites.iter().position(|f| f.place == r.place).unwrap_or(i);
                                        let end = self.places.favourites.len();
                                        favourite(&theme, place_row(r), at, i + 1 == count, end, dragging, self.bookmark_at, drop_target, can_reorder.then_some(reorder), areas)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .children(self.saved.iter().filter(|_| open("Favourites")).map(|s| {
                                place_row(&Row {
                                    place: Place::Search(Box::new(s.clone())),
                                    label: s.name.clone().unwrap_or_default(),
                                    glyph: "list-magnifying-glass",
                                })
                            }))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Locations"))
                            .children(computer.filter(|_| open("Locations") && self.prefs.shows_row("computer")).map(place_row))
                            .children(volume_rows.filter(|_| open("Locations") && self.prefs.shows_row("volumes")))
                            .children(share_rows.filter(|_| open("Locations") && self.prefs.shows_row("volumes")))
                            .children(trash.filter(|_| open("Locations") && self.prefs.shows_row("trash")).map(place_row)),
                    ),
            )
    }
}

/// A Favourite's row with the gaps around it: a drag at its top edge, or the last one's foot,
/// would add a bookmark there, shown by an insertion line.
#[allow(clippy::too_many_arguments)]
fn favourite(
    theme: &isle_tokens::Theme,
    row: Element,
    at: usize,
    last: bool,
    end: usize,
    dragging: bool,
    bookmark_at: State<Option<usize>>,
    drop_target: State<Option<PathBuf>>,
    reorder: Option<State<Option<(usize, f64, Option<usize>)>>>,
    areas: State<std::collections::BTreeMap<usize, (f32, f32)>>,
) -> Element {
    let t = &theme.tokens;
    let (edge, height) = (6.0f64, t.size_row as f64);
    // The insertion line: 2 points of accent over the gap, taking no room.
    let line = |top: bool| {
        rect()
            .width(Size::fill())
            .height(Size::px(2.))
            .margin(if top { (-1., 0., -1., 0.) } else { (-1., 0., -1., 0.) })
            .layer(Layer::Overlay)
            .corner_radius(1.)
            .background(colour(t.colour_accent))
            .into_element()
    };
    // Moving a Favourite: the gap is above this row, or below it in its lower half.
    let moving = reorder.and_then(|r| r.read().and_then(|(_, _, to)| to));
    let shown = if moving.is_some() { moving } else { *bookmark_at.read() };
    let el = rect()
        .width(Size::fill())
        .on_mouse_down(move |e: Event<MouseEventData>| {
            if let (Some(mut reorder), Some(MouseButton::Left)) = (reorder, e.button) {
                reorder.set(Some((at, e.global_location.y, None)));
            }
        })
        .on_sized(move |e: Event<SizedEventData>| {
            let span = (e.area.min_y(), e.area.max_y());
            if areas.peek().get(&at) != Some(&span) {
                let mut areas = areas;
                areas.write().insert(at, span);
            }
        })
        .on_pointer_move(move |e: Event<PointerEventData>| {
            if !dragging {
                return;
            }
            let y = e.element_location().y;
            let gap = if y < edge {
                Some(at)
            } else if last && y > height - edge {
                Some(end)
            } else {
                None
            };
            let mut bookmark_at = bookmark_at;
            bookmark_at.set_if_modified(gap);
            if gap.is_some() {
                let mut drop_target = drop_target;
                drop_target.set_if_modified(None);
            }
        })
        .on_pointer_leave(move |_| {
            if bookmark_at.peek().is_some_and(|g| g == at || g == end) {
                let mut bookmark_at = bookmark_at;
                bookmark_at.set(None);
            }
        });
    let showing = dragging || moving.is_some();
    let below = shown == Some(end) || (moving.is_some() && shown == Some(at + 1));
    el.maybe_child((showing && shown == Some(at)).then(|| line(true)))
        .child(row)
        .maybe_child((showing && last && below).then(|| line(false)))
        .into_element()
}

/// "41 GB available of 512 GB", a volume's tooltip and description.
fn free_space(space: Option<&(u64, u64)>) -> Option<String> {
    let (free, total) = *space?;
    Some(format!("{} available of {}", crate::format::size(free), crate::format::size(total)))
}

/// `row` with `text` shown under the pointer after a dwell.
fn with_tooltip(row: Element, text: Option<String>) -> Element {
    match text {
        Some(text) => TooltipContainer::new(Tooltip::new_text(text)).width(Size::fill()).child(row).into_element(),
        None => row,
    }
}
