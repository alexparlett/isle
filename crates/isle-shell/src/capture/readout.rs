//! The pick's size readout: "1280 × 720" in a capsule by the selection's trailing foot
//! (`p1/surfaces/capture.md`, The pick).

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Layer, Scope, SurfaceDef};

#[derive(Default)]
pub struct ReadoutSurface {
    text: Rc<RefCell<String>>,
    state: Rc<RefCell<Option<State<String>>>>,
}

impl ReadoutSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts `text` in force; call it in the tree's root scope.
    pub fn set(&self, text: String) {
        *self.text.borrow_mut() = text.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(text);
        }
    }

    /// Keeps `text` for the next tree; safe anywhere.
    pub fn set_model(&self, text: String) {
        *self.text.borrow_mut() = text;
    }

    /// The room round the capsule for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_control).ceil()
    }

    /// The surface's box: wide enough for any size, the capsule at its trailing end.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let (font, ..) = text::metrics(theme, text::Style::Callout);
        let width = 13.0 * font * 0.62 + 2.0 * t.space_s;
        (
            (width + 2.0 * m).ceil() as i32,
            (Self::height(theme) + 2.0 * m).ceil() as i32,
        )
            .into()
    }

    /// The capsule's height: the small control's, or the text's line at larger sizes.
    fn height(theme: &Theme) -> f32 {
        let (_, line, _) = text::metrics(theme, text::Style::Callout);
        theme.tokens.size_control_small.max(line)
    }
}

impl SurfaceDef for ReadoutSurface {
    fn name(&self) -> &'static str {
        "capture-readout"
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
        AppComponent::new(ReadoutApp {
            text: self.text.clone(),
            state: self.state.clone(),
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
struct ReadoutApp {
    text: Rc<RefCell<String>>,
    state: Rc<RefCell<Option<State<String>>>>,
}

impl App for ReadoutApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.text.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        let theme = use_theme();
        let t = &theme.tokens;
        let margin = ReadoutSurface::margin(&theme);
        rect()
            .expanded()
            .padding(margin)
            .horizontal()
            .main_align(Alignment::End)
            .child(
                rect()
                    .height(Size::px(ReadoutSurface::height(&theme)))
                    .padding((0., t.space_s))
                    .main_align(Alignment::Center)
                    .background(colour(t.material_solid))
                    .corner_radius(t.size_control_small / 2.0)
                    .map(Some(shadows(&t.shadow_control)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        state.read().clone(),
                    )),
            )
    }
}
