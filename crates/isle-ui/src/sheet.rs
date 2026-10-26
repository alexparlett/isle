//! Sheets, alerts and popovers inside a window (`p1/surfaces/conventions.md` § Sheets,
//! alerts and popovers; `window-chrome.md`).

use freya::prelude::*;

use crate::{button, colour, shadows, text, theme, use_theme};

/// An alert's words: its question and what follows.
pub fn alert_text(title: &str, message: &str) -> Element {
    let theme = theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .spacing(t.space_s)
        .child(
            text::line(&theme, text::Style::Headline, colour(t.colour_text), title)
                .width(Size::fill())
                .max_lines(None)
                .text_overflow(TextOverflow::Clip),
        )
        .child(
            label()
                .width(Size::fill())
                .font_size(text::metrics(&theme, text::Style::Body).0)
                .color(colour(t.colour_text_2))
                .text(message.to_owned()),
        )
        .into_element()
}

/// A sheet over the window under its toolbar: the body, then its buttons trailing, the last
/// the default. Escape presses the one labelled Cancel or Stop, or the only one.
pub fn sheet(width: f32, body: Vec<Element>, buttons: Vec<(String, EventHandler<()>)>) -> Element {
    let theme = theme();
    let t = &theme.tokens;
    let count = buttons.len();
    let cancel = buttons
        .iter()
        .find(|(label, _)| label == "Cancel" || label == "Stop")
        .or(if count == 1 { buttons.first() } else { None })
        .map(|(_, on_press)| on_press.clone());
    rect()
        .position(Position::new_global())
        .layer(Layer::Overlay)
        .width(Size::window_percent(100.))
        .height(Size::window_percent(100.))
        .background(colour(t.colour_scrim))
        .cross_align(Alignment::Center)
        .padding((t.size_toolbar, 0., 0., 0.))
        .a11y_role(AccessibilityRole::Dialog)
        .on_press(|e: Event<PressEventData>| e.stop_propagation())
        // Escape presses Cancel, after the window's own handlers have seen the sheet up.
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            if e.key == Key::Named(NamedKey::Escape)
                && let Some(cancel) = cancel.clone()
            {
                spawn(async move { cancel.call(()) });
            }
        })
        .child(
            rect()
                .width(Size::px(width))
                .padding(t.space_xl)
                .spacing(t.space_l)
                .corner_radius(t.radius_window)
                .background(colour(t.colour_window))
                .maybe(true, |el| {
                    shadows(&t.shadow_window)
                        .into_iter()
                        .fold(el, |el, s| el.shadow(s))
                })
                .children(body)
                .child(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .main_align(Alignment::End)
                        .spacing(t.space_s)
                        .children(buttons.into_iter().enumerate().map(|(i, (label, on_press))| {
                            let kind = if i + 1 == count {
                                button::Kind::Default
                            } else {
                                button::Kind::Regular
                            };
                            crate::Button::new(label)
                                .kind(kind)
                                .on_press(on_press)
                                .into_element()
                        })),
                ),
        )
        .into_element()
}

/// Where a popover hangs from its anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Side {
    #[default]
    Below,
    Above,
}

/// A panel hung from its anchor, with no arrow: its edge `space-xs` from the anchor's,
/// centred on it and kept inside the window. A press outside it or Escape closes it.
#[derive(PartialEq)]
pub struct Popover {
    anchor: Element,
    open: bool,
    side: Side,
    alt: String,
    content: Vec<Element>,
    on_close: EventHandler<()>,
}

impl Popover {
    pub fn new(anchor: impl IntoElement, open: bool, on_close: impl Into<EventHandler<()>>) -> Self {
        Self {
            anchor: anchor.into_element(),
            open,
            side: Side::Below,
            alt: String::new(),
            content: Vec::new(),
            on_close: on_close.into(),
        }
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Its accessible name.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = alt.into();
        self
    }
}

impl ChildrenExt for Popover {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.content
    }
}

impl Component for Popover {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let attached = Attached::new(self.anchor.clone()).offset(t.space_xs);
        let attached = match self.side {
            Side::Below => attached.bottom(),
            Side::Above => attached.top(),
        };
        if !self.open {
            return attached.into_element();
        }
        let (close, close_key) = (self.on_close.clone(), self.on_close.clone());
        let panel = rect()
            .layer(Layer::Overlay)
            .min_width(Size::px(200.))
            .padding(t.space_m)
            .spacing(t.space_s)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_raised))
            .maybe(true, |el| shadows(&t.shadow_menu).into_iter().fold(el, |el, s| el.shadow(s)))
            .a11y_role(AccessibilityRole::Dialog)
            .a11y_alt(self.alt.clone())
            .on_press(|e: Event<PressEventData>| e.stop_propagation())
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                if e.key == Key::Named(NamedKey::Escape) {
                    close_key.call(());
                }
            })
            .children(self.content.clone());
        rect()
            .child(
                rect()
                    .position(Position::new_global())
                    .layer(Layer::Overlay)
                    .width(Size::window_percent(100.))
                    .height(Size::window_percent(100.))
                    .on_press(move |_| close.call(())),
            )
            .child(attached.child(panel))
            .into_element()
    }
}
