//! A pane's rows bound to their settings, and the sheets panes open (`p13/rows.md`).

use freya::prelude::*;
use isle_app::Desktop;
use isle_settings::Value;
use isle_ui::{Glyph, Slider, Switch, button, colour, form, text, use_theme};

use crate::nav::Location;

/// View › Show Changed Settings, and the defaults rows are compared with.
#[derive(Clone)]
pub struct Changed {
    pub shown: State<bool>,
    pub defaults: std::rc::Rc<std::collections::BTreeMap<String, Value>>,
}

impl Changed {
    pub fn new() -> Self {
        let mut defaults = std::collections::BTreeMap::new();
        isle_settings::Section::flatten(
            &isle_settings::schema::Settings::default(),
            "",
            &mut defaults,
        );
        Self {
            shown: State::create(false),
            defaults: std::rc::Rc::new(defaults),
        }
    }

    /// Whether `key`, or any key in the section it names, differs from its default.
    pub fn differs(&self, values: &std::collections::BTreeMap<String, Value>, key: &str) -> bool {
        let section = format!("{key}.");
        let keys = values
            .keys()
            .chain(self.defaults.keys())
            .filter(|k| *k == key || k.starts_with(&section));
        keys.into_iter()
            .any(|k| values.get(k) != self.defaults.get(k))
    }
}

/// What a pane's rows read: the desktop's values, the row a search found and the last refusal.
#[derive(Clone)]
pub struct Rows {
    pub desktop: Desktop,
    pub found: Option<String>,
    changed: Option<Changed>,
}

impl Rows {
    pub fn new(at: &Location) -> Self {
        Self {
            desktop: Desktop::get(),
            found: at.row.clone(),
            changed: use_try_consume::<Changed>(),
        }
    }

    pub fn bool(&self, key: &str) -> bool {
        matches!(self.desktop.value(key), Some(Value::Bool(true)))
    }

    /// A keyed entry's switch, which reads as its default until the entry exists.
    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        match self.desktop.value(key) {
            Some(Value::Bool(b)) => b,
            _ => default,
        }
    }

    /// A switch over a key built at run time, a keyed entry's, with its default.
    pub fn switch_key(
        &self,
        id: &str,
        label: &str,
        key: String,
        default: bool,
        enabled: bool,
    ) -> form::Row {
        let desktop = self.desktop;
        let on = self.bool_or(&key, default);
        self.row(id, label, Some(&key)).enabled(enabled).control(
            Switch::new(on, label, move |on| {
                desktop.set(key.clone(), Value::Bool(on))
            })
            .enabled(enabled),
        )
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        match self.desktop.value(key)? {
            Value::Int(i) => Some(i),
            Value::Float(f) => Some(f as i64),
            _ => None,
        }
    }

    pub fn float(&self, key: &str) -> Option<f64> {
        match self.desktop.value(key)? {
            Value::Float(f) => Some(f),
            Value::Int(i) => Some(i as f64),
            _ => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<String> {
        match self.desktop.value(key)? {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn list(&self, key: &str) -> Vec<String> {
        match self.desktop.value(key) {
            Some(Value::List(l)) => l,
            _ => Vec::new(),
        }
    }

    /// A row with its found ring and, for `key`, the compositor's last refusal.
    pub fn row(&self, id: &str, label: impl Into<String>, key: Option<&str>) -> form::Row {
        let refused = key.and_then(|key| {
            self.desktop
                .refused
                .read()
                .clone()
                .filter(|(k, _)| k == key)
                .map(|(_, why)| why)
        });
        let differs = key
            .zip(self.changed.as_ref())
            .is_some_and(|(key, c)| c.differs(&self.desktop.values.read(), key));
        let shown = self.changed.as_ref().is_some_and(|c| *c.shown.read());
        let desktop = self.desktop;
        let reset = key.filter(|_| differs).map(|key| {
            let key = key.to_owned();
            EventHandler::new(move |_| desktop.reset(key.clone()))
        });
        form::Row::new(label)
            .found(self.found.as_deref() == Some(id))
            .refused(refused)
            .marked(shown && differs)
            .on_reset(reset)
    }

    pub fn switch(&self, id: &str, label: &str, key: &'static str) -> form::Row {
        let desktop = self.desktop;
        self.row(id, label, Some(key))
            .control(Switch::new(self.bool(key), label, move |on| {
                desktop.set(key, Value::Bool(on))
            }))
    }

    /// A switch whose state and change are the pane's own.
    pub fn switch_with(
        &self,
        id: &str,
        label: &str,
        on: bool,
        f: impl Fn(bool) + 'static,
    ) -> form::Row {
        self.row(id, label, None)
            .control(Switch::new(on, label, move |on| f(on)))
    }

    /// A pop-up over a string setting: `options` are each value and its label. A value off
    /// the list, typed by hand, is shown as an item of its own.
    pub fn choice(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        options: &[(&'static str, &'static str)],
    ) -> form::Row {
        let current = self.str(key).unwrap_or_default();
        self.pick(id, label, key, current, options, Value::Str)
    }

    /// A pop-up over a whole-number setting, the values as strings.
    pub fn choice_int(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        options: &[(&'static str, &'static str)],
    ) -> form::Row {
        let current = self.int(key).map(|i| i.to_string()).unwrap_or_default();
        self.pick(id, label, key, current, options, |s| {
            Value::Int(s.parse().unwrap_or(0))
        })
    }

    fn pick(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        current: String,
        options: &[(&'static str, &'static str)],
        value: fn(String) -> Value,
    ) -> form::Row {
        let mut values: Vec<String> = options.iter().map(|(v, _)| (*v).to_owned()).collect();
        let mut labels: Vec<String> = options.iter().map(|(_, l)| (*l).to_owned()).collect();
        let selected = match values.iter().position(|v| *v == current) {
            Some(i) => i,
            None => {
                values.push(current.clone());
                labels.push(current);
                values.len() - 1
            }
        };
        let desktop = self.desktop;
        self.row(id, label, Some(key)).control(form::PopUp::new(
            labels,
            selected,
            label,
            move |i: usize| desktop.set(key, value(values[i].clone())),
        ))
    }

    pub fn segmented(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        options: &[(&'static str, &'static str)],
    ) -> form::Row {
        let current = self.str(key).unwrap_or_default();
        let selected = options.iter().position(|(v, _)| *v == current).unwrap_or(0);
        let values: Vec<&'static str> = options.iter().map(|(v, _)| *v).collect();
        let desktop = self.desktop;
        self.row(id, label, Some(key)).control(form::Segmented::new(
            options.iter().map(|(_, l)| (*l).to_owned()).collect(),
            selected,
            label,
            move |i: usize| desktop.set(key, Value::Str(values[i].to_owned())),
        ))
    }

    /// A slider over a number from `lo` to `hi`, its ends named.
    pub fn slider(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        (lo, hi): (f64, f64),
        words: (&str, &str),
    ) -> form::Row {
        let value = self.float(key).unwrap_or(lo);
        let desktop = self.desktop;
        let at = ((value - lo) / (hi - lo)) as f32;
        self.row(id, label, Some(key)).control(Ranged {
            value: at,
            alt: label.to_owned(),
            words: (words.0.to_owned(), words.1.to_owned()),
            ticks: 0,
            on_change: EventHandler::new(move |v: f32| {
                let v = lo + f64::from(v) * (hi - lo);
                desktop.set(key, Value::Float((v * 100.).round() / 100.))
            }),
        })
    }

    /// A slider over a keyed entry's number, built at run time, with its default.
    pub fn slider_key(
        &self,
        id: &str,
        label: &str,
        key: String,
        (lo, hi): (f64, f64),
        words: (&str, &str),
        default: f64,
    ) -> form::Row {
        let value = self.float(&key).unwrap_or(default);
        let desktop = self.desktop;
        let at = ((value - lo) / (hi - lo)) as f32;
        self.row(id, label, Some(&key)).control(Ranged {
            value: at,
            alt: label.to_owned(),
            words: (words.0.to_owned(), words.1.to_owned()),
            ticks: 0,
            on_change: EventHandler::new(move |v: f32| {
                let v = lo + f64::from(v) * (hi - lo);
                desktop.set(key.clone(), Value::Float((v * 100.).round() / 100.))
            }),
        })
    }

    /// A slider over a whole number that stops only at `steps`, ticks under it.
    pub fn slider_steps(
        &self,
        id: &str,
        label: &str,
        key: &'static str,
        steps: &'static [i64],
        words: (&str, &str),
    ) -> form::Row {
        let value = self.int(key).unwrap_or(steps[0]);
        let desktop = self.desktop;
        self.row(id, label, Some(key)).control(Ranged {
            value: step_position(steps, value),
            alt: label.to_owned(),
            words: (words.0.to_owned(), words.1.to_owned()),
            ticks: steps.len(),
            on_change: EventHandler::new(move |v: f32| {
                desktop.set(key, Value::Int(steps[step_index(steps.len(), v)]))
            }),
        })
    }

    /// A read-only row: its label and value.
    pub fn info(&self, id: &str, label: &str, value: impl Into<String>) -> form::Row {
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        self.row(id, label, None).control(text::line(
            &theme,
            text::Style::Body,
            colour(t.colour_text_2),
            value.into(),
        ))
    }
}

/// Where along a track the step nearest `value` sits.
pub fn step_position(steps: &[i64], value: i64) -> f32 {
    let nearest = steps
        .iter()
        .enumerate()
        .min_by_key(|(_, s)| (*s - value).abs())
        .map_or(0, |(i, _)| i);
    nearest as f32 / (steps.len().max(2) - 1) as f32
}

/// The step a track position falls on.
pub fn step_index(count: usize, at: f32) -> usize {
    ((at * (count.max(2) - 1) as f32).round() as usize).min(count - 1)
}

/// A settings slider: the track between its two end words, ticks under it for steps.
#[derive(Clone, PartialEq)]
pub struct Ranged {
    pub value: f32,
    pub alt: String,
    pub words: (String, String),
    pub ticks: usize,
    pub on_change: EventHandler<f32>,
}

impl Component for Ranged {
    fn render(&self) -> impl IntoElement {
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        let word = |w: String| text::line(&theme, text::Style::Callout, colour(t.colour_text_2), w);
        let step = if self.ticks > 1 {
            1. / (self.ticks - 1) as f32
        } else {
            1. / 16.
        };
        let on_change = self.on_change.clone();
        let ticks = self.ticks;
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(word(self.words.0.clone()))
            .child(
                rect()
                    .width(Size::px(200.))
                    .spacing(t.space_xxs)
                    .child(
                        Slider::new(self.value, self.alt.clone(), move |v: f32| {
                            let v = if ticks > 1 {
                                step_index(ticks, v) as f32 * step
                            } else {
                                v
                            };
                            on_change.call(v)
                        })
                        .step(step),
                    )
                    .maybe(ticks > 1, |el| {
                        el.child(
                            rect()
                                .width(Size::fill())
                                .padding((0., 9.))
                                .horizontal()
                                .main_align(Alignment::SpaceBetween)
                                .children((0..ticks).map(|i| {
                                    rect()
                                        .key(i)
                                        .width(Size::px(1.))
                                        .height(Size::px(4.))
                                        .background(colour(t.colour_text_3))
                                        .into_element()
                                })),
                        )
                    }),
            )
            .child(word(self.words.1.clone()))
    }
}

/// Choices shown as small pictures with their names under them: Appearance's Auto, Light
/// and Dark; Wallpaper's kinds.
#[derive(Clone, PartialEq)]
pub struct Pictures {
    pub choices: Vec<(String, Picture)>,
    pub selected: Option<usize>,
    pub alt: String,
    pub on_select: EventHandler<usize>,
}

/// What a picture draws: a window over a backdrop in two colours, split down the middle
/// for both.
#[derive(Clone, Copy, PartialEq)]
pub struct Picture {
    pub backdrop: (Color, Color),
    pub window: (Color, Color),
}

impl Component for Pictures {
    fn render(&self) -> impl IntoElement {
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        rect()
            .horizontal()
            .spacing(t.space_l)
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt(self.alt.clone())
            .children(self.choices.iter().enumerate().map(|(i, (name, picture))| {
                let chosen = self.selected == Some(i);
                let on_select = self.on_select.clone();
                let half = |backdrop: Color, window: Color| {
                    rect()
                        .width(Size::px(32.))
                        .height(Size::fill())
                        .background(backdrop)
                        .padding((10., 0., 0., 8.))
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::fill())
                                .background(window),
                        )
                };
                rect()
                    .key(i)
                    .spacing(t.space_xs)
                    .cross_align(Alignment::Center)
                    .a11y_role(AccessibilityRole::RadioButton)
                    .a11y_alt(name.clone())
                    .on_press(move |_| on_select.call(i))
                    .child(
                        rect()
                            .width(Size::px(64.))
                            .height(Size::px(44.))
                            .horizontal()
                            .corner_radius(t.radius_control)
                            .overflow(Overflow::Clip)
                            .border(
                                Border::new()
                                    .fill(colour(if chosen {
                                        t.colour_accent
                                    } else {
                                        t.colour_hairline
                                    }))
                                    .width(if chosen {
                                        t.stroke_found
                                    } else {
                                        t.stroke_hairline
                                    })
                                    .alignment(BorderAlignment::Outer),
                            )
                            .child(half(picture.backdrop.0, picture.window.0))
                            .child(half(picture.backdrop.1, picture.window.1)),
                    )
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(if chosen {
                            t.colour_text
                        } else {
                            t.colour_text_2
                        }),
                        name.clone(),
                    ))
                    .into_element()
            }))
    }
}

/// An app's icon at `size`, or a plain app glyph when it has none.
pub fn app_icon(icon: Option<std::path::PathBuf>, size: f32) -> Element {
    let theme = isle_ui::theme();
    match icon {
        Some(path) => isle_ui::AppIcon::new(path, size).into_element(),
        None => Glyph::new("app-window", size, colour(theme.tokens.colour_text_2)).into_element(),
    }
}

/// A sub-page's header with an app's icon in place of a glyph.
pub fn app_header(icon: Option<std::path::PathBuf>, name: &str, subtitle: &str) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(app_icon(icon, 32.))
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t.space_xxs)
                .child(text::line(
                    &theme,
                    text::Style::Title2,
                    colour(t.colour_text),
                    name,
                ))
                .child(
                    label()
                        .font_size(text::metrics(&theme, text::Style::Body).0)
                        .color(colour(t.colour_text_2))
                        .text(subtitle.to_owned()),
                ),
        )
        .into_element()
}

/// A list a user adds to and takes from: its rows, one selected, + and − under it.
#[derive(Clone, PartialEq)]
pub struct ListEditor {
    pub title: Option<String>,
    /// Each item's label and, for an app, its icon.
    pub items: Vec<(String, Option<Option<std::path::PathBuf>>)>,
    pub empty: String,
    pub on_add: EventHandler<()>,
    pub on_remove: EventHandler<usize>,
}

impl Component for ListEditor {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut selected = use_state(|| None::<usize>);
        let count = self.items.len();
        let chosen = selected.read().filter(|i| *i < count);
        let rows: Vec<Element> = if self.items.is_empty() {
            vec![
                form::Row::new(self.empty.clone())
                    .enabled(false)
                    .into_element(),
            ]
        } else {
            self.items
                .iter()
                .enumerate()
                .map(|(i, (label, icon))| {
                    let on = chosen == Some(i);
                    rect()
                        .key(i)
                        .width(Size::fill())
                        .min_height(Size::px(t.size_row))
                        .padding((t.space_xs, t.space_m))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .maybe(on, |el| el.background(colour(t.colour_accent)))
                        .a11y_role(AccessibilityRole::ListItem)
                        .a11y_alt(label.clone())
                        .on_press(move |_| selected.set(Some(i)))
                        .map(icon.clone(), |el, icon| el.child(app_icon(icon, 20.)))
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(if on {
                                t.colour_on_accent
                            } else {
                                t.colour_text
                            }),
                            label.clone(),
                        ))
                        .into_element()
                })
                .collect()
        };
        let (on_add, on_remove) = (self.on_add.clone(), self.on_remove.clone());
        let tool = |glyph: &'static str, alt: &'static str, enabled: bool, f: EventHandler<()>| {
            rect()
                .width(Size::px(28.))
                .height(Size::px(24.))
                .center()
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(alt)
                .on_press(move |_| {
                    if enabled {
                        f.call(())
                    }
                })
                .child(Glyph::new(
                    glyph,
                    14.,
                    colour(if enabled {
                        t.colour_text
                    } else {
                        t.colour_text_disabled
                    }),
                ))
                .into_element()
        };
        rect()
            .width(Size::fill())
            .spacing(t.space_xs)
            .child(form::group(self.title.clone(), rows))
            .child(
                rect()
                    .horizontal()
                    .child(tool("plus", "Add", true, on_add))
                    .child(tool(
                        "minus",
                        "Remove",
                        chosen.is_some(),
                        EventHandler::new(move |_| {
                            if let Some(i) = chosen {
                                selected.set(None);
                                on_remove.call(i)
                            }
                        }),
                    )),
            )
    }
}

/// A sheet listing the installed apps to choose one: Spotlight's favourites, Focus's
/// allowed apps.
#[derive(Clone, PartialEq)]
pub struct AppPicker {
    pub title: String,
    pub on_pick: EventHandler<String>,
    pub on_cancel: EventHandler<()>,
}

impl Component for AppPicker {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let services = crate::services::use_follow(&[crate::services::Follow::Apps]);
        let query = use_state(String::new);
        let mut chosen = use_state(|| None::<String>);
        let q = query.read().to_lowercase();
        let apps: Vec<crate::services::App> = services
            .apps
            .read()
            .state
            .iter()
            .filter(|a| a.listed && (q.is_empty() || a.name.to_lowercase().contains(&q)))
            .cloned()
            .collect();
        let pick = chosen.read().clone();
        let on_pick = self.on_pick.clone();
        sheet(
            440.,
            vec![
                text::line(
                    &theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    self.title.clone(),
                )
                .into_element(),
                isle_ui::TextField::new(query, "Search apps")
                    .placeholder("Search apps")
                    .into_element(),
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::px(280.))
                    .child(
                        rect()
                            .width(Size::fill())
                            .children(apps.into_iter().map(|app| {
                                let on = pick.as_deref() == Some(app.id.as_str());
                                let id = app.id.clone();
                                rect()
                                    .key(app.id.clone())
                                    .width(Size::fill())
                                    .padding((t.space_xs, t.space_s))
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(t.space_s)
                                    .corner_radius(t.radius_row)
                                    .maybe(on, |el| el.background(colour(t.colour_accent)))
                                    .a11y_role(AccessibilityRole::ListItem)
                                    .a11y_alt(app.name.clone())
                                    .on_press(move |_| chosen.set(Some(id.clone())))
                                    .child(app_icon(app.icon.clone(), 20.))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Body,
                                        colour(if on {
                                            t.colour_on_accent
                                        } else {
                                            t.colour_text
                                        }),
                                        app.name.clone(),
                                    ))
                                    .into_element()
                            })),
                    )
                    .into_element(),
            ],
            vec![
                ("Cancel".into(), self.on_cancel.clone()),
                (
                    "Add".into(),
                    EventHandler::new(move |_| {
                        if let Some(id) = chosen.peek().clone() {
                            on_pick.call(id)
                        }
                    }),
                ),
            ],
        )
    }
}

/// A sheet asking for one line of text: a folder's path, a web search's address.
#[derive(Clone, PartialEq)]
pub struct Ask {
    pub title: String,
    pub placeholder: String,
    pub initial: String,
    pub on_done: EventHandler<String>,
    pub on_cancel: EventHandler<()>,
}

impl Component for Ask {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let initial = self.initial.clone();
        let value = use_state(move || initial);
        let on_done = self.on_done.clone();
        let submit = self.on_done.clone();
        sheet(
            400.,
            vec![
                text::line(
                    &theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    self.title.clone(),
                )
                .into_element(),
                isle_ui::TextField::new(value, self.title.clone())
                    .placeholder(self.placeholder.clone())
                    .on_submit(move |text: String| submit.call(text))
                    .into_element(),
            ],
            vec![
                ("Cancel".into(), self.on_cancel.clone()),
                (
                    "OK".into(),
                    EventHandler::new(move |_| on_done.call(value.peek().clone())),
                ),
            ],
        )
    }
}

/// A row's ⋯ button and the menu it opens: each item's label and whether it is checked.
#[derive(Clone, PartialEq)]
pub struct MoreMenu {
    pub alt: String,
    pub items: Vec<(String, bool)>,
    pub on_choose: EventHandler<usize>,
}

impl Component for MoreMenu {
    fn render(&self) -> impl IntoElement {
        use isle_ui::menu::{Mark, Material, Menu, MenuEntry, MenuEvent, MenuItem};
        let theme = use_theme();
        let t = &theme.tokens;
        let mut open = use_state(|| false);
        let entries: std::rc::Rc<Vec<MenuEntry>> = std::rc::Rc::new(
            self.items
                .iter()
                .enumerate()
                .map(|(i, (label, checked))| {
                    let item = MenuItem::new(i as u64, label.clone());
                    MenuEntry::Item(if *checked {
                        item.mark(Mark::Check)
                    } else {
                        item
                    })
                })
                .collect(),
        );
        let on_choose = self.on_choose.clone();
        let button = rect()
            .width(Size::px(24.))
            .height(Size::px(24.))
            .center()
            .corner_radius(12.)
            .background(colour(t.colour_fill))
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(self.alt.clone())
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                open.toggle();
            })
            .child(Glyph::new("dots-three", 14., colour(t.colour_text)));
        Attached::new(button)
            .bottom()
            .maybe_child((*open.read()).then(|| {
                Menu::new(entries, move |event| match event {
                    MenuEvent::Chosen(i) => {
                        open.set(false);
                        on_choose.call(i as usize);
                    }
                    MenuEvent::Back => open.set(false),
                    _ => {}
                })
                .material(Material::Window)
            }))
    }
}

/// A group's title standing alone, over a list that is its own group.
pub fn heading(title: &str) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .padding((0., t.space_m))
        .margin((0., 0., -t.space_s, 0.))
        .child(text::line(
            &theme,
            text::Style::Headline,
            colour(t.colour_text),
            title,
        ))
        .into_element()
}

/// Buttons under a group, trailing: "Hot Corners…", "Detect Displays".
pub fn foot(buttons: Vec<(String, EventHandler<()>)>) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .margin((-t.space_m, 0., 0., 0.))
        .horizontal()
        .main_align(Alignment::End)
        .spacing(t.space_s)
        .children(buttons.into_iter().map(|(label, on_press)| {
            isle_ui::Button::new(label)
                .on_press(on_press)
                .into_element()
        }))
        .into_element()
}

/// A pane's state while what it shows is missing: "No Wi-Fi device is connected."
pub fn missing(message: &str) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding((t.space_xxl, 0.))
        .cross_align(Alignment::Center)
        .child(text::line(
            &theme,
            text::Style::Body,
            colour(t.colour_text_2),
            message,
        ))
        .into_element()
}

/// A sub-page's header: its glyph or icon, its name and one sentence.
pub fn sub_header(glyph: &str, name: &str, subtitle: &str) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(Glyph::new(glyph, 32., colour(t.colour_text_2)))
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t.space_xxs)
                .child(text::line(
                    &theme,
                    text::Style::Title2,
                    colour(t.colour_text),
                    name,
                ))
                .child(
                    label()
                        .font_size(text::metrics(&theme, text::Style::Body).0)
                        .color(colour(t.colour_text_2))
                        .text(subtitle.to_owned()),
                ),
        )
        .into_element()
}

/// An alert's words: its question and what follows.
pub fn alert_text(title: &str, message: &str) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .spacing(t.space_s)
        .child(text::line(
            &theme,
            text::Style::Headline,
            colour(t.colour_text),
            title,
        ))
        .child(
            label()
                .font_size(text::metrics(&theme, text::Style::Body).0)
                .color(colour(t.colour_text_2))
                .text(message.to_owned()),
        )
        .into_element()
}

/// A sheet over the window: the body, then its buttons trailing, the last the default.
pub fn sheet(width: f32, body: Vec<Element>, buttons: Vec<(String, EventHandler<()>)>) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let count = buttons.len();
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
        .child(
            rect()
                .width(Size::px(width))
                .padding(t.space_xl)
                .spacing(t.space_l)
                .corner_radius(t.radius_window)
                .background(colour(t.colour_window))
                .maybe(true, |el| {
                    isle_ui::shadows(&t.shadow_window)
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
                        .children(
                            buttons
                                .into_iter()
                                .enumerate()
                                .map(|(i, (label, on_press))| {
                                    let kind = if i + 1 == count {
                                        button::Kind::Default
                                    } else {
                                        button::Kind::Regular
                                    };
                                    isle_ui::Button::new(label)
                                        .kind(kind)
                                        .on_press(on_press)
                                        .into_element()
                                }),
                        ),
                ),
        )
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_snap_to_the_nearest() {
        let steps = [2, 5, 10, 15, 20, 25, 30, 40, 60];
        assert_eq!(step_position(&steps, 2), 0.);
        assert_eq!(step_position(&steps, 60), 1.);
        assert_eq!(step_position(&steps, 33), 6. / 8.);
        assert_eq!(steps[step_index(steps.len(), 0.49)], 20);
        assert_eq!(steps[step_index(steps.len(), 1.)], 60);
    }
}
