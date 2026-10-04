//! The probe: a surface for tests and the preview that exercises what real surfaces use.

use std::{cell::RefCell, rc::Rc};

use freya::{animation::*, prelude::*};
use freya_embed::{AppComponent, Runner};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Layer, Scope, SurfaceDef};

pub const SIZE: (i32, i32) = (480, 360);
/// The panel's fill, which tests look for.
pub const FILL: (u8, u8, u8) = (28, 28, 34);
/// The square in the panel's top-left corner, 8 points wide.
pub const MARKER: (u8, u8, u8) = (255, 0, 0);

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
    switch: RefCell<Option<State<Option<Fault>>>>,
}

impl Probe {
    /// Makes the current tree panic at `fault`; call it in the tree's root scope.
    pub fn arm(&self, fault: Fault) {
        if let Some(mut switch) = *self.switch.borrow() {
            switch.set(Some(fault));
        }
    }
}

#[derive(Clone)]
struct Model(Rc<RefCell<ProbeModel>>);

#[derive(Clone, Copy)]
struct Switch(State<Option<Fault>>);

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
        AppComponent::new(ProbeApp)
    }

    fn provide(&self, runner: &mut Runner) {
        let model = self.model.clone();
        runner.provide_root_context(move || Model(model));
        let switch = runner.provide_root_context(|| Switch(State::create(None)));
        *self.switch.borrow_mut() = Some(switch.0);
    }
}

struct ProbeApp;

impl App for ProbeApp {
    fn render(&self) -> impl IntoElement {
        probe()
    }
}

fn probe() -> impl IntoElement {
    let model = use_consume::<Model>().0;
    let actions = Actions::get();
    let switch = use_consume::<Switch>().0;
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

    rect()
        .width(Size::px(SIZE.0 as f32))
        .height(Size::px(SIZE.1 as f32))
        .background(FILL)
        .color((235, 235, 240))
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
        .child(
            rect()
                .position(Position::new_absolute().left(0.).top(0.))
                .width(Size::px(8.))
                .height(Size::px(8.))
                .background(MARKER),
        )
        .child(content)
}
