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
    on_tab: Option<EventHandler<()>>,
    leading: Option<Element>,
    auto_focus: bool,
    select_all: bool,
    /// Chars selected as it appears, in place of all of them.
    select_part: Option<std::ops::Range<usize>>,
    a11y_id: Option<AccessibilityId>,
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
            on_tab: None,
            leading: None,
            auto_focus: false,
            select_all: false,
            select_part: None,
            a11y_id: None,
        }
    }

    /// Takes the keyboard when it appears, its text selected: a name being edited in place.
    pub fn editing(mut self) -> Self {
        self.auto_focus = true;
        self.select_all = true;
        self
    }

    /// As `editing`, with only the chars `part` selected: a file's name without its extension.
    pub fn editing_part(mut self, part: std::ops::Range<usize>) -> Self {
        self.select_part = Some(part);
        self.editing()
    }

    /// Escape in the field.
    pub fn on_escape(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_escape = Some(f.into());
        self
    }

    /// Tab in the field, which then keeps the focus: a path field completes.
    pub fn on_tab(mut self, f: impl Into<EventHandler<()>>) -> Self {
        self.on_tab = Some(f.into());
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

    /// The owner's id for the field, so it can focus the field and tell when it has the keys.
    pub fn a11y_id(mut self, id: AccessibilityId) -> Self {
        self.a11y_id = Some(id);
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
        let own = use_a11y();
        let a11y = self.a11y_id.unwrap_or(own);
        // A name edited in place is opaque over the selected row under it.
        let fill = colour(if self.select_all { t.colour_view } else { t.colour_fill });
        let colours = InputColorsThemePartial::new()
            .background(fill)
            .hover_background(fill)
            .focus_background(fill)
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
            if let Some(part) = self.select_part.clone() {
                input = input.select_on_init(part);
            }
        }
        if self.on_escape.is_some() || self.on_tab.is_some() {
            let (on_escape, on_tab) = (self.on_escape.clone(), self.on_tab.clone());
            input = input.on_pre_key_down(move |e: Event<KeyboardEventData>| {
                match (&e.key, &on_escape, &on_tab) {
                    (Key::Named(NamedKey::Escape), Some(on_escape), _) => on_escape.call(()),
                    (Key::Named(NamedKey::Tab), _, Some(on_tab)) => {
                        e.stop_propagation();
                        e.prevent_default();
                        on_tab.call(());
                        return false;
                    }
                    _ => {}
                }
                Input::key_down_default(e)
            });
        }
        rect().a11y_alt(self.alt.clone()).width(self.width.clone()).child(input)
    }
}
