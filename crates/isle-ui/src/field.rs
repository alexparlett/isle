//! A text field whose text is held outside Freya's editor in a buffer that is wiped when it
//! goes, for secrets and anything typed into a system prompt; and a checkbox row.

use std::{cell::RefCell, fmt, rc::Rc};

use freya::prelude::*;
use zeroize::Zeroizing;

use crate::{Glyph, colour, text, use_theme};

/// A secret on its way to the program that asked: wiped when dropped, never printed.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(text: String) -> Self {
        Self(Zeroizing::new(text))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

/// What a field holds, shared with the surface that reads it on the action.
pub type FieldText = Rc<RefCell<Zeroizing<String>>>;

/// The longest text a field takes.
const LIMIT: usize = 512;

/// A field of the prompt sheet (`surfaces/prompts.md`, Fields): its text in a wiped buffer,
/// dots for a secret, copy and cut never offered.
#[derive(Clone)]
pub struct Field {
    text: FieldText,
    label: String,
    secret: bool,
    /// A secret shown in the clear, by "Show password".
    shown: bool,
    enabled: bool,
    error: bool,
    auto_focus: bool,
    mono: bool,
    on_submit: Option<EventHandler<()>>,
    on_change: Option<EventHandler<()>>,
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.text, &other.text)
            && self.label == other.label
            && self.secret == other.secret
            && self.shown == other.shown
            && self.enabled == other.enabled
            && self.error == other.error
            && self.auto_focus == other.auto_focus
            && self.mono == other.mono
    }
}

impl Field {
    pub fn new(text: FieldText, label: impl Into<String>) -> Self {
        Self {
            text,
            label: label.into(),
            secret: false,
            shown: false,
            enabled: true,
            error: false,
            auto_focus: false,
            mono: false,
            on_submit: None,
            on_change: None,
        }
    }

    pub fn secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The error edge, until the next key.
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Return in the field.
    pub fn on_submit(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_submit = Some(f.into());
        self
    }

    pub fn on_change(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_change = Some(f.into());
        self
    }
}

impl Component for Field {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let focused = focus() != Focus::Not;
        // The buffer is not a state; a key bumps this so the field draws again.
        let mut version = use_state(|| 0u32);
        let _ = *version.read();
        let text = self.text.clone();
        let enabled = self.enabled;
        let (on_submit, on_change) = (self.on_submit.clone(), self.on_change.clone());
        let on_key = move |e: Event<KeyboardEventData>| {
            if !enabled {
                return;
            }
            let mods = e.modifiers;
            let control = mods.contains(Modifiers::CONTROL)
                || mods.contains(Modifiers::META)
                || mods.contains(Modifiers::ALT);
            let changed = match &e.key {
                Key::Named(NamedKey::Backspace) => text.borrow_mut().pop().is_some(),
                Key::Named(NamedKey::Enter) => {
                    if let Some(f) = &on_submit {
                        f.call(());
                    }
                    false
                }
                Key::Character(c) if !control => {
                    let mut buffer = text.borrow_mut();
                    let room = LIMIT.saturating_sub(buffer.chars().count());
                    let typed: String = c.chars().filter(|c| !c.is_control()).take(room).collect();
                    buffer.push_str(&typed);
                    !typed.is_empty()
                }
                _ => false,
            };
            if changed {
                *version.write() += 1;
                if let Some(f) = &on_change {
                    f.call(());
                }
            }
        };
        let content = self.text.borrow();
        let ink = if enabled {
            colour(t.colour_text)
        } else {
            colour(t.colour_text_disabled)
        };
        let shown_text = if self.secret && !self.shown {
            "•".repeat(content.chars().count())
        } else {
            content.as_str().to_owned()
        };
        let caret = (focused && enabled).then(|| {
            rect()
                .width(Size::px(1.5))
                .height(Size::px(16.))
                .background(colour(t.colour_text))
        });
        let line = if self.mono {
            label()
                .font_family("JetBrains Mono")
                .font_size(t.type_style_body_size)
                .color(ink)
                .max_lines(1)
                .text(shown_text)
        } else {
            text::line(&theme, text::Style::Body, ink, shown_text)
        };
        let edge = if self.error {
            Border::new()
                .fill(colour(t.colour_danger))
                .width(1.0)
                .alignment(BorderAlignment::Inner)
        } else if focus() == Focus::Keyboard || focused {
            Border::new()
                .fill(colour(t.colour_accent))
                .width(1.0)
                .alignment(BorderAlignment::Inner)
        } else {
            Border::new()
                .fill(colour(t.colour_hairline_strong))
                .width(t.stroke_hairline)
                .alignment(BorderAlignment::Inner)
        };
        rect()
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_auto_focus(self.auto_focus)
            .a11y_role(if self.secret {
                AccessibilityRole::PasswordInput
            } else {
                AccessibilityRole::TextInput
            })
            .a11y_alt(self.label.clone())
            .width(Size::fill())
            .height(Size::px(t.size_control_large))
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill))
            .border(edge)
            .padding((0., t.space_s))
            .horizontal()
            .cross_align(Alignment::Center)
            .overflow(Overflow::Clip)
            .on_key_down(on_key)
            .child(line)
            .maybe_child(caret)
    }
}

/// A checkbox and its label, the whole row clickable.
#[derive(Clone, PartialEq)]
pub struct CheckRow {
    label: String,
    checked: bool,
    on_toggle: Option<EventHandler<bool>>,
}

impl CheckRow {
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            on_toggle: None,
        }
    }

    pub fn on_toggle(mut self, f: impl Into<EventHandler<bool>>) -> Self {
        self.on_toggle = Some(f.into());
        self
    }
}

impl Component for CheckRow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let checked = self.checked;
        let on_toggle = self.on_toggle.clone();
        let toggle = move || {
            if let Some(f) = &on_toggle {
                f.call(!checked);
            }
        };
        let box_size = 14.0;
        let mark = rect()
            .width(Size::px(box_size))
            .height(Size::px(box_size))
            .corner_radius(4.)
            .background(if checked {
                colour(t.colour_accent)
            } else {
                colour(t.colour_control)
            })
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
            .center()
            .maybe_child(checked.then(|| Glyph::new("check", 11., colour(t.colour_on_accent))));
        let toggle_key = toggle.clone();
        rect()
            .a11y_id(a11y)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::CheckBox)
            .a11y_alt(self.label.clone())
            .height(Size::px(t.size_row))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(crate::focus_ring(&theme))
            })
            .on_press(move |_| toggle())
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if e.key == Key::Character(" ".into()) {
                    toggle_key();
                }
            })
            .child(mark)
            .child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text),
                self.label.clone(),
            ))
    }
}
