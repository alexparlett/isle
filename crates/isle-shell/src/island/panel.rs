//! Control Centre: the panel that unfolds from the pill, its top row the pill's own, then
//! Now, the module grid and the footer (`surfaces/island.md`, Control Centre).

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, button::Kind, colour, focus_ring, text, use_theme};

use super::{
    IslandAction,
    grid::{self, TileSize},
    model::{Activity, Privacy},
};
use crate::{Action, Actions};

/// A notice that holds until its state ends.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    /// `settings.kdl` or `tokens.kdl` rejected, or a safe start.
    ConfigError {
        message: String,
        file: String,
        line: u32,
        column: u32,
        safe_start: bool,
    },
    /// A window holds the keys; the chord that takes them back.
    Inhibited { app: String, chord: String },
}

/// A row on a module's page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageRow {
    pub id: String,
    pub label: String,
    pub checked: bool,
}

/// A module as the grid shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub id: String,
    pub size: TileSize,
    pub name: String,
    pub state: String,
    pub glyph: String,
    pub on: bool,
    /// Its glyph is a switch; else a click opens its page.
    pub toggles: bool,
    pub page: Option<Vec<PageRow>>,
    /// The Settings pane its page's foot opens.
    pub pane: String,
}

/// Control Centre as one output's island shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelFrame {
    /// Opened by a click or Ctrl+F8, with the keyboard; a peek has neither.
    pub pinned: bool,
    /// The page open, by module id.
    pub page: Option<String>,
    pub notices: Vec<Notice>,
    pub activities: Vec<Activity>,
    pub privacy: Vec<(Privacy, String)>,
    pub modules: Vec<Module>,
}

/// What the panel asks of the compositor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelAction {
    /// A module's switch.
    Toggle(String),
    /// A module's page, or back from it.
    Page(Option<String>),
    /// A row on a module's page.
    Choose { module: String, row: String },
    /// A notice's button: `editor`, `files`, `use-mine`, `take-back`.
    Notice(String),
    /// An activity's button.
    Activity { kind: String, action: String },
    /// The page's foot: the module's Settings pane.
    Settings(String),
}

const LEADER_ROW: f32 = 56.0;
const NOTICE_ROW: f32 = 64.0;
const INHIBIT_ROW: f32 = 56.0;

fn grid_height(theme: &Theme, modules: &[Module]) -> f32 {
    let t = &theme.tokens;
    let sizes: Vec<TileSize> = modules.iter().map(|m| m.size).collect();
    let (_, rows) = grid::pack(&sizes);
    if rows == 0 {
        return 0.0;
    }
    rows as f32 * t.size_module_cell_height + (rows as f32 - 1.0) * t.space_s
}

fn now_height(panel: &PanelFrame) -> f32 {
    let notices: f32 = panel
        .notices
        .iter()
        .map(|n| match n {
            Notice::ConfigError { .. } => NOTICE_ROW,
            Notice::Inhibited { .. } => INHIBIT_ROW,
        })
        .sum();
    let rows = panel.activities.len() as f32 * LEADER_ROW + panel.privacy.len() as f32 * 28.0;
    notices + rows
}

/// The panel's size under the pill's row of height `pill`, at least `pill_width` wide.
pub fn panel_size(
    theme: &Theme,
    panel: &PanelFrame,
    pill: f32,
    pill_width: f32,
    room: f32,
) -> (f32, f32) {
    let t = &theme.tokens;
    let width = t.size_panel.max(pill_width);
    let pad = t.space_s;
    let body = match panel
        .page
        .as_ref()
        .and_then(|id| panel.modules.iter().find(|m| &m.id == id))
    {
        Some(module) => {
            let rows = module.page.as_ref().map_or(0, |r| r.len()) as f32;
            t.size_module_cell_height + pad + rows * t.size_row + 2.0 * pad + t.size_row
        }
        None => {
            let now = now_height(panel);
            let grid = grid_height(theme, &panel.modules);
            let mut h = 0.0;
            if now > 0.0 {
                h += now + pad;
            }
            if grid > 0.0 {
                h += grid + pad;
            }
            h + t.size_row
        }
    };
    let height = (pill + pad + body + pad).min(room);
    (width.ceil(), height.ceil())
}

/// The panel's body below the pill's row.
#[derive(PartialEq)]
pub struct PanelBody {
    pub panel: PanelFrame,
}

fn send(actions: &Actions, action: PanelAction) {
    actions.send(Action::Island(IslandAction::Panel(action)));
}

impl Component for PanelBody {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let panel = &self.panel;
        let card = || {
            rect()
                .width(Size::fill())
                .corner_radius(t.radius_card)
                .background(colour(t.colour_fill))
        };
        let page = panel
            .page
            .as_ref()
            .and_then(|id| panel.modules.iter().find(|m| &m.id == id));
        if let Some(module) = page {
            let back = actions.clone();
            let settings = actions.clone();
            let pane = module.pane.clone();
            let rows = module.page.clone().unwrap_or_default();
            return rect()
                .width(Size::fill())
                .padding(t.space_s)
                .spacing(t.space_s)
                .child(
                    card()
                        .height(Size::px(t.size_module_cell_height))
                        .padding((0., t.space_l))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(
                            Button::icon("caret-left", "Back")
                                .kind(Kind::Plain)
                                .on_press(move |_| send(&back, PanelAction::Page(None))),
                        )
                        .child(module_circle(&theme, module))
                        .child(text::line(
                            &theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            module.name.clone(),
                        )),
                )
                .child(
                    card()
                        .padding((t.space_xs, 0.))
                        .children(rows.into_iter().map(|row| {
                            let actions = actions.clone();
                            let module = module.id.clone();
                            let id = row.id.clone();
                            Row {
                                label: row.label,
                                checked: row.checked,
                                on_press: (move |_| {
                                    send(
                                        &actions,
                                        PanelAction::Choose {
                                            module: module.clone(),
                                            row: id.clone(),
                                        },
                                    )
                                })
                                .into(),
                            }
                            .into_element()
                        })),
                )
                .child(
                    rect().height(Size::px(t.size_row)).horizontal().child(
                        Button::new(format!("{pane} Settings…"))
                            .kind(Kind::Plain)
                            .small()
                            .on_press(move |_| {
                                send(&settings, PanelAction::Settings(pane.clone()))
                            }),
                    ),
                )
                .into_element();
        }

        let notices = panel
            .notices
            .iter()
            .map(|notice| notice_row(&theme, notice, &actions));
        let activities = panel
            .activities
            .iter()
            .map(|a| activity_row(&theme, a, &actions));
        let privacy = panel.privacy.iter().map(|(mark, app)| {
            let what = match mark {
                Privacy::Microphone => "the microphone",
                Privacy::Camera => "the camera",
                Privacy::Screen => "the screen",
            };
            rect()
                .height(Size::px(t.size_row))
                .padding((0., t.space_m))
                .horizontal()
                .cross_align(Alignment::Center)
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    format!("{app} is using {what}"),
                ))
                .into_element()
        });
        let has_now =
            !panel.notices.is_empty() || !panel.activities.is_empty() || !panel.privacy.is_empty();

        let sizes: Vec<TileSize> = panel.modules.iter().map(|m| m.size).collect();
        let (places, _) = grid::pack(&sizes);
        let cell = (t.size_module_cell_width, t.size_module_cell_height);
        let tiles = panel
            .modules
            .iter()
            .zip(places)
            .map(|(module, (col, row))| {
                let (w, h) = module.size.cells();
                Tile {
                    module: module.clone(),
                    x: col as f32 * (cell.0 + t.space_s),
                    y: row as f32 * (cell.1 + t.space_s),
                    width: w as f32 * cell.0 + (w as f32 - 1.0) * t.space_s,
                    height: h as f32 * cell.1 + (h as f32 - 1.0) * t.space_s,
                }
                .into_element()
            });

        rect()
            .width(Size::fill())
            .padding(t.space_s)
            .spacing(t.space_s)
            .maybe(has_now, |el| {
                el.child(
                    card()
                        .children(notices)
                        .children(activities)
                        .children(privacy),
                )
            })
            .maybe(!panel.modules.is_empty(), |el| {
                el.child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(grid_height(&theme, &panel.modules)))
                        .children(tiles),
                )
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .child(
                        Button::new("Edit controls")
                            .kind(Kind::Plain)
                            .small()
                            .enabled(false),
                    )
                    .child(rect().width(Size::flex(1.)))
                    .child(
                        Button::new("Control Centre Settings…")
                            .kind(Kind::Plain)
                            .small()
                            .enabled(false),
                    ),
            )
            .into_element()
    }
}

fn module_circle(theme: &Theme, module: &Module) -> Rect {
    let t = &theme.tokens;
    let (fill, ink) = if module.on {
        (colour(t.colour_accent), colour(t.colour_on_accent))
    } else {
        (colour(t.colour_fill_strong), colour(t.colour_text))
    };
    rect()
        .width(Size::px(t.size_module_glyph))
        .height(Size::px(t.size_module_glyph))
        .corner_radius(t.size_module_glyph / 2.0)
        .background(fill)
        .center()
        .child(Glyph::new(module.glyph.clone(), t.size_glyph_large, ink).fill(module.on))
}

#[derive(PartialEq)]
struct Tile {
    module: Module,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Component for Tile {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let module = &self.module;
        let id = module.id.clone();
        let toggles = module.toggles;
        let small = module.size == TileSize::Small;
        let open_page = module.page.is_some();
        let press = move |e: Event<PressEventData>| {
            e.stop_propagation();
            let action = if toggles {
                PanelAction::Toggle(id.clone())
            } else if open_page {
                PanelAction::Page(Some(id.clone()))
            } else {
                return;
            };
            send(&actions, action);
        };
        rect()
            .position(Position::new_absolute().left(self.x).top(self.y))
            .width(Size::px(self.width))
            .height(Size::px(self.height))
            .corner_radius(t.radius_card)
            .background(colour(if *hover.read() {
                t.colour_fill_strong
            } else {
                t.colour_fill
            }))
            .a11y_id(a11y)
            .a11y_focusable(true)
            .a11y_role(if toggles {
                AccessibilityRole::Switch
            } else {
                AccessibilityRole::Button
            })
            .a11y_alt(format!("{}, {}", module.name, module.state))
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .on_press(press)
            .horizontal()
            .cross_align(Alignment::Center)
            .maybe(small, |el| el.main_align(Alignment::Center))
            .padding(if small { 0.0 } else { t.space_l })
            .spacing(t.space_s + 4.0)
            .child(module_circle(&theme, module))
            .maybe(!small, |el| {
                el.child(
                    rect()
                        .width(Size::fill())
                        .child(text::line(
                            &theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            module.name.clone(),
                        ))
                        .child(text::line(
                            &theme,
                            text::Style::Subheadline,
                            colour(t.colour_text_2),
                            module.state.clone(),
                        )),
                )
            })
    }
}

#[derive(PartialEq)]
struct Row {
    label: String,
    checked: bool,
    on_press: EventHandler<()>,
}

impl Component for Row {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let on_press = self.on_press.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_m))
            .corner_radius(t.radius_row)
            .background(if *hover.read() {
                colour(t.colour_fill_strong)
            } else {
                Color::TRANSPARENT
            })
            .a11y_id(a11y)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::MenuItemRadio)
            .a11y_alt(self.label.clone())
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                on_press.call(())
            })
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text),
                self.label.clone(),
            )))
            .maybe(self.checked, |el| {
                el.child(Glyph::new("check", t.size_glyph_small, colour(t.colour_text)))
            })
    }
}

fn notice_row(theme: &Theme, notice: &Notice, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let button = |label: &str, id: &str, enabled: bool| {
        let actions = actions.clone();
        let id = id.to_owned();
        Button::new(label)
            .small()
            .enabled(enabled)
            .on_press(move |_| send(&actions, PanelAction::Notice(id.clone())))
            .into_element()
    };
    match notice {
        Notice::ConfigError {
            message,
            file,
            line,
            column,
            safe_start,
        } => rect()
            .width(Size::fill())
            .min_height(Size::px(NOTICE_ROW))
            .padding((t.space_s, t.space_m))
            .spacing(t.space_xs)
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_s)
                    .cross_align(Alignment::Center)
                    .child(
                        Glyph::new("warning-circle", t.size_glyph, colour(t.colour_text))
                            .fill(true),
                    )
                    .child(text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        if *safe_start {
                            "Isle started with its default settings after it stopped several times."
                                .to_string()
                        } else {
                            format!("{message} ({file}:{line}:{column})")
                        },
                    )),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_xs)
                    .maybe(!*safe_start, |el| {
                        el.child(button("Open in editor", "editor", true))
                            .child(button("Show in Files", "files", true))
                    })
                    .maybe(*safe_start, |el| {
                        el.child(button("Use my settings", "use-mine", true))
                            .child(button("Report…", "report", false))
                    }),
            )
            .into_element(),
        Notice::Inhibited { app, chord } => rect()
            .width(Size::fill())
            .height(Size::px(INHIBIT_ROW))
            .padding((0., t.space_m))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(Glyph::new(
                "isle-keyboard-slash",
                t.size_glyph,
                colour(t.colour_text),
            ))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(
                        theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        format!("{app} is receiving all keyboard shortcuts"),
                    ))
                    .child(text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        chord.clone(),
                    )),
            )
            .child(button("Take keys back", "take-back", true))
            .into_element(),
    }
}

fn activity_row(theme: &Theme, activity: &Activity, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let kind = format!("{:?}", activity.kind).to_lowercase();
    rect()
        .width(Size::fill())
        .height(Size::px(LEADER_ROW))
        .padding((0., t.space_m))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(
            rect()
                .width(Size::px(t.size_app_icon))
                .height(Size::px(t.size_app_icon))
                .center()
                .child(
                    Glyph::new(
                        activity.kind.mark(),
                        t.size_glyph_large,
                        colour(t.colour_text),
                    )
                    .fill(true),
                ),
        )
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    activity.title.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    activity.detail.clone(),
                )),
        )
        .children(activity.actions.iter().map(|(id, label)| {
            let actions = actions.clone();
            let (kind, id) = (kind.clone(), id.clone());
            Button::new(label.clone())
                .small()
                .on_press(move |_| {
                    send(
                        &actions,
                        PanelAction::Activity {
                            kind: kind.clone(),
                            action: id.clone(),
                        },
                    )
                })
                .into_element()
        }))
        .into_element()
}
