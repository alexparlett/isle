//! The title bar the compositor draws over a window that lets it (B25), from
//! `p1/surfaces/window-chrome.md`: the traffic lights at the leading edge and the title.

use std::{cell::RefCell, rc::Rc, time::Instant};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Layer, Scope, SurfaceDef, WindowAction};

/// What a bar shows of its window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarModel {
    pub title: String,
    pub active: bool,
    /// Minimise is refused for dialogs and full-screen windows.
    pub can_minimise: bool,
    /// Zoom is refused for a window of fixed size.
    pub can_zoom: bool,
    pub full_screen: bool,
    /// Option is held: zoom fills, close and minimise act on the whole app.
    pub option: bool,
    /// The window asked for a transparent bar: the lights alone, over its own toolbar.
    pub overlay: bool,
}

/// One window's bar. The model outlives the tree, so a rebuilt bar shows the same.
pub struct TitleBar {
    model: Rc<RefCell<BarModel>>,
    state: Rc<RefCell<Option<State<BarModel>>>>,
}

impl TitleBar {
    pub fn new(model: BarModel) -> Rc<Self> {
        Rc::new(Self {
            model: Rc::new(RefCell::new(model)),
            state: Rc::new(RefCell::new(None)),
        })
    }

    pub fn model(&self) -> BarModel {
        self.model.borrow().clone()
    }

    /// Puts `model` in force; call it in the tree's root scope.
    pub fn set(&self, model: BarModel) {
        *self.model.borrow_mut() = model.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(model);
        }
    }
}

/// A bar's height: `size-titlebar`, or the title's line and 16 when the text is larger.
pub fn bar_height(theme: &Theme) -> i32 {
    let t = &theme.tokens;
    let line = t.type_style_headline_line * theme.appearance.text_scale + 16.0;
    t.size_titlebar.max(line).round() as i32
}

impl SurfaceDef for TitleBar {
    fn name(&self) -> &'static str {
        "titlebar"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Attached
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size((output.w, 32).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(BarApp {
            model: self.model.clone(),
            state: self.state.clone(),
        })
    }

    fn corners(&self, theme: &Theme) -> [f32; 4] {
        let r = theme.tokens.radius_window;
        [r, r, 0.0, 0.0]
    }
}

struct BarApp {
    model: Rc<RefCell<BarModel>>,
    state: Rc<RefCell<Option<State<BarModel>>>>,
}

impl App for BarApp {
    fn render(&self) -> impl IntoElement {
        let state = use_hook(|| {
            let state = State::create(self.model.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Bar { state }
    }
}

#[derive(PartialEq)]
struct Bar {
    state: State<BarModel>,
}

/// The lights' hit areas from the leading edge: close's runs to the edge.
const HITS: [(f32, f32); 3] = [(16.0, 32.0), (4.0, 20.0), (4.0, 20.0)];
/// Where the lights group ends, from the leading edge.
pub const LIGHTS_END: f32 = 68.0;
/// How long between two presses makes a double-click.
const DOUBLE_CLICK_MS: u128 = 450;

impl Component for Bar {
    fn render(&self) -> impl IntoElement {
        let theme = isle_ui::use_theme();
        let model = self.state.read().clone();
        let actions = Actions::get();
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| None::<usize>);
        let mut width = use_state(|| 0f32);
        let mut title_width = use_state(|| 0f32);
        let mut last_down = use_state(|| None::<Instant>);
        let t = &theme.tokens;
        let scale = theme.appearance.text_scale;

        let fill = if model.overlay {
            isle_tokens::Colour::TRANSPARENT
        } else if model.active {
            t.colour_titlebar
        } else {
            t.colour_titlebar_inactive
        };
        let text = if model.active {
            t.colour_text
        } else {
            t.colour_text_3
        };
        let font = t.type_style_headline_size * scale;
        let weight = t.type_style_headline_weight as i32;

        // Centred on the window, or laid after the lights, truncated, then dropped.
        let w = *width.read();
        let tw = *title_width.read();
        let after = LIGHTS_END + t.space_m;
        let centred = (w - tw) / 2.0;
        let title_box = if model.overlay {
            None
        } else if centred >= after {
            Some((centred, tw))
        } else {
            let room = w - after - t.space_m;
            (room >= t.size_title_minimum).then_some((after, tw.min(room)))
        };

        let lights = [
            (
                WindowAction::Close,
                WindowAction::CloseApp,
                true,
                (
                    t.traffic_close,
                    t.traffic_close_pressed,
                    t.traffic_close_glyph,
                ),
                "Close",
            ),
            (
                WindowAction::Minimise,
                WindowAction::MinimiseApp,
                model.can_minimise,
                (
                    t.traffic_minimise,
                    t.traffic_minimise_pressed,
                    t.traffic_minimise_glyph,
                ),
                "Minimise",
            ),
            (
                WindowAction::Zoom,
                WindowAction::Fill,
                model.can_zoom,
                (t.traffic_zoom, t.traffic_zoom_pressed, t.traffic_zoom_glyph),
                if model.option {
                    "Zoom"
                } else if model.full_screen {
                    "Exit Full Screen"
                } else {
                    "Enter Full Screen"
                },
            ),
        ];
        let option = model.option;
        let group = rect()
            .position(Position::new_absolute().left(0.).top(0.))
            .horizontal()
            .height(Size::fill())
            .on_pointer_enter(move |_| hovered.set(true))
            .on_pointer_leave(move |_| {
                hovered.set(false);
                pressed.set(None);
            })
            .children(
                lights
                    .into_iter()
                    .enumerate()
                    .map(|(i, light)| (i, light, actions.clone()))
                    .map(
                        |(
                            i,
                            (action, with_option, enabled, (colour, down, glyph_colour), name),
                            actions,
                        )| {
                            let (lead, hit) = HITS[i];
                            let is_pressed = *pressed.read() == Some(i);
                            let show = *hovered.read() && enabled;
                            let (background, ring) = if !enabled {
                                (t.colour_fill, Some(t.colour_hairline_strong))
                            } else if model.active || *hovered.read() {
                                (if is_pressed { down } else { colour }, Some(t.traffic_edge))
                            } else if theme.appearance.increase_contrast {
                                (t.colour_fill_strong, Some(t.colour_hairline_strong))
                            } else {
                                (t.colour_fill_strong, None)
                            };
                            let size = t.size_traffic_light;
                            let kind = match (i, option) {
                                (0, _) => Glyph::Close,
                                (1, _) => Glyph::Minimise,
                                (_, true) => Glyph::Plus,
                                _ => Glyph::FullScreen,
                            };
                            rect()
                                .key(i)
                                .width(Size::px(hit))
                                .height(Size::fill())
                                .padding((0., 0., 0., lead))
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .a11y_role(AccessibilityRole::Button)
                                .a11y_alt(name)
                                .on_mouse_down(move |e: Event<MouseEventData>| {
                                    e.stop_propagation();
                                    if e.button == Some(MouseButton::Left) && enabled {
                                        pressed.set(Some(i));
                                    }
                                })
                                .on_mouse_up(move |e: Event<MouseEventData>| {
                                    e.stop_propagation();
                                    if e.button == Some(MouseButton::Left)
                                        && enabled
                                        && *pressed.peek() == Some(i)
                                    {
                                        actions.send(Action::Window(if option {
                                            with_option
                                        } else {
                                            action
                                        }));
                                    }
                                    pressed.set(None);
                                })
                                .child(
                                    rect()
                                        .width(Size::px(size))
                                        .height(Size::px(size))
                                        .corner_radius(size / 2.)
                                        .background(isle_ui::colour(background))
                                        .map(ring, |el, ring| {
                                            el.border(
                                                Border::new()
                                                    .fill(isle_ui::colour(ring))
                                                    .width(0.5)
                                                    .alignment(BorderAlignment::Inner),
                                            )
                                        })
                                        .center()
                                        .maybe(show, |el| {
                                            el.child(glyph(kind, isle_ui::colour(glyph_colour)))
                                        }),
                                )
                        },
                    ),
            );

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(isle_ui::colour(fill))
            .on_sized(move |e: Event<SizedEventData>| width.set_if_modified(e.area.width()))
            .on_mouse_down(move |e: Event<MouseEventData>| {
                let send = |action| actions.send(Action::Window(action));
                match e.button {
                    Some(MouseButton::Left) => {
                        let now = Instant::now();
                        let double = last_down
                            .peek()
                            .is_some_and(|at| now.duration_since(at).as_millis() < DOUBLE_CLICK_MS);
                        if double {
                            last_down.set(None);
                            send(WindowAction::TitlebarAction);
                        } else {
                            last_down.set(Some(now));
                            send(WindowAction::Move { raise: true });
                        }
                    }
                    Some(MouseButton::Right) => send(WindowAction::Menu),
                    _ => {}
                }
            })
            // Measures the title at its full length, unseen.
            .child(
                label()
                    .position(Position::new_absolute().left(0.).top(0.))
                    .opacity(0.)
                    .font_size(font)
                    .font_weight(weight)
                    .max_lines(1)
                    .on_sized(move |e: Event<SizedEventData>| {
                        title_width.set_if_modified(e.area.width().ceil())
                    })
                    .text(model.title.clone()),
            )
            .map(title_box, |el, (x, tw)| {
                el.child(
                    rect()
                        .position(Position::new_absolute().left(x).top(0.))
                        .width(Size::px(tw))
                        .height(Size::fill())
                        .main_align(Alignment::Center)
                        .child(
                            label()
                                .width(Size::fill())
                                .font_size(font)
                                .font_weight(weight)
                                .color(isle_ui::colour(text))
                                .max_lines(1)
                                .text_overflow(TextOverflow::Ellipsis)
                                .text(model.title.clone()),
                        ),
                )
            })
            .child(group)
            .maybe(!model.overlay, |el| {
                el.child(
                    rect()
                        .position(Position::new_absolute().left(0.).bottom(0.))
                        .width(Size::fill())
                        .height(Size::px(t.stroke_hairline))
                        .background(isle_ui::colour(t.colour_hairline)),
                )
            })
    }
}

#[derive(Clone, Copy)]
enum Glyph {
    Close,
    Minimise,
    FullScreen,
    Plus,
}

/// The lights' glyphs, drawn in an 8 point box, round-capped at 1.25.
fn glyph(kind: Glyph, colour: Color) -> impl IntoElement {
    let bar = |length: f32, rotation: f32| {
        rect()
            .position(
                Position::new_absolute()
                    .left(4. - length / 2.)
                    .top(4. - 0.625),
            )
            .width(Size::px(length))
            .height(Size::px(1.25))
            .corner_radius(0.625)
            .background(colour)
            .rotation(rotation)
    };
    let frame = rect().width(Size::px(8.)).height(Size::px(8.));
    match kind {
        Glyph::Close => frame.child(bar(8.5, 45.)).child(bar(8.5, -45.)),
        Glyph::Minimise => frame.child(bar(6.0, 0.)),
        Glyph::Plus => frame.child(bar(7.0, 0.)).child(bar(7.0, 90.)),
        Glyph::FullScreen => frame.child(
            image(full_screen_glyph(colour))
                .width(Size::px(8.))
                .height(Size::px(8.)),
        ),
    }
}

/// Zoom's glyph: two right triangles of 4.5 legs pointing out along the diagonal, a point
/// apart, drawn into pixels at four times their size since Freya has no paths.
fn full_screen_glyph(colour: Color) -> freya::elements::image::ImageHandle {
    const N: usize = 32;
    const SUB: usize = 4;
    let inside = |x: f32, y: f32| {
        let a = x <= 6.6 && y >= 1.4 && (6.6 - x) + (y - 1.4) <= 4.5;
        let b = x >= 1.4 && y <= 6.6 && (x - 1.4) + (6.6 - y) <= 4.5;
        a || b
    };
    let mut pixels = Vec::with_capacity(N * N * 4);
    for py in 0..N {
        for px in 0..N {
            let mut hits = 0;
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let x = (px as f32 + (sx as f32 + 0.5) / SUB as f32) * 8.0 / N as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / SUB as f32) * 8.0 / N as f32;
                    hits += inside(x, y) as u32;
                }
            }
            let cover = hits as f32 / (SUB * SUB) as f32 * colour.a() as f32 / 255.0;
            pixels.extend([
                (colour.r() as f32 * cover) as u8,
                (colour.g() as f32 * cover) as u8,
                (colour.b() as f32 * cover) as u8,
                (255.0 * cover) as u8,
            ]);
        }
    }
    freya::elements::image::ImageHandle::from_rgba(
        N as u32,
        N as u32,
        Bytes::from(pixels),
        freya::engine::prelude::AlphaType::Premul,
    )
    .expect("a buffer of the size given")
}
