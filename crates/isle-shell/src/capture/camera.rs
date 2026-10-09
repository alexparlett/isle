//! The pointer while a window is picked: a camera (`p1/surfaces/capture.md`, The pick).

use std::rc::Rc;

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{Glyph, colour, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Layer, Scope, SurfaceDef};

#[derive(Default)]
pub struct CameraSurface;

impl CameraSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    /// The room round the glyph for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_control).ceil()
    }

    /// The surface's box, the glyph at its centre.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let side = (theme.tokens.size_capture_camera + 2.0 * Self::margin(theme)).ceil() as i32;
        (side, side).into()
    }
}

impl SurfaceDef for CameraSurface {
    fn name(&self) -> &'static str {
        "capture-camera"
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
        AppComponent::new(CameraApp)
    }

    fn hit_shapes(&self) -> bool {
        true
    }

    fn transitions(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct CameraApp;

impl App for CameraApp {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let size = t.size_capture_camera;
        // A dark copy a point lower keeps the light glyph legible on any window; Freya
        // draws the later of two absolute children underneath.
        rect().expanded().center().child(
            rect()
                .width(Size::px(size))
                .height(Size::px(size))
                .child(
                    rect()
                        .position(Position::new_absolute().left(0.).top(0.))
                        .child(
                            Glyph::new("camera", size, colour(t.colour_capture_line)).fill(true),
                        ),
                )
                .child(
                    rect()
                        .position(Position::new_absolute().left(0.).top(1.))
                        .child(
                            Glyph::new("camera", size, colour(t.colour_capture_line_shadow))
                                .fill(true),
                        ),
                ),
        )
    }
}
