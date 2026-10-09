//! The capture toolbar of Shift+Cmd+5: Close, the six modes, Pick colour, Options and the
//! action, on glass at the bottom centre (`p1/surfaces/capture.md`, The toolbar). The
//! compositor reads its keys and draws the mode over the live screen.

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Glass, Glyph, colour, focus_ring, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// What the toolbar captures, and how.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    ShotScreen,
    ShotWindow,
    ShotPortion,
    RecordScreen,
    RecordWindow,
    RecordPortion,
}

impl Mode {
    pub const ALL: [Mode; 6] = [
        Mode::ShotScreen,
        Mode::ShotWindow,
        Mode::ShotPortion,
        Mode::RecordScreen,
        Mode::RecordWindow,
        Mode::RecordPortion,
    ];

    pub fn records(self) -> bool {
        matches!(
            self,
            Mode::RecordScreen | Mode::RecordWindow | Mode::RecordPortion
        )
    }

    pub fn window(self) -> bool {
        matches!(self, Mode::ShotWindow | Mode::RecordWindow)
    }

    pub fn portion(self) -> bool {
        matches!(self, Mode::ShotPortion | Mode::RecordPortion)
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::ShotScreen => "Capture entire screen",
            Mode::ShotWindow => "Capture selected window",
            Mode::ShotPortion => "Capture selected portion",
            Mode::RecordScreen => "Record entire screen",
            Mode::RecordWindow => "Record selected window",
            Mode::RecordPortion => "Record selected portion",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mode::ShotScreen => "shot-screen",
            Mode::ShotWindow => "shot-window",
            Mode::ShotPortion => "shot-portion",
            Mode::RecordScreen => "record-screen",
            Mode::RecordWindow => "record-window",
            Mode::RecordPortion => "record-portion",
        }
    }

    pub fn from_key(key: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == key)
    }

    fn glyph(self) -> &'static str {
        match self {
            Mode::ShotScreen | Mode::RecordScreen => "monitor",
            Mode::ShotWindow | Mode::RecordWindow => "app-window",
            Mode::ShotPortion | Mode::RecordPortion => "selection",
        }
    }
}

/// The control the keyboard is on, by group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Control {
    Close,
    #[default]
    Modes,
    PickColour,
    Options,
    Act,
}

/// What the bar shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bar {
    pub mode: Mode,
    /// The modes that can be chosen; recordings wait for a recorder.
    pub allowed: Vec<Mode>,
    /// A recording runs: the action stops it.
    pub recording: bool,
    /// A portal's request: the asking app's name and icon; the action reads Share.
    pub request: Option<(String, Option<std::path::PathBuf>)>,
    /// Choosing a portion for the share picker: Close and the action only.
    pub chooser: bool,
    pub focus: Option<Control>,
    pub solid: bool,
    pub contrast: bool,
}

impl Bar {
    pub fn action_label(&self) -> Option<&'static str> {
        if self.mode.window() {
            return None;
        }
        if self.chooser {
            return Some("Choose");
        }
        Some(
            match (self.request.is_some(), self.recording, self.mode.records()) {
                (true, ..) => "Share",
                (_, true, true) => "Stop recording",
                (_, _, true) => "Record",
                _ => "Capture",
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    Close,
    Mode(Mode),
    PickColour,
    Options,
    Act,
    /// A press on the bar between its controls: the bar follows the pointer.
    Grab,
}

#[derive(Default)]
pub struct ToolbarSurface {
    bar: Rc<RefCell<Bar>>,
    state: Rc<RefCell<Option<State<Bar>>>>,
}

impl ToolbarSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts `bar` in force; call it in the tree's root scope.
    pub fn set(&self, bar: Bar) {
        *self.bar.borrow_mut() = bar.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(bar);
        }
    }

    /// Keeps `bar` for the next tree; safe anywhere.
    pub fn set_model(&self, bar: Bar) {
        *self.bar.borrow_mut() = bar;
    }

    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The bar's width for `bar`, its controls and gaps summed as the surface sheet lays
    /// them out.
    pub fn width(theme: &Theme, bar: &Bar) -> f32 {
        let t = &theme.tokens;
        let (font, ..) = text::metrics(theme, text::Style::Body);
        let gap = t.space_m * 2.0 + 1.0;
        let close = t.size_control_large;
        let modes = 6.0 * 36.0 + 2.0 * t.space_xxs + 1.0;
        let pick = t.size_control_large;
        let options = "Options".len() as f32 * font * 0.55 + t.size_glyph_small + 2.0 * t.space_s;
        let action = bar.action_label().map_or(0.0, |l| {
            l.len() as f32 * font * 0.55 + 2.0 * t.space_l + gap
        });
        if bar.chooser {
            return (2.0 * t.space_s + close + action).ceil();
        }
        let request = bar.request.as_ref().map_or(0.0, |(app, _)| {
            t.size_app_icon_small + t.space_xs + (app.len() + 18) as f32 * font * 0.5 + gap
        });
        (2.0 * t.space_s + close + request + gap + modes + gap + pick + gap + options + action)
            .ceil()
    }

    /// The surface's box: the bar and the room for its shadow.
    pub fn size_for(theme: &Theme, bar: &Bar) -> SSize<i32, Logical> {
        let m = Self::margin(theme);
        (
            (Self::width(theme, bar) + 2.0 * m).ceil() as i32,
            (theme.tokens.size_capture_bar + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for ToolbarSurface {
    fn name(&self) -> &'static str {
        "capture-toolbar"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(ToolbarApp {
            bar: self.bar.clone(),
            state: self.state.clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct ToolbarApp {
    bar: Rc<RefCell<Bar>>,
    state: Rc<RefCell<Option<State<Bar>>>>,
}

fn send(actions: &Actions, action: ToolbarAction) {
    actions.send(Action::Toolbar(action));
}

impl App for ToolbarApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.bar.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let bar = state.read().clone();
        let margin = ToolbarSurface::margin(&theme);
        let width = ToolbarSurface::width(&theme, &bar);
        let rule = || {
            rect()
                .width(Size::px(1.0))
                .height(Size::px(20.0))
                .margin((0., t.space_m))
                .background(colour(t.colour_hairline))
        };
        let focused = |c: Control| bar.focus == Some(c);
        let icon_button =
            |glyph: &'static str, name: &'static str, c: Control, a: ToolbarAction| {
                let actions = actions.clone();
                rect()
                    .width(Size::px(t.size_control_large))
                    .height(Size::px(t.size_control_large))
                    .corner_radius(t.radius_card)
                    .center()
                    .maybe(focused(c), |el| el.border(focus_ring(&theme)))
                    .a11y_alt(name)
                    .on_press(move |_| send(&actions, a))
                    .child(Glyph::new(glyph, t.size_glyph, colour(t.colour_text_2)))
            };

        let mut modes = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .height(Size::px(t.size_control_large))
            .padding(t.space_xxs)
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill));
        for (i, mode) in Mode::ALL.into_iter().enumerate() {
            if i == 3 {
                modes = modes.child(
                    rect()
                        .width(Size::px(1.0))
                        .height(Size::px(16.0))
                        .background(colour(t.colour_hairline)),
                );
            }
            let chosen = bar.mode == mode;
            let allowed = bar.allowed.contains(&mode);
            let actions = actions.clone();
            let glyph_colour = if chosen {
                t.colour_text
            } else {
                t.colour_text_2
            };
            let mut segment = rect()
                .width(Size::px(36.0))
                .height(Size::fill())
                .corner_radius(t.radius_small)
                .center()
                .a11y_alt(mode.name())
                .opacity(if allowed { 1.0 } else { 0.35 })
                .maybe(chosen, |el| {
                    el.background(colour(t.material_solid))
                        .map(Some(shadows(&t.shadow_control)), |el, list| {
                            list.into_iter().fold(el, |el, s| el.shadow(s))
                        })
                })
                .maybe(chosen && focused(Control::Modes), |el| {
                    el.border(focus_ring(&theme))
                })
                .maybe(allowed, |el| {
                    el.on_press(move |_| send(&actions, ToolbarAction::Mode(mode)))
                })
                .child(Glyph::new(
                    mode.glyph(),
                    t.size_glyph_large,
                    colour(glyph_colour),
                ));
            if mode.records() {
                // The recording's dot at the glyph's foot, trailing.
                segment = segment.child(
                    rect()
                        .position(Position::new_absolute().right(7.0).bottom(5.0))
                        .width(Size::px(6.0))
                        .height(Size::px(6.0))
                        .corner_radius(3.0)
                        .background(colour(glyph_colour)),
                );
            }
            modes = modes.child(segment);
        }

        let options = {
            let actions = actions.clone();
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .height(Size::px(t.size_control_large))
                .padding((0., t.space_s))
                .corner_radius(t.radius_card)
                .maybe(focused(Control::Options), |el| {
                    el.border(focus_ring(&theme))
                })
                .on_press(move |_| send(&actions, ToolbarAction::Options))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    "Options",
                ))
                .child(rect().width(Size::px(t.space_xxs)))
                .child(Glyph::new(
                    "caret-down",
                    t.size_glyph_small,
                    colour(t.colour_text_2),
                ))
        };

        let grab = actions.clone();
        let mut row = rect()
            .width(Size::px(width))
            .height(Size::px(t.size_capture_bar))
            .horizontal()
            .cross_align(Alignment::Center)
            .padding((0., t.space_s))
            .corner_radius(t.radius_panel)
            .map(Some(shadows(&t.shadow_panel)), |el, list| {
                list.into_iter().fold(el, |el, s| el.shadow(s))
            })
            .maybe(bar.contrast, |el| {
                el.border(
                    Border::new()
                        .fill(colour(t.colour_hairline_strong))
                        .width(1.0)
                        .alignment(BorderAlignment::Inner),
                )
            })
            .maybe(bar.solid, |el| el.background(colour(t.material_solid)))
            .maybe(!bar.solid, |el| el.child(Glass::new(t.radius_panel)))
            .child(isle_ui::Hit)
            .on_mouse_down(move |_: Event<MouseEventData>| send(&grab, ToolbarAction::Grab))
            .child(icon_button(
                "x",
                "Close",
                Control::Close,
                ToolbarAction::Close,
            ));
        if let Some((app, icon)) = &bar.request {
            row = row.child(rule()).child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .maybe(icon.is_some(), |el| {
                        el.child(AppIcon::new(icon.clone().unwrap(), t.size_app_icon_small))
                            .child(rect().width(Size::px(t.space_xs)))
                    })
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        format!("{app} wants a screenshot"),
                    )),
            );
        }
        if !bar.chooser {
            row = row
                .child(rule())
                .child(modes)
                .child(rule())
                .child(icon_button(
                    "eyedropper",
                    "Pick colour",
                    Control::PickColour,
                    ToolbarAction::PickColour,
                ))
                .child(rule())
                .child(options);
        }
        if let Some(label) = bar.action_label() {
            let actions = actions.clone();
            row = row.child(rule()).child(
                rect()
                    .height(Size::px(t.size_control_large))
                    .padding((0., t.space_l))
                    .center()
                    .corner_radius(t.radius_control)
                    .background(colour(t.colour_accent))
                    .maybe(focused(Control::Act), |el| el.border(focus_ring(&theme)))
                    .on_press(move |_| send(&actions, ToolbarAction::Act))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_on_accent),
                        label,
                    )),
            );
        }
        rect().expanded().padding(margin).child(row)
    }
}
