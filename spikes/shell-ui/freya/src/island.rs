//! The island's UI as Freya components.

use std::time::Duration;

use chrono::{DateTime, Local};
use freya::{animation::*, prelude::*};

use crate::widgets::{PRIMARY, SECONDARY, TextField};

const TRACK: Color = Color::from_argb(38, 255, 255, 255);
const FILLED: Color = Color::from_argb(217, 255, 255, 255);
const DOT: Color = Color::from_rgb(0x4a, 0xde, 0x80);
const KNOB: f32 = 14.;

/// What the compositor drives, provided at the root.
#[derive(Clone, Copy, PartialEq)]
pub struct Island {
    pub expanded: State<bool>,
    /// Logical size to lay out in; `None` is the expanded size.
    pub layout: State<Option<(f32, f32)>>,
    pub query: State<String>,
    pub volume: State<f32>,
    pub field: AccessibilityId,
}

impl Island {
    pub fn new() -> Self {
        Self {
            expanded: State::create(false),
            layout: State::create(None),
            query: State::create(String::new()),
            volume: State::create(0.4),
            field: AccessibilityId::new_unique(),
        }
    }
}

type Now = State<DateTime<Local>>;

pub fn app() -> impl IntoElement {
    let island = use_consume::<Island>();
    let expanded = *island.expanded.read();
    let (width, height) = island.layout.read().unwrap_or((520., 240.));

    let now: Now = use_provide_context(|| State::create(Local::now()));
    use_hook(move || {
        let mut now = now;
        spawn(async move {
            loop {
                let ms = 1000 - now.peek().timestamp_subsec_millis().min(999);
                timer(Duration::from_millis(ms as u64)).await;
                now.set(Local::now());
            }
        });
    });

    let rest = use_animation_with_dependencies(&expanded, |conf, expanded| {
        conf.on_creation(OnCreation::Finish);
        conf.on_change(OnChange::Rerun);
        let (from, to) = if *expanded { (1., 0.) } else { (0., 1.) };
        AnimNum::new(from, to)
            .time(180)
            .ease(Ease::Out)
            .function(Function::Cubic)
    });
    let panel = use_animation_with_dependencies(&expanded, |conf, expanded| {
        conf.on_creation(OnCreation::Finish);
        conf.on_change(OnChange::Rerun);
        let fade = |from, to, ms| {
            AnimNum::new(from, to)
                .time(ms)
                .ease(Ease::Out)
                .function(Function::Cubic)
        };
        if *expanded {
            Delayed::new(80, fade(0., 1., 220))
        } else {
            Delayed::new(0, fade(1., 0., 120))
        }
    });
    let rest_opacity = rest.get().value();
    let panel_opacity = panel.get().inner.value();

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .cross_align(Alignment::Center)
        .color(PRIMARY)
        .child(
            rect()
                .width(Size::px(width))
                .height(Size::px(height))
                .child(
                    rect()
                        .position(Position::new_absolute())
                        .width(Size::percent(100.))
                        .cross_align(Alignment::Center)
                        .opacity(rest_opacity)
                        .child(RestPill),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::fill())
                        .opacity(panel_opacity)
                        .child(Panel { expanded }),
                ),
        )
}

/// An animation that holds its start value for `delay` ms before running.
#[derive(Clone, Default)]
struct Delayed {
    delay: u128,
    inner: AnimNum,
}

impl Delayed {
    fn new(delay: u128, inner: AnimNum) -> Self {
        Self { delay, inner }
    }
}

impl AnimatedValue for Delayed {
    fn prepare(&mut self, direction: AnimDirection) {
        self.inner.prepare(direction);
    }

    fn is_finished(&self, index: u128, direction: AnimDirection) -> bool {
        index >= self.delay && self.inner.is_finished(index - self.delay, direction)
    }

    fn advance(&mut self, index: u128, direction: AnimDirection) {
        if index >= self.delay {
            self.inner.advance(index - self.delay, direction);
        }
    }

    fn finish(&mut self, direction: AnimDirection) {
        self.inner.finish(direction);
    }

    fn into_reversed(self) -> Self {
        Self {
            inner: self.inner.into_reversed(),
            ..self
        }
    }
}

#[derive(PartialEq)]
struct Clock {
    seconds: bool,
    size: f32,
    weight: FontWeight,
    color: Color,
}

impl Component for Clock {
    fn render(&self) -> impl IntoElement {
        let now = use_consume::<Now>();
        let format = if self.seconds { "%H:%M:%S" } else { "%H:%M" };
        label()
            .text(now.read().format(format).to_string())
            .font_size(self.size)
            .font_weight(self.weight)
            .font_tabular()
            .color(self.color)
    }
}

#[derive(PartialEq)]
struct RestPill;

impl Component for RestPill {
    fn render(&self) -> impl IntoElement {
        let overlay = || {
            rect()
                .position(Position::new_absolute())
                .width(Size::percent(100.))
                .height(Size::percent(100.))
        };
        rect()
            .width(Size::px(220.))
            .height(Size::px(38.))
            .child(
                rect()
                    .position(Position::new_absolute().left(14.).top(15.))
                    .width(Size::px(8.))
                    .height(Size::px(8.))
                    .corner_radius(4.)
                    .background(DOT),
            )
            .child(overlay().center().child(Clock {
                seconds: true,
                size: 15.,
                weight: FontWeight::SEMI_BOLD,
                color: PRIMARY,
            }))
            .child(
                overlay()
                    .horizontal()
                    .main_align(Alignment::End)
                    .cross_align(Alignment::Center)
                    .padding((0., 16., 0., 0.))
                    .child(label().text("72%").font_size(13.).color(SECONDARY)),
            )
    }
}

#[derive(PartialEq)]
struct Panel {
    expanded: bool,
}

impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        let island = use_consume::<Island>();
        use_side_effect(move || {
            if !*island.expanded.read() {
                island.field.request_unfocus();
            }
        });
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(20.)
            .spacing(10.)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .main_align(Alignment::SpaceBetween)
                    .child(label().text("Now Playing").font_size(13.).color(SECONDARY))
                    .child(Clock {
                        seconds: false,
                        size: 13.,
                        weight: FontWeight::NORMAL,
                        color: SECONDARY,
                    }),
            )
            .child(
                label()
                    .text("Midnight City")
                    .font_size(20.)
                    .font_weight(FontWeight::SEMI_BOLD),
            )
            .child(
                label()
                    .text("M83 · Hurry Up, We're Dreaming")
                    .font_size(14.)
                    .color(SECONDARY),
            )
            .child(Progress {
                running: self.expanded,
            })
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .spacing(12.)
                    .child(label().text("Volume").font_size(13.))
                    .child(Slider {
                        value: island.volume,
                    }),
            )
            .child(TextField {
                text: island.query,
                id: island.field,
                placeholder: "Search…",
                height: 36.,
                radius: 10.,
                padding: 12.,
                font_size: 14.,
            })
    }
}

#[derive(PartialEq)]
struct Progress {
    running: bool,
}

impl Component for Progress {
    fn render(&self) -> impl IntoElement {
        let progress = use_animation_with_dependencies(&self.running, |conf, running| {
            if *running {
                conf.on_change(OnChange::Rerun);
                conf.on_finish(OnFinish::restart());
            } else {
                conf.on_change(OnChange::Reset);
            }
            AnimNum::new(0., 1.).time(8000)
        });
        let time = |text| label().text(text).font_size(12.).color(SECONDARY);
        rect()
            .width(Size::fill())
            .spacing(6.)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(4.))
                    .corner_radius(2.)
                    .background(TRACK)
                    .child(
                        rect()
                            .width(Size::percent(progress.get().value() * 100.))
                            .height(Size::fill())
                            .corner_radius(2.)
                            .background(FILLED),
                    ),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .main_align(Alignment::SpaceBetween)
                    .child(time("1:23"))
                    .child(time("4:03")),
            )
    }
}

#[derive(PartialEq)]
struct Slider {
    value: State<f32>,
}

impl Component for Slider {
    fn render(&self) -> impl IntoElement {
        let mut value = self.value;
        let mut area = use_state(Area::default);
        let mut dragging = use_state(|| false);

        let mut set_from = move |x: f64| {
            let travel = (area.peek().width() - KNOB).max(1.);
            value.set(((x as f32 - KNOB / 2.) / travel).clamp(0., 1.));
        };
        let knob_x = *value.read() * (area.read().width() - KNOB).max(0.);
        let bar = |width: Size, color: Color| {
            rect()
                .position(Position::new_absolute().top((KNOB - 4.) / 2.))
                .width(width)
                .height(Size::px(4.))
                .corner_radius(2.)
                .background(color)
        };

        rect()
            .width(Size::fill())
            .height(Size::px(KNOB))
            .on_sized(move |e: Event<SizedEventData>| area.set_if_modified(e.area))
            .on_pointer_down(move |e: Event<PointerEventData>| {
                dragging.set(true);
                set_from(e.element_location().x);
            })
            .on_capture_global_pointer_move(move |e: Event<PointerEventData>| {
                if *dragging.peek() {
                    set_from(e.global_location().x - area.peek().min_x() as f64);
                }
            })
            .on_global_pointer_up(move |_| dragging.set_if_modified(false))
            .child(bar(Size::percent(100.), TRACK))
            .child(bar(Size::px(knob_x + KNOB / 2.), FILLED))
            .child(
                rect()
                    .position(Position::new_absolute().left(knob_x))
                    .width(Size::px(KNOB))
                    .height(Size::px(KNOB))
                    .corner_radius(KNOB / 2.)
                    .background(Color::WHITE),
            )
    }
}
