//! The gallery's sidebar: search, its three sections, items with live pictures, and the
//! Details panel in its place (`p1/surfaces/widgets.md` The gallery).

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, colour, focus_ring, shadows, text};

use super::{
    grid::Extent,
    surface::{ConsentView, DashboardAction, DetailsView, Frame, GalleryView, ItemView, SourcesView, send, widget},
};
use crate::surface::Actions;

const SECTIONS: [&str; 3] = ["Installed", "Yours", "Browse"];

pub(super) fn view(theme: &Theme, g: &GalleryView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let inner = t.size_panel - 2.0 * t.space_l;
    let body: Element = match (&g.details, &g.sources) {
        (Some(d), _) => details(theme, d, actions),
        (None, Some(s)) => sources(theme, s, actions),
        _ => list(theme, g, inner, actions),
    };
    rect()
        .position(Position::new_absolute().left(t.space_l).top(t.size_strip + t.space_l).bottom(t.space_l))
        .width(Size::px(t.size_panel))
        .height(Size::fill())
        .corner_radius(t.radius_panel)
        .background(colour(t.material_solid))
        .map(Some(shadows(&t.shadow_panel)), |el, list| list.into_iter().fold(el, |el, s| el.shadow(s)))
        .padding(t.space_l)
        .spacing(t.space_s)
        .opacity(if g.faded { 0.0 } else { 1.0 })
        .layer(freya::prelude::Layer::Overlay)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt("Widget gallery")
        .child(body)
        .into_element()
}

fn list(theme: &Theme, g: &GalleryView, inner: f32, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let search = rect()
        .width(Size::fill())
        .height(Size::px(t.size_control))
        .corner_radius(t.radius_control)
        .background(colour(t.colour_fill))
        .padding((0., t.space_s))
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_xs)
        .maybe(g.focused, |el| el.border(focus_ring(theme)))
        .a11y_role(AccessibilityRole::SearchInput)
        .a11y_alt("Search widgets")
        .child(Glyph::new("magnifying-glass", 12., ink_2))
        .child(if g.query.is_empty() {
            text::line(theme, text::Style::Body, ink_2, "Search widgets")
        } else {
            text::line(theme, text::Style::Body, ink, g.query.clone())
        });
    let segments = rect()
        .width(Size::fill())
        .height(Size::px(t.size_control))
        .corner_radius(t.radius_control)
        .background(colour(t.colour_fill))
        .padding(2.)
        .horizontal()
        .content(Content::flex())
        .children(SECTIONS.iter().enumerate().map(|(i, name)| {
            let actions = actions.clone();
            let chosen = g.section == i;
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .corner_radius(t.radius_control - 2.)
                .maybe(chosen, |el| {
                    el.background(colour(t.material_solid))
                        .map(Some(shadows(&t.shadow_control)), |el, list| list.into_iter().fold(el, |el, s| el.shadow(s)))
                })
                .center()
                .horizontal()
                .spacing(t.space_xs)
                .a11y_role(AccessibilityRole::Tab)
                .a11y_alt(*name)
                .on_press(move |_| send(&actions, DashboardAction::GallerySection(i)))
                .child(text::line(theme, text::Style::Body, ink, *name))
                .maybe_child((i == 0 && g.updates > 0).then(|| {
                    rect()
                        .padding((0., 5.))
                        .height(Size::px(16.))
                        .corner_radius(8.)
                        .background(colour(t.colour_fill_strong))
                        .center()
                        .child(text::line(theme, text::Style::Footnote, ink, g.updates.to_string()))
                        .into_element()
                }))
                .into_element()
        }));
    let top = g.top.clone().map(|(label, action)| {
        let actions = actions.clone();
        Button::new(label)
            .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: String::new(), action: action.clone() }))
            .into_element()
    });
    let mut content = rect().width(Size::fill()).spacing(t.space_m);
    if let Some(note) = &g.note {
        content = content.child(text::line(theme, text::Style::Callout, ink_2, note.clone()));
    }
    for group in &g.groups {
        content = content.child(text::line(theme, text::Style::Subheadline, ink_2, group.heading.clone()).font_weight(600));
        // Two-column pictures pair across the width; wider ones take a row.
        let mut row: Vec<Element> = Vec::new();
        let mut rows: Vec<Element> = Vec::new();
        for item in &group.items {
            let narrow = item.picture.at.w <= 2;
            let el = item_view(theme, item, inner, actions);
            if narrow {
                row.push(el);
                if row.len() == 2 {
                    rows.push(rect().horizontal().spacing(t.space_l).children(std::mem::take(&mut row)).into_element());
                }
            } else {
                if !row.is_empty() {
                    rows.push(rect().horizontal().spacing(t.space_l).children(std::mem::take(&mut row)).into_element());
                }
                rows.push(el);
            }
        }
        if !row.is_empty() {
            rows.push(rect().horizontal().spacing(t.space_l).children(row).into_element());
        }
        content = content.child(rect().spacing(t.space_m).children(rows));
    }
    rect()
        .expanded()
        .spacing(t.space_s)
        .child(search)
        .child(segments)
        .maybe_child(top)
        .child(isle_ui::scroll_area(theme).width(Size::fill()).height(Size::fill()).child(content))
        .into_element()
}

fn item_view(theme: &Theme, item: &ItemView, inner: f32, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let p = item.picture.at;
    let (pw, ph) = (p.w as f32 * 88.0 - 16.0, p.h as f32 * 88.0 - 16.0);
    let width = pw.min(inner);
    // The picture is the widget itself, drawn at its default size on the gallery's fill.
    let picture_frame = Frame {
        extent: Extent { cols: p.w, rows: p.h, x: 0.0, y: 0.0, cell: 72.0, gap: 16.0 },
        widgets: vec![item.picture.clone()],
        edit: false,
        selected: None,
        ring: false,
        solid: true,
        lifted: None,
        ghost: None,
        menu: None,
        form: None,
        gallery: None,
        consent: None,
        plates_behind: true,
    };
    let picture = rect()
        .width(Size::px(width))
        .height(Size::px(ph))
        .corner_radius(t.radius_panel)
        .overflow(Overflow::Clip)
        .background(colour(t.colour_fill))
        .child(widget(theme, &picture_frame, &item.picture, None, actions));
    let press = {
        let (actions, key) = (actions.clone(), item.key.clone());
        move |e: Event<MouseEventData>| match e.button {
            Some(MouseButton::Left) => send(&actions, DashboardAction::GalleryPress(key.clone())),
            Some(MouseButton::Right) => send(&actions, DashboardAction::GallerySecondary(key.clone())),
            _ => {}
        }
    };
    let line = if item.added {
        rect()
            .horizontal()
            .spacing(t.space_xs)
            .cross_align(Alignment::Center)
            .child(Glyph::new("check", 12., ink_2))
            .child(text::line(theme, text::Style::Subheadline, ink_2, "Added"))
            .into_element()
    } else if item.no_room {
        label()
            .width(Size::px(width))
            .font_size(t.type_style_callout_size)
            .color(ink_2)
            .text(item.line.clone())
            .into_element()
    } else {
        text::line(theme, text::Style::Subheadline, ink_2, item.line.clone()).into_element()
    };
    rect()
        .width(Size::px(width))
        .spacing(t.space_xs)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(item.name.clone())
        .maybe(item.selected, |el| el.border(focus_ring(theme)))
        .on_mouse_down(press)
        .child(picture)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(text::line(theme, text::Style::Body, ink, item.name.clone()))
                        .child(line)
                        .maybe_child((!item.tools.is_empty()).then(|| {
                            rect()
                                .horizontal()
                                .spacing(t.space_xs)
                                .children(item.tools.iter().map(|(label, action)| {
                                    let (actions, key, action) = (actions.clone(), item.key.clone(), action.clone());
                                    Button::new(label.clone())
                                        .small()
                                        .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: key.clone(), action: action.clone() }))
                                        .into_element()
                                }))
                                .into_element()
                        }))
                        .maybe_child((!item.permissions.is_empty()).then(|| {
                            rect()
                                .horizontal()
                                .spacing(t.space_xs)
                                .cross_align(Alignment::Center)
                                .children(item.permissions.iter().enumerate().map(|(i, (glyph, words))| {
                                    rect()
                                        .horizontal()
                                        .spacing(t.space_xs)
                                        .child(Glyph::new(glyph.clone(), 12., ink_2))
                                        .maybe_child((i == 0).then(|| text::line(theme, text::Style::Subheadline, ink_2, words.clone()).into_element()))
                                        .into_element()
                                }))
                                .into_element()
                        })),
                )
                .maybe_child(item.button.clone().map(|(label, action)| {
                    let (actions, key) = (actions.clone(), item.key.clone());
                    Button::new(label)
                        .small()
                        .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: key.clone(), action: action.clone() }))
                        .into_element()
                })),
        )
        .into_element()
}

fn details(theme: &Theme, d: &DetailsView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let back = {
        let actions = actions.clone();
        Button::icon("caret-left", "Back").on_press(move |_| send(&actions, DashboardAction::GalleryDetails(None)))
    };
    let buttons = rect().horizontal().spacing(t.space_s).children(d.buttons.iter().map(|(label, action)| {
        let (actions, key, action) = (actions.clone(), d.key.clone(), action.clone());
        Button::new(label.clone())
            .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: key.clone(), action: action.clone() }))
            .into_element()
    }));
    let permissions = rect().spacing(t.space_xs).children(d.permissions.iter().map(|(glyph, words, allowed)| {
        let actions = actions.clone();
        let key = d.key.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(Glyph::new(glyph.clone(), 16., ink_2))
            .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Body, ink, words.clone())))
            .child(text::line(theme, text::Style::Body, ink_2, if *allowed { "Allowed" } else { "Not allowed" }))
            .child(
                Button::new("Change…")
                    .kind(isle_ui::button::Kind::Plain)
                    .small()
                    .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: key.clone(), action: "permissions".into() })),
            )
            .into_element()
    }));
    let content = rect()
        .width(Size::fill())
        .spacing(t.space_s)
        .child(text::line(theme, text::Style::Title3, ink, d.name.clone()))
        .child(label().width(Size::fill()).font_size(t.type_style_body_size).color(ink).text(d.description.clone()))
        .child(text::line(theme, text::Style::Subheadline, ink_2, d.byline.clone()))
        .maybe_child(d.source.clone().map(|s| text::line(theme, text::Style::Subheadline, ink_2, s).into_element()))
        .child(buttons)
        .child(text::line(theme, text::Style::Subheadline, ink_2, d.sizes.clone()))
        .maybe_child(d.command.clone().map(|c| {
            rect()
                .width(Size::fill())
                .spacing(t.space_xs)
                .child(text::line(theme, text::Style::Subheadline, ink_2, "What it runs").font_weight(600))
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(t.space_s)
                        .corner_radius(t.radius_small)
                        .background(colour(t.colour_fill))
                        .child(label().font_family(t.type_mono.clone()).font_size(t.type_style_callout_size).color(ink_2).text(c)),
                )
                .into_element()
        }))
        .maybe_child((!d.permissions.is_empty()).then(|| permissions.into_element()))
        .maybe_child(d.readme.clone().map(|r| {
            label().width(Size::fill()).font_size(t.type_style_callout_size).color(ink).text(r).into_element()
        }));
    rect()
        .expanded()
        .spacing(t.space_s)
        .child(back)
        .child(isle_ui::scroll_area(theme).width(Size::fill()).height(Size::fill()).child(content))
        .into_element()
}

fn button(actions: &Actions, label: &str, key: &str, action: &str) -> Button {
    let (actions, key, action) = (actions.clone(), key.to_owned(), action.to_owned());
    Button::new(label.to_owned())
        .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: key.clone(), action: action.clone() }))
}

fn sources(theme: &Theme, s: &SourcesView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let back = {
        let actions = actions.clone();
        Button::icon("caret-left", "Back")
            .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: String::new(), action: "sources-back".into() }))
    };
    let rows = s.rows.iter().map(|row| {
        let (actions, url, allowed) = (actions.clone(), row.url.clone(), row.allowed);
        rect()
            .width(Size::fill())
            .padding(t.space_s)
            .spacing(t.space_xs)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_fill))
            .child(text::line(theme, text::Style::Body, ink, isle_ui::text::cut_middle(&row.url, 44)))
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_xs)
                    .cross_align(Alignment::Center)
                    .maybe_child(row.error.then(|| Glyph::new("warning-circle", 12., ink).into_element()))
                    .child(text::line(theme, text::Style::Subheadline, if row.error { ink } else { ink_2 }, row.line.clone())),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Callout, ink, "Allow widgets from this source to run")))
                    .child(isle_ui::Switch::new(allowed, "Allow widgets from this source to run", {
                        let (actions, url) = (actions.clone(), url.clone());
                        move |on: bool| {
                            send(&actions, DashboardAction::GalleryButton {
                                key: url.clone(),
                                action: if on { "allow-on" } else { "allow-off" }.into(),
                            })
                        }
                    })),
            )
            .child(
                Button::new("Remove")
                    .small()
                    .on_press(move |_| send(&actions, DashboardAction::GalleryButton { key: url.clone(), action: "remove-source".into() })),
            )
            .into_element()
    });
    rect()
        .expanded()
        .spacing(t.space_s)
        .child(back)
        .child(text::line(theme, text::Style::Title3, ink, "Sources"))
        .children(rows)
        .child(if s.adding {
            AddSource {}.into_element()
        } else {
            button(actions, "+", "", "add").into_element()
        })
        .into_element()
}

/// The field a new source's address is typed into; Return adds it.
#[derive(Clone, PartialEq)]
struct AddSource {}

impl Component for AddSource {
    fn render(&self) -> impl IntoElement {
        let value = use_state(String::new);
        let actions = Actions::get();
        isle_ui::TextField::new(value, "Address")
            .editing()
            .on_submit(move |url: String| {
                send(&actions, DashboardAction::GalleryButton { key: url, action: "add-source".into() })
            })
    }
}

pub(super) fn consent(theme: &Theme, c: &ConsentView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let (cancel, confirm) = (actions.clone(), actions.clone());
    let panel = rect()
        .width(Size::px(t.size_panel))
        .padding(t.space_xl)
        .spacing(t.space_s)
        .corner_radius(t.radius_panel)
        .background(colour(t.material_solid))
        .map(Some(shadows(&t.shadow_panel)), |el, list| list.into_iter().fold(el, |el, s| el.shadow(s)))
        .cross_align(Alignment::Center)
        .a11y_role(AccessibilityRole::AlertDialog)
        .a11y_alt(c.title.clone())
        .child(Glyph::new("plug", 48., ink))
        .child(text::line(theme, text::Style::Title3, ink, c.title.clone()))
        .maybe_child(c.command.clone().map(|cmd| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .child(text::line(theme, text::Style::Callout, ink_2, "It runs "))
                .child(label().font_family(t.type_mono.clone()).font_size(t.type_style_callout_size).color(ink).text(cmd))
                .child(text::line(theme, text::Style::Callout, ink_2, "."))
                .into_element()
        }))
        .maybe_child((!c.permissions.is_empty()).then(|| {
            rect()
                .width(Size::fill())
                .spacing(t.space_xs)
                .child(text::line(theme, text::Style::Subheadline, ink_2, c.ask.clone()).font_weight(600))
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(t.space_s)
                        .corner_radius(t.radius_small)
                        .background(colour(t.colour_fill))
                        .children(c.permissions.iter().map(|(glyph, words)| {
                            rect()
                                .height(Size::px(t.size_row))
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .spacing(t.space_s)
                                .child(Glyph::new(glyph.clone(), 16., ink_2))
                                .child(text::line(theme, text::Style::Body, ink, words.clone()))
                                .into_element()
                        })),
                )
                .into_element()
        }))
        .maybe_child(c.source.clone().map(|s| text::line(theme, text::Style::Subheadline, ink_2, s).into_element()))
        .maybe_child(c.also.clone().map(|a| {
            label().width(Size::fill()).font_size(t.type_style_callout_size).color(ink).text(a).into_element()
        }))
        .child(
            rect()
                .horizontal()
                .spacing(t.space_s)
                .child(Button::new("Cancel").on_press(move |_| send(&cancel, DashboardAction::ConsentCancel)))
                .child(
                    Button::new(c.confirm.clone())
                        .kind(if c.armed { isle_ui::button::Kind::Default } else { isle_ui::button::Kind::Regular })
                        .enabled(c.armed)
                        .on_press(move |_| send(&confirm, DashboardAction::ConsentConfirm)),
                ),
        );
    rect()
        .position(Position::new_absolute().left(0.).top(0.))
        .expanded()
        .background(colour(t.colour_scrim))
        .layer(freya::prelude::Layer::Overlay)
        .padding((0., 0., 0., 0.))
        .cross_align(Alignment::Center)
        .child(rect().height(Size::percent(20.)))
        .child(panel)
        .into_element()
}
