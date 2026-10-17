//! Grouped rows in a window, macOS's form style, and the controls they carry
//! (`p1/surfaces/settings.md`, A pane and Controls by kind).

use std::rc::Rc;

use freya::prelude::*;

use crate::{
    Glyph, colour, focus_ring,
    menu::{Material, Menu, MenuEntry, MenuEvent, MenuItem},
    shadows, text, use_theme,
};

/// A pane's header: its tile, name and one sentence.
#[derive(Clone, PartialEq)]
pub struct PaneHeader {
    pub glyph: String,
    pub hue: isle_tokens::Colour,
    pub name: String,
    pub subtitle: String,
}

impl Component for PaneHeader {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let tile = t.size_pane_tile_large;
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .child(
                rect()
                    .width(Size::px(tile))
                    .height(Size::px(tile))
                    .corner_radius(tile / 4.)
                    .background(colour(self.hue))
                    .center()
                    .child(Glyph::new(self.glyph.clone(), 24., Color::WHITE).fill(true)),
            )
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(t.space_xxs)
                    .child(text::line(&theme, text::Style::Title2, colour(t.colour_text), self.name.clone()))
                    .child(
                        label()
                            .font_size(text::metrics(&theme, text::Style::Body).0)
                            .color(colour(t.colour_text_2))
                            .text(self.subtitle.clone()),
                    ),
            )
    }
}

/// The scroll view a form's found row is scrolled into, given by the form's window.
#[derive(Clone, Copy)]
pub struct RevealFound(pub ScrollController);

/// A group of rows on a raised card, hairlines between them, with an optional title above.
pub fn group(title: Option<String>, rows: Vec<Element>) -> impl IntoElement {
    GroupCard { title, rows }
}

#[derive(Clone)]
struct GroupCard {
    title: Option<String>,
    rows: Vec<Element>,
}

impl PartialEq for GroupCard {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Component for GroupCard {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let count = self.rows.len();
        let mut children = Vec::with_capacity(count * 2);
        for (i, row) in self.rows.iter().cloned().enumerate() {
            children.push(row);
            if i + 1 < count {
                children.push(
                    rect()
                        .width(Size::fill())
                        .padding((0., 0., 0., t.space_m))
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(t.stroke_hairline))
                                .background(colour(t.colour_hairline)),
                        )
                        .into(),
                );
            }
        }
        rect()
            .width(Size::fill())
            .spacing(t.space_s)
            .map(self.title.clone(), |el, title| {
                el.child(
                    rect()
                        .padding((0., t.space_m))
                        .child(text::line(&theme, text::Style::Headline, colour(t.colour_text), title)),
                )
            })
            .child(
                rect()
                    .width(Size::fill())
                    .corner_radius(t.radius_card)
                    .background(colour(t.colour_window_raised))
                    .border(
                        Border::new()
                            .fill(colour(if theme.appearance.increase_contrast {
                                t.colour_hairline_strong
                            } else {
                                t.colour_hairline
                            }))
                            .width(t.stroke_hairline)
                            .alignment(BorderAlignment::Inner),
                    )
                    .overflow(Overflow::Clip)
                    .children(children),
            )
    }
}

/// One row: a label, a description where the label leaves something to say, and one
/// control at the trailing edge; or a navigation row's value and chevron.
#[derive(Clone)]
pub struct Row {
    label: String,
    description: Option<String>,
    leading: Option<Element>,
    trailing: Option<Element>,
    value: Option<String>,
    on_open: Option<EventHandler<()>>,
    enabled: bool,
    found: bool,
    refused: Option<String>,
    marked: bool,
    on_reset: Option<EventHandler<()>>,
}

impl PartialEq for Row {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Row {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            leading: None,
            trailing: None,
            value: None,
            on_open: None,
            enabled: true,
            found: false,
            refused: None,
            marked: false,
            on_reset: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn leading(mut self, leading: impl IntoElement) -> Self {
        self.leading = Some(leading.into_element());
        self
    }

    /// The row's control.
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.trailing = Some(control.into_element());
        self
    }

    /// A navigation row: the whole row opens a sub-page, showing `value` before its chevron.
    pub fn opens(mut self, value: Option<String>, on_open: impl Into<EventHandler<()>>) -> Self {
        self.value = value;
        self.on_open = Some(on_open.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Ringed in the accent, as a search result found it.
    pub fn found(mut self, found: bool) -> Self {
        self.found = found;
        self
    }

    /// The reason a value was refused, shown under the row.
    pub fn refused(mut self, reason: Option<String>) -> Self {
        self.refused = reason;
        self
    }

    /// A dot before the label: the value differs from its default.
    pub fn marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// Reset to Default in the row's context menu.
    pub fn on_reset(mut self, on_reset: Option<EventHandler<()>>) -> Self {
        self.on_reset = on_reset;
        self
    }
}

impl Component for Row {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut pressed = use_state(|| false);
        let mut menu = use_state(|| false);
        let reveal = use_try_consume::<RevealFound>();
        let mut revealed = use_state(|| false);
        if !self.found && *revealed.peek() {
            revealed.set(false);
        }
        let ink = if self.enabled { t.colour_text } else { t.colour_text_2 };
        let navigates = self.on_open.clone();
        let body = rect()
            .width(Size::flex(1.))
            .spacing(t.space_xxs)
            .child(text::line(&theme, text::Style::Body, colour(ink), self.label.clone()))
            .map(self.description.clone(), |el, d| {
                el.child(
                    label()
                        .font_size(text::metrics(&theme, text::Style::Callout).0)
                        .color(colour(t.colour_text_2))
                        .text(d),
                )
            })
            .map(self.refused.clone(), |el, reason| {
                el.child(
                    label()
                        .font_size(text::metrics(&theme, text::Style::Callout).0)
                        .color(colour(t.colour_danger_text))
                        .text(reason),
                )
            });
        let trailing = match (&self.trailing, navigates.is_some()) {
            (Some(control), _) => Some(control.clone()),
            (None, true) => Some(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .map(self.value.clone(), |el, v| {
                        el.child(text::line(&theme, text::Style::Body, colour(t.colour_text_2), v))
                    })
                    .child(Glyph::new("caret-right", t.size_glyph_small, colour(t.colour_text_2)))
                    .into(),
            ),
            (None, false) => None,
        };
        let enabled = self.enabled;
        let reset = self.on_reset.clone();
        let row = rect()
            .width(Size::fill())
            .min_height(Size::px(t.size_row_large))
            .padding(t.space_m)
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .a11y_role(if navigates.is_some() {
                AccessibilityRole::Link
            } else {
                AccessibilityRole::Row
            })
            .a11y_alt(self.label.clone())
            .maybe(navigates.is_some() && *pressed.read(), |el| el.background(colour(t.colour_fill)))
            .map(reveal.filter(|_| self.found), |el, reveal| {
                el.on_sized(move |e: Event<SizedEventData>| {
                    if !*revealed.peek() {
                        revealed.set(true);
                        let mut controller = reveal.0;
                        controller.scroll_to_item(e.area);
                    }
                })
            })
            .maybe(self.found, |el| {
                el.border(
                    Border::new()
                        .fill(colour(t.colour_accent))
                        .width(t.stroke_found)
                        .alignment(BorderAlignment::Inner),
                )
                .corner_radius(t.radius_row)
            })
            .map(navigates, |el, open| {
                el.on_pointer_down(move |_| pressed.set_if_modified(enabled))
                    .on_pointer_leave(move |_| pressed.set_if_modified(false))
                    .on_press(move |_| {
                        pressed.set_if_modified(false);
                        if enabled {
                            open.call(());
                        }
                    })
            })
            .map(reset.clone(), |el, _| {
                el.on_pointer_down(move |e: Event<PointerEventData>| {
                    if e.data().button() == Some(MouseButton::Right) {
                        menu.set(true);
                    }
                })
            })
            .maybe(self.marked, |el| {
                el.child(
                    rect()
                        .width(Size::px(6.))
                        .height(Size::px(6.))
                        .corner_radius(3.)
                        .background(colour(t.colour_accent))
                        .a11y_alt("Changed"),
                )
            })
            .map(self.leading.clone(), |el, leading| el.child(leading))
            .child(body)
            .map(trailing, |el, trailing| el.child(trailing));
        let entries: Rc<Vec<MenuEntry>> = Rc::new(vec![MenuEntry::Item(MenuItem::new(0, "Reset to Default"))]);
        row.maybe_child((*menu.read() && reset.is_some()).then(|| {
            rect()
                .position(Position::new_absolute().left(t.space_m).top(t.size_row_large - t.space_xs))
                .layer(Layer::Overlay)
                .child(
                    Menu::new(entries, move |event| match event {
                        MenuEvent::Chosen(_) => {
                            menu.set(false);
                            if let Some(reset) = &reset {
                                reset.call(());
                            }
                        }
                        MenuEvent::Back => menu.set(false),
                        _ => {}
                    })
                    .material(Material::Window),
                )
        }))
    }
}

/// One of two or three choices, all worth seeing.
#[derive(Clone, PartialEq)]
pub struct Segmented {
    options: Vec<String>,
    selected: usize,
    alt: String,
    enabled: bool,
    on_select: EventHandler<usize>,
}

impl Segmented {
    pub fn new(
        options: Vec<String>,
        selected: usize,
        alt: impl Into<String>,
        on_select: impl Into<EventHandler<usize>>,
    ) -> Self {
        Self {
            options,
            selected,
            alt: alt.into(),
            enabled: true,
            on_select: on_select.into(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for Segmented {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let (selected, count, enabled) = (self.selected, self.options.len(), self.enabled);
        let keys = self.on_select.clone();
        rect()
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt(self.alt.clone())
            .horizontal()
            .height(Size::px(t.size_control))
            .padding(2.)
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill))
            .maybe(focus() == Focus::Keyboard, |el| el.border(focus_ring(&theme)))
            .on_key_down(move |e: Event<KeyboardEventData>| {
                let next = match e.key {
                    Key::Named(NamedKey::ArrowLeft) => selected.checked_sub(1),
                    Key::Named(NamedKey::ArrowRight) => Some(selected + 1).filter(|n| *n < count),
                    _ => None,
                };
                if let Some(next) = next.filter(|_| enabled) {
                    keys.call(next);
                }
            })
            .children(self.options.iter().enumerate().map(|(i, option)| {
                let on_select = self.on_select.clone();
                let chosen = i == selected;
                rect()
                    .key(i)
                    .height(Size::fill())
                    .padding((0., t.space_m))
                    .center()
                    .corner_radius(t.radius_control - 2.)
                    .a11y_role(AccessibilityRole::RadioButton)
                    .a11y_alt(option.clone())
                    .maybe(chosen, |el| {
                        shadows(&t.shadow_control)
                            .into_iter()
                            .fold(el.background(colour(t.colour_control)), |el, s| el.shadow(s))
                    })
                    .on_press(move |_| {
                        if enabled && !chosen {
                            on_select.call(i);
                        }
                    })
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(if enabled { t.colour_text } else { t.colour_text_disabled }),
                        option.clone(),
                    ))
                    .into_element()
            }))
    }
}

/// One of several choices, or a duration, in a button that opens a menu of them.
#[derive(Clone, PartialEq)]
pub struct PopUp {
    options: Vec<String>,
    selected: usize,
    alt: String,
    enabled: bool,
    on_select: EventHandler<usize>,
}

impl PopUp {
    pub fn new(
        options: Vec<String>,
        selected: usize,
        alt: impl Into<String>,
        on_select: impl Into<EventHandler<usize>>,
    ) -> Self {
        Self {
            options,
            selected,
            alt: alt.into(),
            enabled: true,
            on_select: on_select.into(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for PopUp {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut open = use_state(|| false);
        let enabled = self.enabled;
        let current = self.options.get(self.selected).cloned().unwrap_or_default();
        let entries: Rc<Vec<MenuEntry>> = Rc::new(
            self.options
                .iter()
                .enumerate()
                .map(|(i, o)| {
                    let item = MenuItem::new(i as u64, o.clone());
                    MenuEntry::Item(if i == self.selected {
                        item.mark(crate::menu::Mark::Check)
                    } else {
                        item
                    })
                })
                .collect(),
        );
        let on_select = self.on_select.clone();
        let button = rect()
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::ComboBox)
            .a11y_alt(self.alt.clone())
            .height(Size::px(t.size_control))
            .padding((0., t.space_xs, 0., t.space_m))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .corner_radius(t.radius_control)
            .background(colour(t.colour_control))
            .maybe(enabled, |el| {
                shadows(&t.shadow_control).into_iter().fold(el, |el, s| el.shadow(s))
            })
            .maybe(focus() == Focus::Keyboard, |el| el.border(focus_ring(&theme)))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                if enabled {
                    open.toggle();
                }
            })
            .child(
                rect().min_width(Size::px(120. - t.space_m - t.space_xs - t.space_s - 16.)).child(
                    text::line(
                        &theme,
                        text::Style::Body,
                        colour(if enabled { t.colour_text } else { t.colour_text_disabled }),
                        current,
                    ),
                ),
            )
            .child(
                rect()
                    .width(Size::px(16.))
                    .height(Size::px(16.))
                    .corner_radius(t.radius_small)
                    .background(colour(if enabled { t.colour_accent } else { t.colour_fill_strong }))
                    .center()
                    .child(Glyph::new("caret-up-down", 10., colour(t.colour_on_accent)).fill(true)),
            );
        Attached::new(button)
            .bottom()
            .maybe_child((*open.read()).then(|| {
                Menu::new(
                    entries,
                    move |event| match event {
                        MenuEvent::Chosen(i) => {
                            open.set(false);
                            on_select.call(i as usize);
                        }
                        MenuEvent::Back => open.set(false),
                        _ => {}
                    },
                )
                .material(Material::Window)
                .first_selected(true)
            }))
    }
}

/// A number read exactly: a field with a stepper and the unit after it.
#[derive(Clone, PartialEq)]
pub struct Stepper {
    value: i64,
    range: (i64, i64),
    step: i64,
    unit: Option<String>,
    alt: String,
    on_change: EventHandler<i64>,
}

impl Stepper {
    pub fn new(
        value: i64,
        range: (i64, i64),
        alt: impl Into<String>,
        on_change: impl Into<EventHandler<i64>>,
    ) -> Self {
        Self {
            value,
            range,
            step: 1,
            unit: None,
            alt: alt.into(),
            on_change: on_change.into(),
        }
    }

    pub fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

impl Component for Stepper {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let (value, (low, high), step) = (self.value, self.range, self.step);
        let half = |up: bool, on_change: EventHandler<i64>, alt: String| {
            let next = if up { (value + step).min(high) } else { (value - step).max(low) };
            rect()
                .width(Size::px(14.))
                .height(Size::px(12.))
                .center()
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(alt)
                .on_press(move |_| {
                    if next != value {
                        on_change.call(next);
                    }
                })
                .child(Glyph::new(if up { "caret-up" } else { "caret-down" }, 8., colour(t.colour_text_2)).fill(true))
        };
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(
                rect()
                    .width(Size::px(64.))
                    .height(Size::px(t.size_control))
                    .padding((0., t.space_s))
                    .corner_radius(t.radius_control)
                    .background(colour(t.colour_fill))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .main_align(Alignment::End)
                    .a11y_role(AccessibilityRole::SpinButton)
                    .a11y_alt(self.alt.clone())
                    .child(text::line(&theme, text::Style::Body, colour(t.colour_text), value.to_string())),
            )
            .child(
                rect()
                    .child(half(true, self.on_change.clone(), format!("Increase {}", self.alt)))
                    .child(half(false, self.on_change.clone(), format!("Decrease {}", self.alt))),
            )
            .map(self.unit.clone(), |el, unit| {
                el.child(text::line(&theme, text::Style::Body, colour(t.colour_text_2), unit))
            })
    }
}

/// A row of colours to choose from; the chosen one holds a dot in its `on` colour.
#[derive(Clone, PartialEq)]
pub struct Swatches {
    /// Each swatch's name, fill and the colour drawn on it.
    swatches: Vec<(String, Color, Color)>,
    selected: Option<usize>,
    on_select: EventHandler<usize>,
}

impl Swatches {
    pub fn new(
        swatches: Vec<(String, Color, Color)>,
        selected: Option<usize>,
        on_select: impl Into<EventHandler<usize>>,
    ) -> Self {
        Self {
            swatches,
            selected,
            on_select: on_select.into(),
        }
    }
}

impl Component for Swatches {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .a11y_role(AccessibilityRole::RadioGroup)
            .children(self.swatches.iter().enumerate().map(|(i, (name, fill, on))| {
                let on_select = self.on_select.clone();
                let chosen = self.selected == Some(i);
                rect()
                    .key(i)
                    .width(Size::px(24.))
                    .height(Size::px(24.))
                    .center()
                    .a11y_role(AccessibilityRole::RadioButton)
                    .a11y_alt(name.clone())
                    .on_press(move |_| on_select.call(i))
                    .child(
                        rect()
                            .width(Size::px(16.))
                            .height(Size::px(16.))
                            .corner_radius(8.)
                            .background(*fill)
                            .center()
                            .maybe(chosen, |el| {
                                el.child(
                                    rect()
                                        .width(Size::px(t.size_mark))
                                        .height(Size::px(t.size_mark))
                                        .corner_radius(t.size_mark / 2.)
                                        .background(*on),
                                )
                            }),
                    )
                    .into_element()
            }))
    }
}

/// The card at the top of a pane while `settings.kdl` or `tokens.kdl` holds a rejected
/// edit: the message, the parser's words and the two ways to it.
#[derive(Clone, PartialEq)]
pub struct Notice {
    pub message: String,
    pub detail: String,
    pub actions: Vec<(String, EventHandler<()>)>,
}

impl Component for Notice {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .padding(t.space_m)
            .horizontal()
            .content(Content::flex())
            .spacing(t.space_m)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_raised))
            .border(
                Border::new()
                    .fill(colour(t.colour_warn))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
            .a11y_role(AccessibilityRole::Alert)
            .child(Glyph::new("warning-circle", t.size_glyph_large, colour(t.colour_warn)).fill(true))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .spacing(t.space_xxs)
                    .child(
                        label()
                            .font_size(text::metrics(&theme, text::Style::Body).0)
                            .color(colour(t.colour_text))
                            .text(self.message.clone()),
                    )
                    .child(
                        label()
                            .font_family("JetBrains Mono")
                            .font_size(text::metrics(&theme, text::Style::Callout).0)
                            .color(colour(t.colour_text_2))
                            .text(self.detail.clone()),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .margin((t.space_m - t.space_xxs, 0., 0., 0.))
                            .spacing(t.space_s)
                            .children(self.actions.iter().map(|(label, on_press)| {
                                crate::Button::new(label.clone())
                                    .on_press(on_press.clone())
                                    .into_element()
                            })),
                    ),
            )
    }
}
