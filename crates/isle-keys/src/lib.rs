//! The keyboard's logic with no compositor in it: profiles, keymaps, chords and the engine.

pub mod keymap;
pub mod profile;

pub use xkbcommon::xkb;
