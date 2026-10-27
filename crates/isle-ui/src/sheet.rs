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
    sheet_under(theme().tokens.size_toolbar, width, body, buttons)
}

/// The button Escape presses: the one labelled Cancel or Stop, or the only one.
fn cancel_of(buttons: &[(String, EventHandler<()>)]) -> Option<EventHandler<()>> {
    buttons
        .iter()
        .find(|(label, _)| label == "Cancel" || label == "Stop")
        .or(if buttons.len() == 1 { buttons.first() } else { None })
        .map(|(_, on_press)| on_press.clone())
}

/// An alert's buttons trailing, the last the default.
fn button_row(t: &isle_tokens::Tokens, buttons: Vec<(String, EventHandler<()>)>) -> Element {
    let count = buttons.len();
    rect()
        .width(Size::fill())
        .horizontal()
        .main_align(Alignment::End)
        .spacing(t.space_s)
        .children(buttons.into_iter().enumerate().map(|(i, (label, on_press))| {
            let kind = if i + 1 == count { button::Kind::Default } else { button::Kind::Regular };
            crate::Button::new(label).kind(kind).on_press(on_press).into_element()
        }))
        .into_element()
}

thread_local! {
    static ALERT_ICON: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// The app's icon, which its alerts show above their words; set once as the app starts.
pub fn set_alert_icon(path: Option<std::path::PathBuf>) {
    ALERT_ICON.with(|icon| *icon.borrow_mut() = path);
}

/// A button of an alert; a destructive one is drawn in the danger colour and is never the
/// default.
pub struct AlertButton {
    pub label: String,
    pub on_press: EventHandler<()>,
    pub destructive: bool,
}

impl AlertButton {
    pub fn destructive(label: impl Into<String>, on_press: EventHandler<()>) -> Self {
        Self { label: label.into(), on_press, destructive: true }
    }
}

impl From<(String, EventHandler<()>)> for AlertButton {
    fn from((label, on_press): (String, EventHandler<()>)) -> Self {
        Self { label, on_press, destructive: false }
    }
}

/// An alert in macOS 11's compact form, as a sheet under the window's toolbar: the app's
/// icon, the title and message centred, `extra` (a check box) under them, then the buttons.
/// Two buttons share the width with the default (the last) trailing; more stack with the
/// default first and Cancel last. Return presses the default and Escape Cancel.
pub fn alert<B: Into<AlertButton>>(title: &str, message: &str, extra: Vec<Element>, buttons: Vec<B>) -> Element {
    alert_under(theme().tokens.size_toolbar, title, message, extra, buttons)
}

/// As `alert`, hung `top` below the window's top edge.
pub fn alert_under<B: Into<AlertButton>>(top: f32, title: &str, message: &str, extra: Vec<Element>, buttons: Vec<B>) -> Element {
    let theme = theme();
    let t = &theme.tokens;
    let buttons: Vec<AlertButton> = buttons.into_iter().map(Into::into).collect();
    let cancel = buttons
        .iter()
        .find(|b| b.label == "Cancel" || b.label == "Stop")
        .or(if buttons.len() == 1 { buttons.first() } else { None })
        .map(|b| b.on_press.clone());
    let default = buttons.last().map(|b| b.on_press.clone());
    let kind = |b: &AlertButton, default: bool| match (default, b.destructive) {
        (true, _) => button::Kind::Default,
        (false, true) => button::Kind::Destructive,
        (false, false) => button::Kind::Regular,
    };
    let icon = ALERT_ICON.with(|icon| icon.borrow().clone());
    let centred = |style: text::Style, ink, words: &str| {
        let (size, height, weight) = text::metrics(&theme, style);
        label()
            .width(Size::fill())
            .font_size(size)
            .line_height(height / size)
            .font_weight(weight as i32)
            .color(colour(ink))
            .text_align(TextAlign::Center)
            .text(words.to_owned())
    };
    let count = buttons.len();
    let row = if count <= 2 {
        rect()
            .width(Size::fill())
            .horizontal()
            .spacing(t.space_s)
            .children(buttons.into_iter().enumerate().map(|(i, b)| {
                let kind = kind(&b, i + 1 == count);
                rect().width(Size::flex(1.)).child(crate::Button::new(b.label).kind(kind).wide().on_press(b.on_press)).into_element()
            }))
            .content(Content::flex())
    } else {
        // The default first, Cancel (or Stop) last, the rest between in their order.
        let mut stacked: Vec<(AlertButton, bool)> = Vec::new();
        let mut rest = buttons;
        if let Some(b) = rest.pop() {
            stacked.push((b, true));
        }
        let at_cancel = rest.iter().position(|b| b.label == "Cancel" || b.label == "Stop");
        let last = at_cancel.map(|i| rest.remove(i));
        stacked.extend(rest.into_iter().map(|b| (b, false)));
        stacked.extend(last.map(|b| (b, false)));
        rect().width(Size::fill()).spacing(t.space_s).children(stacked.into_iter().map(|(b, default)| {
            let kind = kind(&b, default);
            crate::Button::new(b.label).kind(kind).wide().on_press(b.on_press).into_element()
        }))
    };
    let card = rect()
        .width(Size::px(t.size_alert_compact))
        .padding((t.space_l + 2., t.space_l, t.space_l, t.space_l))
        .spacing(t.space_s + 2.)
        .cross_align(Alignment::Center)
        .corner_radius(t.radius_alert)
        .background(colour(t.colour_window_raised))
        .maybe(true, |el| shadows(&t.shadow_window).into_iter().fold(el, |el, s| el.shadow(s)))
        .maybe_child(icon.map(|path| crate::AppIcon::new(path, t.size_alert_icon)))
        .child(centred(text::Style::Headline, t.colour_text, title))
        .maybe_child((!message.is_empty()).then(|| centred(text::Style::Callout, t.colour_text_2, message)))
        .children(extra)
        .child(rect().width(Size::fill()).padding((t.space_xs, 0., 0., 0.)).child(row));
    rect()
        .position(Position::new_global())
        .layer(Layer::Overlay)
        .width(Size::window_percent(100.))
        .height(Size::window_percent(100.))
        .background(colour(t.colour_scrim))
        .cross_align(Alignment::Center)
        .padding((top, 0., 0., 0.))
        .a11y_role(AccessibilityRole::AlertDialog)
        .on_press(|e: Event<PressEventData>| e.stop_propagation())
        // After the window's own handlers have seen the alert up.
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            let pressed = match e.key {
                Key::Named(NamedKey::Escape) => cancel.clone(),
                Key::Named(NamedKey::Enter) => default.clone(),
                _ => None,
            };
            if let Some(pressed) = pressed {
                spawn(async move { pressed.call(()) });
            }
        })
        .child(card)
        .into_element()
}

/// As `sheet`, hung `top` below the window's top edge.
fn sheet_under(top: f32, width: f32, body: Vec<Element>, buttons: Vec<(String, EventHandler<()>)>) -> Element {
    let theme = theme();
    let t = &theme.tokens;
    let cancel = cancel_of(&buttons);
    rect()
        .position(Position::new_global())
        .layer(Layer::Overlay)
        .width(Size::window_percent(100.))
        .height(Size::window_percent(100.))
        .background(colour(t.colour_scrim))
        .cross_align(Alignment::Center)
        .padding((top, 0., 0., 0.))
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
                .child(button_row(t, buttons)),
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
