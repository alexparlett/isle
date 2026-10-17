//! The Freya side shared by the shell and Isle's apps: tokens as context, and primitives.

pub mod button;
mod controls;
pub mod field;
pub mod form;
mod glyph;
pub mod menu;
pub mod search;
pub mod slider;
pub mod text;
mod text_field;

use std::rc::Rc;

use freya::prelude::*;
pub use isle_tokens;
use isle_tokens::{Colour, Theme};

pub use button::{Button, LevelBar};
pub use controls::{Spinner, Switch, scroll_area, scroll_area_controlled};
pub use field::{CheckRow, Field, FieldText, Secret};
pub use glyph::{AppIcon, Glyph, LevelGlyph, glyph_svg};
pub use menu::{Menu, MenuEntry, MenuEvent, MenuItem};
pub use search::SearchField;
pub use slider::Slider;
pub use text_field::TextField;

/// The theme every tree reads; a change re-renders whoever read it.
#[derive(Clone, Copy)]
pub struct ThemeContext(pub State<Rc<Theme>>);

/// The theme in force, subscribing the component to changes.
pub fn use_theme() -> Rc<Theme> {
    use_consume::<ThemeContext>().0.read().clone()
}

/// The theme in force, as `use_theme` but not a hook: for helpers called any number of times
/// in a render.
pub fn theme() -> Rc<Theme> {
    consume_context::<ThemeContext>().0.read().clone()
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

/// How far a shadow list reaches past its shape, in points. A surface sized to a shadowed
/// shape cuts the shadow off at its edge, so it leaves this much room round the shape.
pub fn shadow_reach(list: &[isle_tokens::Shadow]) -> f32 {
    list.iter()
        .filter(|s| !s.inset)
        .map(|s| s.blur + s.spread + s.x.abs().max(s.y.abs()))
        .fold(0.0, f32::max)
}

/// The keyboard focus ring: the accent, outside the shape (DESIGN.md, Components).
pub fn focus_ring(theme: &Theme) -> Border {
    Border::new()
        .fill(colour(theme.tokens.colour_accent))
        .width(theme.tokens.stroke_focus_ring)
        .alignment(BorderAlignment::Outer)
}

/// Where a surface's glass lies, in points from its origin, for the compositor to blur behind.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlassShape {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    /// Glass with no text on it, never raised by the contrast floor (B58).
    pub clear: bool,
    /// How much of the glass's tint is left out, 0 to 1, for a surface that draws its own
    /// fill over the blur, as the island's pill does.
    pub faded: f32,
    /// Drawn over the surface's other glass, as a strip behind text on clear glass is.
    pub front: bool,
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

/// A box's own laid-out area as taking the pointer: the handler goes on the box's `on_sized`,
/// and `clear` takes the area away while the box is not drawn. For a box sized by its content,
/// which a `Hit` inside would grow to its parent's room.
#[derive(Clone)]
pub struct HitArea {
    shapes: Option<HitShapes>,
    id: u64,
}

impl HitArea {
    pub fn on_sized(&self) -> impl Fn(Event<SizedEventData>) + 'static {
        let (shapes, id) = (self.shapes.clone(), self.id);
        move |e: Event<SizedEventData>| {
            if let Some(shapes) = &shapes {
                shapes.shapes.borrow_mut().insert(
                    id,
                    GlassShape {
                        x: e.area.min_x(),
                        y: e.area.min_y(),
                        width: e.area.width(),
                        height: e.area.height(),
                        radius: 0.0,
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn clear(&self) {
        if let Some(shapes) = &self.shapes {
            shapes.shapes.borrow_mut().remove(&self.id);
        }
    }
}

pub fn use_hit_area() -> HitArea {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1 << 32);
    let shapes = use_try_consume::<HitShapes>();
    let id = use_hook(|| NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let area = HitArea { shapes, id };
    use_drop({
        let area = area.clone();
        move || area.clear()
    });
    area
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
                            ..Default::default()
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
    pub clear: bool,
    pub front: bool,
    pub faded: f32,
}

impl Glass {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            clear: false,
            front: false,
            faded: 0.0,
        }
    }

    /// Leaves out `share` of the tint, for a fill drawn over the blur instead.
    pub fn faded(mut self, share: f32) -> Self {
        self.faded = share.clamp(0.0, 1.0);
        self
    }

    /// Glass that carries no text, never darkened for contrast.
    pub fn clear(mut self) -> Self {
        self.clear = true;
        self
    }

    /// Drawn over the surface's other glass.
    pub fn front(mut self) -> Self {
        self.front = true;
        self
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
        let (radius, clear, front, faded) = (self.radius, self.clear, self.front, self.faded);
        // The tint changes without the box changing size, which `on_sized` would not report.
        if let Some(shape) = shapes.shapes.borrow_mut().get_mut(&id) {
            shape.faded = faded;
        }
        let theme = use_theme();
        let tint = if shapes.draw_tint {
            let c = colour(theme.tokens.material_glass);
            c.with_a((c.a() as f32 * (1.0 - faded)).round() as u8)
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
                        clear,
                        front,
                        faded,
                    },
                );
            })
    }
}
