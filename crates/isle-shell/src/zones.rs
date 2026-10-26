//! The zoom menu and the pointer panel: the zones as thumbnails in the shape of the window's
//! output (`p1/surfaces/window-chrome.md`, Zoom menu, Pointer panel).

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::Glass;
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Layer, Scope, SurfaceDef};

/// What the zoom menu asks of its window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneAction {
    /// Shows a zone's preview on the screen, or none.
    Preview(Option<String>),
    Choose(String),
    Restore,
    FullScreen,
    /// Split View with this window on the left of the screen, or the right.
    Split { left: bool },
    /// In Split View: the panes change sides.
    Swap,
    /// In Split View: this pane is chosen again.
    Replace,
    /// To another display, by its place in the menu's list.
    Display(usize),
}

/// The other windows and the widest gap among them, as fractions of the work area.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Beside {
    pub windows: Vec<[f32; 4]>,
    pub gap: [f32; 4],
}

/// What the zone grid shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoneModel {
    /// The output's width over its height.
    pub aspect: f32,
    pub beside: Option<Beside>,
    /// The window's zone now.
    pub current: Option<String>,
    /// The zone picked by keyboard or by the pointer panel.
    pub selected: Option<String>,
    pub can_restore: bool,
    pub full_screen: bool,
    /// The window is a pane of Split View.
    pub split_pane: bool,
    /// The other displays' names.
    pub displays: Vec<String>,
    /// Opened by a chord: it takes the keyboard.
    pub by_key: bool,
}

/// The thumbnails in macOS's order, each a set of zones as fractions of the thumbnail.
const THUMBS: &[&[(&str, [f32; 4])]] = &[
    &[
        ("left-half", [0., 0., 0.5, 1.]),
        ("right-half", [0.5, 0., 0.5, 1.]),
    ],
    &[
        ("left-third", [0., 0., 1. / 3., 1.]),
        ("centre-third", [1. / 3., 0., 1. / 3., 1.]),
        ("right-third", [2. / 3., 0., 1. / 3., 1.]),
    ],
    &[
        ("left-two-thirds", [0., 0., 2. / 3., 1.]),
        ("right-third", [2. / 3., 0., 1. / 3., 1.]),
    ],
    &[
        ("left-third", [0., 0., 1. / 3., 1.]),
        ("right-two-thirds", [1. / 3., 0., 2. / 3., 1.]),
    ],
    &[
        ("top-left", [0., 0., 0.5, 0.5]),
        ("top-right", [0.5, 0., 0.5, 0.5]),
        ("bottom-left", [0., 0.5, 0.5, 0.5]),
        ("bottom-right", [0.5, 0.5, 0.5, 0.5]),
    ],
    &[("centre", [0.2, 0., 0.6, 1.])],
    &[("fill", [0., 0., 1., 1.])],
];

const PER_ROW: usize = 4;

/// A zone's place in the grid, in points from the grid's origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub zone: String,
    pub rect: [f32; 4],
}

/// The thumbnails and their slots, laid out.
pub struct Grid {
    pub thumbs: Vec<[f32; 4]>,
    pub slots: Vec<Slot>,
    pub outlines: Vec<[f32; 4]>,
    pub width: f32,
    pub height: f32,
}

impl Grid {
    pub fn of(model: &ZoneModel, theme: &Theme) -> Self {
        let t = &theme.tokens;
        let th = t.size_zone_thumbnail;
        let tw = (th * model.aspect).round();
        let gap = t.space_s;
        let inset = t.space_xxs;
        let mut sets: Vec<Vec<(String, [f32; 4])>> = THUMBS
            .iter()
            .map(|s| s.iter().map(|(z, r)| (z.to_string(), *r)).collect())
            .collect();
        let mut outlines = Vec::new();
        if let Some(beside) = &model.beside {
            sets.push(vec![("beside".to_string(), beside.gap)]);
        }
        let mut thumbs = Vec::new();
        let mut slots = Vec::new();
        for (i, set) in sets.iter().enumerate() {
            let x = (i % PER_ROW) as f32 * (tw + gap);
            let y = (i / PER_ROW) as f32 * (th + gap);
            thumbs.push([x, y, tw, th]);
            let inner = [x + inset, y + inset, tw - 2. * inset, th - 2. * inset];
            for (zone, f) in set {
                // Slots that meet keep half the inset between them on each side.
                let half = inset / 2.;
                let l = if f[0] > 0. { half } else { 0. };
                let r = if f[0] + f[2] < 0.999 { half } else { 0. };
                let tp = if f[1] > 0. { half } else { 0. };
                let b = if f[1] + f[3] < 0.999 { half } else { 0. };
                slots.push(Slot {
                    zone: zone.clone(),
                    rect: [
                        inner[0] + f[0] * inner[2] + l,
                        inner[1] + f[1] * inner[3] + tp,
                        f[2] * inner[2] - l - r,
                        f[3] * inner[3] - tp - b,
                    ],
                });
            }
            if set.first().is_some_and(|(z, _)| z == "beside")
                && let Some(beside) = &model.beside
            {
                for w in &beside.windows {
                    outlines.push([
                        inner[0] + w[0] * inner[2],
                        inner[1] + w[1] * inner[3],
                        w[2] * inner[2],
                        w[3] * inner[3],
                    ]);
                }
            }
        }
        let rows = sets.len().div_ceil(PER_ROW);
        let cols = sets.len().min(PER_ROW);
        Self {
            thumbs,
            slots,
            outlines,
            width: cols as f32 * tw + (cols - 1) as f32 * gap,
            height: rows as f32 * th + (rows - 1) as f32 * gap,
        }
    }

    /// The zone under `at`, in points from the grid's origin.
    pub fn zone_at(&self, at: (f32, f32)) -> Option<&str> {
        self.slots
            .iter()
            .find(|s| {
                let r = s.rect;
                at.0 >= r[0] && at.0 < r[0] + r[2] && at.1 >= r[1] && at.1 < r[1] + r[3]
            })
            .map(|s| s.zone.as_str())
    }
}

/// The zone's name as the menu shows it.
pub fn zone_label(zone: &str) -> &'static str {
    match zone {
        "left-half" => "Left half",
        "right-half" => "Right half",
        "left-third" => "Left third",
        "centre-third" => "Centre third",
        "right-third" => "Right third",
        "left-two-thirds" => "Left two-thirds",
        "right-two-thirds" => "Right two-thirds",
        "top-left" => "Top left",
        "top-right" => "Top right",
        "bottom-left" => "Bottom left",
        "bottom-right" => "Bottom right",
        "centre" => "Centre",
        "fill" => "Fill",
        "beside" => "Fill the space beside",
        _ => "",
    }
}

fn label_line(theme: &Theme) -> f32 {
    theme.tokens.type_style_subheadline_line * theme.appearance.text_scale
}

fn name_line(theme: &Theme) -> f32 {
    theme.tokens.type_style_callout_line * theme.appearance.text_scale
}

fn row(theme: &Theme) -> f32 {
    theme.tokens.size_menu_row
}

/// The zoom menu's size for `model`.
pub fn menu_size(model: &ZoneModel, theme: &Theme) -> SSize<i32, Logical> {
    let t = &theme.tokens;
    let grid = Grid::of(model, theme);
    let pad = t.space_m;
    let mut h = pad + label_line(theme) + t.space_xs + grid.height + t.space_xs + name_line(theme);
    if model.can_restore {
        h += row(theme);
    }
    h += t.space_m + label_line(theme) + t.space_xs + t.size_zone_thumbnail;
    if model.split_pane {
        h += 2.0 * row(theme);
    }
    h += model.displays.len() as f32 * row(theme);
    h += pad;
    ((grid.width + 2. * pad).ceil() as i32, h.ceil() as i32).into()
}

/// The pointer panel's size: the grid alone.
pub fn panel_size(model: &ZoneModel, theme: &Theme) -> SSize<i32, Logical> {
    let grid = Grid::of(model, theme);
    let pad = theme.tokens.space_m;
    (
        (grid.width + 2. * pad).ceil() as i32,
        (grid.height + 2. * pad).ceil() as i32,
    )
        .into()
}

/// The zone grid's origin within the pointer panel.
pub fn panel_grid_origin(theme: &Theme) -> (f32, f32) {
    (theme.tokens.space_m, theme.tokens.space_m)
}

/// A zone surface: the zoom menu, or the pointer panel when `panel`.
pub struct ZoneSurface {
    panel: bool,
    model: Rc<RefCell<ZoneModel>>,
    state: Rc<RefCell<Option<State<ZoneModel>>>>,
}

impl ZoneSurface {
    pub fn menu() -> Rc<Self> {
        Self::new(false)
    }

    pub fn panel() -> Rc<Self> {
        Self::new(true)
    }

    fn new(panel: bool) -> Rc<Self> {
        Rc::new(Self {
            panel,
            model: Rc::default(),
            state: Rc::default(),
        })
    }

    pub fn model(&self) -> ZoneModel {
        self.model.borrow().clone()
    }

    /// Keeps `model` for the next tree built; safe anywhere.
    pub fn set_model(&self, model: ZoneModel) {
        *self.model.borrow_mut() = model;
    }

    /// Puts `model` in force; call it in the tree's root scope.
    pub fn set(&self, model: ZoneModel) {
        *self.model.borrow_mut() = model.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(model);
        }
    }
}

impl SurfaceDef for ZoneSurface {
    fn name(&self) -> &'static str {
        if self.panel {
            "zone-panel"
        } else {
            "zoom-menu"
        }
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        !self.panel && self.model.borrow().by_key
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size((1, 1).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(ZoneApp {
            panel: self.panel,
            model: self.model.clone(),
            state: self.state.clone(),
        })
    }

    fn corner_radius(&self, theme: &Theme) -> f32 {
        theme.tokens.radius_panel
    }

    fn anchor(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
}

struct ZoneApp {
    panel: bool,
    model: Rc<RefCell<ZoneModel>>,
    state: Rc<RefCell<Option<State<ZoneModel>>>>,
}

impl App for ZoneApp {
    fn render(&self) -> impl IntoElement {
        let state = use_hook(|| {
            let state = State::create(self.model.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Zones {
            panel: self.panel,
            state,
        }
    }
}

#[derive(PartialEq)]
struct Zones {
    panel: bool,
    state: State<ZoneModel>,
}

impl Component for Zones {
    fn render(&self) -> impl IntoElement {
        let theme = isle_ui::use_theme();
        let t = &theme.tokens;
        let model = self.state.read().clone();
        let actions = Actions::get();
        let mut hovered = use_state(|| None::<String>);
        let mut keyed = use_state(|| None::<String>);
        let grid = Grid::of(&model, &theme);
        let panel = self.panel;
        let scale = theme.appearance.text_scale;
        let lit = hovered
            .read()
            .clone()
            .or_else(|| keyed.read().clone())
            .or(model.selected.clone());

        let thumbs = grid.thumbs.iter().map(|r| {
            rect()
                .position(Position::new_absolute().left(r[0]).top(r[1]))
                .width(Size::px(r[2]))
                .height(Size::px(r[3]))
                .corner_radius(t.radius_control)
                .background(isle_ui::colour(t.colour_fill))
                .into_element()
        });
        let outlines = grid.outlines.iter().map(|r| {
            rect()
                .position(Position::new_absolute().left(r[0]).top(r[1]))
                .width(Size::px(r[2]))
                .height(Size::px(r[3]))
                .border(
                    Border::new()
                        .fill(isle_ui::colour(t.colour_hairline_strong))
                        .width(1.)
                        .alignment(BorderAlignment::Inner),
                )
                .into_element()
        });
        let slots = grid.slots.iter().map(|slot| {
            let zone = slot.zone.clone();
            let r = slot.rect;
            let fill = if lit.as_deref() == Some(zone.as_str()) {
                t.colour_accent
            } else if model.current.as_deref() == Some(zone.as_str()) {
                t.colour_text_2
            } else {
                t.colour_fill_strong
            };
            rect()
                .position(Position::new_absolute().left(r[0]).top(r[1]))
                .width(Size::px(r[2]))
                .height(Size::px(r[3]))
                .corner_radius(t.radius_small)
                .background(isle_ui::colour(fill))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(zone_label(&zone))
                .into_element()
        });
        // The grid finds the slot under the pointer itself, as the compositor does for the
        // pointer panel, from the same layout.
        let hit = grid.slots.clone();
        let zone_at = move |x: f32, y: f32| {
            hit.iter()
                .find(|s| {
                    let r = s.rect;
                    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
                })
                .map(|s| s.zone.clone())
        };
        let (move_actions, leave_actions, press_actions) =
            (actions.clone(), actions.clone(), actions.clone());
        let grid_box = rect()
            .width(Size::px(grid.width))
            .height(Size::px(grid.height))
            .maybe(!panel, |el| {
                el.on_mouse_move(move |e: Event<MouseEventData>| {
                    let at = e.element_location;
                    let zone = zone_at(at.x as f32, at.y as f32);
                    if *hovered.peek() != zone {
                        hovered.set(zone.clone());
                        move_actions.send(Action::Zone(ZoneAction::Preview(zone)));
                    }
                })
                .on_pointer_leave(move |_| {
                    if hovered.peek().is_some() {
                        hovered.set(None);
                        leave_actions.send(Action::Zone(ZoneAction::Preview(None)));
                    }
                })
                .on_press(move |_| {
                    if let Some(zone) = hovered.peek().clone() {
                        press_actions.send(Action::Zone(ZoneAction::Choose(zone)));
                    }
                })
            })
            .children(thumbs.chain(outlines).chain(slots));

        let section = |text: &str| {
            label()
                .font_size(t.type_style_subheadline_size * scale)
                .font_weight(t.type_weight_semibold as i32)
                .color(isle_ui::colour(t.colour_text_2))
                .text(text.to_string())
        };
        let shown_name = lit
            .as_deref()
            .or(model.current.as_deref())
            .map(zone_label)
            .unwrap_or("");
        let menu_row = |text: String, action: ZoneAction| {
            let actions = actions.clone();
            rect()
                .width(Size::fill())
                .height(Size::px(row(&theme)))
                .main_align(Alignment::Center)
                .padding((0., t.space_xs))
                .corner_radius(t.radius_row)
                .a11y_role(AccessibilityRole::MenuItem)
                .on_press(move |_| actions.send(Action::Zone(action.clone())))
                .child(
                    label()
                        .font_size(t.type_style_body_size * scale)
                        .color(isle_ui::colour(t.colour_text))
                        .text(text),
                )
        };

        // Keyboard: arrows move through the slots in order, Return chooses, Escape closes.
        let order: Vec<String> = grid.slots.iter().map(|s| s.zone.clone()).collect();
        let key_actions = actions.clone();
        let on_key = move |e: Event<KeyboardEventData>| {
            let step = match e.key {
                Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) => 1,
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) => -1,
                Key::Named(NamedKey::Enter) => {
                    if let Some(zone) = keyed.peek().clone() {
                        key_actions.send(Action::Zone(ZoneAction::Choose(zone)));
                    }
                    return;
                }
                Key::Named(NamedKey::Escape) => {
                    key_actions.send(Action::Close);
                    return;
                }
                _ => return,
            };
            let at = keyed
                .peek()
                .as_ref()
                .and_then(|k| order.iter().position(|z| z == k));
            let next = match at {
                None => 0,
                Some(i) => (i as i32 + step).rem_euclid(order.len() as i32) as usize,
            };
            keyed.set(Some(order[next].clone()));
            key_actions.send(Action::Zone(ZoneAction::Preview(Some(order[next].clone()))));
        };

        let content = rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(t.space_m)
            .color(isle_ui::colour(t.colour_text))
            .maybe(panel, |el| el.child(grid_box.clone()))
            .maybe(!panel, |el| {
                el.spacing(t.space_xs)
                    .child(section("Move and resize"))
                    .child(grid_box)
                    .child(
                        label()
                            .font_size(t.type_style_callout_size * scale)
                            .color(isle_ui::colour(t.colour_text_2))
                            .text(shown_name.to_string()),
                    )
                    .maybe(model.can_restore, |el| {
                        el.child(menu_row(
                            "Return to previous size".into(),
                            ZoneAction::Restore,
                        ))
                    })
                    .child(rect().height(Size::px(t.space_m - t.space_xs)))
                    .child(section("Full screen"))
                    .child({
                        // Full screen, then Split View's two sides, as macOS's three
                        // thumbnails.
                        let tw = (t.size_zone_thumbnail * model.aspect).round();
                        let thumb = |alt: &'static str, action: ZoneAction, half: Option<bool>| {
                            let actions = actions.clone();
                            let w = (tw / 2.0 - 1.0).floor();
                            rect()
                                .width(Size::px(tw))
                                .height(Size::px(t.size_zone_thumbnail))
                                .corner_radius(t.radius_control)
                                .overflow(Overflow::Clip)
                                .background(isle_ui::colour(match half {
                                    None => t.colour_fill_strong,
                                    Some(_) => t.colour_fill,
                                }))
                                .direction(Direction::Horizontal)
                                .main_align(match half {
                                    Some(false) => Alignment::End,
                                    _ => Alignment::Start,
                                })
                                .maybe(half.is_some(), |el| {
                                    el.child(
                                        rect()
                                            .width(Size::px(w))
                                            .height(Size::fill())
                                            .background(isle_ui::colour(t.colour_fill_strong)),
                                    )
                                })
                                .a11y_role(AccessibilityRole::Button)
                                .a11y_alt(alt)
                                .on_press(move |_| actions.send(Action::Zone(action.clone())))
                        };
                        let mut row = rect()
                            .direction(Direction::Horizontal)
                            .spacing(t.space_s)
                            .child(thumb(
                                if model.full_screen { "Exit Full Screen" } else { "Enter Full Screen" },
                                ZoneAction::FullScreen,
                                None,
                            ));
                        if !model.full_screen {
                            row = row
                                .child(thumb("Left of screen", ZoneAction::Split { left: true }, Some(true)))
                                .child(thumb("Right of screen", ZoneAction::Split { left: false }, Some(false)));
                        }
                        row
                    })
                    .maybe(model.split_pane, |el| {
                        el.child(menu_row("Swap sides".into(), ZoneAction::Swap))
                            .child(menu_row("Replace window…".into(), ZoneAction::Replace))
                    })
                    .children(model.displays.iter().enumerate().map(|(i, name)| {
                        menu_row(format!("Move to {name}"), ZoneAction::Display(i)).into_element()
                    }))
            });

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .maybe(!panel, |el| el.on_global_key_down(on_key))
            .child(Glass::new(t.radius_panel))
            .child(content)
    }
}
