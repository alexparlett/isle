//! The dashboard's surface: the widgets on their plates over the compositor's softened
//! backdrop, and the two controls (`p1/surfaces/widgets.md`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    Button, Glyph, Menu, MenuEntry, MenuEvent, Switch, TextField, colour, focus_ring,
    form::{PopUp, Stepper},
    menu::Material, shadows, text, use_theme,
};
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
    Media(crate::island::rows::MediaCard),
    Devices(Vec<BatteryRow>),
    Storage(Vec<VolumeRow>),
    Desktops(Vec<DesktopRow>),
    Notifications(Vec<NoteRow>),
    Controls(Vec<crate::island::panel::Module>),
    System(Vec<SeriesView>),
    Weather(WeatherView),
    Timer(TimerView),
    /// One line at the top of its place with what it needs, as an empty widget's: a
    /// provider failing, not allowed or not installed.
    Strip {
        glyph: Option<String>,
        line: String,
        /// A small button's label and the row it sends.
        button: Option<(String, String)>,
    },
    Provider(ProviderView),
}

/// A provider's widget: its title line and its template's answer.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderView {
    pub title: Option<String>,
    /// "Updated {time}" once it is stale.
    pub updated: Option<String>,
    pub data: ProviderData,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProviderData {
    Text(String),
    Number { value: String, unit: Option<String>, label: Option<String> },
    Gauge { value: f32, label: Option<String>, text: Option<String> },
    Sparkline { values: Vec<f64>, value: Option<String> },
    Graph(Vec<(String, Vec<f64>)>),
    /// Label, value, glyph.
    List(Vec<(String, Option<String>, Option<String>)>),
    Card { title: String, body: Option<String>, image: Option<std::path::PathBuf> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Running,
    Paused,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimerView {
    pub label: String,
    pub time: String,
    pub state: TimerState,
    /// What is left of it, 1 to 0.
    pub left: f32,
    /// Lengths in minutes.
    pub presets: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherView {
    pub place: String,
    pub now: String,
    pub glyph: String,
    pub condition: String,
    pub high_low: String,
    /// Hour, glyph, temperature.
    pub hours: Vec<(String, String, String)>,
    /// Weekday, glyph, low, high, and where low and high sit in the week's span, 0 to 1.
    pub days: Vec<(String, String, String, String, f32, f32)>,
}

/// One of System's series as drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesView {
    pub label: String,
    pub value: String,
    /// Its share of the whole, for a ring.
    pub fraction: Option<f32>,
    /// The last minute, 0 to 1, oldest first, for a graph.
    pub history: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockView {
    pub city: String,
    pub time: String,
    pub weekday: String,
    pub date: String,
    /// The analogue face: the hour and minute hands' turns from twelve, 0 to 1, the
    /// seconds' when shown, and night in the clock's zone.
    pub dial: Option<Dial>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dial {
    pub hour: f32,
    pub minute: f32,
    pub second: Option<f32>,
    pub night: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryRow {
    pub name: String,
    pub glyph: String,
    pub level: Option<u8>,
    pub charging: bool,
    pub low: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolumeRow {
    pub name: String,
    /// What it opens in Files.
    pub path: String,
    pub used: f32,
    pub free: String,
    pub nearly_full: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRow {
    pub name: String,
    pub icons: Vec<Option<std::path::PathBuf>>,
    pub more: usize,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteRow {
    pub app: String,
    pub name: String,
    pub icon: Option<std::path::PathBuf>,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetView {
    pub id: String,
    pub name: String,
    pub at: Cells,
    pub body: Body,
}

/// The widget lifted by a drag, at the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct Lifted {
    pub id: String,
    /// Its top left in points.
    pub x: f32,
    pub y: f32,
}

/// Where a drag or resize will land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ghost {
    pub at: Cells,
    pub refused: bool,
}

/// A menu open on the dashboard: a widget's, a gap's.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuView {
    pub entries: Rc<Vec<MenuEntry>>,
    pub x: f32,
    pub y: f32,
    pub by_key: bool,
}

/// A widget's settings popover.
#[derive(Clone, Debug, PartialEq)]
pub struct FormView {
    pub id: String,
    pub rows: Vec<FormRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormRow {
    pub key: String,
    pub label: String,
    pub control: Control,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    Toggle(bool),
    Choice { options: Vec<String>, selected: usize },
    Number { value: i64, min: i64, max: i64 },
    Text(String),
}

/// A value set in a widget's form.
#[derive(Clone, Debug, PartialEq)]
pub enum FormValue {
    Bool(bool),
    /// The chosen option's place.
    Index(usize),
    Int(i64),
    Text(String),
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
    pub lifted: Option<Lifted>,
    pub ghost: Option<Ghost>,
    pub menu: Option<MenuView>,
    pub form: Option<FormView>,
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
            lifted: None,
            ghost: None,
            menu: None,
            form: None,
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
    Menu(MenuEvent),
    /// A row inside a widget: a volume's path, a desktop's place, an app's id, a transport
    /// key, or `more` for its list's place.
    Row { id: String, row: String },
    Form { id: String, key: String, value: FormValue },
    /// The popover dismissed.
    FormClosed,
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
        if frame.edit {
            root = root.child(cells(&theme, &frame));
        }
        if let Some(ghost) = frame.ghost {
            root = root.child(ghost_view(&theme, &frame.extent, ghost));
        }
        let lifted = frame.lifted.as_ref().map(|l| l.id.clone());
        for w in &frame.widgets {
            if lifted.as_deref() == Some(w.id.as_str()) {
                continue;
            }
            root = root.child(widget(&theme, &frame, w, None, &actions));
        }
        if let Some(l) = &frame.lifted
            && let Some(w) = frame.widgets.iter().find(|w| w.id == l.id)
        {
            root = root.child(widget(&theme, &frame, w, Some((l.x, l.y)), &actions));
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
        root = root.child(controls(&theme, &frame, &actions));
        if let Some(form) = &frame.form
            && let Some(w) = frame.widgets.iter().find(|w| w.id == form.id)
        {
            root = root.child(form_view(&theme, &frame.extent, w.at, form, &actions));
        }
        if let Some(menu) = &frame.menu {
            let on_menu = actions.clone();
            root = root.child(
                rect()
                    .position(Position::new_absolute().left(menu.x).top(menu.y))
                    .child(
                        Menu::new(menu.entries.clone(), move |e: MenuEvent| {
                            send(&on_menu, DashboardAction::Menu(e))
                        })
                        .material(Material::Solid)
                        .first_selected(menu.by_key),
                    ),
            );
        }
        root
    }
}

/// Every free cell, so places can be read in edit mode.
fn cells(theme: &Theme, frame: &Frame) -> Element {
    let t = &theme.tokens;
    let e = &frame.extent;
    let taken = |x: i32, y: i32| frame.widgets.iter().any(|w| w.at.contains(x, y));
    let mut layer = rect().position(Position::new_absolute().left(0.).top(0.)).expanded();
    for y in 0..e.rows {
        for x in 0..e.cols {
            if taken(x, y) {
                continue;
            }
            let (px, py, w, h) = e.rect(Cells::new(x, y, 1, 1));
            layer = layer.child(
                rect()
                    .position(Position::new_absolute().left(px).top(py))
                    .width(Size::px(w))
                    .height(Size::px(h))
                    .corner_radius(t.radius_control)
                    .background(colour(t.colour_fill)),
            );
        }
    }
    layer.into_element()
}

fn ghost_view(theme: &Theme, e: &Extent, ghost: Ghost) -> Element {
    let t = &theme.tokens;
    let (x, y, w, h) = e.rect(ghost.at);
    let edge = if ghost.refused { t.colour_danger } else { t.colour_hairline_strong };
    rect()
        .position(Position::new_absolute().left(x).top(y))
        .width(Size::px(w))
        .height(Size::px(h))
        .corner_radius(t.radius_panel)
        .maybe(!ghost.refused, |el| el.background(colour(t.colour_fill)))
        .border(
            Border::new()
                .fill(colour(edge))
                .width(t.stroke_hairline)
                .alignment(BorderAlignment::Inner),
        )
        .center()
        .maybe_child(
            ghost
                .refused
                .then(|| Glyph::new("prohibit", 24., colour(t.colour_text))),
        )
        .into_element()
}

fn form_view(theme: &Theme, e: &Extent, at: Cells, form: &FormView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (x, y, w, _) = e.rect(at);
    let width = 240.0;
    // Beside the trailing edge, or the leading one where the screen ends first.
    let right_room = e.x + e.cols as f32 * e.pitch() - (x + w + t.space_s);
    let left = if right_room >= width { x + w + t.space_s } else { x - t.space_s - width };
    let rows = form.rows.iter().map(|row| {
        let (actions, id, key) = (actions.clone(), form.id.clone(), row.key.clone());
        let set = move |value: FormValue| {
            send(
                &actions,
                DashboardAction::Form {
                    id: id.clone(),
                    key: key.clone(),
                    value,
                },
            )
        };
        let control: Element = match &row.control {
            Control::Toggle(on) => {
                Switch::new(*on, row.label.clone(), move |on| set(FormValue::Bool(on))).into_element()
            }
            Control::Choice { options, selected } => PopUp::new(
                options.clone(),
                *selected,
                row.label.clone(),
                move |i| set(FormValue::Index(i)),
            )
            .into_element(),
            Control::Number { value, min, max } => {
                Stepper::new(*value, (*min, *max), row.label.clone(), move |v| set(FormValue::Int(v)))
                    .into_element()
            }
            Control::Text(text) => TextFieldRow {
                value: text.clone(),
                alt: row.label.clone(),
                on_submit: EventHandler::new(move |v: String| set(FormValue::Text(v))),
            }
            .into_element(),
        };
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(
                rect().width(Size::flex(1.)).child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    row.label.clone(),
                )),
            )
            .child(control)
            .into_element()
    });
    rect()
        .position(Position::new_absolute().left(left).top(y))
        .width(Size::px(width))
        .padding(t.space_m)
        .spacing(t.space_s)
        .corner_radius(t.radius_card)
        .background(colour(t.material_solid))
        .map(Some(shadows(&t.shadow_menu)), |el, list| {
            list.into_iter().fold(el, |el, s| el.shadow(s))
        })
        .a11y_role(AccessibilityRole::Dialog)
        .children(rows)
        .into_element()
}

/// A text setting, sent when Return is pressed in it.
#[derive(Clone, PartialEq)]
struct TextFieldRow {
    value: String,
    alt: String,
    on_submit: EventHandler<String>,
}

impl Component for TextFieldRow {
    fn render(&self) -> impl IntoElement {
        let value = use_state(|| self.value.clone());
        let submit = self.on_submit.clone();
        rect()
            .width(Size::px(120.))
            .child(TextField::new(value, self.alt.clone()).on_submit(move |v: String| submit.call(v)))
    }
}

fn widget(
    theme: &Theme,
    frame: &Frame,
    w: &WidgetView,
    lifted: Option<(f32, f32)>,
    actions: &Actions,
) -> Element {
    let t = &theme.tokens;
    let (x, y, width, height) = frame.extent.rect(w.at);
    let (x, y) = lifted.unwrap_or((x, y));
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
    let place = place
        .maybe(lifted.is_some(), |el| {
            el.map(Some(shadows(&t.shadow_panel)), |el, list| {
                list.into_iter().fold(el, |el, s| el.shadow(s))
            })
        })
        .maybe(frame.edit && matches!(w.body, Body::Empty(_)), |el| {
            el.corner_radius(t.radius_panel).background(colour(t.colour_fill))
        });
    let chrome = frame.edit.then(|| edit_chrome(theme, width, height, selected));
    let body: Element = match &w.body {
        Body::Strip { glyph, line, button } => {
            return place
                .maybe_child(chrome)
                .child(ring(
                    plate(theme, frame.solid, t.radius_panel)
                        .width(Size::fill())
                        .height(Size::px(t.size_row_large))
                        .padding((0., t.space_m))
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .a11y_alt(line.clone())
                        .maybe_child(glyph.clone().map(|g| Glyph::new(g, 16., colour(t.colour_text)).into_element()))
                        .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Body, colour(t.colour_text), line.clone())))
                        .maybe_child(button.clone().map(|(label, row)| {
                            let (actions, id) = (actions.clone(), w.id.clone());
                            Button::new(label)
                                .small()
                                .on_press(move |_| send(&actions, DashboardAction::Row { id: id.clone(), row: row.clone() }))
                                .into_element()
                        })),
                ))
                .into_element();
        }
        Body::Empty(line) => {
            return place
                .maybe_child(chrome)
                .child(ring(
                    plate(theme, frame.solid, t.radius_panel)
                        .width(Size::fill())
                        .height(Size::px(t.size_row_large))
                        .padding((0., t.space_m))
                        .horizontal()
            .content(Content::flex())
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
        Body::Media(card) => media(theme, w, card, actions),
        Body::Devices(rows) => devices(theme, w, rows, actions),
        Body::Storage(rows) => storage(theme, w, rows, actions),
        Body::Desktops(rows) => desktops(theme, w, rows, actions),
        Body::Notifications(rows) => notifications(theme, w, rows, actions),
        Body::System(series) => system(theme, w, series),
        Body::Weather(view) => weather(theme, w, view),
        Body::Timer(view) => timer(theme, w, view, actions),
        Body::Provider(view) => provider(theme, w, view),
        Body::Controls(modules) => {
            let inner = width - 2.0 * t.space_s;
            crate::island::panel::ModuleTiles { modules: modules.clone(), width: inner }.into_element()
        }
    };
    let inset = if matches!(w.body, Body::Session | Body::Controls(_)) { t.space_s } else { t.space_m };
    place
        .child(ring(
            plate(theme, frame.solid, t.radius_panel)
                .expanded()
                .padding(inset)
                .on_press(open)
                .child(body),
        ))
        .maybe_child(chrome)
        .into_element()
}

/// The remove badge and the resize handles over a widget in edit mode; the compositor reads
/// presses on them by their places.
fn edit_chrome(theme: &Theme, width: f32, height: f32, selected: bool) -> Element {
    let t = &theme.tokens;
    let ink = colour(if selected { t.colour_text } else { t.colour_text_2 });
    let handle = |w: f32, h: f32, left: f32, top: f32| {
        rect()
            .position(Position::new_absolute().left(left).top(top))
            .width(Size::px(w))
            .height(Size::px(h))
            .corner_radius(w.min(h) / 2.0)
            .background(ink)
            .into_element()
    };
    let long = t.size_control_small;
    rect()
        .position(Position::new_absolute().left(0.).top(0.))
        .width(Size::px(width))
        .height(Size::px(height))
        .layer(freya::prelude::Layer::Overlay)
        .child(
            rect()
                .position(Position::new_absolute().left(5.).top(5.))
                .width(Size::px(20.))
                .height(Size::px(20.))
                .corner_radius(10.)
                .background(colour(t.material_solid))
                .map(Some(shadows(&t.shadow_control)), |el, list| {
                    list.into_iter().fold(el, |el, s| el.shadow(s))
                })
                .border(
                    Border::new()
                        .fill(colour(t.colour_hairline))
                        .width(t.stroke_hairline)
                        .alignment(BorderAlignment::Inner),
                )
                .center()
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt("Remove widget")
                .child(Glyph::new("minus", 12., colour(t.colour_text))),
        )
        .child(handle(4., long, width - 8., (height - long) / 2.))
        .child(handle(4., long, 4., (height - long) / 2.))
        .child(handle(long, 4., (width - long) / 2., 4.))
        .child(handle(long, 4., (width - long) / 2., height - 8.))
        .child(
            rect()
                .position(Position::new_absolute().left(width - 16.).top(height - 16.))
                .width(Size::px(12.))
                .height(Size::px(12.))
                .border(Border::new().fill(ink).width(3.).alignment(BorderAlignment::Inner))
                .corner_radius(t.radius_control),
        )
        .into_element()
}

fn clock(theme: &Theme, at: Cells, c: &ClockView) -> Element {
    let t = &theme.tokens;
    let ink = colour(t.colour_text);
    let ink_2 = colour(t.colour_text_2);
    if let (Some(dial), true) = (c.dial, at.h >= 2) {
        let size = (at.h as f32 * 88.0 - 16.0) - 2.0 * t.space_m - t.type_style_subheadline_line;
        let face = dial_view(theme, dial, size, &format!("{}, {}", c.time, c.city));
        if at.w >= 4 {
            return rect()
                .expanded()
                .horizontal()
            .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_m)
                .child(face)
                .child(
                    rect()
                        .child(text::line(theme, text::Style::Title3, ink, c.city.clone()))
                        .child(text::line(theme, text::Style::Subheadline, ink_2, format!("{} {}", c.weekday, c.date))),
                )
                .into_element();
        }
        return rect()
            .expanded()
            .cross_align(Alignment::Center)
            .child(face)
            .child(text::line(theme, text::Style::Subheadline, ink_2, c.city.clone()))
            .into_element();
    }
    if at.h == 1 {
        return rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
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
            .content(Content::flex())
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
            .content(Content::flex())
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

/// The analogue face as an SVG of the tokens' colours (`p1/surfaces/widgets.md` Dial).
fn dial_view(theme: &Theme, dial: Dial, size: f32, alt: &str) -> Element {
    let t = &theme.tokens;
    let css = |c: isle_tokens::Colour| format!("rgba({},{},{},{})", c.r, c.g, c.b, c.a);
    let face = css(if dial.night { t.colour_fill_strong } else { t.colour_fill });
    let (ink, ink_2) = (css(t.colour_text), css(t.colour_text_2));
    let hand = |turn: f32, length: f32, width: f32, ink: &str| {
        let a = turn * std::f32::consts::TAU;
        format!(
            r#"<line x1="50" y1="50" x2="{:.2}" y2="{:.2}" stroke="{ink}" stroke-width="{width}" stroke-linecap="round"/>"#,
            50.0 + a.sin() * 50.0 * length,
            50.0 - a.cos() * 50.0 * length,
        )
    };
    let mut svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="50" fill="{face}"/>"#);
    let unit = 100.0 / size.max(1.0);
    for i in 0..12 {
        let quarter = i % 3 == 0;
        let (len, w, ink) = if quarter { (6.0, 2.0, &ink) } else { (4.0, 1.0, &ink_2) };
        let a = i as f32 / 12.0 * std::f32::consts::TAU;
        let (r0, r1) = (50.0 - 4.0 * unit, 50.0 - (4.0 + len) * unit);
        svg.push_str(&format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{ink}" stroke-width="{:.2}"/>"#,
            50.0 + a.sin() * r0, 50.0 - a.cos() * r0, 50.0 + a.sin() * r1, 50.0 - a.cos() * r1, w * unit,
        ));
    }
    svg.push_str(&hand(dial.hour, 0.5, 4.0 * unit, &ink));
    svg.push_str(&hand(dial.minute, 0.8, 3.0 * unit, &ink));
    if let Some(second) = dial.second {
        svg.push_str(&hand(second, 0.85, unit, &ink_2));
    }
    svg.push_str(&format!(r#"<circle cx="50" cy="50" r="{:.2}" fill="{ink}"/></svg>"#, 3.0 * unit));
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        svg.hash(&mut h);
        h.finish()
    };
    SvgViewer::new((key, Bytes::from(svg.into_bytes())))
        .show_loader(false)
        .width(Size::px(size))
        .height(Size::px(size))
        .a11y_alt(alt.to_owned())
        .into_element()
}

fn row_press(actions: &Actions, id: &str, row: &str) -> impl FnMut(Event<PressEventData>) + 'static {
    let (actions, id, row) = (actions.clone(), id.to_owned(), row.to_owned());
    move |e: Event<PressEventData>| {
        e.stop_propagation();
        send(&actions, DashboardAction::Row { id: id.clone(), row: row.clone() });
    }
}

/// How many rows of `row` points with `gap` between fit `height`.
fn rows_in(height: f32, row: f32, gap: f32) -> usize {
    (((height + gap) / (row + gap)).floor() as usize).max(1)
}

/// The rows a list widget shows, the last "{n} more" when the rest do not fit.
fn list_rows<T>(items: &[T], fit: usize) -> (&[T], usize) {
    if items.len() <= fit {
        (items, 0)
    } else {
        let shown = fit.saturating_sub(1);
        (&items[..shown], items.len() - shown)
    }
}

fn more_line(theme: &Theme, w: &WidgetView, more: usize, actions: &Actions) -> Option<Element> {
    let t = &theme.tokens;
    (more > 0).then(|| {
        rect()
            .on_press(row_press(actions, &w.id, "more"))
            .a11y_role(AccessibilityRole::Button)
            .child(text::line(theme, text::Style::Callout, colour(t.colour_text_2), format!("{more} more")))
            .into_element()
    })
}

fn inner(theme: &Theme, w: &WidgetView) -> (f32, f32) {
    let t = &theme.tokens;
    (
        w.at.w as f32 * 88.0 - 16.0 - 2.0 * t.space_m,
        w.at.h as f32 * 88.0 - 16.0 - 2.0 * t.space_m,
    )
}

fn transport(theme: &Theme, w: &WidgetView, card: &crate::island::rows::MediaCard, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let button = |glyph: &'static str, size: f32, row: &str, alt: &'static str, enabled: bool| {
        rect()
            .width(Size::px(t.size_control_large))
            .height(Size::px(t.size_control_large))
            .center()
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(alt)
            .maybe(enabled, |el| el.on_press(row_press(actions, &w.id, row)))
            .child(Glyph::new(glyph, size, colour(if enabled { t.colour_text } else { t.colour_text_disabled })).fill(true))
            .into_element()
    };
    rect()
        .horizontal()
            .content(Content::flex())
        .cross_align(Alignment::Center)
        .child(button("skip-back", 20., "previous", "Previous", card.can_previous))
        .child(button(if card.playing { "pause" } else { "play" }, 24., "play-pause", if card.playing { "Pause" } else { "Play" }, true))
        .child(button("skip-forward", 20., "next", "Next", card.can_next))
        .into_element()
}

fn art(theme: &Theme, card: &crate::island::rows::MediaCard, size: f32) -> Element {
    let t = &theme.tokens;
    let frame = rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(t.radius_control)
        .overflow(Overflow::Clip)
        .background(colour(t.colour_fill_strong))
        .center();
    match &card.art {
        Some(path) => frame
            .child(ImageViewer::new(path.clone()).width(Size::px(size)).height(Size::px(size)))
            .into_element(),
        None => frame.child(Glyph::new("music-notes", size / 2.0, colour(t.colour_text_2))).into_element(),
    }
}

fn media(theme: &Theme, w: &WidgetView, card: &crate::island::rows::MediaCard, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let titles = rect()
        .width(Size::flex(1.))
        .child(text::line(theme, text::Style::Headline, ink, card.title.clone()))
        .child(text::line(theme, text::Style::Subheadline, ink_2, card.artist.clone()));
    match (w.at.w, w.at.h) {
        (4, 1) => rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .child(art(theme, card, 48.))
            .child(titles)
            .child(transport(theme, w, card, actions))
            .into_element(),
        (2, 2) => rect()
            .expanded()
            .main_align(Alignment::SpaceBetween)
            .child(art(theme, card, t.size_media_art))
            .child(titles)
            .child(transport(theme, w, card, actions))
            .into_element(),
        _ => {
            let (_, h) = inner(theme, w);
            rect()
                .expanded()
                .horizontal()
            .content(Content::flex())
                .spacing(t.space_m)
                .child(art(theme, card, h))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .main_align(Alignment::SpaceBetween)
                        .child(text::line(theme, text::Style::Title3, ink, card.title.clone()))
                        .child(text::line(theme, text::Style::Subheadline, ink_2, card.artist.clone()))
                        .child(
                            rect()
                                .on_press(row_press(actions, &w.id, "raise"))
                                .child(text::line(theme, text::Style::Callout, ink_2, card.player.clone())),
                        )
                        .maybe_child(card.position.map(|p| {
                            rect()
                                .width(Size::fill())
                                .height(Size::px(4.))
                                .corner_radius(2.)
                                .background(colour(t.colour_fill_strong))
                                .child(rect().width(Size::percent(p * 100.)).height(Size::fill()).corner_radius(2.).background(ink))
                                .into_element()
                        }))
                        .child(transport(theme, w, card, actions)),
                )
                .into_element()
        }
    }
}

fn devices(theme: &Theme, w: &WidgetView, rows: &[BatteryRow], _actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (iw, ih) = inner(theme, w);
    let per_column = rows_in(ih, 18., t.space_xs);
    let columns = if w.at.w >= 4 { 2 } else { 1 };
    let (shown, more) = list_rows(rows, per_column * columns);
    let row = |b: &BatteryRow| {
        let ink = colour(t.colour_text);
        rect()
            .height(Size::px(18.))
            .width(Size::px(iw / columns as f32 - if columns > 1 { t.space_m / 2.0 } else { 0.0 }))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(Glyph::new(b.glyph.clone(), 12., colour(t.colour_text_2)))
            .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Callout, ink, b.name.clone())))
            .maybe_child(b.charging.then(|| Glyph::new("lightning", 12., ink).into_element()))
            .maybe_child(b.low.then(|| Glyph::new("battery-warning", 12., ink).into_element()))
            .child(
                text::line(theme, text::Style::Callout, ink, b.level.map_or(String::new(), |l| format!("{l}%")))
                    .font_weight(if b.low { 600 } else { 400 }),
            )
            .into_element()
    };
    let mut items: Vec<Element> = shown.iter().map(row).collect();
    if let Some(more) = more_line(theme, w, more, _actions) {
        items.push(more);
    }
    if columns == 1 {
        return rect().expanded().spacing(t.space_xs).children(items).into_element();
    }
    let split = items.len().div_ceil(2);
    let right = items.split_off(split);
    rect()
        .expanded()
        .horizontal()
            .content(Content::flex())
        .spacing(t.space_m)
        .child(rect().spacing(t.space_xs).children(items))
        .child(rect().spacing(t.space_xs).children(right))
        .into_element()
}

fn storage(theme: &Theme, w: &WidgetView, rows: &[VolumeRow], actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (_, ih) = inner(theme, w);
    let (shown, more) = list_rows(rows, rows_in(ih, 16., t.space_s));
    rect()
        .expanded()
        .spacing(t.space_s)
        .children(shown.iter().map(|v| {
            let ink = colour(t.colour_text);
            rect()
                .height(Size::px(16.))
                .width(Size::fill())
                .horizontal()
            .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .on_press(row_press(actions, &w.id, &v.path))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(format!("{}, {}", v.name, v.free))
                .child(Glyph::new(if v.nearly_full { "warning" } else { "hard-drive" }, 12., colour(if v.nearly_full { t.colour_text } else { t.colour_text_2 })))
                .child(rect().width(Size::px(96.)).child(text::line(theme, text::Style::Callout, ink, v.name.clone())))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::px(6.))
                        .corner_radius(3.)
                        .background(colour(t.colour_fill_strong))
                        .child(rect().width(Size::percent(v.used.clamp(0.0, 1.0) * 100.)).height(Size::fill()).corner_radius(3.).background(ink)),
                )
                .child(
                    text::line(theme, text::Style::Subheadline, colour(if v.nearly_full { t.colour_text } else { t.colour_text_2 }), v.free.clone())
                        .font_weight(if v.nearly_full { 600 } else { 400 }),
                )
                .into_element()
        }))
        .maybe_child(more_line(theme, w, more, actions))
        .into_element()
}

fn desktops(theme: &Theme, w: &WidgetView, rows: &[DesktopRow], actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (iw, ih) = inner(theme, w);
    let (shown, more) = list_rows(rows, rows_in(ih, 20., t.space_s));
    let icon_room = iw - 72.0 - 3.0 * t.space_s - 24.0;
    let per_row = ((icon_room + t.space_xs) / (t.size_app_icon_row + t.space_xs)).floor().max(1.0) as usize;
    rect()
        .expanded()
        .spacing(t.space_s)
        .children(shown.iter().enumerate().map(|(i, d)| {
            let (icons, extra) = if d.icons.len() > per_row {
                (&d.icons[..per_row], d.icons.len() - per_row + d.more)
            } else {
                (&d.icons[..], d.more)
            };
            rect()
                .height(Size::px(if d.current { 24. } else { 20. }))
                .width(Size::fill())
                .corner_radius(t.radius_row)
                .padding((0., t.space_xs))
                .maybe(d.current, |el| el.background(colour(t.colour_fill_strong)))
                .horizontal()
            .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .on_press(row_press(actions, &w.id, &i.to_string()))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(d.name.clone())
                .child(
                    rect().width(Size::px(72.)).child(
                        text::line(theme, text::Style::Subheadline, colour(if d.current { t.colour_text } else { t.colour_text_2 }), d.name.clone())
                            .font_weight(if d.current { 600 } else { 400 }),
                    ),
                )
                .children(icons.iter().map(|icon| match icon {
                    Some(path) => isle_ui::AppIcon::new(path.clone(), t.size_app_icon_row).into_element(),
                    None => rect().width(Size::px(t.size_app_icon_row)).height(Size::px(t.size_app_icon_row)).corner_radius(4.).background(colour(t.colour_fill_strong)).into_element(),
                }))
                .maybe_child((extra > 0).then(|| text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), format!("+{extra}")).into_element()))
                .into_element()
        }))
        .maybe_child(more_line(theme, w, more, actions))
        .into_element()
}

fn notifications(theme: &Theme, w: &WidgetView, rows: &[NoteRow], actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (_, ih) = inner(theme, w);
    let (shown, more) = list_rows(rows, rows_in(ih, t.size_row, t.space_xs));
    rect()
        .expanded()
        .spacing(t.space_xs)
        .children(shown.iter().map(|n| {
            rect()
                .height(Size::px(t.size_row))
                .width(Size::fill())
                .horizontal()
            .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .on_press(row_press(actions, &w.id, &n.app))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(format!("{}, {}", n.name, n.count))
                .child(match &n.icon {
                    Some(path) => isle_ui::AppIcon::new(path.clone(), 20.).into_element(),
                    None => Glyph::new("bell", 16., colour(t.colour_text_2)).into_element(),
                })
                .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Body, colour(t.colour_text), n.name.clone())))
                .child(
                    rect()
                        .padding((0., 6.))
                        .height(Size::px(18.))
                        .corner_radius(9.)
                        .background(colour(t.colour_fill_strong))
                        .center()
                        .child(text::line(theme, text::Style::Footnote, colour(t.colour_text), n.count.to_string())),
                )
                .into_element()
        }))
        .maybe_child(more_line(theme, w, more, actions))
        .into_element()
}

fn series_colour(theme: &Theme, i: usize) -> isle_tokens::Colour {
    let t = &theme.tokens;
    [t.series_a, t.series_b, t.series_c, t.series_d][i % 4]
}

fn css(c: isle_tokens::Colour, alpha: f32) -> String {
    format!("rgba({},{},{},{})", c.r, c.g, c.b, c.a * alpha)
}

fn svg_view(svg: String, width: f32, height: f32, alt: String) -> Element {
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        svg.hash(&mut h);
        h.finish()
    };
    SvgViewer::new((key, Bytes::from(svg.into_bytes())))
        .show_loader(false)
        .width(Size::px(width))
        .height(Size::px(height))
        .a11y_alt(alt)
        .into_element()
}

fn system(theme: &Theme, w: &WidgetView, series: &[SeriesView]) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let number = |s: &SeriesView| {
        rect()
            .a11y_alt(format!("{} {}", s.label, s.value))
            .child(text::line(theme, text::Style::Subheadline, ink_2, s.label.clone()))
            .child(text::line(theme, text::Style::Title3, ink, s.value.clone()))
            .into_element()
    };
    let (iw, ih) = inner(theme, w);
    match (w.at.w, w.at.h) {
        (2, 1) => series.first().map_or_else(|| rect().into_element(), |s| {
            rect()
                .expanded()
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Headline, ink, s.label.clone())))
                .child(text::line(theme, text::Style::Title, ink, s.value.clone()))
                .into_element()
        }),
        (2, 2) => rect()
            .expanded()
            .spacing(t.space_s)
            .children(series.chunks(2).map(|pair| {
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .horizontal()
                    .content(Content::flex())
                    .children(pair.iter().map(|s| rect().width(Size::flex(1.)).child(number(s)).into_element()))
                    .into_element()
            }))
            .content(Content::flex())
            .into_element(),
        (4, 1) => rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .children(series.iter().map(|s| rect().width(Size::flex(1.)).child(number(s)).into_element()))
            .into_element(),
        (4, 2) => rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .children(series.iter().enumerate().map(|(i, s)| {
                let c = series_colour(theme, i);
                let p = s.fraction.unwrap_or(0.0).clamp(0.0, 1.0);
                // A circle of radius r whose stroke runs p of the way round from twelve.
                let r = 22.0;
                let around = std::f32::consts::TAU * r;
                let svg = format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="{r}" fill="none" stroke="{}" stroke-width="4"/><circle cx="24" cy="24" r="{r}" fill="none" stroke="{}" stroke-width="4" stroke-linecap="round" stroke-dasharray="{:.2} {:.2}" transform="rotate(-90 24 24)"/></svg>"#,
                    css(t.colour_fill_strong, 1.0),
                    css(c, 1.0),
                    around * p,
                    around,
                );
                rect()
                    .width(Size::flex(1.))
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .child(
                        rect()
                            .width(Size::px(48.))
                            .height(Size::px(48.))
                            .child(svg_view(svg, 48., 48., format!("{} {}", s.label, s.value)))
                            .child(
                                rect()
                                    .position(Position::new_absolute().left(0.).top(0.))
                                    .width(Size::px(48.))
                                    .height(Size::px(48.))
                                    .center()
                                    .child(text::line(theme, text::Style::Callout, ink, s.value.clone())),
                            ),
                    )
                    .child(text::line(theme, text::Style::Subheadline, ink_2, s.label.clone()))
                    .into_element()
            }))
            .into_element(),
        _ => {
            let gap = t.space_m;
            let gw = (iw - gap * (series.len().max(1) as f32 - 1.0)) / series.len().max(1) as f32;
            let gh = ih - t.type_style_subheadline_line - t.type_style_title_3_line - t.space_xs;
            rect()
                .expanded()
                .horizontal()
                .spacing(gap)
                .children(series.iter().enumerate().map(|(i, s)| {
                    let c = series_colour(theme, i);
                    let n = s.history.len().max(2);
                    let points: Vec<(f32, f32)> = s
                        .history
                        .iter()
                        .enumerate()
                        .map(|(j, v)| (j as f32 / (n - 1) as f32 * gw, gh - v.clamp(0.0, 1.0) * (gh - 2.0) - 1.0))
                        .collect();
                    let line: String = points.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ");
                    let area = match (points.first(), points.last()) {
                        (Some(first), Some(last)) => format!("{line} {:.1},{gh:.1} {:.1},{gh:.1}", last.0, first.0),
                        _ => String::new(),
                    };
                    let svg = format!(
                        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {gw:.1} {gh:.1}"><polygon points="{area}" fill="{}"/><polyline points="{line}" fill="none" stroke="{}" stroke-width="1.5" stroke-linejoin="round"/></svg>"#,
                        css(c, t.series_area_opacity),
                        css(c, 1.0),
                    );
                    rect()
                        .width(Size::px(gw))
                        .spacing(t.space_xs)
                        .child(number(s))
                        .child(svg_view(svg, gw, gh, format!("{} {}", s.label, s.value)))
                        .into_element()
                }))
                .into_element()
        }
    }
}

fn weather(theme: &Theme, w: &WidgetView, v: &WeatherView) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let alt = format!("{}, {}, {}", v.place, v.now, v.condition);
    if w.at.h == 1 {
        return rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .a11y_alt(alt)
            .child(Glyph::new(v.glyph.clone(), 24., ink))
            .child(text::line(theme, text::Style::Title, ink, v.now.clone()))
            .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Subheadline, ink_2, v.place.clone())))
            .into_element();
    }
    let now = rect()
        .width(Size::px(if w.at.w >= 4 { 120. } else { 136. }))
        .height(if w.at.h >= 4 { Size::px(136.) } else { Size::fill() })
        .main_align(Alignment::SpaceBetween)
        .child(text::line(theme, text::Style::Subheadline, ink_2, v.place.clone()).font_weight(600))
        .child(text::line(theme, text::Style::Figure, ink, v.now.clone()))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .child(Glyph::new(v.glyph.clone(), 16., ink).alt(v.condition.clone()))
                .child(text::line(theme, text::Style::Callout, ink, v.condition.clone())),
        )
        .child(text::line(theme, text::Style::Subheadline, ink_2, v.high_low.clone()))
        .a11y_alt(alt);
    if w.at.w < 4 {
        return now.into_element();
    }
    let hours = rect()
        .width(Size::flex(1.))
        .height(Size::px(136.))
        .horizontal()
        .content(Content::flex())
        .children(v.hours.iter().map(|(h, g, temp)| {
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .cross_align(Alignment::Center)
                .main_align(Alignment::SpaceEvenly)
                .child(text::line(theme, text::Style::Subheadline, ink_2, h.clone()))
                .child(Glyph::new(g.clone(), 20., ink))
                .child(text::line(theme, text::Style::Callout, ink, temp.clone()))
                .into_element()
        }));
    let top = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_m)
        .child(now)
        .child(hours);
    if w.at.h < 4 {
        return rect().expanded().child(top).into_element();
    }
    rect()
        .expanded()
        .spacing(t.space_s)
        .child(top)
        .children(v.days.iter().map(|(d, g, lo, hi, from, to)| {
            rect()
                .height(Size::px(t.size_row))
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(rect().width(Size::px(48.)).child(text::line(theme, text::Style::Body, ink, d.clone())))
                .child(Glyph::new(g.clone(), 16., ink))
                .child(rect().width(Size::px(36.)).child(text::line(theme, text::Style::Callout, ink_2, lo.clone())))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::px(4.))
                        .corner_radius(2.)
                        .background(colour(t.colour_fill_strong))
                        .child(
                            rect()
                                .position(Position::new_absolute().left(0.).top(0.))
                                .width(Size::fill())
                                .height(Size::fill())
                                .padding((0., 0., 0., 0.))
                                .horizontal()
                                .content(Content::flex())
                                .child(rect().width(Size::flex(from.max(0.0001))))
                                .child(rect().width(Size::flex((to - from).max(0.05))).height(Size::fill()).corner_radius(2.).background(ink))
                                .child(rect().width(Size::flex((1.0 - to).max(0.0001)))),
                        ),
                )
                .child(rect().width(Size::px(36.)).child(text::line(theme, text::Style::Callout, ink, hi.clone())))
                .into_element()
        }))
        .into_element()
}

fn timer(theme: &Theme, w: &WidgetView, v: &TimerView, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let icon = |glyph: &'static str, row: &str, alt: &'static str| {
        rect()
            .width(Size::px(30.))
            .height(Size::px(30.))
            .center()
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(alt)
            .on_press(row_press(actions, &w.id, row))
            .child(Glyph::new(glyph, 20., ink).fill(glyph != "arrow-counter-clockwise"))
            .into_element()
    };
    let start_pause = match v.state {
        TimerState::Running => icon("pause", "pause", "Pause"),
        _ => icon("play", "start", "Start"),
    };
    if w.at.h == 1 {
        return rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(Glyph::new("timer", 16., ink_2))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(theme, text::Style::Subheadline, ink_2, v.label.clone()))
                    .child(text::line(theme, text::Style::Title, ink, v.time.clone())),
            )
            .child(start_pause)
            .into_element();
    }
    if v.state == TimerState::Idle {
        let (actions_p, id) = (actions.clone(), w.id.clone());
        return rect()
            .expanded()
            .main_align(Alignment::SpaceBetween)
            .child(text::line(theme, text::Style::Subheadline, ink_2, v.label.clone()))
            .child(text::line(theme, text::Style::Figure, ink, v.time.clone()))
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_xs)
                    .children(v.presets.iter().map(|&m| {
                        let (actions, id) = (actions_p.clone(), id.clone());
                        Button::new(format!("{m}"))
                            .small()
                            .alt(format!("{m} minutes"))
                            .on_press(move |_| {
                                send(&actions, DashboardAction::Row { id: id.clone(), row: format!("preset:{m}") })
                            })
                            .into_element()
                    })),
            )
            .child({
                let (actions, id) = (actions.clone(), w.id.clone());
                Button::new("Start").small().on_press(move |_| {
                    send(&actions, DashboardAction::Row { id: id.clone(), row: "start".into() })
                })
            })
            .into_element();
    }
    let (_, ih) = inner(theme, w);
    let size = (ih - 40.0).max(40.0);
    let arc = if v.state == TimerState::Running { t.colour_accent } else { t.colour_text_2 };
    let r = 48.0;
    let around = std::f32::consts::TAU * r;
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="{r}" fill="none" stroke="{}" stroke-width="{sw:.2}"/><circle cx="50" cy="50" r="{r}" fill="none" stroke="{}" stroke-width="{sw:.2}" stroke-linecap="round" stroke-dasharray="{:.2} {:.2}" transform="rotate(-90 50 50)"/></svg>"#,
        css(t.colour_fill_strong, 1.0),
        css(arc, 1.0),
        around * v.left.clamp(0.0, 1.0),
        around,
        sw = 4.0 * 100.0 / size,
    );
    rect()
        .expanded()
        .cross_align(Alignment::Center)
        .child(
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .child(svg_view(svg, size, size, format!("{} left, {}", v.time, v.label)))
                .child(
                    rect()
                        .position(Position::new_absolute().left(0.).top(0.))
                        .width(Size::px(size))
                        .height(Size::px(size))
                        .center()
                        .child(text::line(theme, text::Style::Title3, ink, v.time.clone())),
                ),
        )
        .child(
            rect()
                .horizontal()
                .spacing(t.space_s)
                .child(start_pause)
                .child(icon("arrow-counter-clockwise", "reset", "Reset")),
        )
        .into_element()
}

fn provider(theme: &Theme, w: &WidgetView, v: &ProviderView) -> Element {
    let t = &theme.tokens;
    let (ink, ink_2) = (colour(t.colour_text), colour(t.colour_text_2));
    let (iw, ih) = inner(theme, w);
    let title = (v.title.is_some() || v.updated.is_some()).then(|| {
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .child(rect().width(Size::flex(1.)).maybe_child(v.title.clone().map(|n| {
                text::line(theme, text::Style::Subheadline, ink_2, n).font_weight(600).into_element()
            })))
            .maybe_child(v.updated.clone().map(|u| text::line(theme, text::Style::Footnote, ink_2, u).into_element()))
            .into_element()
    });
    let room = ih - if title.is_some() { t.type_style_subheadline_line + t.space_xs } else { 0.0 };
    let body: Element = match &v.data {
        ProviderData::Text(text_) => label()
            .width(Size::fill())
            .height(Size::px(room))
            .font_size(t.type_style_body_size)
            .color(ink)
            .text_overflow(TextOverflow::Ellipsis)
            .text(text_.clone())
            .into_element(),
        ProviderData::Number { value, unit, label } => rect()
            .child(text::line(
                theme,
                if w.at.h >= 2 { text::Style::Figure } else { text::Style::Title },
                ink,
                value.clone(),
            ))
            .maybe_child(
                (unit.is_some() || label.is_some()).then(|| {
                    let words = [unit.clone(), label.clone()].into_iter().flatten().collect::<Vec<_>>().join(" ");
                    text::line(theme, text::Style::Subheadline, ink_2, words).into_element()
                }),
            )
            .into_element(),
        ProviderData::Gauge { value, label, text: words } => {
            let r = 22.0;
            let around = std::f32::consts::TAU * r;
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="{r}" fill="none" stroke="{}" stroke-width="4"/><circle cx="24" cy="24" r="{r}" fill="none" stroke="{}" stroke-width="4" stroke-linecap="round" stroke-dasharray="{:.2} {:.2}" transform="rotate(-90 24 24)"/></svg>"#,
                css(t.colour_fill_strong, 1.0),
                css(t.colour_text, 1.0),
                around * value.clamp(0.0, 1.0),
                around,
            );
            let shown = words.clone().unwrap_or_else(|| format!("{:.0}%", value * 100.0));
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_m)
                .child(svg_view(svg, 48., 48., format!("{} {shown}", label.clone().unwrap_or_default())))
                .child(
                    rect()
                        .child(text::line(theme, text::Style::Title3, ink, shown))
                        .maybe_child(label.clone().map(|l| text::line(theme, text::Style::Subheadline, ink_2, l).into_element())),
                )
                .into_element()
        }
        ProviderData::Sparkline { values, value } => {
            let gh = (room - if value.is_some() { t.type_style_title_3_line } else { 0.0 }).max(8.0);
            rect()
                .maybe_child(value.clone().map(|v| text::line(theme, text::Style::Title3, ink, v).into_element()))
                .child(line_graph(values, iw, gh, t.colour_text, None))
                .into_element()
        }
        ProviderData::Graph(series) => {
            let gh = room.max(8.0);
            let mut layer = rect().width(Size::px(iw)).height(Size::px(gh));
            for (i, (label, values)) in series.iter().enumerate() {
                layer = layer.child(
                    rect()
                        .position(Position::new_absolute().left(0.).top(0.))
                        .child(line_graph(values, iw, gh, series_colour(theme, i), Some(label))),
                );
            }
            layer.into_element()
        }
        ProviderData::List(items) => {
            let (shown, more) = list_rows(items, rows_in(room, t.size_row, t.space_xs));
            rect()
                .spacing(t.space_xs)
                .children(shown.iter().map(|(label, value, glyph)| {
                    rect()
                        .height(Size::px(t.size_row))
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .maybe_child(glyph.clone().map(|g| Glyph::new(g, 16., ink_2).into_element()))
                        .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Body, ink, label.clone())))
                        .maybe_child(value.clone().map(|v| text::line(theme, text::Style::Body, ink_2, v).into_element()))
                        .into_element()
                }))
                .maybe_child((more > 0).then(|| {
                    text::line(theme, text::Style::Callout, ink_2, format!("{more} more")).into_element()
                }))
                .into_element()
        }
        ProviderData::Card { title, body, image } => rect()
            .horizontal()
            .content(Content::flex())
            .spacing(t.space_m)
            .maybe_child(image.clone().map(|path| {
                rect()
                    .width(Size::px(room.min(96.)))
                    .height(Size::px(room.min(96.)))
                    .corner_radius(t.radius_control)
                    .overflow(Overflow::Clip)
                    .child(ImageViewer::new(path).width(Size::px(room.min(96.))).height(Size::px(room.min(96.))))
                    .into_element()
            }))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(theme, text::Style::Headline, ink, title.clone()))
                    .maybe_child(body.clone().map(|b| {
                        label()
                            .width(Size::fill())
                            .font_size(t.type_style_callout_size)
                            .color(ink_2)
                            .text_overflow(TextOverflow::Ellipsis)
                            .max_lines(((room - t.type_style_headline_line) / t.type_style_callout_line).floor().max(1.0) as usize)
                            .text(b)
                            .into_element()
                    })),
            )
            .into_element(),
    };
    rect().expanded().spacing(t.space_xs).maybe_child(title).child(body).into_element()
}

/// A 1.5 line over its area, scaled to its own range.
fn line_graph(values: &[f64], width: f32, height: f32, c: isle_tokens::Colour, label: Option<&String>) -> Element {
    let (lo, hi) = values.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let span = (hi - lo).max(f64::EPSILON);
    let n = values.len().max(2);
    let points: Vec<(f32, f32)> = values
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f32 / (n - 1) as f32 * width, height - (((v - lo) / span) as f32) * (height - 2.0) - 1.0))
        .collect();
    let line: String = points.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ");
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.1} {height:.1}"><polyline points="{line}" fill="none" stroke="{}" stroke-width="1.5" stroke-linejoin="round"/></svg>"#,
        css(c, 1.0),
    );
    svg_view(svg, width, height, label.cloned().unwrap_or_default())
}
