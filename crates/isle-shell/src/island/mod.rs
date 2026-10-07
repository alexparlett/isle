//! The island: v1's pill at the top centre of every screen, with the clock, the status
//! glyphs that are not normal, live activity and the shell's brief acknowledgements
//! (`p1/structure/island.md`, `p1/surfaces/island.md`).

pub mod grid;
pub mod model;
pub mod notice;
pub mod panel;
pub mod rows;
pub mod surface;

pub use surface::{Frame, IslandAction, IslandSurface};
