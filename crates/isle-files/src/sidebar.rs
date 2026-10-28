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
    pub dragging: bool,
    pub drop_target: State<Option<PathBuf>>,
    pub on_go: EventHandler<Place>,
    pub on_volume: EventHandler<isle_disks::Volume>,
    pub on_eject: EventHandler<isle_disks::Volume>,
    /// A secondary click on a row, at a point in the window.
    pub on_menu: EventHandler<(crate::menus::Side, (f32, f32))>,
    pub width: f32,
    /// Leaves the window's toolbar band clear at its top.
    pub under_toolbar: bool,
}

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dragging = self.dragging;
        let drop_target = self.drop_target;
        // A row: its glyph and label, selected while its place is shown, and under a drag the
        // folder (or the Trash) a drop goes into.
        let row = |key: String, glyph: &'static str, label: String, selected: bool, dimmed: bool, target: Option<PathBuf>, on_press: Box<dyn Fn()>, trailing: Option<Element>, side: crate::menus::Side| {
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
                .a11y_builder(move |node| node.set_selected(selected))
                .on_pointer_enter(move |_| {
                    if dragging && enter.is_some() {
                        let mut drop_target = drop_target;
                        drop_target.set(enter.clone());
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
            row(format!("{:?}", r.place), r.glyph, r.label.clone(), r.place == self.shown, false, target, Box::new(move || on_go.call(place.clone())), None, side)
        };
        let header = |title: &str| {
            rect()
                .width(Size::fill())
                .height(Size::px(24.))
                .padding((0., t.space_s))
                .main_align(Alignment::End)
                .child(text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), title.to_owned()).font_weight(600))
                .into_element()
        };
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
            if mounted.is_some() && (v.removable || v.loop_device) {
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
            row(
                v.path.clone(),
                glyph,
                v.label.clone(),
                shown,
                mounted.is_none(),
                mounted.clone(),
                Box::new(move || on_volume.call(pressed.clone())),
                trailing,
                crate::menus::Side::Volume(v.clone()),
            )
        });
        let share_rows = self.shares.iter().map(|p| {
            let on_go = self.on_go.clone();
            let place = Place::Folder(p.clone());
            let side = crate::menus::Side::Place(place.clone(), crate::volumes::share_name(p));
            row(p.display().to_string(), "network", crate::volumes::share_name(p), place == self.shown, false, Some(p.clone()), Box::new(move || on_go.call(place.clone())), None, side)
        });
        let (computer, trash) = (self.places.locations.first(), self.places.locations.get(1));
        rect()
            .width(Size::px(self.width))
            .height(Size::fill())
            .background(colour(t.colour_sidebar))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 1., bottom: 0., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .maybe_child(self.under_toolbar.then(|| rect().width(Size::fill()).height(Size::px(t.size_toolbar)).toolbar()))
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
                            .children(
                                self.places
                                    .favourites
                                    .iter()
                                    .filter(|r| self.prefs.shows_row(&r.label.to_lowercase()))
                                    .map(place_row),
                            )
                            .children(self.saved.iter().map(|s| {
                                place_row(&Row {
                                    place: Place::Search(Box::new(s.clone())),
                                    label: s.name.clone().unwrap_or_default(),
                                    glyph: "list-magnifying-glass",
                                })
                            }))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Locations"))
                            .children(computer.filter(|_| self.prefs.shows_row("computer")).map(place_row))
                            .children(volume_rows.filter(|_| self.prefs.shows_row("volumes")))
                            .children(share_rows.filter(|_| self.prefs.shows_row("volumes")))
                            .children(trash.filter(|_| self.prefs.shows_row("trash")).map(place_row)),
                    ),
            )
    }
}
