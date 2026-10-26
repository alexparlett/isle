//! Stage Manager's column of recent sets at the screen's leading edge
//! (`p1/surfaces/mission-control.md`, Stage Manager).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_ui::{AppIcon, Glyph, colour, shadows, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{
    Here,
    surface::{Action, Actions, Layer, Scope, SurfaceDef},
    switcher::surface::PreviewImage,
};

/// One set as the column shows it: its windows' images front first, and its app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub set: u64,
    pub name: String,
    pub icon: Option<String>,
    pub images: Vec<PreviewImage>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub entries: Vec<Entry>,
    pub hover: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageAction {
    Hover(Option<usize>),
    /// A set brought onto the stage.
    Choose(u64),
}

/// An entry's box: the front window and up to two behind fit in it.
pub const ENTRY: (f32, f32) = (160.0, 100.0);
/// From one entry's top to the next's.
pub const PITCH: f32 = 140.0;

#[derive(Default)]
pub struct StageSurface {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl StageSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, output: &str, frame: Frame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
    }

    pub fn set(&self, output: &str, frame: Frame) {
        self.frames
            .borrow_mut()
            .insert(output.to_owned(), frame.clone());
        if let Some(mut state) = self.states.borrow().get(output).copied() {
            state.set_if_modified(frame);
        }
    }
}

impl SurfaceDef for StageSurface {
    fn name(&self) -> &'static str {
        "stage"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    /// The whole output, as Mission Control's: a closed instance with a box of its own
    /// costs the output a frame. The tree places the column inside it.
    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(StageApp {
            frames: self.frames.clone(),
            states: self.states.clone(),
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
struct StageApp {
    frames: Rc<RefCell<HashMap<String, Frame>>>,
    states: Rc<RefCell<HashMap<String, State<Frame>>>>,
}

impl App for StageApp {
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
        Column { frame: state }
    }
}

#[derive(PartialEq)]
struct Column {
    frame: State<Frame>,
}

/// Over the stack's images, which draw in a layer of their own for their clip.
const ABOVE: freya::prelude::Layer = freya::prelude::Layer::Relative(2);

impl Component for Column {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let mut root = rect().expanded();
        for (i, entry) in frame.entries.iter().enumerate() {
            let top = i as f32 * PITCH;
            if top + ENTRY.1 > 2000.0 {
                break;
            }
            let hovered = frame.hover == Some(i);
            let mut stack = rect()
                .position(
                    Position::new_absolute()
                        .left(t.space_l)
                        .top(t.size_strip + t.space_l + top),
                )
                .width(Size::px(ENTRY.0))
                .height(Size::px(ENTRY.1 + 16.0));
            // Up to two windows behind, each `space-xs` up and along.
            for (k, shot) in entry.images.iter().take(3).enumerate().rev() {
                let (iw, ih) = (shot.0.image.width() as f32, shot.0.image.height() as f32);
                let fit = (ENTRY.0 / iw.max(1.0)).min(ENTRY.1 / ih.max(1.0));
                let (w, h) = ((iw * fit).round(), (ih * fit).round());
                let offset = k as f32 * t.space_xs;
                let shadow = if hovered && k == 0 {
                    &t.shadow_window
                } else {
                    &t.shadow_window_inactive
                };
                stack = stack.child(
                    rect()
                        .position(
                            Position::new_absolute()
                                .left((ENTRY.0 - w) / 2.0 + offset)
                                .top((ENTRY.1 - h) / 2.0 - offset),
                        )
                        .width(Size::px(w))
                        .height(Size::px(h))
                        .corner_radius(t.radius_small)
                        .overflow(Overflow::Clip)
                        .map(Some(shadows(shadow)), |el, list| {
                            list.into_iter().fold(el, |el, s| el.shadow(s))
                        })
                        .child(image(shot.0.clone()).width(Size::px(w)).height(Size::px(h))),
                );
            }
            stack = stack.child(
                rect()
                    .layer(ABOVE)
                    .position(
                        Position::new_absolute()
                            .left(ENTRY.0 / 2.0 - 16.0)
                            .top(ENTRY.1 - 16.0),
                    )
                    .child(match &entry.icon {
                        Some(path) => AppIcon::new(path.clone(), 32.0).into_element(),
                        None => {
                            Glyph::new("app-window", 24.0, colour(t.colour_text_2)).into_element()
                        }
                    }),
            );
            let (hover, choose, set) = (actions.clone(), actions.clone(), entry.set);
            root = root.child(
                stack
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(entry.name.clone())
                    .child(isle_ui::Hit)
                    .on_pointer_enter(move |_| {
                        hover.send(Action::Stage(StageAction::Hover(Some(i))))
                    })
                    .on_press(move |_| choose.send(Action::Stage(StageAction::Choose(set)))),
            );
        }
        root
    }
}
