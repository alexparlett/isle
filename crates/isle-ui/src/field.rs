//! A text field whose text is held outside Freya's editor in a buffer that is wiped when it
//! goes, for secrets and anything typed into a system prompt; and a checkbox row.

use std::{cell::RefCell, fmt, rc::Rc};

use freya::{clipboard::Clipboard, prelude::*};
use zeroize::Zeroizing;

use crate::{
    Glyph, colour,
    edit::{EditText, To},
    text, use_theme,
};

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
pub type FieldText = Rc<RefCell<EditText>>;

/// A field of the prompt sheet (`surfaces/prompts.md`, Fields): its text in a wiped buffer,
/// dots for a secret, copy and cut never offered.
#[derive(Clone)]
pub struct Field {
    text: FieldText,
    /// The buffer's revision when this was built, so a change made outside draws again.
    revision: u64,
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
    on_paste: Option<EventHandler<()>>,
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.text, &other.text)
            && self.revision == other.revision
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
        let revision = text.borrow().revision();
        Self {
            text,
            revision,
            label: label.into(),
            secret: false,
            shown: false,
            enabled: true,
            error: false,
            auto_focus: false,
            mono: false,
            on_submit: None,
            on_change: None,
            on_paste: None,
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

    /// Paste, where the tree has no clipboard of its own: the owner puts the text in.
    pub fn on_paste(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_paste = Some(f.into());
        self
    }
}

/// The clipboard's text into `text` at the caret, line breaks dropped.
pub fn paste_into(text: &FieldText, pasted: &str) -> bool {
    text.borrow_mut()
        .insert(pasted.chars().filter(|c| !c.is_control()))
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
        let (enabled, secure) = (self.enabled, self.secret);
        let (on_submit, on_change, on_paste) = (
            self.on_submit.clone(),
            self.on_change.clone(),
            self.on_paste.clone(),
        );
        // Isle's apps take Cmd as Meta; the shell's surfaces get it translated to Ctrl (B21).
        let mac = try_consume_context::<TargetPlatform>().unwrap_or_else(TargetPlatform::get)
            == TargetPlatform::MacOs;
        let (primary, word) = if mac {
            (Modifiers::META, Modifiers::ALT)
        } else {
            (Modifiers::CONTROL, Modifiers::CONTROL)
        };
        let mut on_key = move |e: Event<KeyboardEventData>| {
            if !enabled {
                return;
            }
            let mods = e.modifiers;
            let cmd = mods.contains(primary);
            let shift = mods.contains(Modifiers::SHIFT);
            // The Emacs keys reach Isle's apps as Ctrl; the shell has them translated.
            let emacs = mac && mods.contains(Modifiers::CONTROL) && !cmd;
            let along = |plain: To, by_word: To, line: To| {
                if mac && cmd {
                    line
                } else if mods.contains(word) {
                    by_word
                } else {
                    plain
                }
            };
            let mut field = text.borrow_mut();
            let before = field.revision();
            match &e.key {
                Key::Named(NamedKey::ArrowLeft) => {
                    field.move_caret(along(To::Left, To::WordLeft, To::Start), secure, shift)
                }
                Key::Named(NamedKey::ArrowRight) => {
                    field.move_caret(along(To::Right, To::WordRight, To::End), secure, shift)
                }
                Key::Named(NamedKey::ArrowUp | NamedKey::Home) => {
                    field.move_caret(To::Start, secure, shift)
                }
                Key::Named(NamedKey::ArrowDown | NamedKey::End) => {
                    field.move_caret(To::End, secure, shift)
                }
                Key::Named(NamedKey::Backspace) => {
                    field.delete(along(To::Left, To::WordLeft, To::Start), secure)
                }
                Key::Named(NamedKey::Delete) => {
                    field.delete(along(To::Right, To::WordRight, To::End), secure)
                }
                Key::Named(NamedKey::Enter) => {
                    drop(field);
                    if let Some(f) = &on_submit {
                        f.call(());
                    }
                    return;
                }
                Key::Character(c) if emacs => match c.to_lowercase().as_str() {
                    "a" => field.move_caret(To::Start, secure, false),
                    "e" => field.move_caret(To::End, secure, false),
                    "b" => field.move_caret(To::Left, secure, false),
                    "f" => field.move_caret(To::Right, secure, false),
                    "h" => field.delete(To::Left, secure),
                    "d" => field.delete(To::Right, secure),
                    "k" => field.delete(To::End, secure),
                    _ => {}
                },
                Key::Character(c) if cmd => match c.to_lowercase().as_str() {
                    "a" => field.select_all(),
                    "v" => {
                        drop(field);
                        match &on_paste {
                            Some(f) => f.call(()),
                            None => {
                                if let Ok(pasted) = Clipboard::get() {
                                    paste_into(&text, &Zeroizing::new(pasted));
                                }
                            }
                        }
                        field = text.borrow_mut();
                    }
                    // Copy and cut are never offered.
                    _ => {}
                },
                Key::Character(c)
                    if !mods.intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::ALT) =>
                {
                    field.insert(c.chars().filter(|c| !c.is_control()));
                }
                _ => {}
            }
            let changed = field.revision() != before;
            drop(field);
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
        let shown: Vec<char> = if self.secret && !self.shown {
            vec!['•'; content.chars()]
        } else {
            content.as_str().chars().collect()
        };
        let caret = (focused && enabled).then(|| {
            rect()
                .width(Size::px(1.5))
                .height(Size::px(16.))
                .background(colour(t.colour_text))
                .into_element()
        });
        let mono = self.mono;
        let run = |range: std::ops::Range<usize>, selected: bool| {
            (!range.is_empty()).then(|| {
                let typed: String = shown[range].iter().collect();
                let line = if mono {
                    label()
                        .font_family("JetBrains Mono")
                        .font_size(t.type_style_body_size)
                        .color(ink)
                        .max_lines(1)
                        .text(typed)
                } else {
                    text::line(&theme, text::Style::Body, ink, typed)
                };
                rect()
                    .maybe(selected, |el| {
                        el.background(
                            colour(t.colour_accent).with_a((t.selection_mix * 255.) as u8),
                        )
                    })
                    .child(line)
                    .into_element()
            })
        };
        let line = rect().horizontal().cross_align(Alignment::Center);
        // The caret only where nothing is selected, as on a Mac.
        let line = match content.selection() {
            Some(range) => line
                .maybe_child(run(0..range.start, false))
                .maybe_child(run(range.clone(), true))
                .maybe_child(run(range.end..shown.len(), false)),
            None => {
                let at = content.caret().min(shown.len());
                line.maybe_child(run(0..at, false))
                    .maybe_child(caret)
                    .maybe_child(run(at..shown.len(), false))
            }
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
            // Keys are taken while this field holds focus, whichever node Freya routes them to.
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                if focus() != Focus::Not {
                    on_key(e);
                }
            })
            .on_press(move |_| {
                if enabled {
                    a11y.request_focus();
                }
            })
            .child(line)
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
