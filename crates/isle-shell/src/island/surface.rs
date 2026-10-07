//! The island's surface on each output: one shape whose size the compositor springs, and
//! the content of its forms, clipped to it (`p1/surfaces/island.md`).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{Glass, Glyph, Hit, LevelBar, LevelGlyph, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::model::{Activity, ActivityKind, Event as IslandEvent, Form, GlyphKind, Privacy, Shown};
use crate::{Action, Actions, Here, Layer, Scope, SurfaceDef};

/// What the island asks of the compositor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IslandAction {
    /// A click on the pill: Control Centre, pinned.
    Pin,
    /// A click on the clock.
    Clock,
    /// A click on a glyph: its detail in Control Centre.
    Glyph(GlyphKind),
    /// A click on an event: an urgent one acknowledged, attention's app raised.
    Event,
    /// From Control Centre.
    Panel(super::panel::PanelAction),
    /// Escape: back from a page, or Control Centre closed.
    Close,
}

/// One frame of an output's island: what it shows, what it is leaving, how far between
/// them, and the shape's size now.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub shown: Shown,
    /// The form being left while the shape springs, faded out by the first half.
    pub before: Option<Shown>,
    /// The share of the size change done, 0 to 1.
    pub p: f32,
    pub width: f32,
    pub height: f32,
    pub clock: String,
    pub clock_width: f32,
    /// Game Mode or Reduce Transparency: no glass.
    pub solid: bool,
    /// Control Centre, open under the pill's row.
    pub panel: Option<super::panel::PanelFrame>,
    /// Whether the panel was open before this morph, so its body fades with the growth.
    pub panel_before: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            shown: Shown::Nothing,
            before: None,
            p: 1.0,
            width: 0.0,
            height: 0.0,
            clock: String::new(),
            clock_width: 0.0,
            solid: false,
            panel: None,
            panel_before: false,
        }
    }
}

/// The width a clock in `format` takes, its widest string measured once.
pub fn clock_width(theme: &Theme, widest: &str) -> f32 {
    let size = text::metrics(theme, text::Style::Clock).0;
    widest
        .chars()
        .map(|c| match c {
            '0'..='9' => size * 0.62,
            ':' | ' ' => size * 0.3,
            _ => size * 0.62,
        })
        .sum::<f32>()
        .ceil()
}

fn estimate(theme: &Theme, style: text::Style, s: &str) -> f32 {
    s.chars().count() as f32 * text::metrics(theme, style).0 * 0.56
}

fn well_width(theme: &Theme, marks: usize) -> f32 {
    let t = &theme.tokens;
    if marks == 0 {
        return 0.0;
    }
    t.size_island_well + (marks as f32 - 1.0) * (t.size_glyph_small + t.space_xs)
}

/// The shape's size for `shown`: the size the compositor springs toward.
pub fn target_size(
    shown: &Shown,
    panel: Option<&super::panel::PanelFrame>,
    theme: &Theme,
    clock_width: f32,
    room: f32,
) -> (f32, f32) {
    let pill = pill_size(shown, theme, clock_width);
    match panel {
        Some(panel) => super::panel::panel_size(
            theme,
            panel,
            pill.1.max(theme.tokens.size_island_height),
            pill.0,
            room,
        ),
        None => pill,
    }
}

fn pill_size(shown: &Shown, theme: &Theme, clock_width: f32) -> (f32, f32) {
    let t = &theme.tokens;
    let pill = t.size_island_height * theme.appearance.text_scale.min(t.type_scale_max).max(1.0);
    let pill = pill.max(t.size_island_height);
    let well = |p: &[Privacy]| {
        if p.is_empty() {
            0.0
        } else {
            t.space_s + well_width(theme, p.len())
        }
    };
    match shown {
        Shown::Nothing => (0.0, 0.0),
        Shown::Privacy(p) => (well_width(theme, p.len()), t.size_island_well),
        Shown::Alone(event, p) | Shown::Event(event, p) => {
            let (w, h) = event_size(event, theme, pill);
            ((w + well(p)).min(t.size_island_max), h)
        }
        Shown::Pill {
            leader,
            marks,
            more,
            glyphs,
            privacy,
        } => {
            let mut w = 2.0 * t.space_m + clock_width;
            if let Some(leader) = leader {
                w += face_width(leader, theme) + t.space_s;
            }
            if !marks.is_empty() {
                w += marks.len() as f32 * (t.size_glyph_small + t.space_xs) + t.space_s;
            } else if *more > 0 {
                w += estimate(theme, text::Style::Subheadline, &format!("+{more}"))
                    + 2.0 * t.space_xs
                    + t.space_s;
            }
            if !glyphs.is_empty() {
                w += t.space_m
                    + glyphs.len() as f32 * 24.0
                    + (glyphs.len() as f32 - 1.0) * t.space_s;
            }
            w += well(privacy);
            (w.min(t.size_island_max).ceil(), pill)
        }
    }
}

fn event_size(event: &IslandEvent, theme: &Theme, pill: f32) -> (f32, f32) {
    let t = &theme.tokens;
    match &event.form {
        Form::Line { text: s, .. } => (
            (2.0 * t.space_m + t.size_glyph + t.space_s + estimate(theme, text::Style::Body, s))
                .ceil(),
            pill,
        ),
        Form::Level { .. } => (t.size_island_osd, t.size_island_event),
        Form::StartRow(_) => (t.size_panel, 56.0 + 2.0 * t.space_s),
    }
}

fn face_width(activity: &Activity, theme: &Theme) -> f32 {
    let t = &theme.tokens;
    let text = estimate(theme, text::Style::Clock, &activity.face);
    let w = match activity.kind {
        ActivityKind::Media => t.size_island_well + t.space_s + t.size_glyph_small,
        ActivityKind::Prompt => 2.0 * t.size_glyph + t.space_xs,
        _ => t.size_glyph + t.space_xs + text + t.space_s,
    };
    w.min(t.size_island_face_max)
}

/// The island's definition: one instance per output, its frame kept by output name so a
/// rebuilt tree comes back as it was.
#[derive(Default)]
pub struct IslandSurface {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl IslandSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts `frame` in force on `output`; call it in that instance's root scope.
    pub fn set(&self, output: &str, frame: Frame) {
        self.frames
            .borrow_mut()
            .insert(output.to_owned(), frame.clone());
        if let Some(mut state) = self.states.borrow().get(output).copied() {
            state.set_if_modified(frame);
        }
    }

    /// Keeps `frame` for `output`'s next tree; safe anywhere.
    pub fn set_model(&self, output: &str, frame: Frame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
    }

    pub fn frame(&self, output: &str) -> Option<Frame> {
        self.frames.borrow().get(output).cloned()
    }

    /// The surface's box: a column at the top centre wide enough for the widest form and
    /// its shadow, the output's full height so a panel can grow down it.
    pub fn box_width(theme: &Theme) -> i32 {
        let t = &theme.tokens;
        let widest = t.size_island_max.max(t.size_panel * t.type_scale_max);
        (widest + 2.0 * SHADOW).ceil() as i32
    }
}

/// Room each side of the shape for its shadow.
const SHADOW: f32 = 48.0;

impl SurfaceDef for IslandSurface {
    fn name(&self) -> &'static str {
        "island"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        let w = 700.min(output.w);
        Rectangle::new(((output.w - w) / 2, 0).into(), (w, output.h).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(IslandApp {
            frames: self.frames.clone(),
            states: self.states.clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }

    fn transitions(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct IslandApp {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl App for IslandApp {
    fn render(&self) -> impl IntoElement {
        let here = use_consume::<Here>().0;
        // A closed surface's tree goes; the compositor must not write to its state after.
        use_drop({
            let (states, here) = (self.states.clone(), here.clone());
            move || {
                states.borrow_mut().remove(&here);
            }
        });
        let state = use_hook(|| {
            let frame = self.frames.borrow().get(&here).cloned().unwrap_or_default();
            let state = State::create(frame);
            self.states.borrow_mut().insert(here.clone(), state);
            state
        });
        Island { frame: state }
    }
}

#[derive(PartialEq)]
struct Island {
    frame: State<Frame>,
}

impl Component for Island {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        if frame.width < 1.0 || frame.height < 1.0 {
            return rect().expanded().into_element();
        }
        let radius = (frame.height / 2.0).min(t.radius_panel);
        let privacy_form = matches!(frame.shown, Shown::Privacy(_));
        let solid = frame.solid || privacy_form;
        // Opening to the panel the pill's fill gives way to the glass by the growth.
        let to_panel = match (frame.panel_before, frame.panel.is_some()) {
            (false, true) => frame.p,
            (true, false) => 1.0 - frame.p,
            (true, true) => 1.0,
            (false, false) => 0.0,
        };
        let fill = if solid {
            colour(t.material_solid)
        } else {
            let c = colour(t.colour_island);
            c.with_a(((1.0 - to_panel) * c.a() as f32).round() as u8)
        };
        let top = if privacy_form {
            t.size_island_top + (t.size_island_height - t.size_island_well) / 2.0
        } else {
            t.size_island_top
        };
        let (outgoing, incoming) = if frame.before.is_some() && frame.panel.is_none() && !frame.panel_before {
            (
                (1.0 - 2.0 * frame.p).max(0.0),
                (2.0 * frame.p - 1.0).max(0.0),
            )
        } else {
            (0.0, 1.0)
        };
        let shape_actions = actions.clone();
        let shape = rect()
            .width(Size::px(frame.width))
            .height(Size::px(frame.height))
            .corner_radius(radius)
            .background(fill)
            .map(Some(shadows(&t.shadow_menu)), |el, list| {
                list.into_iter().fold(el, |el, s| el.shadow(s))
            })
            .overflow(Overflow::Clip)
            // The pill is its own fill over the blur; the glass's tint comes in as it opens.
            .maybe(!solid, |el| el.child(Glass::new(radius).faded(1.0 - to_panel)))
            .child(Hit)
            .on_press(move |_| shape_actions.send(Action::Island(IslandAction::Pin)))
            .map(
                frame.before.clone().filter(|_| outgoing > 0.0),
                |el, before| el.child(layer(content(&theme, &before, &frame, &actions), outgoing)),
            )
            .maybe(incoming > 0.0, |el| {
                let pill = t.size_island_height;
                let opened = frame.panel.is_some() || frame.panel_before;
                let body = if frame.panel_before { 1.0 } else { (2.0 * frame.p - 1.0).max(0.0) };
                el.child(
                    rect()
                        .position(Position::new_absolute())
                        .width(Size::fill())
                        .height(Size::fill())
                        .opacity(incoming)
                        .child(
                            rect()
                                .width(Size::fill())
                                .height(Size::px(if opened { pill } else { frame.height }))
                                .child(content(&theme, &frame.shown, &frame, &actions)),
                        )
                        .map(frame.panel.clone().filter(|_| body > 0.0), |el, panel| {
                            el.child(
                                rect()
                                    .width(Size::fill())
                                    .opacity(body)
                                    .child(super::panel::PanelBody { panel }),
                            )
                        }),
                )
            });
        let pinned = frame.panel.as_ref().is_some_and(|p| p.pinned);
        let escape = actions.clone();
        rect()
            .expanded()
            .maybe(pinned, |el| {
                el.on_global_key_down(move |e: Event<KeyboardEventData>| {
                    if e.key == Key::Named(NamedKey::Escape) {
                        escape.send(Action::Island(IslandAction::Close));
                    }
                })
            })
            .padding((top, 0., 0., 0.))
            .cross_align(Alignment::Center)
            .child(shape)
            .into_element()
    }
}

/// Content over the whole shape at `opacity`.
fn layer(content: Element, opacity: f32) -> Rect {
    rect()
        .position(Position::new_absolute())
        .width(Size::fill())
        .height(Size::fill())
        .opacity(opacity)
        .child(content)
}

fn privacy_symbol(mark: Privacy) -> (&'static str, &'static str) {
    match mark {
        Privacy::Microphone => ("microphone", "Microphone in use"),
        Privacy::Camera => ("video-camera", "Camera in use"),
        Privacy::Screen => ("screencast", "Screen being recorded"),
    }
}

/// The privacy marks in their solid well.
fn well(theme: &Theme, marks: &[Privacy]) -> Element {
    let t = &theme.tokens;
    rect()
        .height(Size::px(t.size_island_well))
        .min_width(Size::px(t.size_island_well))
        .padding((0., (t.size_island_well - t.size_glyph_small) / 2.0))
        .corner_radius(t.size_island_well / 2.0)
        .background(colour(t.material_solid))
        .horizontal()
        .center()
        .spacing(t.space_xs)
        .children(marks.iter().map(|m| {
            let (symbol, alt) = privacy_symbol(*m);
            let ink = match m {
                Privacy::Microphone => t.colour_privacy_microphone,
                Privacy::Camera => t.colour_privacy_camera,
                Privacy::Screen => t.colour_privacy_screen,
            };
            Glyph::new(symbol, t.size_glyph_small, colour(ink))
                .fill(true)
                .alt(alt)
                .into_element()
        }))
        .into_element()
}

fn content(theme: &Theme, shown: &Shown, frame: &Frame, actions: &Actions) -> Element {
    let t = &theme.tokens;
    let ink = colour(t.colour_island_text);
    let row = || {
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
    };
    match shown {
        Shown::Nothing => rect().into_element(),
        Shown::Privacy(marks) => row()
            .main_align(Alignment::Center)
            .child(well(theme, marks))
            .into_element(),
        Shown::Alone(event, privacy) | Shown::Event(event, privacy) => {
            event_content(theme, event, privacy, actions)
        }
        Shown::Pill {
            leader,
            marks,
            more,
            glyphs,
            privacy,
        } => {
            let clock_actions = actions.clone();
            row()
                .padding((0., t.space_m))
                .main_align(Alignment::Center)
                .map(leader.clone(), |el, leader| {
                    el.child(face(theme, &leader))
                        .child(rect().width(Size::px(t.space_s)))
                })
                .maybe(!marks.is_empty(), |el| {
                    el.children(marks.iter().map(|k| {
                        rect()
                            .padding((0., t.space_xs / 2.0))
                            .child(Glyph::new(k.mark(), t.size_glyph_small, ink).fill(true))
                            .into_element()
                    }))
                    .child(rect().width(Size::px(t.space_s)))
                })
                .maybe(*more > 0, |el| {
                    el.child(
                        rect()
                            .height(Size::px(16.))
                            .padding((0., t.space_xs))
                            .corner_radius(8.)
                            .background(colour(t.colour_fill_strong))
                            .center()
                            .a11y_alt(format!("{more} more activities"))
                            .child(
                                text::line(
                                    theme,
                                    text::Style::Subheadline,
                                    ink,
                                    format!("+{more}"),
                                )
                                .font_weight(t.type_weight_semibold as i32),
                            ),
                    )
                    .child(rect().width(Size::px(t.space_s)))
                })
                .child(
                    rect()
                        .width(Size::px(frame.clock_width))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .on_press(move |e: Event<PressEventData>| {
                            e.stop_propagation();
                            clock_actions.send(Action::Island(IslandAction::Clock));
                        })
                        .child(text::line(
                            theme,
                            text::Style::Clock,
                            ink,
                            frame.clock.clone(),
                        )),
                )
                .maybe(!glyphs.is_empty(), |el| {
                    el.child(rect().width(Size::px(t.space_m - (24.0 - t.size_glyph) / 2.0)))
                        .children(glyphs.iter().enumerate().map(|(i, (kind, glyph))| {
                            let actions = actions.clone();
                            let kind = *kind;
                            rect()
                                .width(Size::px(24.))
                                .height(Size::fill())
                                .center()
                                .maybe(i > 0, |el| {
                                    el.margin((0., 0., 0., t.space_s - (24.0 - t.size_glyph)))
                                })
                                .a11y_role(AccessibilityRole::Button)
                                .a11y_alt(glyph.alt.clone())
                                .on_press(move |e: Event<PressEventData>| {
                                    e.stop_propagation();
                                    actions.send(Action::Island(IslandAction::Glyph(kind)));
                                })
                                .child(match &glyph.level {
                                    Some(level) => LevelGlyph {
                                        whole: glyph.symbol.clone(),
                                        level: level.clone(),
                                        size: t.size_glyph,
                                        colour: ink,
                                        dimmed: colour(t.colour_text_disabled),
                                        alt: glyph.alt.clone(),
                                    }
                                    .into_element(),
                                    None => Glyph::new(glyph.symbol.clone(), t.size_glyph, ink)
                                        .fill(glyph.fill)
                                        .into_element(),
                                })
                                .into_element()
                        }))
                })
                .maybe(!privacy.is_empty(), |el| {
                    el.child(rect().width(Size::px(t.space_s)))
                        .child(well(theme, privacy))
                })
                .into_element()
        }
    }
}

fn face(theme: &Theme, activity: &Activity) -> Element {
    let t = &theme.tokens;
    let ink = colour(t.colour_island_text);
    let max = Size::px(t.size_island_face_max);
    match activity.kind {
        ActivityKind::Media => rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .a11y_alt(format!("Now playing: {}", activity.title))
            .child(
                rect()
                    .width(Size::px(t.size_island_well))
                    .height(Size::px(t.size_island_well))
                    .corner_radius(t.size_island_well / 2.0)
                    .background(colour(t.colour_fill_strong))
                    .center()
                    .child(Glyph::new("music-note", t.size_glyph_small, ink)),
            )
            .child(Glyph::new("isle-equaliser", t.size_glyph_small, ink))
            .into_element(),
        ActivityKind::Recording => rect()
            .height(Size::px(t.size_island_well))
            .padding((0., t.space_s))
            .corner_radius(t.size_island_well / 2.0)
            .background(colour(t.material_solid))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .a11y_alt(format!("Recording, {}", activity.face))
            .child(
                rect()
                    .width(Size::px(8.))
                    .height(Size::px(8.))
                    .corner_radius(4.)
                    .background(colour(t.colour_live)),
            )
            .child(text::line(
                theme,
                text::Style::Clock,
                colour(t.colour_text),
                activity.face.clone(),
            ))
            .into_element(),
        kind => rect()
            .max_width(max)
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(Glyph::new(kind.mark(), t.size_glyph, ink).fill(true))
            .child(text::line(
                theme,
                text::Style::Clock,
                ink,
                activity.face.clone(),
            ))
            .into_element(),
    }
}

fn event_content(
    theme: &Theme,
    event: &IslandEvent,
    privacy: &[Privacy],
    actions: &Actions,
) -> Element {
    let t = &theme.tokens;
    let ink = colour(t.colour_island_text);
    let press = actions.clone();
    let body = match &event.form {
        Form::Line {
            symbol,
            fill,
            text: s,
        } => rect()
            .height(Size::fill())
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(Glyph::new(symbol.clone(), t.size_glyph, ink).fill(*fill))
            .child(
                text::line(theme, text::Style::Body, ink, s.clone())
                    .font_weight(t.type_weight_medium as i32),
            )
            .into_element(),
        Form::Level {
            symbol,
            name,
            level,
            muted,
        } => rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding((t.space_m, t.space_l))
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .a11y_alt(if *muted {
                "Muted".to_string()
            } else {
                format!("{name} {}%", (level * 100.0).round())
            })
            .child(Glyph::new(symbol.clone(), t.size_glyph_large, ink))
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(t.space_xs)
                    .child(text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        if *muted {
                            "Muted".to_string()
                        } else {
                            name.clone()
                        },
                    ))
                    .child(LevelBar {
                        level: if *muted { 0.0 } else { *level },
                        track: colour(t.colour_fill_strong),
                        fill: ink,
                    }),
            )
            .into_element(),
        Form::StartRow(activity) => rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(t.space_s + t.space_m)
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .child(Glyph::new(activity.kind.mark(), 32.0, ink).fill(true))
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
            .into_element(),
    };
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .padding((0., t.space_m))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            press.send(Action::Island(IslandAction::Event));
        })
        .child(
            rect()
                .width(Size::flex(1.))
                .height(Size::fill())
                .center()
                .child(body),
        )
        .maybe(!privacy.is_empty(), |el| el.child(well(theme, privacy)))
        .into_element()
}
