//! The banners' surface: one placed surface at the top right of the main screen holding the
//! stack, its hit shapes the cards, so the screen beside a short stack stays the window's
//! (`p1/surfaces/notifications.md`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use freya::{animation::*, prelude::*};
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    AppIcon, Button, Glass, Glyph, Hit, LevelBar, button::Kind, colour, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::model::Card;
use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// What the stack shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Newest first.
    pub cards: Vec<Card>,
    pub overflow: u32,
    /// The card whose reply field is open.
    pub replying: Option<u32>,
    /// Solid and still: Game Mode, a full-screen game, Reduce Transparency.
    pub solid: bool,
    pub still: bool,
}

/// What the stack asks of the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum BannerAction {
    /// The pointer came onto the stack, or left it.
    Hover(bool),
    /// A click on a card: its default action, or its app.
    Open(u32),
    Action(u32, String),
    Close(u32),
    /// Reply: the field opens, with the keyboard.
    StartReply(u32),
    Send(u32, String),
    CancelReply,
    /// The overflow row: Notification Centre.
    Overflow,
}

/// A card's height at rest: the icon's 32 and padding, then a line per body line, and the
/// progress bar's; hovered, the full body and the action row (`surfaces/notifications.md`).
pub fn card_height(theme: &Theme, card: &Card, hovered: bool, replying: bool) -> f32 {
    let t = &theme.tokens;
    let line = text::metrics(theme, text::Style::Body).1;
    let lines = if card.body.is_empty() {
        0
    } else if hovered {
        (card.body.len() / 40 + 1).min(t.limit_banner_lines_expanded as usize)
    } else {
        (card.body.len() / 40 + 1).min(t.limit_banner_lines as usize)
    };
    let mut h =
        (2.0 * t.space_m + t.size_app_icon).max(2.0 * t.space_m + 16.0 + line * (1 + lines) as f32);
    if card.progress.is_some() {
        h += t.space_s + 6.0;
    }
    if replying {
        h += t.space_m + t.size_control + t.space_s + t.size_control;
    } else if hovered && (!card.actions.is_empty() || card.reply) {
        h += t.space_m + t.size_control;
    }
    h
}

#[derive(Default)]
pub struct BannersSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
}

impl BannersSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame;
    }

    /// Puts `frame` in force; call it in the tree's root scope.
    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    /// The room round the stack for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The surface's size: wide enough for a card, tall enough for every card hovered.
    pub fn size_for(frame: &Frame, theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let width = t.size_panel - 2.0 * t.space_s;
        let cards: f32 = frame
            .cards
            .iter()
            .map(|c| card_height(theme, c, true, frame.replying == Some(c.id)))
            .sum::<f32>()
            + t.space_s * frame.cards.len().saturating_sub(1) as f32;
        let overflow = if frame.overflow > 0 {
            t.space_s + t.size_row
        } else {
            0.0
        };
        (
            (width + 2.0 * m).ceil() as i32,
            (cards + overflow + 2.0 * m).ceil() as i32,
        )
            .into()
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    /// Two banners, one with actions and a picture, for the preview.
    pub fn sample() -> Rc<Self> {
        let surface = Self::new();
        surface.set_model(Frame {
            cards: vec![
                Card {
                    id: 2,
                    app: "chat".into(),
                    app_name: "Fractal".into(),
                    summary: "Sam".into(),
                    body: "Are we still on for the climb on Saturday? The forecast looks dry."
                        .into(),
                    actions: vec![("like".into(), "Like".into())],
                    reply: true,
                    count: 3,
                    time: "Now".into(),
                    ..Default::default()
                },
                Card {
                    id: 1,
                    app: "update".into(),
                    app_name: "Software Update".into(),
                    glyph: Some("arrow-circle-up".into()),
                    summary: "Update installed".into(),
                    body: "Restart to finish updating.".into(),
                    actions: vec![("restart".into(), "Restart…".into())],
                    count: 1,
                    time: "Now".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let theme = isle_tokens::Source::shipped()
            .theme(&isle_tokens::Appearance::default())
            .unwrap();
        surface.set_size(Self::size_for(&surface.frame.borrow(), &theme));
        surface
    }
}

impl SurfaceDef for BannersSurface {
    fn name(&self) -> &'static str {
        "banners"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(self.size.get())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(BannersApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct BannersApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for BannersApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.frame.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Stack { frame: state }
    }
}

#[derive(PartialEq)]
struct Stack {
    frame: State<Frame>,
}

fn send(actions: &Actions, action: BannerAction) {
    actions.send(Action::Banner(action));
}

impl Component for Stack {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let frame = self.frame.read().clone();
        let margin = BannersSurface::margin(&theme);
        let overflow = actions.clone();
        rect()
            .expanded()
            .padding(margin)
            .cross_align(Alignment::End)
            .child(
                rect()
                    .width(Size::px(t.size_panel - 2.0 * t.space_s))
                    .spacing(t.space_s)
                    .children(frame.cards.iter().map(|card| {
                        rect()
                            .width(Size::fill())
                            .key(card.id)
                            .child(Banner {
                                card: card.clone(),
                                replying: frame.replying == Some(card.id),
                                solid: frame.solid,
                                still: frame.still,
                            })
                            .into_element()
                    }))
                    .maybe(frame.overflow > 0, |el| {
                        el.child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(t.size_row))
                                .corner_radius(t.radius_control)
                                .background(colour(t.material_solid))
                                .center()
                                .child(Hit)
                                .a11y_role(AccessibilityRole::Button)
                                .on_press(move |_| send(&overflow, BannerAction::Overflow))
                                .child(text::line(
                                    &theme,
                                    text::Style::Callout,
                                    colour(t.colour_text),
                                    format!("{} more in Notification Centre", frame.overflow),
                                )),
                        )
                    }),
            )
    }
}

#[derive(PartialEq)]
struct Banner {
    card: Card,
    replying: bool,
    solid: bool,
    still: bool,
}

impl Component for Banner {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let mut hover = use_state(|| false);
        // A card that goes from under the pointer sends no leave.
        use_drop({
            let actions = actions.clone();
            move || {
                if *hover.peek() {
                    send(&actions, BannerAction::Hover(false));
                }
            }
        });
        let reply = use_state(String::new);
        let still = self.still;
        // Arrives from the trailing edge, short and damped; still in Game Mode.
        let arrival = use_animation(move |conf| {
            conf.on_creation(if still {
                OnCreation::Finish
            } else {
                OnCreation::Run
            });
            (
                AnimNum::new(40.0, 0.0).time(220).ease(Ease::Out),
                AnimNum::new(0.0, 1.0).time(160),
            )
        });
        let (offset, opacity) = arrival.get().value();
        let c = &self.card;
        let hovered = *hover.read();
        let id = c.id;
        let open = actions.clone();
        let (enter, leave) = (actions.clone(), actions.clone());
        let height = card_height(&theme, c, hovered, self.replying);
        let lines = if hovered {
            t.limit_banner_lines_expanded as usize
        } else {
            t.limit_banner_lines as usize
        };
        let leading = rect()
            .width(Size::px(t.size_app_icon))
            .height(Size::px(t.size_app_icon))
            .center()
            .map(c.icon.clone(), |el, icon| {
                el.child(AppIcon::new(icon, t.size_app_icon))
            })
            .maybe(c.icon.is_none(), |el| {
                el.corner_radius(t.size_app_icon / 2.0)
                    .background(colour(t.colour_fill_strong))
                    .child(Glyph::new(
                        c.glyph.clone().unwrap_or("bell".into()),
                        t.size_glyph_large,
                        colour(t.colour_text),
                    ))
            });
        let header = rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(rect().width(Size::flex(1.)).child(text::line(
                &theme,
                text::Style::Subheadline,
                colour(t.colour_text_2),
                c.app_name.clone(),
            )))
            .maybe(c.count > 1, |el| {
                el.child(
                    rect()
                        .height(Size::px(t.size_badge_quiet))
                        .padding((0., t.space_xs))
                        .corner_radius(t.size_badge_quiet / 2.0)
                        .background(colour(t.colour_fill_strong))
                        .center()
                        .child(text::line(
                            &theme,
                            text::Style::Subheadline,
                            colour(t.colour_text),
                            format!("{} new", c.count),
                        )),
                )
            })
            .child(text::line(
                &theme,
                text::Style::Subheadline,
                colour(t.colour_text_2),
                c.time.clone(),
            ));
        let (size, line, weight) = text::metrics(&theme, text::Style::Body);
        let body = label()
            .font_size(size)
            .line_height(line / size)
            .font_weight(weight as i32)
            .color(colour(t.colour_text))
            .max_lines(lines)
            .text_overflow(TextOverflow::Ellipsis)
            .text(c.body.clone());
        let buttons = c.actions.iter().take(3).map(|(key, label)| {
            let actions = actions.clone();
            let key = key.clone();
            rect()
                .width(Size::flex(1.))
                .child(
                    Button::new(label.clone())
                        .wide()
                        .on_press(move |_| send(&actions, BannerAction::Action(id, key.clone()))),
                )
                .into_element()
        });
        let reply_button = {
            let actions = actions.clone();
            Button::new("Reply")
                .wide()
                .on_press(move |_| send(&actions, BannerAction::StartReply(id)))
        };
        let field = {
            let send_actions = actions.clone();
            let cancel = actions.clone();
            let mut text_state = reply;
            rect()
                .width(Size::fill())
                .spacing(t.space_s)
                .child(
                    Input::new(reply)
                        .placeholder("Reply")
                        .auto_focus(true)
                        .width(Size::fill())
                        .color(colour(t.colour_text))
                        .placeholder_color(colour(t.colour_text_2)),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .main_align(Alignment::End)
                        .spacing(t.space_s)
                        .child(
                            Button::new("Cancel")
                                .small()
                                .on_press(move |_| send(&cancel, BannerAction::CancelReply)),
                        )
                        .child(
                            Button::new("Send")
                                .small()
                                .kind(Kind::Default)
                                .enabled(!reply.read().is_empty())
                                .on_press(move |_| {
                                    let text = text_state.read().clone();
                                    text_state.set(String::new());
                                    send(&send_actions, BannerAction::Send(id, text));
                                }),
                        ),
                )
        };
        let close = {
            let actions = actions.clone();
            rect()
                .position(Position::new_absolute().left(-5.).top(-5.))
                .width(Size::px(t.size_control_small))
                .height(Size::px(t.size_control_small))
                .corner_radius(t.size_control_small / 2.0)
                .background(colour(t.material_solid))
                .center()
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt("Close notification")
                .a11y_focusable(true)
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    send(&actions, BannerAction::Close(id))
                })
                .child(Glyph::new("x", t.size_glyph_small, colour(t.colour_text)))
        };
        let card = shadows(&t.shadow_panel).into_iter().fold(
            rect()
                .width(Size::fill())
                .height(Size::px(height))
                .corner_radius(t.radius_panel)
                .background(if self.solid {
                    colour(t.material_solid)
                } else {
                    Color::TRANSPARENT
                })
                .maybe(theme.appearance.increase_contrast, |el| {
                    el.border(
                        Border::new()
                            .fill(colour(t.colour_hairline_strong))
                            .width(t.stroke_hairline)
                            .alignment(BorderAlignment::Inner),
                    )
                }),
            |el, s| el.shadow(s),
        );
        // The glass fills the card inside its padding, so the padding is the content's.
        let content = rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(t.space_m)
            .horizontal()
            .content(Content::flex())
            .spacing(t.space_m)
            .child(leading)
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(header)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .content(Content::flex())
                            .spacing(t.space_s)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Headline,
                                        colour(t.colour_text),
                                        c.summary.clone(),
                                    ))
                                    .maybe(!c.body.is_empty(), |el| el.child(body)),
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
                            }),
                    )
                    .map(c.progress, |el, level| {
                        el.child(rect().padding((t.space_s, 0., 0., 0.)).child(LevelBar {
                            level,
                            track: colour(t.colour_fill_strong),
                            fill: colour(t.colour_accent),
                        }))
                    })
                    .maybe(self.replying, |el| {
                        el.child(rect().padding((t.space_m, 0., 0., 0.)).child(field))
                    })
                    .maybe(
                        hovered && !self.replying && (!c.actions.is_empty() || c.reply),
                        |el| {
                            el.child(
                                rect()
                                    .padding((t.space_m, 0., 0., 0.))
                                    .width(Size::fill())
                                    .horizontal()
                                    .content(Content::flex())
                                    .spacing(t.space_s)
                                    .children(buttons)
                                    .maybe(c.reply, |el| {
                                        el.child(rect().width(Size::flex(1.)).child(reply_button))
                                    }),
                            )
                        },
                    ),
            );
        card.offset_x(offset)
            .opacity(opacity)
            .maybe(!self.solid, |el| el.child(Glass::new(t.radius_panel)))
            .child(Hit)
            .a11y_role(AccessibilityRole::Alert)
            .a11y_alt(format!("{}: {}. {}", c.app_name, c.summary, c.body))
            // Over and out count the card's buttons as the card, where enter and leave do
            // not. The card tells the stack's model too, whose grace covers the gap between
            // cards and the out-then-over of moving onto a button.
            .on_pointer_over(move |_| {
                if !*hover.peek() {
                    hover.set(true);
                    send(&enter, BannerAction::Hover(true));
                }
            })
            .on_pointer_out(move |_| {
                hover.set_if_modified(false);
                send(&leave, BannerAction::Hover(false));
            })
            .on_press(move |_| send(&open, BannerAction::Open(id)))
            .child(content)
            .maybe(hovered || c.persistent, |el| el.child(close))
    }
}
