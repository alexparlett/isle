//! A desktop's arrangement (B8): what the work area is on it, which of its windows are out
//! of view, and how Mission Control groups them. Stacking is every desktop's by default;
//! Stage Manager is the second, chosen per desktop.

use super::geometry::Rect;

pub trait Layout {
    /// The area windows are placed in, from the one stacking gives.
    fn work_area(&self, area: Rect) -> Rect {
        area
    }

    /// The window is parked out of view, though shown on this desktop.
    fn parked(&self, _id: u64) -> bool {
        false
    }

    /// The set the window belongs to, which Mission Control clusters by.
    fn set_of(&self, _id: u64) -> Option<u64> {
        None
    }
}

/// Windows where the user puts them, overlapping: macOS's desktop.
pub struct Stacking;

impl Layout for Stacking {}

/// A desktop's arrangement and its state.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Arrangement {
    #[default]
    Stacking,
    Stage(super::stage::Stage),
}

impl Arrangement {
    pub fn layout(&self) -> &dyn Layout {
        match self {
            Arrangement::Stacking => &Stacking,
            Arrangement::Stage(stage) => stage,
        }
    }

    pub fn is_stage(&self) -> bool {
        matches!(self, Arrangement::Stage(_))
    }
}
