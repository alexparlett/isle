//! Force Quit: every running app, those not responding marked, and the sheet that asks once
//! (`surfaces/prompts.md`, Force Quit).

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Button, Glass, Glyph, Hit, button::Kind, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// Over the list, which scrolls in layers of its own; the sheet comes later, so above.
const SCRIM: freya::prelude::Layer = freya::prelude::Layer::Overlay;
const SHEET: freya::prelude::Layer = freya::prelude::Layer::Overlay;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub app: String,
    pub name: String,
    pub icon: Option<PathBuf>,
    pub not_responding: bool,
    /// A kill was refused.
    pub failed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub rows: Vec<Row>,
    pub selected: Vec<String>,
    /// The sheet asking once is open.
    pub asking: bool,
    pub solid: bool,
}

impl Frame {
    /// The sheet's question, for one app or several.
    pub fn question(&self) -> String {
        match self.selected.as_slice() {
            [app] => {
                let name = self
                    .rows
                    .iter()
                    .find(|r| &r.app == app)
                    .map_or(app.as_str(), |r| r.name.as_str());
                format!("Do you want to force {name} to quit? You will lose any unsaved changes.")
            }
            several => format!(
                "Do you want to force {} apps to quit? You will lose any unsaved changes.",
                several.len()
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForceQuitAction {
    /// A click on a row; the compositor reads Cmd and Shift.
    Select(String),
    /// Up or Down, extending the selection with Shift or Cmd.
    Step {
        step: i32,
        extend: bool,
    },
    /// The panel's button or Return: the sheet.
    Ask,
    Confirm,
    Cancel,
    Close,
}

#[derive(Default)]
pub struct ForceQuitSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
}

impl ForceQuitSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame;
    }

    /// Puts `frame` in force; call it in the tree's root scope.
    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let rows = t.limit_force_quit_rows as f32 * t.size_row + 2.0 * t.space_xxs;
        let height = 160.0 * t.type_scale_max.max(1.0) + rows + 2.0 * t.space_m;
        (
            (t.size_panel + 2.0 * m).ceil() as i32,
            (height + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for ForceQuitSurface {
    fn name(&self) -> &'static str {
        "force-quit"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(self.size.get())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(ForceQuitApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.5, 0.0)
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct ForceQuitApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for ForceQuitApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.frame.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Panel { frame: state }
    }
}

#[derive(PartialEq)]
struct Panel {
    frame: State<Frame>,
}

fn send(actions: &Actions, action: ForceQuitAction) {
    actions.send(Action::ForceQuit(action));
}

impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);
        let solid = frame.solid || theme.appearance.reduce_transparency;
        let mut content_height = use_state(|| 0.0f32);
        let inner = t.size_panel - 2.0 * t.space_m;

        let shown = frame.rows.len().clamp(1, t.limit_force_quit_rows as usize) as f32;
        let rows = rect()
            .width(Size::fill())
            .height(Size::px(shown * t.size_row + 2.0 * t.space_xxs))
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill))
            .padding(t.space_xxs)
            .child(
                isle_ui::scroll_area(&theme).children(frame.rows.iter().map(|row| {
                    RowView {
                        row: row.clone(),
                        selected: frame.selected.contains(&row.app),
                    }
                    .into_element()
                })),
            )
            // Under the sheet the list takes the scrim.
            .maybe_child(frame.asking.then(|| {
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .layer(SCRIM)
                    .width(Size::fill())
                    .height(Size::fill())
                    .corner_radius(t.radius_control)
                    .background(colour(t.colour_scrim))
            }));

        let ask = actions.clone();
        let mut body = rect()
            .width(Size::px(inner))
            .child(text::line(&theme, text::Style::Title3, ink, "Force Quit"))
            .child(rect().height(Size::px(t.space_xs)))
            .child(
                label()
                    .width(Size::fill())
                    .font_size(t.type_style_callout_size)
                    .color(ink_2)
                    .text(
                        "If an app doesn't respond for a while, select it and choose Force Quit.",
                    ),
            )
            .child(rect().height(Size::px(t.space_m)))
            .child(rows)
            .child(rect().height(Size::px(t.space_m)))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .main_align(Alignment::End)
                    .child(
                        Button::new("Force Quit")
                            .kind(Kind::Default)
                            .enabled(!frame.selected.is_empty() && !frame.asking)
                            .on_press(move |_| send(&ask, ForceQuitAction::Ask)),
                    ),
            );

        // The sheet hangs under the title row, the list under the scrim.
        if frame.asking {
            let (cancel, confirm) = (actions.clone(), actions.clone());
            body = body.child(
                rect()
                    .position(
                        Position::new_absolute()
                            .top(t.type_style_title_3_line)
                            .left(0.),
                    )
                    .layer(SHEET)
                    .width(Size::px(inner))
                    .child(
                        rect()
                            .width(Size::fill())
                            .background(colour(t.material_solid))
                            .corner_radius((0., 0., t.radius_card, t.radius_card))
                            .padding(t.space_l)
                            .map(Some(shadows(&t.shadow_panel)), |el, list| {
                                list.into_iter().fold(el, |el, s| el.shadow(s))
                            })
                            .child(
                                label()
                                    .width(Size::fill())
                                    .font_size(t.type_style_body_size)
                                    .color(ink)
                                    .text(frame.question()),
                            )
                            .child(rect().height(Size::px(t.space_l)))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .main_align(Alignment::End)
                                    .spacing(t.space_s)
                                    .child(
                                        Button::new("Force Quit").kind(Kind::Destructive).on_press(
                                            move |_| send(&confirm, ForceQuitAction::Confirm),
                                        ),
                                    )
                                    .child(
                                        Button::new("Cancel").kind(Kind::Default).on_press(
                                            move |_| send(&cancel, ForceQuitAction::Cancel),
                                        ),
                                    ),
                            ),
                    ),
            );
        }

        let keys = actions.clone();
        let asking = frame.asking;
        rect()
            .expanded()
            .padding(ForceQuitSurface::margin(&theme))
            .cross_align(Alignment::Center)
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let cmd = e.modifiers.contains(Modifiers::META);
                let shift = e.modifiers.contains(Modifiers::SHIFT);
                let action = match &e.key {
                    Key::Named(NamedKey::Escape) if asking => Some(ForceQuitAction::Cancel),
                    Key::Named(NamedKey::Enter) if asking => Some(ForceQuitAction::Cancel),
                    _ if asking => None,
                    Key::Named(NamedKey::Escape) => Some(ForceQuitAction::Close),
                    Key::Character(c) if cmd && c.eq_ignore_ascii_case("w") => {
                        Some(ForceQuitAction::Close)
                    }
                    Key::Named(NamedKey::Enter) => Some(ForceQuitAction::Ask),
                    Key::Named(NamedKey::ArrowUp) => Some(ForceQuitAction::Step {
                        step: -1,
                        extend: cmd || shift,
                    }),
                    Key::Named(NamedKey::ArrowDown) => Some(ForceQuitAction::Step {
                        step: 1,
                        extend: cmd || shift,
                    }),
                    _ => None,
                };
                if let Some(action) = action {
                    send(&keys, action);
                }
            })
            .child(
                rect()
                    .width(Size::px(t.size_panel))
                    .corner_radius(t.radius_panel)
                    .background(if solid {
                        colour(t.material_solid)
                    } else {
                        Color::TRANSPARENT
                    })
                    .map(Some(shadows(&t.shadow_panel)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .maybe(*content_height.read() > 0.0, |el| {
                        el.height(Size::px(*content_height.read() + 2.0 * t.space_m))
                    })
                    .maybe(!solid && *content_height.read() > 0.0, |el| {
                        el.child(Glass::new(t.radius_panel))
                    })
                    .child(Hit)
                    .child(rect().padding(t.space_m).child(body.on_sized(
                        move |e: Event<SizedEventData>| {
                            let h = e.area.height();
                            if (h - *content_height.peek()).abs() > 0.5 {
                                content_height.set(h);
                            }
                        },
                    ))),
            )
            .into_element()
    }
}

#[derive(PartialEq)]
struct RowView {
    row: Row,
    selected: bool,
}

impl Component for RowView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let row = &self.row;
        let (ink, ink_2) = if self.selected {
            (colour(t.colour_on_accent), colour(t.colour_on_accent))
        } else {
            (colour(t.colour_text), colour(t.colour_text_2))
        };
        let note = if row.failed {
            Some("Couldn't force quit")
        } else if row.not_responding {
            Some("Not responding")
        } else {
            None
        };
        let app = row.app.clone();
        let size = t.size_app_icon_small;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .corner_radius(t.radius_small)
            .background(if self.selected {
                colour(t.colour_accent)
            } else {
                Color::TRANSPARENT
            })
            .padding((0., t.space_s))
            .content(Content::flex())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .a11y_role(AccessibilityRole::ListItem)
            .a11y_alt(match note {
                Some(note) => format!("{}, {note}", row.name),
                None => row.name.clone(),
            })
            .on_press(move |_| send(&actions, ForceQuitAction::Select(app.clone())))
            .child(
                rect()
                    .width(Size::px(size))
                    .height(Size::px(size))
                    .center()
                    .child(match &row.icon {
                        Some(icon) => AppIcon::new(icon.clone(), size).into_element(),
                        None => Glyph::new("app-window", size, ink_2).into_element(),
                    }),
            )
            .child(rect().width(Size::flex(1.)).child(text::line(
                &theme,
                text::Style::Body,
                ink,
                row.name.clone(),
            )))
            .maybe_child(note.map(|note| {
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .maybe_child(row.failed.then(|| Glyph::new("warning-circle", 12., ink_2)))
                    .child(text::line(&theme, text::Style::Callout, ink_2, note))
            }))
    }
}
