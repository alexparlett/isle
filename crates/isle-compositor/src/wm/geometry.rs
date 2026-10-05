//! Where windows go, as plain arithmetic on rectangles in logical points.

use smithay::utils::{Logical, Point, Rectangle, Size};

pub type Rect = Rectangle<i32, Logical>;

/// The part of an output windows may use: what layer-shell leaves free, less the island's
/// strip along the top.
pub fn work_area(output: Rect, free: Rect, strip: i32) -> Rect {
    let top = free.loc.y.max(output.loc.y + strip);
    let bottom = free.loc.y + free.size.h;
    Rectangle::new(
        (free.loc.x, top).into(),
        (free.size.w, (bottom - top).max(1)).into(),
    )
}

/// The size a window that asks for none, or for more than the area, opens at.
pub fn default_size(area: Rect) -> Size<i32, Logical> {
    (area.size.w / 2, area.size.h * 7 / 10).into()
}

pub fn centred(area: Rect, size: Size<i32, Logical>) -> Point<i32, Logical> {
    (
        area.loc.x + (area.size.w - size.w) / 2,
        area.loc.y + (area.size.h - size.h) / 2,
    )
        .into()
}

/// `rect` moved, and shrunk if it must be, so all of it is inside `area`.
pub fn clamp_whole(rect: Rect, area: Rect) -> Rect {
    let w = rect.size.w.min(area.size.w);
    let h = rect.size.h.min(area.size.h);
    let x = rect.loc.x.clamp(area.loc.x, area.loc.x + area.size.w - w);
    let y = rect.loc.y.clamp(area.loc.y, area.loc.y + area.size.h - h);
    Rectangle::new((x, y).into(), (w, h).into())
}

/// Steps `at` down and right while another of the app's windows is already there.
pub fn cascade(
    mut at: Point<i32, Logical>,
    taken: &[Point<i32, Logical>],
    step: i32,
    area: Rect,
    size: Size<i32, Logical>,
) -> Point<i32, Logical> {
    let mut steps = 0;
    while taken.contains(&at) && steps < 32 {
        at += Point::from((step, step));
        if at.x + size.w > area.loc.x + area.size.w || at.y + size.h > area.loc.y + area.size.h {
            at = area.loc;
        }
        steps += 1;
    }
    at
}

/// A dialog of `size` centred over its parent's frame, kept inside `area`.
pub fn over_parent(parent: Rect, size: Size<i32, Logical>, area: Rect) -> Point<i32, Logical> {
    clamp_whole(Rectangle::new(centred(parent, size), size), area).loc
}

/// A modal dialog flush under its parent's bar, centred across it.
pub fn under_bar(parent: Rect, bar: i32, size: Size<i32, Logical>) -> Point<i32, Logical> {
    (
        parent.loc.x + (parent.size.w - size.w) / 2,
        parent.loc.y + bar,
    )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    fn ultrawide() -> Rect {
        rect(0, 0, 3440, 1440)
    }

    #[test]
    fn the_work_area_leaves_the_strip_and_exclusive_zones() {
        assert_eq!(
            work_area(ultrawide(), ultrawide(), 48),
            rect(0, 48, 3440, 1392)
        );
        // A bar taking 100 along the top already clears the strip.
        assert_eq!(
            work_area(ultrawide(), rect(0, 100, 3440, 1340), 48),
            rect(0, 100, 3440, 1340)
        );
        // A second output to the right keeps its own origin.
        let second = rect(3440, 0, 1920, 1080);
        assert_eq!(work_area(second, second, 48), rect(3440, 48, 1920, 1032));
    }

    #[test]
    fn a_box_is_clamped_whole_and_shrunk_only_when_larger() {
        let area = rect(0, 48, 1920, 1032);
        assert_eq!(
            clamp_whole(rect(1800, 20, 400, 300), area),
            rect(1520, 48, 400, 300)
        );
        assert_eq!(
            clamp_whole(rect(-50, 900, 400, 300), area),
            rect(0, 780, 400, 300)
        );
        assert_eq!(
            clamp_whole(rect(10, 10, 4000, 300), area),
            rect(0, 48, 1920, 300)
        );
    }

    #[test]
    fn windows_open_centred_and_cascade() {
        let area = work_area(ultrawide(), ultrawide(), 48);
        let size = default_size(area);
        assert_eq!(size, Size::from((1720, 974)));
        let at = centred(area, size);
        assert_eq!(at, Point::from((860, 257)));
        let next = cascade(at, &[at], 32, area, size);
        assert_eq!(next, Point::from((892, 289)));
        assert_eq!(cascade(at, &[], 32, area, size), at);
    }

    #[test]
    fn dialogs_open_over_their_parent() {
        let area = rect(0, 48, 1920, 1032);
        let parent = rect(100, 100, 800, 600);
        assert_eq!(
            over_parent(parent, (300, 200).into(), area),
            Point::from((350, 300))
        );
        // Kept on the output when the parent is at its edge.
        let edge = rect(-500, 100, 600, 400);
        assert_eq!(over_parent(edge, (300, 200).into(), area).x, 0);
        assert_eq!(
            under_bar(parent, 32, (300, 200).into()),
            Point::from((350, 132))
        );
    }
}
