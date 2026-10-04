//! The Freya side shared by the shell and Isle's apps: tokens as context, and primitives.

use std::rc::Rc;

use freya::prelude::*;
pub use isle_tokens;
use isle_tokens::{Colour, Theme};

/// The theme every tree reads; a change re-renders whoever read it.
#[derive(Clone, Copy)]
pub struct ThemeContext(pub State<Rc<Theme>>);

/// The theme in force, subscribing the component to changes.
pub fn use_theme() -> Rc<Theme> {
    use_consume::<ThemeContext>().0.read().clone()
}

/// A token colour as Freya draws it.
pub fn colour(c: Colour) -> Color {
    Color::from_argb((c.a * 255.0).round() as u8, c.r, c.g, c.b)
}

/// Where a surface's glass lies, in points from its origin, for the compositor to blur behind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassShape {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
}

/// The glass shapes of one tree, by the id of the [`Glass`] that reported each.
#[derive(Clone, Default)]
pub struct GlassShapes(pub Rc<std::cell::RefCell<std::collections::BTreeMap<u64, GlassShape>>>);

/// Glass behind its parent's content: it fills the parent, draws nothing itself, and reports
/// its box so the compositor blurs what lies behind it and tints it (DESIGN.md, Materials).
#[derive(Clone, PartialEq)]
pub struct Glass {
    pub radius: f32,
}

impl Glass {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Component for Glass {
    fn render(&self) -> impl IntoElement {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let shapes = use_consume::<GlassShapes>();
        let id = use_hook(|| NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        use_drop({
            let shapes = shapes.clone();
            move || {
                shapes.0.borrow_mut().remove(&id);
            }
        });
        let radius = self.radius;
        rect()
            .position(Position::new_absolute())
            .width(Size::fill())
            .height(Size::fill())
            .on_sized(move |e: Event<SizedEventData>| {
                shapes.0.borrow_mut().insert(
                    id,
                    GlassShape {
                        x: e.area.min_x(),
                        y: e.area.min_y(),
                        width: e.area.width(),
                        height: e.area.height(),
                        radius,
                    },
                );
            })
    }
}
