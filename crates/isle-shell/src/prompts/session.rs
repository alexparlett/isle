//! The session dialog: the chooser and the confirmation, then the apps quitting
//! (`surfaces/prompts.md`, The session dialog).

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    AppIcon, Button, Glass, Glyph, Hit, Spinner, button::Kind, colour, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// The chooser's actions, in its row's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Lock,
    LogOut,
    Sleep,
    Restart,
    ShutDown,
}

impl Choice {
    pub const ROW: [Choice; 5] = [
        Choice::Lock,
        Choice::LogOut,
        Choice::Sleep,
        Choice::Restart,
        Choice::ShutDown,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Choice::Lock => "Lock Screen",
            Choice::LogOut => "Log Out",
            Choice::Sleep => "Sleep",
            Choice::Restart => "Restart",
            Choice::ShutDown => "Shut Down",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Choice::Lock => "lock",
            Choice::LogOut => "sign-out",
            Choice::Sleep => "moon-stars",
            Choice::Restart => "arrow-clockwise",
            Choice::ShutDown => "power",
        }
    }

    /// By the chooser's letters: macOS's S and R, and L and O.
    pub fn by_letter(letter: &str) -> Option<Choice> {
        match letter {
            "l" => Some(Choice::Lock),
            "o" => Some(Choice::LogOut),
            "s" => Some(Choice::Sleep),
            "r" => Some(Choice::Restart),
            _ => None,
        }
    }

    fn verb(self) -> &'static str {
        match self {
            Choice::Lock => "lock",
            Choice::LogOut => "log out",
            Choice::Sleep => "sleep",
            Choice::Restart => "restart",
            Choice::ShutDown => "shut down",
        }
    }

    pub fn question(self) -> String {
        format!("Are you sure you want to {}?", self.verb())
    }

    pub fn countdown(self, seconds: u32) -> String {
        match self {
            Choice::LogOut => {
                format!("If you do nothing, you will be logged out in {seconds} seconds.")
            }
            other => format!(
                "If you do nothing, the computer will {} in {seconds} seconds.",
                other.verb()
            ),
        }
    }

    pub fn stopped_by(self, app: &str) -> String {
        format!("Couldn't {} because {app} didn't quit.", self.verb())
    }
}

/// Which form the dialog has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// The row of five with this one selected.
    Chooser(usize),
    /// One action's question.
    Confirm(Choice),
}

impl Default for Form {
    fn default() -> Self {
        Form::Chooser(4)
    }
}

/// An open app: its id, name, icon and whether it missed its ping.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenApp {
    pub app: String,
    pub name: String,
    pub icon: Option<PathBuf>,
    pub not_responding: bool,
}

/// What stopped the session ending.
#[derive(Clone, Debug, PartialEq)]
pub enum Stop {
    /// The app is still open, likely asking to save.
    Open(OpenApp),
    NotResponding(OpenApp),
    /// logind refused, or a program's block inhibitor holds it.
    Refused(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub form: Form,
    pub apps: Vec<OpenApp>,
    pub countdown: Option<u32>,
    /// The apps are being asked to quit for this action.
    pub quitting: Option<Choice>,
    pub stop: Option<Stop>,
    pub armed: bool,
    pub solid: bool,
}

impl Frame {
    /// The action the dialog is about: the one chosen, the confirmation's, else the selection.
    pub fn choice(&self) -> Choice {
        match (self.quitting, self.form) {
            (Some(choice), _) | (None, Form::Confirm(choice)) => choice,
            (None, Form::Chooser(i)) => Choice::ROW[i.min(4)],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionAction {
    /// A click on an action, or its letter.
    Choose(Choice),
    /// Left or Right in the chooser.
    Step(i32),
    /// Return: the selection in the chooser, the action in the confirmation.
    Default,
    Cancel,
    /// Any key or click: the countdown stops.
    Hold,
    ShowApp(String),
    ForceQuit(String),
}

#[derive(Default)]
pub struct SessionSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
}

impl SessionSurface {
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

    /// The chooser's inner width: five columns, then Cancel.
    fn chooser_width(theme: &Theme) -> f32 {
        let t = &theme.tokens;
        5.0 * t.size_module_cell_width + 4.0 * t.space_s + t.space_l + t.size_button_minimum
    }

    /// The surface's box: the wider form, and room for it to grow down.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let width = Self::chooser_width(theme) + 2.0 * t.space_xl;
        (
            (width + 2.0 * m).ceil() as i32,
            (480.0 * t.type_scale_max.max(1.0) + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for SessionSurface {
    fn name(&self) -> &'static str {
        "session"
    }

    fn layer(&self) -> Layer {
        Layer::Prompt
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
        AppComponent::new(SessionApp {
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
struct SessionApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for SessionApp {
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
        SessionView { frame: state }
    }
}

#[derive(PartialEq)]
struct SessionView {
    frame: State<Frame>,
}

fn send(actions: &Actions, action: SessionAction) {
    actions.send(Action::Session(action));
}

impl Component for SessionView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);
        let mut content_height = use_state(|| 0.0f32);
        let busy = frame.quitting.is_some() && frame.stop.is_none();
        let choice = frame.choice();
        let shown_apps = t.limit_session_apps as usize;

        // The open apps, centred in rows the form's inner width holds.
        let inner = match frame.form {
            Form::Chooser(_) => SessionSurface::chooser_width(&theme),
            Form::Confirm(_) => t.size_panel - 2.0 * t.space_xl,
        };
        let per_row = (((inner + t.space_s) / (t.size_app_icon + t.space_s)) as usize).max(1);
        let icons: Vec<Element> = frame
            .apps
            .iter()
            .take(shown_apps)
            .map(|app| {
                rect()
                    .a11y_alt(app.name.clone())
                    .width(Size::px(t.size_app_icon))
                    .height(Size::px(t.size_app_icon))
                    .center()
                    .child(match &app.icon {
                        Some(icon) => AppIcon::new(icon.clone(), t.size_app_icon).into_element(),
                        None => Glyph::new("app-window", t.size_app_icon, ink_2).into_element(),
                    })
                    .maybe_child(app.not_responding.then(|| {
                        rect()
                            .position(Position::new_absolute().right(-4.).bottom(-4.))
                            .width(Size::px(16.))
                            .height(Size::px(16.))
                            .corner_radius(8.)
                            .background(colour(t.material_solid))
                            .center()
                            .child(Glyph::new("hourglass-medium", 12., ink).fill(true))
                    }))
                    .into_element()
            })
            .chain((frame.apps.len() > shown_apps).then(|| {
                rect()
                    .height(Size::px(t.size_app_icon))
                    .center()
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        ink_2,
                        format!("+{}", frame.apps.len() - shown_apps),
                    ))
                    .into_element()
            }))
            .collect();
        let mut rows: Vec<Vec<Element>> = Vec::new();
        for icon in icons {
            match rows.last_mut() {
                Some(row) if row.len() < per_row => row.push(icon),
                _ => rows.push(vec![icon]),
            }
        }
        let apps = rect()
            .width(Size::fill())
            .spacing(t.space_s)
            .children(rows.into_iter().map(|row| {
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .main_align(Alignment::Center)
                    .spacing(t.space_s)
                    .children(row)
                    .into_element()
            }));

        // The countdown's slot: the count, the spinner while quitting, or what stopped it.
        let (glyph, words) = match &frame.stop {
            Some(Stop::Open(app)) => (Some("warning-circle"), choice.stopped_by(&app.name)),
            Some(Stop::NotResponding(_)) => (Some("hourglass-medium"), "Not responding".into()),
            Some(Stop::Refused(why)) => (Some("warning-circle"), why.clone()),
            None => (
                None,
                frame
                    .countdown
                    .filter(|_| !busy)
                    .map(|n| choice.countdown(n))
                    .unwrap_or_default(),
            ),
        };
        let line = rect()
            .width(Size::fill())
            .height(Size::px(16.))
            .horizontal()
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .maybe_child(busy.then(|| Spinner::new(12., "Quitting apps")))
            .maybe_child(glyph.map(|g| Glyph::new(g, t.size_glyph_small, ink)))
            .child(text::line(
                &theme,
                text::Style::Callout,
                if frame.stop.is_some() { ink } else { ink_2 },
                words,
            ));

        // Cancel, then the action; once stopped, Show {app} or Force Quit {app}.
        let main = match &frame.stop {
            Some(Stop::Open(app)) => Some((
                format!("Show {}", app.name),
                SessionAction::ShowApp(app.app.clone()),
                Kind::Default,
            )),
            Some(Stop::NotResponding(app)) => Some((
                format!("Force Quit {}", app.name),
                SessionAction::ForceQuit(app.app.clone()),
                Kind::Destructive,
            )),
            Some(Stop::Refused(_)) => None,
            None => match frame.form {
                Form::Confirm(choice) => Some((
                    choice.label().to_owned(),
                    SessionAction::Choose(choice),
                    if frame.armed {
                        Kind::Default
                    } else {
                        Kind::Regular
                    },
                )),
                Form::Chooser(_) => None,
            },
        };
        let longest = main
            .iter()
            .map(|(l, _, _)| l.chars().count())
            .chain(["Cancel".len()])
            .max()
            .unwrap_or(0) as f32;
        let button_width = (longest * t.type_style_body_size * 0.6 + 24.0)
            .max(t.size_button_minimum)
            .ceil();
        // The chooser's Cancel sits beside the circles at the minimum width.
        let cancel_width = match frame.form {
            Form::Chooser(_) => t.size_button_minimum,
            Form::Confirm(_) => button_width,
        };
        let cancel_button = {
            let actions = actions.clone();
            rect().width(Size::px(cancel_width)).child(
                Button::new("Cancel")
                    .wide()
                    .enabled(!busy)
                    .on_press(move |_| send(&actions, SessionAction::Cancel)),
            )
        };
        let main_button = main.map(|(label, action, kind)| {
            let actions = actions.clone();
            rect().width(Size::px(button_width)).child(
                Button::new(label)
                    .kind(kind)
                    .wide()
                    .enabled(!busy)
                    .on_press(move |_| send(&actions, action.clone())),
            )
        });

        let body = match frame.form {
            Form::Confirm(choice) => rect()
                .width(Size::px(t.size_panel - 2.0 * t.space_xl))
                .cross_align(Alignment::Center)
                .child(Glyph::new(choice.glyph(), t.size_app_icon_medium, ink))
                .child(rect().height(Size::px(t.space_m)))
                .child(text::line(
                    &theme,
                    text::Style::Title3,
                    ink,
                    choice.question(),
                ))
                .child(rect().height(Size::px(t.space_l)))
                .child(apps)
                .child(rect().height(Size::px(t.space_l)))
                .child(line)
                .child(rect().height(Size::px(t.space_l)))
                .child(
                    rect()
                        .width(Size::fill())
                        .content(Content::flex())
                        .horizontal()
                        .spacing(t.space_s)
                        .child(rect().width(Size::flex(1.)))
                        .child(cancel_button)
                        .maybe_child(main_button),
                ),
            Form::Chooser(selected) => {
                let circles = rect().horizontal().spacing(t.space_s).children(
                    Choice::ROW.iter().enumerate().map(|(i, &choice)| {
                        Circle {
                            choice,
                            selected: i == selected,
                        }
                        .into_element()
                    }),
                );
                rect()
                    .width(Size::px(SessionSurface::chooser_width(&theme)))
                    .cross_align(Alignment::Center)
                    .child(
                        rect().horizontal().spacing(t.space_l).child(circles).child(
                            rect()
                                .height(Size::px(48.))
                                .main_align(Alignment::Center)
                                .child(cancel_button),
                        ),
                    )
                    .child(rect().height(Size::px(t.space_l)))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::px(t.stroke_hairline))
                            .background(colour(t.colour_hairline)),
                    )
                    .child(rect().height(Size::px(t.space_l)))
                    .child(apps)
                    .child(rect().height(Size::px(t.space_l)))
                    .child(line)
                    .maybe_child(main_button.map(|button| {
                        rect()
                            .width(Size::fill())
                            .padding((t.space_l, 0., 0., 0.))
                            .horizontal()
                            .main_align(Alignment::End)
                            .child(button)
                    }))
            }
        };

        let keys = actions.clone();
        let clicks = actions.clone();
        let chooser = matches!(frame.form, Form::Chooser(_));
        let solid = frame.solid || theme.appearance.reduce_transparency;
        rect()
            .expanded()
            .padding(SessionSurface::margin(&theme))
            .cross_align(Alignment::Center)
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                send(&keys, SessionAction::Hold);
                let action = match &e.key {
                    Key::Named(NamedKey::Escape) => Some(SessionAction::Cancel),
                    Key::Named(NamedKey::Enter) => Some(SessionAction::Default),
                    Key::Named(NamedKey::ArrowLeft) if chooser => Some(SessionAction::Step(-1)),
                    Key::Named(NamedKey::ArrowRight) if chooser => Some(SessionAction::Step(1)),
                    Key::Character(c) if chooser => {
                        Choice::by_letter(&c.to_lowercase()).map(SessionAction::Choose)
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    send(&keys, action);
                }
            })
            .child(
                rect()
                    .width(Size::px(inner + 2.0 * t.space_xl))
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
                        el.height(Size::px(*content_height.read() + 2.0 * t.space_xl))
                    })
                    .maybe(!solid && *content_height.read() > 0.0, |el| {
                        el.child(Glass::new(t.radius_panel))
                    })
                    .on_press(move |_| send(&clicks, SessionAction::Hold))
                    .child(Hit)
                    .child(rect().padding(t.space_xl).child(body.on_sized(
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

/// One of the chooser's actions: its circle, the accent and the filled glyph when selected,
/// and its name under it.
#[derive(PartialEq)]
struct Circle {
    choice: Choice,
    selected: bool,
}

impl Component for Circle {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let mut hover = use_state(|| false);
        let fill = if self.selected {
            colour(t.colour_accent)
        } else if *hover.read() {
            colour(t.colour_control_pressed)
        } else {
            colour(t.colour_fill_strong)
        };
        let glyph_ink = if self.selected {
            colour(t.colour_on_accent)
        } else {
            colour(t.colour_text)
        };
        let choice = self.choice;
        rect()
            .width(Size::px(t.size_module_cell_width))
            .cross_align(Alignment::Center)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(choice.label())
            .on_pointer_enter(move |_| hover.set_if_modified(true))
            .on_pointer_leave(move |_| hover.set_if_modified(false))
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&actions, SessionAction::Choose(choice));
            })
            .child(
                rect()
                    .width(Size::px(48.))
                    .height(Size::px(48.))
                    .corner_radius(24.)
                    .background(fill)
                    .center()
                    .child(Glyph::new(choice.glyph(), 24., glyph_ink).fill(self.selected)),
            )
            .child(rect().height(Size::px(t.space_s)))
            .child(
                label()
                    .width(Size::fill())
                    .font_size(t.type_style_callout_size)
                    .color(colour(t.colour_text))
                    .text_align(TextAlign::Center)
                    .max_lines(2)
                    .text(choice.label()),
            )
    }
}
