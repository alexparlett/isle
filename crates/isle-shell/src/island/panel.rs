//! Control Centre: the panel that unfolds from the pill, its top row the pill's own, then
//! Now, the module grid and the footer (`surfaces/island.md`, Control Centre).

use std::{cell::RefCell, rc::Rc};

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
#[derive(Clone, Debug, Default, PartialEq)]
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
    /// A level its Wide and Large forms set with a slider: sound's volume, brightness.
    pub slider: Option<f32>,
    /// The slider's level is not in force, as a muted output's.
    pub muted: bool,
    /// A player, which its Large and Wide forms show with their transport.
    pub media: Option<super::rows::MediaCard>,
    /// Its page's rows hold its control, so the page's head carries name and state only.
    pub plain_head: bool,
    /// The sizes it comes in, which edit mode offers.
    pub sizes: Vec<TileSize>,
    /// An icon in its glyph's place: a tray item's.
    pub icon: Option<std::path::PathBuf>,
}

/// A module the gallery offers, or lists as unable to show here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GalleryModule {
    pub id: String,
    pub name: String,
    pub glyph: String,
    pub group: String,
    /// Why it cannot show here; none when it can be added.
    pub why: Option<String>,
    /// The Settings pane its "Set up…" opens.
    pub pane: String,
}

/// Edit mode: the gallery, and the module whose menu is open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditFrame {
    pub gallery: Vec<GalleryModule>,
    pub menu: Option<String>,
}

/// Another program's status item in Control Centre's tray.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayIcon {
    pub key: String,
    pub title: String,
    pub icon: Option<std::path::PathBuf>,
    pub attention: bool,
}

/// How a tray item was pressed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrayPress {
    Primary,
    Secondary,
    Middle,
    /// Wheel steps, down positive.
    Scroll(i32),
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
    pub tray: Vec<TrayIcon>,
    /// The page of a tray item's menu while one is open, at page `tray/<key>`.
    pub tray_menu: Option<Module>,
    /// Notification Centre, in Control Centre's place.
    pub centre: Option<super::centre::CentreFrame>,
    pub edit: Option<EditFrame>,
    /// The module the keyboard starts on in the grid: the one whose page was left.
    pub focus_module: Option<String>,
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
    /// A tray item pressed.
    Tray { key: String, press: TrayPress },
    /// A tray item dragged onto another's place, which counts only with Cmd held.
    TrayMove { key: String, to: usize },
    /// Edit mode entered or left.
    Edit(bool),
    /// A module's menu in edit mode opened or closed.
    EditMenu(Option<String>),
    EditRemove(String),
    EditAdd(String),
    EditSize { id: String, size: TileSize },
    /// A module moved to place `to` in the arrangement.
    EditMove { id: String, to: usize },
}

const LEADER_ROW: f32 = 56.0;
const NOTICE_ROW: f32 = 64.0;
const INHIBIT_ROW: f32 = 56.0;
const TRAY_CELL: f32 = 36.0;
const GALLERY_HEAD: f32 = 28.0;
const GALLERY_ROW: f32 = 44.0;

/// The gallery's modules by group, the groups in the order they first come.
fn gallery_groups(gallery: &[GalleryModule]) -> Vec<(&str, Vec<&GalleryModule>)> {
    let mut groups: Vec<(&str, Vec<&GalleryModule>)> = Vec::new();
    for module in gallery {
        match groups.iter_mut().find(|(g, _)| *g == module.group) {
            Some((_, list)) => list.push(module),
            None => groups.push((&module.group, vec![module])),
        }
    }
    groups
}

fn gallery_height(theme: &Theme, edit: &EditFrame) -> f32 {
    if edit.gallery.is_empty() {
        return 0.0;
    }
    let groups = gallery_groups(&edit.gallery);
    groups.len() as f32 * GALLERY_HEAD
        + edit.gallery.len() as f32 * GALLERY_ROW
        + 2.0 * theme.tokens.space_s
}

const SIZE_NAMES: [(TileSize, &str); 4] = [
    (TileSize::Small, "Small"),
    (TileSize::Medium, "Medium"),
    (TileSize::Large, "Large"),
    (TileSize::Wide, "Wide"),
];
const TRAY_ICON: f32 = 20.0;

/// Tray items to a row, in the card's width.
fn tray_columns(theme: &Theme) -> usize {
    let t = &theme.tokens;
    ((t.size_panel - 4.0 * t.space_s) / TRAY_CELL).floor().max(1.0) as usize
}

fn tray_height(theme: &Theme, tray: &[TrayIcon]) -> f32 {
    if tray.is_empty() {
        return 0.0;
    }
    let rows = tray.len().div_ceil(tray_columns(theme));
    rows as f32 * TRAY_CELL + 2.0 * theme.tokens.space_s
}

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
    if id == "tray" {
        return panel.tray_menu.as_ref();
    }
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
    if let Some(centre) = &panel.centre {
        return super::centre::centre_height(theme, centre);
    }
    match page_module(panel) {
        Some(module) => {
            let rows = rows_height(theme, module.page.as_deref().unwrap_or(&[]));
            let foot = if module.pane.is_empty() { 0.0 } else { pad + t.size_row };
            t.size_module_cell_height + pad + rows + pad + foot
        }
        None => {
            let now = now_height(panel);
            let grid = grid_height(theme, &panel.modules);
            let mut h = 0.0;
            if grid > 0.0 {
                h += grid + pad;
            } else {
                h += t.size_row + pad;
            }
            if let Some(edit) = &panel.edit {
                let gallery = gallery_height(theme, edit);
                if gallery > 0.0 {
                    h += gallery + pad;
                }
                return h + t.size_row;
            }
            if now > 0.0 {
                h += now + pad;
            }
            let tray = tray_height(theme, &panel.tray);
            if tray > 0.0 {
                h += tray + pad;
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

/// A scroll area's height: the body less `used` by its padding and the rest beside it. In
/// whole pixels where it can be, since a flex share comes out a hair short of content of
/// the same height and shows a scrollbar.
fn scroll_size(panel: &PanelFrame, used: f32) -> Size {
    if panel.body > 0.0 {
        Size::px((panel.body - used).max(0.0))
    } else {
        Size::flex(1.)
    }
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
        let tile_ids = use_hook(|| Rc::new(RefCell::new(Vec::<AccessibilityId>::new())));
        let drag = use_state(|| None::<String>);
        let start = panel
            .focus_module
            .as_ref()
            .and_then(|id| panel.modules.iter().position(|m| &m.id == id))
            .unwrap_or(0);
        let roving = use_state(move || start);
        let card = || {
            rect()
                .width(Size::fill())
                .corner_radius(t.radius_card)
                .background(colour(t.colour_fill))
        };
        if let Some(centre) = panel.centre.clone() {
            return rect()
                .width(Size::fill())
                .height(body_size(panel))
                .padding(t.space_s)
                .content(Content::flex())
                .child(
                    scroll_area(&theme)
                        .height(scroll_size(panel, 2.0 * t.space_s))
                        .child(super::centre::CentreBody { centre }),
                )
                .into_element();
        }
        if let Some(module) = page_module(panel) {
            let back = actions.clone();
            let settings = actions.clone();
            let toggle = actions.clone();
            let pane = module.pane.clone();
            let rows = module.page.clone().unwrap_or_default();
            let page = panel.page.clone().unwrap_or_default();
            let sub = module.page_title.clone();
            let (id, on) = (module.id.clone(), module.on);
            let foot = 3.0 * t.space_s
                + t.size_module_cell_height
                + if pane.is_empty() { 0.0 } else { t.space_s + t.size_row };
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
                        .map(module.slider.filter(|_| sub.is_none()), |el, level| {
                            let actions = actions.clone();
                            let id = module.id.clone();
                            el.child(
                                isle_ui::Slider::new(level, module.name.clone(), move |value| {
                                    send(
                                        &actions,
                                        PanelAction::Slide {
                                            module: id.clone(),
                                            row: String::new(),
                                            value,
                                        },
                                    )
                                })
                                .glass(true)
                                .muted(module.muted),
                            )
                        })
                        .maybe(
                            sub.is_none() && module.slider.is_none() && !module.state.is_empty(),
                            |el| {
                                el.child(text::line(
                                    &theme,
                                    text::Style::Subheadline,
                                    colour(t.colour_text_2),
                                    module.state.clone(),
                                ))
                            },
                        ),
                )
                .maybe(
                    sub.is_none() && module.toggles && module.slider.is_none() && !module.plain_head,
                    |el| {
                        el.child(Switch::new(on, module.name.clone(), move |_| {
                            send(&toggle, PanelAction::Toggle(id.clone()))
                        }))
                    },
                );
            return rect()
                .width(Size::fill())
                .height(body_size(panel))
                .padding(t.space_s)
                .spacing(t.space_s)
                .content(Content::flex())
                .child(head)
                .child(
                    scroll_area(&theme).height(scroll_size(panel, foot)).child(
                        card().padding((t.space_xs, 0.)).child(Rows {
                            rows,
                            on_event: on_event.into(),
                        }),
                    ),
                )
                .maybe(!pane.is_empty(), |el| {
                    el.child(
                        rect().height(Size::px(t.size_row)).horizontal().child(
                            Button::new(format!("{pane} Settings…"))
                                .kind(Kind::Plain)
                                .small()
                                .on_press(move |_| {
                                    send(&settings, PanelAction::Settings(pane.clone()))
                                }),
                        ),
                    )
                })
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

        let editing = panel.edit.is_some();
        let ids: Rc<Vec<AccessibilityId>> = {
            let mut ids = tile_ids.borrow_mut();
            while ids.len() < panel.modules.len() {
                ids.push(AccessibilityId::new_unique());
            }
            Rc::new(ids[..panel.modules.len()].to_vec())
        };
        let sizes: Vec<TileSize> = panel.modules.iter().map(|m| m.size).collect();
        let (places, _) = grid::pack(&sizes);
        let (sizes, places) = (Rc::new(sizes), Rc::new(places));
        let active = (*roving.read()).min(panel.modules.len().saturating_sub(1));
        let cell = (t.size_module_cell_width, t.size_module_cell_height);
        let place = |module: &Module, (col, row): (usize, usize)| {
            let (w, h) = module.size.cells();
            (
                col as f32 * (cell.0 + t.space_s),
                row as f32 * (cell.1 + t.space_s),
                w as f32 * cell.0 + (w as f32 - 1.0) * t.space_s,
                h as f32 * cell.1 + (h as f32 - 1.0) * t.space_s,
            )
        };
        let tiles = panel
            .modules
            .iter()
            .zip(places.iter().copied())
            .enumerate()
            .map(|(index, (module, at))| {
                let (x, y, width, height) = place(module, at);
                Tile {
                    module: module.clone(),
                    x,
                    y,
                    width,
                    height,
                    index,
                    ids: ids.clone(),
                    sizes: sizes.clone(),
                    places: places.clone(),
                    active: index == active,
                    roving,
                    editing,
                    autofocus: panel.pinned,
                    drag,
                }
                .into_element()
            });
        // The open module's menu in edit mode, over the tiles.
        let edit_menu = panel
            .edit
            .as_ref()
            .and_then(|e| e.menu.as_ref())
            .and_then(|id| {
                let i = panel.modules.iter().position(|m| &m.id == id)?;
                Some((&panel.modules[i], places[i]))
            })
            .map(|(module, at)| {
                let (x, y, _, height) = place(module, at);
                let entries: Vec<isle_ui::MenuEntry> = SIZE_NAMES
                    .iter()
                    .enumerate()
                    .filter(|(_, (size, _))| module.sizes.contains(size))
                    .map(|(i, (size, name))| {
                        isle_ui::MenuEntry::Item(
                            isle_ui::menu::MenuItem::new(i as u64 + 1, *name).mark(
                                if *size == module.size {
                                    isle_ui::menu::Mark::Check
                                } else {
                                    isle_ui::menu::Mark::None
                                },
                            ),
                        )
                    })
                    .chain([
                        isle_ui::MenuEntry::Separator,
                        isle_ui::MenuEntry::Item(isle_ui::menu::MenuItem::new(9, "Remove")),
                    ])
                    .collect();
                let (actions, id) = (actions.clone(), module.id.clone());
                rect()
                    .position(Position::new_absolute().left(x + t.space_l).top(y + height - t.space_s))
                    .layer(Layer::Overlay)
                    .child(
                        isle_ui::Menu::new(Rc::new(entries), move |event| {
                            if let isle_ui::menu::MenuEvent::Chosen(n) = event {
                                let action = match SIZE_NAMES.get(n as usize - 1) {
                                    Some((size, _)) if n < 9 => PanelAction::EditSize {
                                        id: id.clone(),
                                        size: *size,
                                    },
                                    _ => PanelAction::EditRemove(id.clone()),
                                };
                                send(&actions, action);
                            }
                        })
                        .material(isle_ui::menu::Material::Solid)
                        .first_selected(true),
                    )
                    .into_element()
            });
        let gallery = panel
            .edit
            .as_ref()
            .filter(|e| !e.gallery.is_empty())
            .map(|edit| gallery(&theme, edit, &actions));

        let fits = panel.body <= 0.0 || body_height(&theme, panel) + 2.0 * t.space_s <= panel.body;
        let sections = rect()
            .width(Size::fill())
            .spacing(t.space_s)
            .maybe(has_now && !editing, |el| {
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
            .maybe(panel.modules.is_empty(), |el| {
                el.child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.size_row))
                        .center()
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text_2),
                            "No controls",
                        )),
                )
            })
            .maybe(!panel.tray.is_empty() && !editing, |el| {
                el.child(TrayRow {
                    icons: panel.tray.clone(),
                })
            })
            .maybe_child(gallery)
            // In edit mode the grid is the first section, so the menu shares its origin.
            .maybe_child(edit_menu);
        let (edit, done) = (actions.clone(), actions.clone());
        rect()
            .width(Size::fill())
            .height(body_size(panel))
            .padding(t.space_s)
            .spacing(t.space_s)
            .content(Content::flex())
            .child(
                scroll_area(&theme)
                    .height(scroll_size(panel, 3.0 * t.space_s + t.size_row))
                    // Freya measures the absolutely placed tiles a little past the grid, so
                    // whether the grid scrolls is the panel's own measure.
                    .show_scrollbar(!fits)
                    .scroll_with_arrows(false)
                    .child(sections),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .maybe(!editing, |el| {
                        let settings = edit.clone();
                        el.child(
                            Button::new("Edit controls")
                                .kind(Kind::Plain)
                                .small()
                                .on_press(move |_| send(&edit, PanelAction::Edit(true))),
                        )
                        .child(rect().width(Size::flex(1.)))
                        .child(
                            Button::new("Control Centre Settings…")
                                .kind(Kind::Plain)
                                .small()
                                .on_press(move |_| {
                                    send(&settings, PanelAction::Settings("Control Centre".into()))
                                }),
                        )
                    })
                    .maybe(editing, |el| {
                        el.child(rect().width(Size::flex(1.))).child(
                            Button::new("Done")
                                .small()
                                .on_press(move |_| send(&done, PanelAction::Edit(false))),
                        )
                    }),
            )
            .into_element()
    }
}

/// The tray: other programs' items in the user's order, a press passed to the item and a
/// drag onto another item's place reordering them.
#[derive(PartialEq)]
struct TrayRow {
    icons: Vec<TrayIcon>,
}

impl Component for TrayRow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dragging = use_state(|| None::<String>);
        let cell_ids = use_hook(|| Rc::new(RefCell::new(Vec::<AccessibilityId>::new())));
        let ids: Rc<Vec<AccessibilityId>> = {
            let mut ids = cell_ids.borrow_mut();
            while ids.len() < self.icons.len() {
                ids.push(AccessibilityId::new_unique());
            }
            Rc::new(ids[..self.icons.len()].to_vec())
        };
        let roving = use_state(|| 0usize);
        let active = (*roving.read()).min(self.icons.len().saturating_sub(1));
        let columns = tray_columns(&theme);
        let rows = self.icons.chunks(columns).enumerate().map(|(r, chunk)| {
            rect()
                .height(Size::px(TRAY_CELL))
                .horizontal()
                .children(chunk.iter().enumerate().map(|(c, icon)| {
                    TrayCell {
                        icon: icon.clone(),
                        index: r * columns + c,
                        ids: ids.clone(),
                        active: r * columns + c == active,
                        roving,
                        dragging,
                    }
                    .into_element()
                }))
                .into_element()
        });
        rect()
            .width(Size::fill())
            .padding(t.space_s)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_fill))
            .children(rows)
    }
}

#[derive(PartialEq)]
struct TrayCell {
    icon: TrayIcon,
    index: usize,
    /// Every item's id, by place; one at a time takes the keyboard.
    ids: Rc<Vec<AccessibilityId>>,
    active: bool,
    roving: State<usize>,
    dragging: State<Option<String>>,
}

impl Component for TrayCell {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let a11y = self.ids[self.index];
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let mut dragging = self.dragging;
        let mut roving = self.roving;
        let ids = self.ids.clone();
        let key = self.icon.key.clone();
        let index = self.index;
        let press = {
            let (actions, key) = (actions.clone(), key.clone());
            move |press: TrayPress| {
                send(
                    &actions,
                    PanelAction::Tray {
                        key: key.clone(),
                        press,
                    },
                )
            }
        };
        let (down, up, wheel, keyed) = (press.clone(), press.clone(), press.clone(), press);
        let mut moved = {
            let actions = actions.clone();
            move || {
                let from = dragging.peek().clone();
                dragging.set(None);
                match from {
                    Some(from) if from != key => {
                        send(&actions, PanelAction::TrayMove { key: from, to: index });
                        true
                    }
                    _ => false,
                }
            }
        };
        let drag_key = self.icon.key.clone();
        let (keyed_actions, keyed_key) = (actions.clone(), self.icon.key.clone());
        rect()
            .width(Size::px(TRAY_CELL))
            .height(Size::px(TRAY_CELL))
            .corner_radius(t.radius_control)
            .background(colour(if *hover.read() {
                t.colour_fill_strong
            } else {
                t.colour_fill
            }))
            .center()
            .a11y_id(a11y)
            .a11y_focusable(self.active)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(self.icon.title.clone())
            .on_pointer_enter(move |_| hover.set(true))
            .on_pointer_leave(move |_| hover.set(false))
            .on_mouse_down(move |e: Event<MouseEventData>| {
                e.stop_propagation();
                match e.button {
                    Some(MouseButton::Left) => dragging.set(Some(drag_key.clone())),
                    Some(MouseButton::Right) => down(TrayPress::Secondary),
                    Some(MouseButton::Middle) => down(TrayPress::Middle),
                    _ => {}
                }
            })
            .on_mouse_up(move |e: Event<MouseEventData>| {
                if e.button == Some(MouseButton::Left) && !moved() {
                    up(TrayPress::Primary);
                }
            })
            .on_wheel(move |e: Event<WheelEventData>| {
                let steps = e.delta_y.signum() as i32;
                if steps != 0 {
                    wheel(TrayPress::Scroll(-steps));
                }
            })
            .on_key_down(move |e: Event<KeyboardEventData>| {
                let cmd = e.modifiers.contains(Modifiers::META);
                let alt = e.modifiers.contains(Modifiers::ALT);
                let shift = e.modifiers.contains(Modifiers::SHIFT);
                let last = ids.len() - 1;
                let to = match &e.key {
                    Key::Named(NamedKey::ArrowLeft) => Some(index.saturating_sub(1)),
                    Key::Named(NamedKey::ArrowRight) => Some((index + 1).min(last)),
                    _ => None,
                };
                match &e.key {
                    _ if cmd && to.is_some() => {
                        let to = to.unwrap();
                        send(
                            &keyed_actions,
                            PanelAction::TrayMove {
                                key: keyed_key.clone(),
                                to,
                            },
                        );
                        roving.set(to);
                        ids[to].request_focus();
                    }
                    _ if e.modifiers.is_empty() && to.is_some() => {
                        roving.set(to.unwrap());
                        ids[to.unwrap()].request_focus()
                    }
                    Key::Named(NamedKey::Enter) if alt => keyed(TrayPress::Middle),
                    Key::Named(NamedKey::Enter) => keyed(TrayPress::Primary),
                    Key::Character(c) if c == " " => keyed(TrayPress::Primary),
                    Key::Named(NamedKey::ContextMenu) => keyed(TrayPress::Secondary),
                    Key::Named(NamedKey::F10) if shift => keyed(TrayPress::Secondary),
                    _ => {}
                }
            })
            .maybe(focus() == Focus::Keyboard, |el| {
                el.border(focus_ring(&theme))
            })
            .child(match &self.icon.icon {
                Some(path) => isle_ui::AppIcon::new(path.clone(), TRAY_ICON).into_element(),
                None => Glyph::new("app-window", TRAY_ICON, colour(t.colour_text)).into_element(),
            })
            .maybe(self.icon.attention, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().right(5.).top(5.))
                        .width(Size::px(7.))
                        .height(Size::px(7.))
                        .corner_radius(3.5)
                        .background(colour(t.colour_accent)),
                )
            })
    }
}

/// Edit mode's gallery: every module not in the grid by group, those that cannot show here
/// with why and "Set up…".
fn gallery(theme: &Theme, edit: &EditFrame, actions: &Actions) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding(t.space_s)
        .corner_radius(t.radius_card)
        .background(colour(t.colour_fill))
        .children(gallery_groups(&edit.gallery).into_iter().map(|(group, modules)| {
            rect()
                .width(Size::fill())
                .child(
                    rect()
                        .height(Size::px(GALLERY_HEAD))
                        .padding((0., t.space_s))
                        .main_align(Alignment::End)
                        .child(text::line(
                            theme,
                            text::Style::Subheadline,
                            colour(t.colour_text_2),
                            group.to_owned(),
                        )),
                )
                .children(modules.into_iter().map(|module| {
                    let actions = actions.clone();
                    let (id, pane) = (module.id.clone(), module.pane.clone());
                    rect()
                        .width(Size::fill())
                        .height(Size::px(GALLERY_ROW))
                        .padding((0., t.space_s))
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(
                            rect()
                                .width(Size::px(28.))
                                .height(Size::px(28.))
                                .corner_radius(14.)
                                .background(colour(t.colour_fill_strong))
                                .center()
                                .child(Glyph::new(
                                    module.glyph.clone(),
                                    t.size_glyph,
                                    colour(if module.why.is_some() {
                                        t.colour_text_3
                                    } else {
                                        t.colour_text
                                    }),
                                )),
                        )
                        .child(
                            rect()
                                .width(Size::flex(1.))
                                .child(text::line(
                                    theme,
                                    text::Style::Body,
                                    colour(if module.why.is_some() {
                                        t.colour_text_2
                                    } else {
                                        t.colour_text
                                    }),
                                    module.name.clone(),
                                ))
                                .map(module.why.clone(), |el, why| {
                                    el.child(text::line(
                                        theme,
                                        text::Style::Footnote,
                                        colour(t.colour_text_2),
                                        why,
                                    ))
                                }),
                        )
                        .child(match &module.why {
                            None => Button::icon("plus-circle", format!("Add {}", module.name))
                                .kind(Kind::Plain)
                                .on_press(move |_| send(&actions, PanelAction::EditAdd(id.clone())))
                                .into_element(),
                            Some(_) => Button::new("Set up…")
                                .small()
                                .on_press(move |_| {
                                    send(&actions, PanelAction::Settings(pane.clone()))
                                })
                                .into_element(),
                        })
                        .into_element()
                }))
                .into_element()
        }))
        .into_element()
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
        .map(module.icon.clone(), |el, icon| {
            el.child(isle_ui::AppIcon::new(icon, t.size_glyph_large))
        })
        .maybe(module.icon.is_none(), |el| {
            el.child(Glyph::new(module.glyph.clone(), t.size_glyph_large, ink).fill(module.on))
        })
}

#[derive(PartialEq)]
struct Tile {
    module: Module,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    index: usize,
    /// Every tile's id, by place. One tile at a time takes the keyboard, so Tab passes
    /// the grid as one stop and the arrows move within it.
    ids: Rc<Vec<AccessibilityId>>,
    sizes: Rc<Vec<TileSize>>,
    places: Rc<Vec<(usize, usize)>>,
    active: bool,
    roving: State<usize>,
    editing: bool,
    /// The panel has the keyboard, which the active tile takes as it shows.
    autofocus: bool,
    /// The module being dragged in edit mode.
    drag: State<Option<String>>,
}

impl Component for Tile {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let a11y = self.ids[self.index];
        let focus = use_focus(a11y);
        let mut hover = use_state(|| false);
        let module = &self.module;
        let id = module.id.clone();
        let toggles = module.toggles;
        let small = module.size == TileSize::Small;
        let open_page = module.page.is_some();
        let editing = self.editing;
        let index = self.index;
        let mut drag = self.drag;
        let mut roving = self.roving;
        let keys = {
            let (actions, id, ids) = (actions.clone(), id.clone(), self.ids.clone());
            let (sizes, places) = (self.sizes.clone(), self.places.clone());
            let level = module.slider;
            move |e: Event<KeyboardEventData>| {
                let cmd = e.modifiers.contains(Modifiers::META);
                let shift = e.modifiers.contains(Modifiers::SHIFT);
                let last = ids.len() - 1;
                let mut go = |to: usize| {
                    roving.set(to);
                    ids[to].request_focus();
                };
                let toward = match &e.key {
                    Key::Named(NamedKey::ArrowLeft) => Some(grid::Toward::Left),
                    Key::Named(NamedKey::ArrowRight) => Some(grid::Toward::Right),
                    Key::Named(NamedKey::ArrowUp) => Some(grid::Toward::Up),
                    Key::Named(NamedKey::ArrowDown) => Some(grid::Toward::Down),
                    _ => None,
                };
                if let Some(toward) = toward.filter(|_| e.modifiers.is_empty()) {
                    if let Some(to) = grid::neighbour(&sizes, &places, index, toward) {
                        go(to);
                    }
                    return;
                }
                let mut moved = |to: usize| {
                    send(&actions, PanelAction::EditMove { id: id.clone(), to });
                    go(to);
                };
                match &e.key {
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) if cmd && editing => {
                        if index > 0 {
                            moved(index - 1)
                        }
                    }
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) if cmd && editing => {
                        if index < last {
                            moved(index + 1)
                        }
                    }
                    Key::Named(NamedKey::Home) => go(0),
                    Key::Named(NamedKey::End) => go(last),
                    Key::Named(NamedKey::Delete | NamedKey::Backspace) if editing => {
                        send(&actions, PanelAction::EditRemove(id.clone()))
                    }
                    Key::Named(NamedKey::Enter | NamedKey::ContextMenu) if editing => {
                        send(&actions, PanelAction::EditMenu(Some(id.clone())))
                    }
                    Key::Named(NamedKey::F10) if editing && shift => {
                        send(&actions, PanelAction::EditMenu(Some(id.clone())))
                    }
                    // Its own key handler replaces `on_press`'s, so Return and Space are here.
                    Key::Named(NamedKey::Enter) if !editing => {
                        send(
                            &actions,
                            if open_page {
                                PanelAction::Page(Some(id.clone()))
                            } else {
                                PanelAction::Toggle(id.clone())
                            },
                        )
                    }
                    Key::Character(c) if !editing && c == " " => {
                        send(
                            &actions,
                            if toggles || !open_page {
                                PanelAction::Toggle(id.clone())
                            } else {
                                PanelAction::Page(Some(id.clone()))
                            },
                        )
                    }
                    Key::Character(c) if !editing && !cmd && c.eq_ignore_ascii_case("e") => {
                        send(&actions, PanelAction::Edit(true))
                    }
                    Key::Character(c) if !cmd && matches!(c.as_str(), "+" | "=" | "-") => {
                        if let Some(level) = level {
                            let step = if shift { 0.1 } else { 0.05 };
                            let value = if c == "-" { level - step } else { level + step };
                            send(
                                &actions,
                                PanelAction::Slide {
                                    module: id.clone(),
                                    row: String::new(),
                                    value: value.clamp(0.0, 1.0),
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
        };
        let (down, up) = {
            let (actions, id, up_id) = (actions.clone(), id.clone(), id.clone());
            let up_actions = actions.clone();
            (
                move |e: Event<MouseEventData>| {
                    roving.set(index);
                    if !editing {
                        return;
                    }
                    match e.button {
                        Some(MouseButton::Left) => drag.set(Some(id.clone())),
                        Some(MouseButton::Right) => {
                            send(&actions, PanelAction::EditMenu(Some(id.clone())))
                        }
                        _ => {}
                    }
                },
                move |e: Event<MouseEventData>| {
                    if !editing || e.button != Some(MouseButton::Left) {
                        return;
                    }
                    let from = drag.peek().clone();
                    drag.set(None);
                    if let Some(from) = from.filter(|f| *f != up_id) {
                        send(&up_actions, PanelAction::EditMove { id: from, to: index });
                    }
                },
            )
        };
        let remove = {
            let (actions, id) = (actions.clone(), id.clone());
            move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&actions, PanelAction::EditRemove(id.clone()));
            }
        };
        // At Small the whole tile is the glyph's switch; larger, the glyph switches and the
        // rest opens the page.
        let press = {
            let (actions, id) = (actions.clone(), id.clone());
            move |e: Event<PressEventData>| {
                e.stop_propagation();
                if editing {
                    return;
                }
                let action = if toggles && (small || !open_page) {
                    PanelAction::Toggle(id.clone())
                } else if open_page {
                    PanelAction::Page(Some(id.clone()))
                } else {
                    return;
                };
                send(&actions, action);
            }
        };
        let glyph_press = {
            let (actions, id) = (actions.clone(), id.clone());
            move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&actions, PanelAction::Toggle(id.clone()));
            }
        };
        let slider = module.slider.filter(|_| {
            matches!(module.size, TileSize::Wide | TileSize::Large)
        });
        let card = module
            .media
            .clone()
            .filter(|_| matches!(module.size, TileSize::Wide | TileSize::Large));
        let transport = {
            let (actions, id) = (actions.clone(), id.clone());
            move |event: super::rows::RowEvent| {
                if let super::rows::RowEvent::Choose(row) = event {
                    send(
                        &actions,
                        PanelAction::Choose {
                            module: id.clone(),
                            row,
                        },
                    )
                }
            }
        };
        let slide = {
            let (actions, id) = (actions.clone(), id.clone());
            move |value: f32| {
                send(
                    &actions,
                    PanelAction::Slide {
                        module: id.clone(),
                        row: String::new(),
                        value,
                    },
                )
            }
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
            .a11y_focusable(self.active)
            .a11y_auto_focus(self.active && self.autofocus)
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
            .on_key_down(keys)
            .on_mouse_down(down)
            .on_mouse_up(up)
            .maybe(editing, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().right(6.).top(6.))
                        .width(Size::px(20.))
                        .height(Size::px(20.))
                        .corner_radius(10.)
                        .background(colour(t.colour_fill_strong))
                        .center()
                        .on_press(remove)
                        .child(Glyph::new("minus", 12., colour(t.colour_text))),
                )
            })
            .horizontal()
            .cross_align(Alignment::Center)
            .maybe(small, |el| el.main_align(Alignment::Center))
            .padding(if small { 0.0 } else { t.space_l })
            .spacing(t.space_s + 4.0)
            .map(card.clone(), |el, card| {
                let head = rect()
                    .width(Size::flex(1.))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(
                        rect()
                            .width(Size::px(t.size_module_glyph))
                            .height(Size::px(t.size_module_glyph))
                            .corner_radius(t.radius_control)
                            .overflow(Overflow::Clip)
                            .background(colour(t.colour_fill_strong))
                            .center()
                            .map(card.art.clone(), |el, art| {
                                el.child(isle_ui::AppIcon::new(art, t.size_module_glyph))
                            })
                            .maybe(card.art.is_none(), |el| {
                                el.child(Glyph::new(
                                    "music-note",
                                    t.size_glyph,
                                    colour(t.colour_text_2),
                                ))
                            }),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(text::line(
                                &theme,
                                text::Style::Headline,
                                colour(t.colour_text),
                                card.title.clone(),
                            ))
                            .child(text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(t.colour_text_2),
                                card.artist.clone(),
                            )),
                    );
                let controls = super::rows::transport(&card, &transport.clone().into());
                // Wide is one row: art and track, then the transport; Large stacks them.
                if module.size == TileSize::Wide {
                    el.horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(head)
                        .child(controls)
                } else {
                    el.main_align(Alignment::Start)
                        .cross_align(Alignment::Start)
                        .vertical()
                        .content(Content::flex())
                        .spacing(t.space_s)
                        .child(head.width(Size::fill()))
                        .child(rect().width(Size::fill()).height(Size::flex(1.)))
                        .child(
                            rect()
                                .width(Size::fill())
                                .main_align(Alignment::Center)
                                .horizontal()
                                .child(controls),
                        )
                }
            })
            .maybe(card.is_none(), |el| {
                el.child(
                    module_circle(&theme, module)
                        .maybe(toggles && !small && open_page, |el| el.on_press(glyph_press)),
                )
            })
            .maybe(!small && card.is_none(), |el| {
                el.child(
                    rect()
                        .width(Size::fill())
                        .child(text::line(
                            &theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            module.name.clone(),
                        ))
                        .map(slider, |el, level| {
                            el.child(
                                isle_ui::Slider::new(level, module.name.clone(), slide)
                                    .glass(true)
                                    .muted(module.muted),
                            )
                        })
                        .maybe(slider.is_none(), |el| {
                            el.child(text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(t.colour_text_2),
                                module.state.clone(),
                            ))
                        }),
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
                ..Default::default()
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
