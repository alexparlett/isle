//! Control Centre: the panel that unfolds from the pill, its top row the pill's own, then
//! Now, the module grid and the footer (`surfaces/island.md`, Control Centre).

use freya::prelude::*;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, Switch, button::Kind, colour, focus_ring, scroll_area, text, use_theme};

use super::{
    IslandAction,
    grid::{self, TileSize},
    model::{Activity, Privacy},
    rows::{PageRow, RowEvent, Rows, rows_height},
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
    /// The rows of its page open now: its own, or a sub-page's.
    pub page: Option<Vec<PageRow>>,
    /// The sub-page's title, while one is open.
    pub page_title: Option<String>,
    /// The Settings pane its page's foot opens.
    pub pane: String,
}

/// Control Centre as one output's island shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelFrame {
    /// Opened by a click or Ctrl+F8, with the keyboard; a peek has neither.
    pub pinned: bool,
    /// The page open: a module's id, or `module/row` for a row's sub-page.
    pub page: Option<String>,
    pub notices: Vec<Notice>,
    pub activities: Vec<Activity>,
    pub privacy: Vec<(Privacy, String)>,
    pub modules: Vec<Module>,
    /// The height the body is laid out at under the pill's row: the shape's settled height,
    /// not the spring's, so the body does not reflow or scroll while the shape grows. Zero
    /// fills what there is.
    pub body: f32,
}

/// What the panel asks of the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelAction {
    /// A module's switch.
    Toggle(String),
    /// A module's page, or back from it.
    Page(Option<String>),
    /// A row on a module's page chosen or acted on.
    Choose { module: String, row: String },
    /// A slider row moved.
    Slide { module: String, row: String, value: f32 },
    /// A switch row turned on or off.
    Switch { module: String, row: String, on: bool },
    /// A folded group of rows opened or closed.
    Expand { module: String, row: String },
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

/// The module whose page is open.
fn page_module(panel: &PanelFrame) -> Option<&Module> {
    let id = panel.page.as_deref()?.split('/').next()?;
    panel.modules.iter().find(|m| m.id == id)
}

/// The page a page's back control leads to: a sub-page's module, a module's the grid.
pub fn parent_page(page: &str) -> Option<String> {
    page.rsplit_once('/').map(|(parent, _)| parent.to_owned())
}

/// The height of the panel's body below the pill's row, all of it, scrolled or not.
pub fn body_height(theme: &Theme, panel: &PanelFrame) -> f32 {
    let t = &theme.tokens;
    let pad = t.space_s;
    match page_module(panel) {
        Some(module) => {
            let rows = rows_height(theme, module.page.as_deref().unwrap_or(&[]));
            t.size_module_cell_height + pad + rows + 2.0 * pad + t.size_row
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
    }
}

/// The panel's size under the pill's row of height `pill`, at least `pill_width` wide, no
/// taller than `room`; past it the body scrolls.
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
    let height = (pill + pad + body_height(theme, panel) + pad).min(room);
    (width.ceil(), height.ceil())
}

/// Whether the body is taller than the room it has and scrolls.
pub fn scrolls(theme: &Theme, panel: &PanelFrame, pill: f32, room: f32) -> bool {
    let pad = theme.tokens.space_s;
    pill + pad + body_height(theme, panel) + pad > room
}

fn body_size(panel: &PanelFrame) -> Size {
    if panel.body > 0.0 {
        Size::px(panel.body)
    } else {
        Size::fill()
    }
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
        if let Some(module) = page_module(panel) {
            let back = actions.clone();
            let settings = actions.clone();
            let toggle = actions.clone();
            let pane = module.pane.clone();
            let rows = module.page.clone().unwrap_or_default();
            let page = panel.page.clone().unwrap_or_default();
            let sub = module.page_title.clone();
            let (id, on) = (module.id.clone(), module.on);
            let on_event = {
                let actions = actions.clone();
                let page = page.clone();
                let module = module.id.clone();
                move |event: RowEvent| {
                    let (module, action) = (module.clone(), &actions);
                    send(
                        action,
                        match event {
                            RowEvent::Choose(row) => PanelAction::Choose { module, row },
                            RowEvent::Open(row) => PanelAction::Page(Some(format!("{page}/{row}"))),
                            RowEvent::Slide(row, value) => PanelAction::Slide { module, row, value },
                            RowEvent::Switch(row, on) => PanelAction::Switch { module, row, on },
                            RowEvent::Expand(row) => PanelAction::Expand { module, row },
                        },
                    )
                }
            };
            let head = card()
                .height(Size::px(t.size_module_cell_height))
                .padding((0., t.space_l))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(
                    Button::icon(
                        "caret-left",
                        if sub.is_some() {
                            format!("Back to {}", module.name)
                        } else {
                            "Back to Control Centre".to_owned()
                        },
                    )
                    .kind(Kind::Plain)
                    .on_press(move |_| send(&back, PanelAction::Page(parent_page(&page)))),
                )
                .maybe(sub.is_none(), |el| el.child(module_circle(&theme, module)))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(text::line(
                            &theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            sub.clone().unwrap_or_else(|| module.name.clone()),
                        ))
                        .maybe(sub.is_none() && !module.state.is_empty(), |el| {
                            el.child(text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(t.colour_text_2),
                                module.state.clone(),
                            ))
                        }),
                )
                .maybe(sub.is_none() && module.toggles, |el| {
                    el.child(Switch::new(on, module.name.clone(), move |_| {
                        send(&toggle, PanelAction::Toggle(id.clone()))
                    }))
                });
            return rect()
                .width(Size::fill())
                .height(body_size(panel))
                .padding(t.space_s)
                .spacing(t.space_s)
                .content(Content::flex())
                .child(head)
                .child(
                    scroll_area(&theme).height(Size::flex(1.)).child(
                        card().padding((t.space_xs, 0.)).child(Rows {
                            rows,
                            on_event: on_event.into(),
                        }),
                    ),
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

        let sections = rect()
            .width(Size::fill())
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
            });
        rect()
            .width(Size::fill())
            .height(body_size(panel))
            .padding(t.space_s)
            .spacing(t.space_s)
            .content(Content::flex())
            .child(scroll_area(&theme).height(Size::flex(1.)).child(sections))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::rows::Choice;

    fn theme() -> Theme {
        isle_tokens::Source::shipped()
            .theme(&isle_tokens::Appearance::default())
            .unwrap()
    }

    fn panel(rows: usize) -> PanelFrame {
        PanelFrame {
            page: Some("wifi".into()),
            modules: vec![Module {
                id: "wifi".into(),
                size: TileSize::Medium,
                name: "Wi-Fi".into(),
                state: String::new(),
                glyph: "wifi-high".into(),
                on: true,
                toggles: true,
                page: Some(
                    (0..rows)
                        .map(|i| {
                            PageRow::Choice(Choice {
                                id: i.to_string(),
                                label: format!("Network {i}"),
                                ..Default::default()
                            })
                        })
                        .collect(),
                ),
                page_title: None,
                pane: "Wi-Fi".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_page_is_as_tall_as_its_rows_until_the_room_runs_out() {
        let theme = theme();
        let t = &theme.tokens;
        let pill = t.size_island_height;
        let short = panel(4);
        let (_, h) = panel_size(&theme, &short, pill, 0.0, 1000.0);
        assert_eq!(h, (pill + t.space_s + body_height(&theme, &short) + t.space_s).ceil());
        assert!(!scrolls(&theme, &short, pill, 1000.0));

        let long = panel(60);
        let (_, h) = panel_size(&theme, &long, pill, 0.0, 600.0);
        assert_eq!(h, 600.0);
        assert!(scrolls(&theme, &long, pill, 600.0));
    }

    #[test]
    fn a_sub_page_belongs_to_its_module_and_backs_out_to_it() {
        let mut frame = panel(2);
        frame.page = Some("wifi/known".into());
        assert_eq!(page_module(&frame).map(|m| m.id.as_str()), Some("wifi"));
        assert_eq!(parent_page("wifi/known").as_deref(), Some("wifi"));
        assert_eq!(parent_page("wifi"), None);
    }
}
