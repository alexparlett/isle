//! Notification Centre: the clock's panel, with the date and the month, then the
//! notifications stacked by app and Earlier's records (`p1/structure/notification-centre.md`,
//! `p1/surfaces/notification-centre.md`).

use std::path::PathBuf;

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Button, Glyph, button::Kind, colour, focus_ring, text, use_theme};

use super::{IslandAction, panel::PanelAction};
use crate::{Action, Actions};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Day {
    pub number: u8,
    pub this_month: bool,
    pub today: bool,
}

/// A fixed six weeks, the week starting on Monday.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Month {
    pub title: String,
    pub weekdays: Vec<String>,
    pub days: Vec<Day>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CentreCard {
    pub id: u32,
    pub summary: String,
    pub body: String,
    pub time: String,
    pub image: Option<PathBuf>,
    pub live: bool,
}

/// One app's notifications, newest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Group {
    pub app: String,
    pub app_name: String,
    pub icon: Option<PathBuf>,
    pub cards: Vec<CentreCard>,
    pub open: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CentreFrame {
    pub date: String,
    pub month: Month,
    pub groups: Vec<Group>,
    /// Records of what was missed.
    pub earlier: Vec<Group>,
    /// Clear All asked once: the next press clears.
    pub clear_armed: bool,
}

const CARD: f32 = 72.0;

fn group_height(theme: &Theme, g: &Group) -> f32 {
    let t = &theme.tokens;
    let layers = (g.cards.len().saturating_sub(1)).min(t.limit_stack_layers as usize) as f32;
    t.size_row
        + if g.open {
            g.cards.len() as f32 * (CARD + t.space_s)
        } else {
            CARD + layers * t.size_stack_layer
        }
}

/// The centre's whole height, scrolled or not.
pub fn centre_height(theme: &Theme, c: &CentreFrame) -> f32 {
    let t = &theme.tokens;
    let month = t.size_row + t.size_row + 6.0 * t.size_month_row;
    let groups: f32 = c
        .groups
        .iter()
        .map(|g| group_height(theme, g) + t.space_s)
        .sum();
    let earlier: f32 = if c.earlier.is_empty() {
        0.0
    } else {
        t.size_row
            + c.earlier
                .iter()
                .map(|g| group_height(theme, g) + t.space_s)
                .sum::<f32>()
    };
    let empty = if c.groups.is_empty() && c.earlier.is_empty() {
        t.size_row
    } else {
        0.0
    };
    t.size_row + month + t.space_s + t.size_row + groups + earlier + empty + t.size_row
}

fn send(actions: &Actions, row: String) {
    actions.send(Action::Island(IslandAction::Panel(PanelAction::Choose {
        module: "centre".into(),
        row,
    })));
}

#[derive(PartialEq)]
pub struct CentreBody {
    pub centre: CentreFrame,
}

impl Component for CentreBody {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let c = &self.centre;
        let clear = actions.clone();
        let month = rect()
            .width(Size::fill())
            .corner_radius(t.radius_card)
            .background(colour(t.colour_fill))
            .padding(t.space_s)
            .child(
                rect()
                    .height(Size::px(t.size_row))
                    .main_align(Alignment::Center)
                    .child(text::line(
                        &theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        c.month.title.clone(),
                    )),
            )
            .child(rect().horizontal().height(Size::px(t.size_row)).children(
                c.month.weekdays.iter().map(|w| {
                    rect()
                        .width(Size::percent(100.0 / 7.0))
                        .center()
                        .child(text::line(
                            &theme,
                            text::Style::Footnote,
                            colour(t.colour_text_2),
                            w.clone(),
                        ))
                        .into_element()
                }),
            ))
            .children(c.month.days.chunks(7).map(|week| {
                rect()
                    .horizontal()
                    .height(Size::px(t.size_month_row))
                    .children(week.iter().map(|d| {
                        let (fill, ink) = if d.today {
                            (colour(t.colour_accent), colour(t.colour_on_accent))
                        } else if d.this_month {
                            (Color::TRANSPARENT, colour(t.colour_text))
                        } else {
                            (Color::TRANSPARENT, colour(t.colour_text_disabled))
                        };
                        rect()
                            .width(Size::percent(100.0 / 7.0))
                            .center()
                            .child(
                                rect()
                                    .width(Size::px(t.size_month_row - 4.0))
                                    .height(Size::px(t.size_month_row - 4.0))
                                    .corner_radius((t.size_month_row - 4.0) / 2.0)
                                    .background(fill)
                                    .center()
                                    .child(text::line(
                                        &theme,
                                        text::Style::Callout,
                                        ink,
                                        d.number.to_string(),
                                    )),
                            )
                            .into_element()
                    }))
                    .into_element()
            }));
        let has_any = !c.groups.is_empty() || !c.earlier.is_empty();
        rect()
            .width(Size::fill())
            .spacing(t.space_s)
            .child(
                rect()
                    .height(Size::px(t.size_row))
                    .padding((0., t.space_s))
                    .main_align(Alignment::Center)
                    .child(text::line(
                        &theme,
                        text::Style::Title,
                        colour(t.colour_text),
                        c.date.clone(),
                    )),
            )
            .child(month)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .padding((0., t.space_s))
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        &theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        "Notifications",
                    )))
                    .maybe(has_any, |el| {
                        el.child(
                            Button::new(if c.clear_armed { "Clear" } else { "Clear All" })
                                .kind(if c.clear_armed {
                                    Kind::Default
                                } else {
                                    Kind::Plain
                                })
                                .small()
                                .on_press(move |_| send(&clear, "clear-all".into())),
                        )
                    }),
            )
            .maybe(!has_any, |el| {
                el.child(
                    rect()
                        .height(Size::px(t.size_row))
                        .padding((0., t.space_s))
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text_2),
                            "No notifications",
                        )),
                )
            })
            .children(
                c.groups
                    .iter()
                    .map(|g| GroupView { group: g.clone() }.into_element()),
            )
            .maybe(!c.earlier.is_empty(), |el| {
                el.child(
                    rect()
                        .height(Size::px(t.size_row))
                        .padding((0., t.space_s))
                        .main_align(Alignment::Center)
                        .child(text::line(
                            &theme,
                            text::Style::Headline,
                            colour(t.colour_text_2),
                            "Earlier",
                        )),
                )
                .children(
                    c.earlier
                        .iter()
                        .map(|g| GroupView { group: g.clone() }.into_element()),
                )
            })
    }
}

#[derive(PartialEq)]
struct GroupView {
    group: Group,
}

impl Component for GroupView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let g = &self.group;
        let expand = actions.clone();
        let clear = actions.clone();
        let app = g.app.clone();
        let app2 = g.app.clone();
        let shown: Vec<CentreCard> = if g.open {
            g.cards.clone()
        } else {
            g.cards.iter().take(1).cloned().collect()
        };
        let layers = if g.open {
            0
        } else {
            g.cards
                .len()
                .saturating_sub(1)
                .min(t.limit_stack_layers as usize)
        };
        rect()
            .width(Size::fill())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .padding((0., t.space_s))
                    .spacing(t.space_s)
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        &theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        g.app_name.clone(),
                    )))
                    .maybe(g.cards.len() > 1, |el| {
                        el.child(
                            Button::new(if g.open { "Show Less" } else { "Show All" })
                                .kind(Kind::Plain)
                                .small()
                                .on_press(move |_| send(&expand, format!("expand:{app}"))),
                        )
                    })
                    .child(
                        Button::icon("x", format!("Clear {}", g.app_name))
                            .kind(Kind::Plain)
                            .small()
                            .on_press(move |_| send(&clear, format!("clear-app:{app2}"))),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(t.space_s)
                    .children(shown.into_iter().map(|card| {
                        CardView {
                            card,
                            icon: g.icon.clone(),
                        }
                        .into_element()
                    }))
                    .children((0..layers).map(|i| {
                        let inset = t.space_s * (i + 1) as f32;
                        rect()
                            .width(Size::fill())
                            .height(Size::px(t.size_stack_layer))
                            .padding((0., inset))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::fill())
                                    .corner_radius((0., 0., t.radius_card, t.radius_card))
                                    .background(colour(t.colour_fill)),
                            )
                            .into_element()
                    })),
            )
    }
}

#[derive(PartialEq)]
struct CardView {
    card: CentreCard,
    icon: Option<PathBuf>,
}

impl Component for CardView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let c = &self.card;
        let open = actions.clone();
        let close = actions.clone();
        let (id, id2) = (c.id, c.id);
        rect()
            .width(Size::fill())
            .height(Size::px(CARD))
            .corner_radius(t.radius_card)
            .background(colour(if *hover.read() {
                t.colour_fill_strong
            } else {
                t.colour_fill
            }))
            .a11y_id(a11y)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(format!("{}. {}", c.summary, c.body))
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&open, format!("open:{id}"))
            })
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if matches!(
                    e.key,
                    Key::Named(NamedKey::Backspace) | Key::Named(NamedKey::Delete)
                ) {
                    send(&close, format!("close:{id2}"));
                }
            })
            .padding(t.space_m)
            .horizontal()
            .content(Content::flex())
            .spacing(t.space_m)
            .child(
                rect()
                    .width(Size::px(t.size_app_icon))
                    .height(Size::px(t.size_app_icon))
                    .center()
                    .map(self.icon.clone(), |el, icon| {
                        el.child(AppIcon::new(icon, t.size_app_icon))
                    })
                    .maybe(self.icon.is_none(), |el| {
                        el.corner_radius(t.size_app_icon / 2.0)
                            .background(colour(t.colour_fill_strong))
                            .child(Glyph::new(
                                "bell",
                                t.size_glyph_large,
                                colour(t.colour_text),
                            ))
                    }),
            )
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(
                        rect()
                            .horizontal()
                            .content(Content::flex())
                            .width(Size::fill())
                            .child(rect().width(Size::flex(1.)).child(text::line(
                                &theme,
                                text::Style::Headline,
                                colour(t.colour_text),
                                c.summary.clone(),
                            )))
                            .child(text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(t.colour_text_2),
                                c.time.clone(),
                            )),
                    )
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        c.body.clone(),
                    )),
            )
            .map(c.image.clone(), |el, image| {
                el.child(
                    rect()
                        .width(Size::px(t.size_banner_thumbnail))
                        .height(Size::px(t.size_banner_thumbnail))
                        .corner_radius(t.radius_control)
                        .overflow(Overflow::Clip)
                        .child(AppIcon::new(image, t.size_banner_thumbnail)),
                )
            })
    }
}
