//! The probe: a surface for tests and the preview that exercises what real surfaces use.

use std::{cell::RefCell, rc::Rc};

use freya::{animation::*, prelude::*};
use isle_ui::Glass;
use freya_embed::AppComponent;
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Layer, Scope, SurfaceDef};

pub const SIZE: (i32, i32) = (480, 360);
/// The square inside the panel's top-right corner, 8 points wide, 24 in from each edge.
pub const MARKER: (u8, u8, u8) = (255, 0, 0);
/// The glass strip along the bottom, in points.
pub const GLASS_HEIGHT: i32 = 80;

/// What outlives the tree: the field's text and the presses counted.
#[derive(Default, Clone, Debug)]
pub struct ProbeModel {
    pub text: String,
    pub presses: u32,
    pub animating: bool,
}

/// Where an armed probe panics: rendering, a key handler, or drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Render,
    Event,
    Draw,
}

#[derive(Default)]
pub struct Probe {
    pub model: Rc<RefCell<ProbeModel>>,
    switch: Rc<RefCell<Option<State<Option<Fault>>>>>,
}

impl Probe {
    /// Makes the current tree panic at `fault`; call it in the tree's root scope.
    pub fn arm(&self, fault: Fault) {
        if let Some(mut switch) = *self.switch.borrow() {
            switch.set(Some(fault));
        }
    }
}



impl SurfaceDef for Probe {
    fn name(&self) -> &'static str {
        "probe"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::MainOutput
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        let size = SSize::from(SIZE);
        Rectangle::new(
            ((output.w - size.w) / 2, (output.h - size.h) / 2).into(),
            size,
        )
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(ProbeApp {
            model: self.model.clone(),
            switch: self.switch.clone(),
        })
    }

    fn corner_radius(&self, theme: &isle_tokens::Theme) -> f32 {
        theme.tokens.radius_panel
    }

}

/// The tree; it holds the model, so it reads it back however it is hosted and rebuilt.
struct ProbeApp {
    model: Rc<RefCell<ProbeModel>>,
    switch: Rc<RefCell<Option<State<Option<Fault>>>>>,
}

impl App for ProbeApp {
    fn render(&self) -> impl IntoElement {
        let switch = use_hook(|| {
            let switch = State::create(None);
            *self.switch.borrow_mut() = Some(switch);
            switch
        });
        probe(self.model.clone(), switch)
    }
}

fn probe(model: Rc<RefCell<ProbeModel>>, switch: State<Option<Fault>>) -> impl IntoElement {
    let actions = Actions::get();
    if *switch.read() == Some(Fault::Render) {
        panic!("a render panic injected into the probe");
    }
    let text = use_state({
        let model = model.clone();
        move || model.borrow().text.clone()
    });
    let mut presses = use_state({
        let model = model.clone();
        move || model.borrow().presses
    });
    let mut animating = use_state({
        let model = model.clone();
        move || model.borrow().animating
    });
    use_side_effect({
        let model = model.clone();
        move || {
            let mut model = model.borrow_mut();
            model.text = text.read().clone();
            model.presses = *presses.read();
            model.animating = *animating.read();
        }
    });

    let mut bar = use_animation(|conf| {
        conf.on_finish(OnFinish::restart());
        AnimNum::new(0., 100.).time(1200)
    });
    use_side_effect(move || {
        if *animating.read() {
            bar.start();
        } else {
            bar.reset();
        }
    });
    let progress = bar.read().value();
    // A fade inside the tree, which Skia draws through a layer per element and caches.
    let rows = use_animation(|conf| {
        conf.on_creation(OnCreation::Run);
        AnimNum::new(0., 1.).time(200)
    });
    let rows_opacity = rows.read().value();
    let theme = isle_ui::use_theme();
    let fill = isle_ui::colour(theme.tokens.colour_window_raised);
    let text_colour = isle_ui::colour(theme.tokens.colour_text);

    let content = rect()
        .expanded()
        .padding(16.)
        .spacing(10.)
        .child(label().text("Isle probe · 東京 🎧").font_size(18.))
        .maybe_child((*switch.read()).map(|fault| label().text(format!("Armed: {fault:?}"))))
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .child(
                    Button::new()
                        .on_press(move |_| *presses.write() += 1)
                        .child(format!("Pressed {}", presses.read())),
                )
                .child(
                    Button::new()
                        .on_press(move |_| animating.toggle())
                        .child(if *animating.read() { "Stop" } else { "Animate" }),
                ),
        )
        .child(
            Input::new(text)
                .placeholder("Type here")
                .width(Size::fill())
                .auto_focus(true)
                .on_submit(move |_| {
                    // Enter closes the probe and types an "a" into what has the keyboard next.
                    actions.send(Action::Close);
                    actions.send(Action::Key(30));
                }),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(4.))
                .corner_radius(2.)
                .background((255, 255, 255, 0.15))
                .child(
                    rect()
                        .width(Size::percent(progress))
                        .height(Size::fill())
                        .corner_radius(2.)
                        .background((255, 255, 255, 0.85)),
                ),
        )
        .child(
            ScrollView::new().height(Size::fill()).child(
                rect()
                    .width(Size::fill())
                    .spacing(4.)
                    .opacity(rows_opacity)
                    .children((1..=30).map(|n| {
                        rect()
                            .width(Size::fill())
                            .padding(6.)
                            .background((255, 255, 255, 0.06))
                            .child(format!("Row {n}"))
                            .into_element()
                    })),
            ),
        );

    let panel = rect()
        .width(Size::fill())
        .height(Size::px((SIZE.1 - GLASS_HEIGHT) as f32))
        .background(fill)
        .child(
            rect()
                .position(Position::new_absolute().right(24.).top(24.))
                .width(Size::px(8.))
                .height(Size::px(8.))
                .background(MARKER),
        )
        .child(content);
    let strip = rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(Glass::new(0.))
        .child(rect().padding(16.).child(label().text("Glass")));

    rect()
        .width(Size::px(SIZE.0 as f32))
        .height(Size::px(SIZE.1 as f32))
        .color(text_colour)
        .on_global_key_down(move |_| {
            if *switch.peek() == Some(Fault::Event) {
                panic!("an event panic injected into the probe");
            }
        })
        .child(
            canvas(RenderCallback::new(move |_| {
                if *switch.peek() == Some(Fault::Draw) {
                    panic!("a draw panic injected into the probe");
                }
            }))
            .position(Position::new_absolute())
            .width(Size::px(1.))
            .height(Size::px(1.)),
        )
        .child(panel)
        .child(strip)
}

/// A full-output surface shaped like a launcher, for measuring a draw the size of the screen:
/// a backdrop, a 1200 by 760 panel with a field and 24 tiles, and a fade on every tile that loops
/// while it animates, the worst case for Skia's layers.
#[derive(Default)]
pub struct ProbeFull {
    animate: Rc<RefCell<Option<State<bool>>>>,
}

impl ProbeFull {
    /// Starts or stops the tiles' fade; call it in the tree's root scope.
    pub fn set_animating(&self, on: bool) {
        if let Some(mut animate) = *self.animate.borrow() {
            animate.set(on);
        }
    }
}

impl SurfaceDef for ProbeFull {
    fn name(&self) -> &'static str {
        "probe-full"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::MainOutput
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(ProbeFullApp {
            animate: self.animate.clone(),
        })
    }
}

struct ProbeFullApp {
    animate: Rc<RefCell<Option<State<bool>>>>,
}

impl App for ProbeFullApp {
    fn render(&self) -> impl IntoElement {
        let animate = use_hook(|| {
            let animate = State::create(false);
            *self.animate.borrow_mut() = Some(animate);
            animate
        });
        let text = use_state(String::new);
        let mut fade = use_animation(|conf| {
            conf.on_finish(OnFinish::reverse());
            AnimNum::new(1., 0.4).time(300)
        });
        use_side_effect(move || {
            if *animate.read() {
                fade.start();
            } else {
                fade.reset();
            }
        });
        let opacity = fade.read().value();
        let theme = isle_ui::use_theme();
        let tiles = (0..24).map(|i| {
            let hue = i as f32 * 360. / 48.;
            rect()
                .width(Size::px(160.))
                .height(Size::px(140.))
                .center()
                .spacing(8.)
                .opacity(opacity)
                .child(
                    rect()
                        .width(Size::px(64.))
                        .height(Size::px(64.))
                        .corner_radius(16.)
                        .background(hsl(hue, 0.55, 0.55)),
                )
                .child(label().text(format!("App {i}")).font_size(14.))
                .into_element()
        });
        rect()
            .expanded()
            .center()
            .background((0, 0, 0, 0.45))
            .color(isle_ui::colour(theme.tokens.colour_text))
            .child(
                rect()
                    .width(Size::px(1200.))
                    .height(Size::px(760.))
                    .corner_radius(28.)
                    .padding(32.)
                    .spacing(24.)
                    .background(isle_ui::colour(theme.tokens.colour_window_raised))
                    .child(
                        Input::new(text)
                            .placeholder("Search apps")
                            .width(Size::fill())
                            .auto_focus(true),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .width(Size::fill())
                            .spacing(16.)
                            .content(Content::Wrap { wrap_spacing: Some(16.) })
                            .children(tiles),
                    ),
            )
    }
}

fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let x = c * (1. - ((h / 60.) % 2. - 1.).abs());
    let m = l - c / 2.;
    let (r, g, b) = match (h / 60.) as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let byte = |v: f32| ((v + m) * 255.).round() as u8;
    Color::from_rgb(byte(r), byte(g), byte(b))
}
