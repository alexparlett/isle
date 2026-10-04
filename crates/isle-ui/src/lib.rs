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
