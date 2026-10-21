//! A text field for Isle's apps: Freya's editor, with its caret, selection, clipboard and
//! undo, dressed in Isle's tokens. Under `isle-app` it takes the Mac's editing keys.

use freya::prelude::*;

use crate::{colour, use_theme};

#[derive(Clone, PartialEq)]
pub struct TextField {
    value: Writable<String>,
    placeholder: Option<String>,
    alt: String,
    width: Size,
    enabled: bool,
    on_submit: Option<EventHandler<String>>,
    on_escape: Option<EventHandler<()>>,
    leading: Option<Element>,
    auto_focus: bool,
    select_all: bool,
}

impl TextField {
    pub fn new(value: impl Into<Writable<String>>, alt: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: None,
            alt: alt.into(),
            width: Size::fill(),
            enabled: true,
            on_submit: None,
            on_escape: None,
            leading: None,
            auto_focus: false,
            select_all: false,
        }
    }

    /// Takes the keyboard when it appears, its text selected: a name being edited in place.
    pub fn editing(mut self) -> Self {
        self.auto_focus = true;
        self.select_all = true;
        self
    }

    /// Escape in the field.
    pub fn on_escape(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_escape = Some(f.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Return in the field, with its text.
    pub fn on_submit(mut self, f: impl Into<EventHandler<String>>) -> Self {
        self.on_submit = Some(f.into());
        self
    }

    /// A glyph before the text, as the search field's magnifier.
    pub fn leading(mut self, leading: impl IntoElement) -> Self {
        self.leading = Some(leading.into_element());
        self
    }
}

impl Component for TextField {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let colours = InputColorsThemePartial::new()
            .background(colour(t.colour_fill))
            .hover_background(colour(t.colour_fill))
            .focus_background(colour(t.colour_fill))
            .color(colour(t.colour_text))
            .placeholder_color(colour(t.colour_text_3))
            .border_fill(colour(t.colour_hairline_strong))
            .hover_border_fill(colour(t.colour_hairline_strong))
            .focus_border_fill(colour(t.colour_accent))
            .focus_ring_fill(colour(t.colour_accent));
        let layout = InputLayoutThemePartial::new()
            .corner_radius(CornerRadius::new_all(t.radius_control))
            .padding(Gaps::new(
                (t.size_control_large - t.type_style_body_line) / 2.,
                t.space_s,
                (t.size_control_large - t.type_style_body_line) / 2.,
                t.space_s,
            ));
        let mut input = Input::new(self.value.clone())
            .a11y_id(a11y)
            .width(self.width.clone())
            .enabled(self.enabled)
            .theme_colors(colours)
            .theme_layout(layout);
        if let Some(placeholder) = self.placeholder.clone() {
            input = input.placeholder(placeholder);
        }
        if let Some(on_submit) = self.on_submit.clone() {
            input = input.on_submit(on_submit);
        }
        if let Some(leading) = self.leading.clone() {
            input = input.leading(leading);
        }
        if self.auto_focus {
            input = input.auto_focus(true).select_all_on_init(self.select_all);
        }
        if let Some(on_escape) = self.on_escape.clone() {
            input = input.on_pre_key_down(move |e: Event<KeyboardEventData>| {
                if e.key == Key::Named(NamedKey::Escape) {
                    on_escape.call(());
                }
                true
            });
        }
        rect().a11y_alt(self.alt.clone()).width(self.width.clone()).child(input)
    }
}
