//! The app switcher: Cmd+Tab's row of apps (`p1/structure/switcher.md`,
//! `p1/surfaces/switcher.md`).

pub mod model;
pub mod surface;

pub use surface::{Frame, SwitcherAction, SwitcherSurface};
