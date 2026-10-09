//! The thumbnail that follows a shot, in the bottom-trailing corner of the screen shot
//! (`p1/surfaces/capture.md`, The thumbnail). The compositor reads its pointer.

use std::{cell::RefCell, rc::Rc};

use freya::{elements::image::ImageHandle, prelude::*};
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{colour, shadows, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Layer, Scope, SurfaceDef};

/// The shot shown, compared by its image.
#[derive(Clone, Default)]
pub struct Shown(pub Option<ImageHandle>);

impl PartialEq for Shown {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct ThumbnailSurface {
    shown: Rc<RefCell<Shown>>,
    state: Rc<RefCell<Option<State<Shown>>>>,
}

impl ThumbnailSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts `image` in force; call it in the tree's root scope.
    pub fn set(&self, image: Option<ImageHandle>) {
        let shown = Shown(image);
        *self.shown.borrow_mut() = shown.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(shown);
        }
    }

    /// Keeps `image` for the next tree; safe anywhere.
    pub fn set_model(&self, image: Option<ImageHandle>) {
        *self.shown.borrow_mut() = Shown(image);
    }

    /// The room round the frame for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The image's size, fitted inside the thumbnail's box keeping its shape.
    pub fn fit(theme: &Theme, w: f32, h: f32) -> (f32, f32) {
        let t = &theme.tokens;
        let f = (t.size_thumbnail_width / w.max(1.0)).min(t.size_thumbnail_height / h.max(1.0));
        ((w * f).round().max(1.0), (h * f).round().max(1.0))
    }

    /// The surface's size for a shot `w` by `h`: the image, its mat and the shadow's room.
    pub fn size_for(theme: &Theme, w: f32, h: f32) -> SSize<i32, Logical> {
        let (iw, ih) = Self::fit(theme, w, h);
        let pad = theme.tokens.space_xs + Self::margin(theme);
        (
            (iw + 2.0 * pad).ceil() as i32,
            (ih + 2.0 * pad).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for ThumbnailSurface {
    fn name(&self) -> &'static str {
        "capture-thumbnail"
    }

    fn layer(&self) -> Layer {
        Layer::Top
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
        AppComponent::new(ThumbnailApp {
            shown: self.shown.clone(),
            state: self.state.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (1.0, 1.0)
    }

    fn fades(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct ThumbnailApp {
    shown: Rc<RefCell<Shown>>,
    state: Rc<RefCell<Option<State<Shown>>>>,
}

impl App for ThumbnailApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.shown.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        let theme = use_theme();
        let t = &theme.tokens;
        let margin = ThumbnailSurface::margin(&theme);
        let shown = state.read().clone();
        let Some(handle) = shown.0 else {
            return rect().into_element();
        };
        let (w, h) = ThumbnailSurface::fit(
            &theme,
            handle.image.width() as f32,
            handle.image.height() as f32,
        );
        rect()
            .expanded()
            .padding(margin)
            .child(
                rect()
                    .padding(t.space_xs)
                    .background(colour(t.colour_window_raised))
                    .corner_radius(t.radius_card)
                    .map(Some(shadows(&t.shadow_panel)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .child(
                        rect()
                            .width(Size::px(w))
                            .height(Size::px(h))
                            .corner_radius(t.radius_control)
                            .overflow(Overflow::Clip)
                            .child(image(handle).width(Size::px(w)).height(Size::px(h))),
                    ),
            )
            .into_element()
    }
}
