//! The Freya side shared by the shell and Isle's apps: tokens as context, and primitives.

pub mod button;
mod glyph;
pub mod menu;
pub mod search;
pub mod text;

use std::rc::Rc;

use freya::prelude::*;
pub use isle_tokens;
use isle_tokens::{Colour, Theme};

pub use button::{Button, LevelBar};
pub use glyph::{Glyph, LevelGlyph, glyph_svg};
pub use menu::{Menu, MenuEntry, MenuEvent, MenuItem};
pub use search::SearchField;

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

/// A list of token shadows as Freya draws them.
pub fn shadows(list: &[isle_tokens::Shadow]) -> Vec<freya::prelude::Shadow> {
    list.iter()
        .map(|s| {
            let shadow = freya::prelude::Shadow::new()
                .x(s.x)
                .y(s.y)
                .blur(s.blur)
                .spread(s.spread)
                .color(colour(s.colour));
            if s.inset { shadow.inset() } else { shadow }
        })
        .collect()
}

/// The keyboard focus ring: the accent, outside the shape (DESIGN.md, Components).
pub fn focus_ring(theme: &Theme) -> Border {
    Border::new()
        .fill(colour(theme.tokens.colour_accent))
        .width(theme.tokens.stroke_focus_ring)
        .alignment(BorderAlignment::Outer)
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
pub struct GlassShapes {
    pub shapes: Rc<std::cell::RefCell<std::collections::BTreeMap<u64, GlassShape>>>,
    /// With no compositor to blur behind it, glass draws its tint itself.
    pub draw_tint: bool,
}

/// Where a surface takes the pointer, for one whose box is larger than what it draws: the
/// rest of its box passes the pointer to what lies below.
#[derive(Clone, Default)]
pub struct HitShapes {
    pub shapes: Rc<std::cell::RefCell<std::collections::BTreeMap<u64, GlassShape>>>,
}

impl HitShapes {
    /// Whether `(x, y)`, in points from the surface's origin, falls in a shape.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.shapes
            .borrow()
            .values()
            .any(|s| x >= s.x && x < s.x + s.width && y >= s.y && y < s.y + s.height)
    }
}

/// Marks its parent's box as taking the pointer, for a surface that reports hit shapes.
#[derive(Clone, PartialEq)]
pub struct Hit;

impl Component for Hit {
    fn render(&self) -> impl IntoElement {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let shapes = use_try_consume::<HitShapes>();
        let id = use_hook(|| NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        use_drop({
            let shapes = shapes.clone();
            move || {
                if let Some(shapes) = &shapes {
                    shapes.shapes.borrow_mut().remove(&id);
                }
            }
        });
        rect()
            .position(Position::new_absolute())
            .width(Size::fill())
            .height(Size::fill())
            .on_sized(move |e: Event<SizedEventData>| {
                if let Some(shapes) = &shapes {
                    shapes.shapes.borrow_mut().insert(
                        id,
                        GlassShape {
                            x: e.area.min_x(),
                            y: e.area.min_y(),
                            width: e.area.width(),
                            height: e.area.height(),
                            radius: 0.0,
                        },
                    );
                }
            })
    }
}

/// Glass behind its parent's content: it fills the parent, draws nothing itself, and reports
/// its box so the compositor blurs what lies behind it and tints it (DESIGN.md, Materials).
/// It fills the parent inside its padding, so a padded shape pads a child instead.
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
                shapes.shapes.borrow_mut().remove(&id);
            }
        });
        let radius = self.radius;
        let theme = use_theme();
        let tint = if shapes.draw_tint {
            colour(theme.tokens.material_glass)
        } else {
            Color::TRANSPARENT
        };
        rect()
            .background(tint)
            .corner_radius(radius)
            .position(Position::new_absolute())
            .width(Size::fill())
            .height(Size::fill())
            .on_sized(move |e: Event<SizedEventData>| {
                shapes.shapes.borrow_mut().insert(
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
