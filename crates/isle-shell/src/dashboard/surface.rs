//! The dashboard's surface: the widgets on their plates over the compositor's softened
//! backdrop, and the two controls (`p1/surfaces/widgets.md`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, colour, focus_ring, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::grid::{Cells, Extent};
use crate::prompts::session::Choice;
use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// What a widget's place holds now.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// Before its first data: its name alone.
    Title,
    /// Nothing to show: one line in a strip at the top of its place.
    Empty(String),
    Clock(ClockView),
    Session,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockView {
    pub city: String,
    pub time: String,
    pub weekday: String,
    pub date: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetView {
    pub id: String,
    pub name: String,
    pub at: Cells,
    pub body: Body,
}

/// What the dashboard shows now.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub extent: Extent,
    pub widgets: Vec<WidgetView>,
    pub edit: bool,
    pub selected: Option<String>,
    /// The ring shows: the selection came from a key.
    pub ring: bool,
    /// Plates solid: Game Mode or Reduce Transparency.
    pub solid: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            extent: Extent {
                cols: 0,
                rows: 0,
                x: 0.0,
                y: 0.0,
                cell: 72.0,
                gap: 16.0,
            },
            widgets: Vec::new(),
            edit: false,
            selected: None,
            ring: false,
            solid: false,
        }
    }
}

/// What the pointer does on the dashboard.
#[derive(Clone, Debug, PartialEq)]
pub enum DashboardAction {
    Close,
    Edit,
    Done,
    /// "Add widgets": the gallery shown or hidden.
    Gallery,
    /// A click on a widget's body.
    Open(String),
    Session(Choice),
}

#[derive(Default)]
pub struct DashboardSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
}

impl DashboardSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    pub fn frame(&self) -> Frame {
        self.frame.borrow().clone()
    }
}

impl SurfaceDef for DashboardSurface {
    fn name(&self) -> &'static str {
        "dashboard"
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
        AppComponent::new(DashboardApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
        })
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct DashboardApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for DashboardApp {
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
        Root { frame: state }
    }
}

fn send(actions: &Actions, action: DashboardAction) {
    actions.send(Action::Dashboard(action));
}

/// A plate: glass at the user's position with no blur pass, since the backdrop is already
/// soft (`lock.md`'s); solid under Reduce Transparency, edged under Increase Contrast.
pub fn plate(theme: &Theme, solid: bool, radius: f32) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let contrast = theme.appearance.increase_contrast;
    let fill = if solid || theme.appearance.reduce_transparency {
        t.material_solid
    } else if contrast {
        t.material_glass_tinted
    } else {
        t.material_glass
    };
    rect()
        .corner_radius(radius)
        .background(colour(fill))
        .map(Some(shadows(&t.shadow_menu)), |el, list| {
            list.into_iter().fold(el, |el, s| el.shadow(s))
        })
        .maybe(contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(t.stroke_hairline)
                    .alignment(BorderAlignment::Inner),
            )
        })
}

#[derive(PartialEq)]
struct Root {
    frame: State<Frame>,
}

impl Component for Root {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let mut root = rect().expanded().a11y_role(AccessibilityRole::Grid).a11y_alt("Widgets");
        for w in &frame.widgets {
            root = root.child(widget(&theme, &frame, w, &actions));
        }
        if frame.widgets.is_empty() {
            root = root.child(
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .expanded()
                    .center()
                    .child(text::line(
                        &theme,
                        text::Style::Title3,
                        colour(t.colour_text_2),
                        "No widgets",
                    )),
            );
        }
        root.child(controls(&theme, &frame, &actions))
    }
}

fn widget(theme: &Theme, frame: &Frame, w: &WidgetView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (x, y, width, height) = frame.extent.rect(w.at);
    let selected = frame.selected.as_deref() == Some(w.id.as_str());
    let place = rect()
        .position(Position::new_absolute().left(x).top(y))
        .width(Size::px(width))
        .height(Size::px(height))
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(w.name.clone());
    let ring = |el: freya::prelude::Rect| el.maybe(selected && frame.ring, |el| el.border(focus_ring(theme)));
    let open = {
        let (actions, id) = (actions.clone(), w.id.clone());
        move |e: Event<PressEventData>| {
            e.stop_propagation();
            send(&actions, DashboardAction::Open(id.clone()));
        }
    };
    let body: Element = match &w.body {
        Body::Empty(line) => {
            return place
                .child(ring(
                    plate(theme, frame.solid, t.radius_panel)
                        .width(Size::fill())
                        .height(Size::px(t.size_row_large))
                        .padding((0., t.space_m))
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .child(text::line(theme, text::Style::Body, colour(t.colour_text_2), line.clone())),
                ))
                .into_element();
        }
        Body::Title => text::line(
            theme,
            text::Style::Subheadline,
            colour(t.colour_text_2),
            w.name.clone(),
        )
        .font_weight(600)
        .into_element(),
        Body::Clock(c) => clock(theme, w.at, c),
        Body::Session => session(theme, frame.solid, w.at, actions),
    };
    let inset = if matches!(w.body, Body::Session) { t.space_s } else { t.space_m };
    place
        .child(ring(
            plate(theme, frame.solid, t.radius_panel)
                .expanded()
                .padding(inset)
                .on_press(open)
                .child(body),
        ))
        .into_element()
}

fn clock(theme: &Theme, at: Cells, c: &ClockView) -> Element {
    let t = &theme.tokens;
    let ink = colour(t.colour_text);
    let ink_2 = colour(t.colour_text_2);
    if at.h == 1 {
        return rect()
            .expanded()
            .horizontal()
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(theme, text::Style::Headline, ink, c.city.clone()))
                    .child(text::line(theme, text::Style::Subheadline, ink_2, c.weekday.clone())),
            )
            .child(text::line(theme, text::Style::Title, ink, c.time.clone()))
            .a11y_alt(format!("{}, {}", c.time, c.city))
            .into_element();
    }
    rect()
        .expanded()
        .main_align(Alignment::SpaceBetween)
        .child(
            text::line(theme, text::Style::Subheadline, ink_2, c.city.clone()).font_weight(600),
        )
        .child(
            rect()
                .child(text::line(theme, text::Style::Figure, ink, c.time.clone()))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    ink_2,
                    format!("{} {}", c.weekday, c.date),
                )),
        )
        .a11y_alt(format!("{}, {}", c.time, c.city))
        .into_element()
}

fn session(theme: &Theme, solid: bool, at: Cells, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let named = at.h >= 2;
    let circle = |choice: Choice| {
        let (glyph, label) = match choice {
            Choice::Lock => ("lock", "Lock Screen"),
            Choice::Sleep => ("moon-stars", "Sleep"),
            Choice::Restart => ("arrow-clockwise", "Restart…"),
            Choice::ShutDown => ("power", "Shut Down…"),
            Choice::LogOut => ("sign-out", "Log Out…"),
        };
        let actions = actions.clone();
        rect()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(
                rect()
                    .width(Size::px(t.size_lock_button))
                    .height(Size::px(t.size_lock_button))
                    .corner_radius(t.size_lock_button / 2.0)
                    .background(colour(t.colour_fill_strong))
                    .center()
                    .a11y_focusable(true)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(label)
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        send(&actions, DashboardAction::Session(choice));
                    })
                    .child(Glyph::new(glyph, t.size_glyph_large, colour(t.colour_text))),
            )
            .maybe_child(named.then(|| {
                text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), label)
            }))
            .into_element()
    };
    let _ = solid;
    rect()
        .expanded()
        .horizontal()
        .main_align(Alignment::SpaceAround)
        .children(
            [Choice::Lock, Choice::Sleep, Choice::Restart, Choice::ShutDown, Choice::LogOut]
                .map(circle),
        )
        .into_element()
}

fn controls(theme: &Theme, frame: &Frame, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let foot = t.space_xxxl;
    let capsule = |label: &'static str, action: DashboardAction| {
        let actions = actions.clone();
        plate(theme, frame.solid, t.size_control_large / 2.0)
            .height(Size::px(t.size_control_large))
            .padding((0., t.space_m))
            .center()
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(label)
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&actions, action.clone());
            })
            .child(text::line(theme, text::Style::Body, colour(t.colour_text), label))
            .into_element()
    };
    let row = rect()
        .position(Position::new_absolute().bottom(foot).left(0.))
        .width(Size::fill())
        .horizontal()
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .spacing(t.space_s);
    if frame.edit {
        let done = actions.clone();
        row.child(capsule("Add widgets", DashboardAction::Gallery)).child(
            Button::new("Done")
                .kind(isle_ui::button::Kind::Default)
                .on_press(move |_| send(&done, DashboardAction::Done)),
        )
    } else {
        let close = actions.clone();
        row.child(capsule("Edit widgets", DashboardAction::Edit)).child(
            plate(theme, frame.solid, t.size_control_large / 2.0)
                .width(Size::px(t.size_control_large))
                .height(Size::px(t.size_control_large))
                .center()
                .a11y_focusable(true)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt("Close widgets")
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    send(&close, DashboardAction::Close);
                })
                .child(Glyph::new("x", t.size_glyph, colour(t.colour_text))),
        )
    }
    .into_element()
}
