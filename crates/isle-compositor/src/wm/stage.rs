//! Stage Manager as a layout (B25): sets of windows, the front one on the stage and the rest
//! parked in a column at the leading edge (`compositor.md` Stage Manager).

use super::{geometry::Rect, layout::Layout};

/// One set: an app's windows and any the user joined to it, most recent first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Set {
    pub id: u64,
    pub windows: Vec<u64>,
    /// Staged before, so its arrangement is the user's and it is not centred again.
    pub placed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stage {
    /// Front first.
    pub sets: Vec<Set>,
    /// The column is shown, so the stage keeps its width clear.
    pub column: bool,
    /// The column's width with its margins, kept clear while it shows.
    pub reserve: i32,
    /// Only the front set's most recent window is staged.
    pub one_at_a_time: bool,
}

impl Layout for Stage {
    fn work_area(&self, area: Rect) -> Rect {
        if !self.column {
            return area;
        }
        Rect::new(
            (area.loc.x + self.reserve, area.loc.y).into(),
            ((area.size.w - self.reserve).max(1), area.size.h).into(),
        )
    }

    fn parked(&self, id: u64) -> bool {
        match self.sets.first() {
            Some(front) if self.one_at_a_time => {
                front.windows.first() != Some(&id) && self.set_of(id).is_some()
            }
            Some(front) => !front.windows.contains(&id) && self.set_of(id).is_some(),
            None => false,
        }
    }

    fn set_of(&self, id: u64) -> Option<u64> {
        self.sets
            .iter()
            .find(|s| s.windows.contains(&id))
            .map(|s| s.id)
    }
}
