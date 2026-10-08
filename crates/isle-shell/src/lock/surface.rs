//! The lock's face on every screen: the date and time on all of them, the rest on the one
//! with the pointer (`p1/surfaces/lock.md`).

use std::{cell::Cell, cell::RefCell, collections::HashMap, rc::Rc};

use freya::{animation::*, prelude::*};
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    AppIcon, Button, Glyph, Spinner, button::Kind, colour, focus_ring, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::{Face, Frame, Line, LockAction, Note, Playing, Power};
use crate::surface::{Action, Actions, Here, Layer, Scope, SurfaceDef};

#[derive(Default)]
pub struct LockSurface {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl LockSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Keeps `frame` for `output`'s next tree; safe anywhere.
    pub fn set_model(&self, output: &str, frame: Frame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
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
}

impl SurfaceDef for LockSurface {
    fn name(&self) -> &'static str {
        "lock"
    }

    fn layer(&self) -> Layer {
        Layer::Lock
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    /// The compositor gives the keyboard to the screen with the face itself.
    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(LockApp {
            frames: self.frames.clone(),
            states: self.states.clone(),
        })
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct LockApp {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl App for LockApp {
    fn render(&self) -> impl IntoElement {
        let here = use_consume::<Here>().0;
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
        FaceView { frame: state }
    }
}

fn send(actions: &Actions, action: LockAction) {
    actions.send(Action::Lock(action));
}

/// A style's size, line height and weight from the tokens, for the styles `text` lacks.
fn styled(theme: &Theme, size: f32, line: f32, weight: f32, ink: Color, words: String) -> Label {
    let scale = theme.appearance.text_scale.min(theme.tokens.type_scale_max);
    label()
        .font_size(size * scale)
        .line_height(line / size)
        .font_weight(weight as i32)
        .color(ink)
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis)
        .text(words)
}

/// A lock plate: glass at the user's position with no blur pass, since the backdrop is
/// already soft; solid under Reduce Transparency, edged under Increase Contrast.
fn plate(theme: &Theme, solid: bool, radius: f32) -> freya::prelude::Rect {
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
struct FaceView {
    frame: State<Frame>,
}

impl Component for FaceView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let face = &frame.face;
        let seen_panics = use_hook(|| Rc::new(Cell::new(frame.panics)));
        if seen_panics.get() != frame.panics {
            seen_panics.set(frame.panics);
            panic!("a panic injected into the lock's face");
        }
        let actions = Actions::get();
        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);

        // The lower parts fade as the face dozes and come back as it wakes.
        let mut doze = use_animation(|_| AnimNum::new(1., 0.).time(200));
        let was_awake = use_hook(|| Rc::new(Cell::new(true)));
        let fade_ms = t.motion_fade_ms;
        let exit_ms = t.motion_exit_fade_ms;
        let awake = face.awake;
        let still = face.still;
        use_side_effect(move || {
            if was_awake.get() == awake {
                return;
            }
            was_awake.set(awake);
            if still {
                return;
            }
            if awake {
                doze.reset();
            } else {
                doze.run(AnimDirection::Forward);
            }
            let _ = (fade_ms, exit_ms);
        });
        let lower = if face.awake {
            1.0
        } else if face.still {
            0.0
        } else {
            doze.read().value()
        };

        // Each wrong password shakes the field along x: two cycles, decaying to rest.
        let mut shake =
            use_animation(|_| AnimNum::new(0., 1.).time(320).function(Function::Linear));
        let shook = use_hook(|| Rc::new(Cell::new(face.shakes)));
        let shakes = face.shakes;
        let reduce = face.still || theme.appearance.reduce_motion;
        use_side_effect(move || {
            if shook.get() != shakes {
                shook.set(shakes);
                if !reduce {
                    shake.start();
                }
            }
        });
        let p = shake.read().value();
        let offset = if p > 0.0 && p < 1.0 {
            t.motion_shake_distance * (1.0 - p) * (p * 4.0 * std::f32::consts::PI).sin()
        } else {
            0.0
        };

        let top = t.size_strip + t.space_xxxl;
        let mut root = rect()
            .expanded()
            .on_pointer_move({
                let actions = actions.clone();
                move |_| send(&actions, LockAction::Wake)
            })
            .child(date_and_time(&theme, face, top));
        if !frame.here {
            return root.into_element();
        }

        let time_line = t.type_style_display_line;
        let date_line = t.type_style_title_2_line;
        let column_top = top + date_line + t.space_xs + time_line + t.space_xxxl;
        root = root.child(column(&theme, face, column_top, frame.width, &actions));

        let field_h = t.size_control_large;
        let field_top = frame.height * 0.8 - field_h / 2.0;
        let name_line = t.type_style_title_3_line;
        let name_top = field_top - t.space_m - name_line;
        let picture_top = name_top - t.space_s - t.size_avatar;
        root = root.child(
            rect()
                .position(Position::new_absolute().top(picture_top).left(0.))
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .opacity(lower)
                .child(picture(&theme, face))
                .child(rect().height(Size::px(t.space_s)))
                .child(styled(
                    &theme,
                    t.type_style_title_3_size,
                    t.type_style_title_3_line,
                    t.type_style_title_3_weight,
                    ink,
                    face.user.clone(),
                ))
                .child(rect().height(Size::px(t.space_m)))
                .child(
                    rect()
                        .width(Size::px(t.size_lock_field + 2.0 * t.motion_shake_distance))
                        .height(Size::px(field_h))
                        .child(
                            rect()
                                .position(
                                    Position::new_absolute()
                                        .left(t.motion_shake_distance + offset)
                                        .top(0.),
                                )
                                .child(Field {
                                    dots: face.dots,
                                    held: face.held,
                                    locked_out: face.locked_out,
                                    error_edge: face.error_edge,
                                    solid: face.solid,
                                }),
                        ),
                )
                .child(rect().height(Size::px(t.space_s)))
                .child(status_line(&theme, &face.line, ink, ink_2)),
        );
        root.child(power_row(&theme, face, frame.height, lower, &actions))
            .child(utilities(
                &theme,
                face,
                frame.width,
                frame.height,
                lower,
                &actions,
            ))
            .into_element()
    }
}

fn date_and_time(theme: &Theme, face: &Face, top: f32) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let ink = colour(t.colour_text);
    let date = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .maybe_child(
            face.focus
                .clone()
                .map(|g| Glyph::new(g, t.size_glyph, ink).fill(true)),
        )
        .child(styled(
            theme,
            t.type_style_title_2_size,
            t.type_style_title_2_line,
            t.type_style_title_2_weight,
            ink,
            face.date.clone(),
        ));
    // The time keeps its size at the largest text, and its box is the widest time's.
    let time = label()
        .font_size(t.type_style_display_size)
        .line_height(t.type_style_display_line / t.type_style_display_size)
        .font_weight(t.type_style_display_weight as i32)
        .color(ink)
        .max_lines(1)
        .text_align(TextAlign::Center)
        .width(Size::px(t.type_style_display_size * 3.0))
        .a11y_alt(face.time.clone())
        .text(face.time.clone());
    rect()
        .position(Position::new_absolute().top(top).left(0.))
        .width(Size::fill())
        .cross_align(Alignment::Center)
        .spacing(t.space_xs)
        .child(date)
        .child(time)
}

fn column(
    theme: &Theme,
    face: &Face,
    top: f32,
    width: f32,
    actions: &Actions,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let column_w = t.size_panel - 2.0 * t.space_s;
    let mut column = rect()
        .position(
            Position::new_absolute()
                .top(top)
                .left((width - column_w) / 2.0),
        )
        .width(Size::px(column_w))
        .spacing(t.space_s);
    if let Some(playing) = &face.playing {
        column = column.child(now_playing(theme, face.solid, playing, actions));
    }
    for note in &face.notes {
        column = column.child(note_plate(theme, face.solid, note, actions));
    }
    if face.more > 0 {
        column = column.child(
            plate(theme, face.solid, t.radius_panel)
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .center()
                .child(text::line(
                    theme,
                    text::Style::Callout,
                    colour(t.colour_text_2),
                    format!("{} more", face.more),
                )),
        );
    }
    column
}

fn now_playing(
    theme: &Theme,
    solid: bool,
    playing: &Playing,
    actions: &Actions,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let art_size = 48.0;
    let art = match &playing.art {
        Some(path) => rect()
            .width(Size::px(art_size))
            .height(Size::px(art_size))
            .corner_radius(t.radius_control)
            .overflow(Overflow::Clip)
            .child(AppIcon::new(path.clone(), art_size)),
        None => rect()
            .width(Size::px(art_size))
            .height(Size::px(art_size))
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill_strong))
            .center()
            .child(Glyph::new("music-note", 20., colour(t.colour_text_2))),
    };
    let button = |glyph: &'static str, fill: bool, alt: &'static str, action: LockAction| {
        let actions = actions.clone();
        rect()
            .width(Size::px(t.size_control))
            .height(Size::px(t.size_control))
            .center()
            .corner_radius(t.radius_control)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(alt)
            .on_press(move |e: Event<PressEventData>| {
                e.stop_propagation();
                send(&actions, action.clone());
            })
            .child(Glyph::new(glyph, t.size_glyph, colour(t.colour_text)).fill(fill))
    };
    let (play_glyph, play_alt) = if playing.playing {
        ("pause", "Pause")
    } else {
        ("play", "Play")
    };
    plate(theme, solid, t.radius_panel)
        .width(Size::fill())
        .height(Size::px(72.))
        .padding(t.space_m)
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(art)
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    playing.title.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    playing.artist.clone(),
                )),
        )
        .child(
            rect()
                .horizontal()
                .spacing(t.space_xxs)
                .child(button(
                    "skip-back",
                    true,
                    "Previous track",
                    LockAction::Previous,
                ))
                .child(button(play_glyph, true, play_alt, LockAction::PlayPause))
                .child(button("skip-forward", true, "Next track", LockAction::Next)),
        )
}

fn note_plate(theme: &Theme, solid: bool, note: &Note, actions: &Actions) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let second = match &note.summary {
        Some(summary) => summary.clone(),
        None if note.count == 1 => "1 notification".into(),
        None => format!("{} notifications", note.count),
    };
    let alt = format!("{}, {}", note.name, second);
    let actions = actions.clone();
    plate(theme, solid, t.radius_panel)
        .width(Size::fill())
        .height(Size::px(56.))
        .padding(t.space_m)
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(alt)
        .on_press(move |_| send(&actions, LockAction::Wake))
        .map(note.icon.clone(), |el, icon| {
            el.child(AppIcon::new(icon, t.size_app_icon))
        })
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    note.name.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    second,
                )),
        )
}

fn picture(theme: &Theme, face: &Face) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let size = t.size_avatar;
    let circle = rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(size / 2.0)
        .overflow(Overflow::Clip)
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(t.stroke_hairline)
                .alignment(BorderAlignment::Inner),
        );
    match &face.picture {
        Some(path) => circle.child(AppIcon::new(path.clone(), size)),
        None => circle
            .background(colour(t.colour_fill_strong))
            .center()
            .child(Glyph::new("user", 32., colour(t.colour_text_2))),
    }
}

/// The password field: dots for what is typed, never the text, which the compositor holds.
#[derive(PartialEq)]
struct Field {
    dots: usize,
    held: bool,
    locked_out: bool,
    error_edge: bool,
    solid: bool,
}

impl Component for Field {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let focused = focus() != Focus::Not;
        let reported = use_hook(|| Rc::new(Cell::new(None::<bool>)));
        {
            let actions = actions.clone();
            use_side_effect(move || {
                let now = focus() != Focus::Not;
                if reported.get() != Some(now) {
                    reported.set(Some(now));
                    send(&actions, LockAction::FieldFocus(now));
                }
            });
        }
        let placeholder = if self.locked_out {
            colour(t.colour_text_disabled)
        } else {
            colour(t.colour_text_2)
        };
        let dot = 6.0;
        let width = t.size_lock_field;
        let height = t.size_control_large;
        let shown = self
            .dots
            .min(((width - 2.0 * t.space_m) / (dot + t.space_xs)) as usize);
        let caret = (focused && !self.held).then(|| {
            rect()
                .width(Size::px(1.5))
                .height(Size::px(16.))
                .background(colour(t.colour_text))
                .into_element()
        });
        // The caret marks the field's focus: before the placeholder, after the dots.
        let content = if self.dots == 0 {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xxs)
                .maybe_child(caret)
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    placeholder,
                    "Enter password",
                ))
        } else {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .children((0..shown).map(|_| {
                    rect()
                        .width(Size::px(dot))
                        .height(Size::px(dot))
                        .corner_radius(dot / 2.0)
                        .background(colour(t.colour_text))
                        .into_element()
                }))
                .maybe_child(caret)
        };
        plate(&theme, self.solid, height / 2.0)
            .a11y_id(a11y)
            .a11y_focusable(!self.locked_out)
            .a11y_auto_focus(true)
            .a11y_role(AccessibilityRole::PasswordInput)
            .a11y_alt("Password")
            .width(Size::px(width))
            .height(Size::px(height))
            .padding((0., t.space_m))
            .horizontal()
            .cross_align(Alignment::Center)
            .maybe(self.error_edge, |el| {
                el.border(
                    Border::new()
                        .fill(colour(t.colour_danger))
                        .width(1.0)
                        .alignment(BorderAlignment::Inner),
                )
            })
            .maybe(focus() == Focus::Keyboard && !self.error_edge, |el| {
                el.border(focus_ring(&theme))
            })
            .child(rect().width(Size::flex(1.)).child(content))
    }
}

fn status_line(theme: &Theme, line: &Line, ink: Color, ink_2: Color) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let colour_of = if line.acts() || theme.appearance.increase_contrast {
        ink
    } else {
        ink_2
    };
    let mut slot = rect()
        .height(Size::px(16.))
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_xs);
    match line.text() {
        None => {}
        Some((glyph, words)) => {
            slot = match (glyph, line) {
                (_, Line::Checking { .. }) => slot.child(Spinner::new(12., "Checking")),
                (Some(glyph), _) => slot.child(Glyph::new(glyph, t.size_glyph_small, colour_of)),
                (None, _) => slot,
            };
            slot = slot.child(text::line(theme, text::Style::Callout, colour_of, words));
        }
    }
    slot
}

fn power_row(
    theme: &Theme,
    face: &Face,
    height: f32,
    lower: f32,
    actions: &Actions,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let callout = t.type_style_callout_line;
    let top = height - t.space_xxxl - callout - t.space_xs - t.size_lock_button;
    let circle = |glyph: &'static str, label: &'static str, action: LockAction| {
        let actions = actions.clone();
        rect()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(
                plate(theme, face.solid, t.size_lock_button / 2.0)
                    .width(Size::px(t.size_lock_button))
                    .height(Size::px(t.size_lock_button))
                    .center()
                    .a11y_focusable(true)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(label)
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        send(&actions, action.clone());
                    })
                    .child(Glyph::new(glyph, t.size_glyph_large, colour(t.colour_text))),
            )
            .child(text::line(
                theme,
                text::Style::Callout,
                colour(t.colour_text_2),
                label,
            ))
            .into_element()
    };
    let row = rect()
        .position(Position::new_absolute().top(top).left(0.))
        .width(Size::fill())
        .opacity(lower)
        .horizontal()
        .main_align(Alignment::Center);
    // The confirm and the quitting line replace the circles in one capsule where they were.
    let capsule = || {
        plate(theme, face.solid, t.size_lock_button / 2.0)
            .height(Size::px(t.size_lock_button))
            .padding((0., t.space_l))
            .horizontal()
            .cross_align(Alignment::Center)
    };
    let ink = colour(t.colour_text);
    match face.power {
        Power::Row => row.spacing(t.space_xxl).children([
            circle("moon-stars", "Sleep", LockAction::Sleep),
            circle("arrow-clockwise", "Restart", LockAction::Restart),
            circle("power", "Shut Down", LockAction::ShutDown),
        ]),
        Power::Confirm { restart } => {
            let (question, action) = if restart {
                ("Restart now?", "Restart")
            } else {
                ("Shut down now?", "Shut Down")
            };
            let (cancel, confirm) = (actions.clone(), actions.clone());
            row.child(
                capsule()
                    .child(text::line(theme, text::Style::Body, ink, question))
                    .child(rect().width(Size::px(t.space_l)))
                    .child(
                        rect().horizontal().spacing(t.space_s).child(
                            Button::new("Cancel")
                                .on_press(move |_| send(&cancel, LockAction::PowerCancel)),
                        )
                        .child(
                            Button::new(action)
                                .kind(Kind::Default)
                                .on_press(move |_| send(&confirm, LockAction::PowerConfirm)),
                        ),
                    ),
            )
        }
        Power::Quitting => row.child(
            capsule()
                .spacing(t.space_s)
                .child(Spinner::new(12., "Quitting apps"))
                .child(text::line(theme, text::Style::Body, ink, "Quitting apps…")),
        ),
    }
}

fn utilities(
    theme: &Theme,
    face: &Face,
    width: f32,
    height: f32,
    lower: f32,
    actions: &Actions,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let callout = t.type_style_callout_line;
    let circles_centre = height - t.space_xxxl - callout - t.space_xs - t.size_lock_button / 2.0;
    let capsule_h = t.size_control_large;
    let mut row = rect()
        .position(
            Position::new_absolute()
                .top(circles_centre - capsule_h / 2.0)
                .right(t.space_xxxl),
        )
        .opacity(lower)
        .horizontal()
        .spacing(t.space_s);
    let _ = width;
    if let Some(layout) = &face.layout {
        let actions = actions.clone();
        let alt = if face.layout_marked {
            format!("Keyboard layout: {layout}, check it")
        } else {
            format!("Keyboard layout: {layout}")
        };
        row = row.child(
            plate(theme, face.solid, capsule_h / 2.0)
                .height(Size::px(capsule_h))
                .padding((0., t.space_m))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .a11y_focusable(true)
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(alt)
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    send(&actions, LockAction::NextLayout);
                })
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    layout.clone(),
                ))
                .child(Glyph::new("caret-up", 12., colour(t.colour_text)))
                .maybe_child(face.layout_marked.then(|| {
                    rect()
                        .position(Position::new_absolute().top(-2.).right(-2.))
                        .width(Size::px(8.))
                        .height(Size::px(8.))
                        .corner_radius(4.)
                        .background(colour(t.material_solid))
                        .center()
                        .child(
                            rect()
                                .width(Size::px(6.))
                                .height(Size::px(6.))
                                .corner_radius(3.)
                                .background(colour(t.colour_warn)),
                        )
                })),
        );
    }
    if let Some((percent, charging)) = face.battery {
        let alt = if charging {
            format!("Battery {percent}%, charging")
        } else {
            format!("Battery {percent}%")
        };
        let glyph = match percent {
            0..=20 => "battery-low",
            21..=60 => "battery-medium",
            _ => "battery-high",
        };
        row = row.child(
            plate(theme, face.solid, capsule_h / 2.0)
                .height(Size::px(capsule_h))
                .padding((0., t.space_m))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .a11y_role(AccessibilityRole::Label)
                .a11y_alt(alt)
                .child(Glyph::new(
                    if charging { "battery-charging" } else { glyph },
                    t.size_glyph,
                    colour(t.colour_text),
                ))
                .child(text::line(
                    theme,
                    text::Style::Callout,
                    colour(t.colour_text),
                    format!("{percent}%"),
                )),
        );
    }
    row
}
