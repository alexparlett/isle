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

/// The places a window tiles to (`compositor.md` Snapping 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    LeftHalf,
    RightHalf,
    LeftThird,
    CentreThird,
    RightThird,
    LeftTwoThirds,
    RightTwoThirds,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// The centred 60% of the width, the work area's full height.
    Centre,
    Fill,
    /// The widest gap beside the other windows on the output.
    Beside,
}

impl Zone {
    pub const ALL: [Zone; 14] = [
        Zone::LeftHalf,
        Zone::RightHalf,
        Zone::LeftThird,
        Zone::CentreThird,
        Zone::RightThird,
        Zone::LeftTwoThirds,
        Zone::RightTwoThirds,
        Zone::TopLeft,
        Zone::TopRight,
        Zone::BottomLeft,
        Zone::BottomRight,
        Zone::Centre,
        Zone::Fill,
        Zone::Beside,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Zone::LeftHalf => "left-half",
            Zone::RightHalf => "right-half",
            Zone::LeftThird => "left-third",
            Zone::CentreThird => "centre-third",
            Zone::RightThird => "right-third",
            Zone::LeftTwoThirds => "left-two-thirds",
            Zone::RightTwoThirds => "right-two-thirds",
            Zone::TopLeft => "top-left",
            Zone::TopRight => "top-right",
            Zone::BottomLeft => "bottom-left",
            Zone::BottomRight => "bottom-right",
            Zone::Centre => "centre",
            Zone::Fill => "fill",
            Zone::Beside => "beside",
        }
    }

    pub fn from_name(name: &str) -> Option<Zone> {
        Zone::ALL.into_iter().find(|z| z.name() == name)
    }

    /// The zone a repeated chord moves on to: halves to two-thirds to thirds, the centre to
    /// the centre third.
    pub fn next_in_cycle(self) -> Zone {
        match self {
            Zone::LeftHalf => Zone::LeftTwoThirds,
            Zone::LeftTwoThirds => Zone::LeftThird,
            Zone::LeftThird => Zone::LeftHalf,
            Zone::RightHalf => Zone::RightTwoThirds,
            Zone::RightTwoThirds => Zone::RightThird,
            Zone::RightThird => Zone::RightHalf,
            Zone::Centre => Zone::CentreThird,
            Zone::CentreThird => Zone::Centre,
            other => other,
        }
    }

    /// The zone's box in `area`, with `margin` around and between tiles. `others` are the
    /// other windows on the output, for `Beside`.
    pub fn rect(self, area: Rect, margin: i32, others: &[Rect]) -> Option<Rect> {
        let inner = inset(area, margin);
        let column = |from: f64, to: f64| span(inner.loc.x, inner.size.w, margin, from, to);
        let row = |from: f64, to: f64| span(inner.loc.y, inner.size.h, margin, from, to);
        let full = (inner.loc.y, inner.size.h);
        let (x, y) = match self {
            Zone::LeftHalf => (column(0.0, 0.5), full),
            Zone::RightHalf => (column(0.5, 1.0), full),
            Zone::LeftThird => (column(0.0, 1.0 / 3.0), full),
            Zone::CentreThird => (column(1.0 / 3.0, 2.0 / 3.0), full),
            Zone::RightThird => (column(2.0 / 3.0, 1.0), full),
            Zone::LeftTwoThirds => (column(0.0, 2.0 / 3.0), full),
            Zone::RightTwoThirds => (column(1.0 / 3.0, 1.0), full),
            Zone::TopLeft => (column(0.0, 0.5), row(0.0, 0.5)),
            Zone::TopRight => (column(0.5, 1.0), row(0.0, 0.5)),
            Zone::BottomLeft => (column(0.0, 0.5), row(0.5, 1.0)),
            Zone::BottomRight => (column(0.5, 1.0), row(0.5, 1.0)),
            Zone::Centre => {
                let w = (inner.size.w as f64 * 0.6).round() as i32;
                ((inner.loc.x + (inner.size.w - w) / 2, w), full)
            }
            Zone::Fill => ((inner.loc.x, inner.size.w), full),
            Zone::Beside => {
                let (start, end) = widest_gap(area, others)?;
                let gap = Rectangle::new(
                    (start, area.loc.y).into(),
                    (end - start, area.size.h).into(),
                );
                let inner = inset(gap, margin);
                return (inner.size.w > 0).then_some(inner);
            }
        };
        Some(Rectangle::new((x.0, y.0).into(), (x.1, y.1).into()))
    }
}

fn inset(rect: Rect, by: i32) -> Rect {
    Rectangle::new(
        (rect.loc.x + by, rect.loc.y + by).into(),
        ((rect.size.w - 2 * by).max(1), (rect.size.h - 2 * by).max(1)).into(),
    )
}

/// The part from `from` to `to` of a length starting at `start`, with `gap` between parts.
fn span(start: i32, length: i32, gap: i32, from: f64, to: f64) -> (i32, i32) {
    let at = |f: f64| start + ((length + gap) as f64 * f).round() as i32;
    let a = at(from);
    let b = if to >= 1.0 {
        start + length
    } else {
        at(to) - gap
    };
    (a, (b - a).max(1))
}

/// The widest stretch of `area`'s width no window in `others` covers, as start and end.
fn widest_gap(area: Rect, others: &[Rect]) -> Option<(i32, i32)> {
    if others.is_empty() {
        return None;
    }
    let mut covered: Vec<(i32, i32)> = others
        .iter()
        .filter(|r| r.overlaps(area))
        .map(|r| {
            (
                r.loc.x.max(area.loc.x),
                (r.loc.x + r.size.w).min(area.loc.x + area.size.w),
            )
        })
        .collect();
    covered.sort();
    let mut best: Option<(i32, i32)> = None;
    let mut cursor = area.loc.x;
    let mut consider = |a: i32, b: i32| {
        if b - a > best.map_or(0, |(x, y)| y - x) {
            best = Some((a, b));
        }
    };
    for (a, b) in covered {
        if a > cursor {
            consider(cursor, a);
        }
        cursor = cursor.max(b);
    }
    consider(cursor, area.loc.x + area.size.w);
    best.filter(|(a, b)| b - a >= MIN_BESIDE)
}

/// A gap narrower than this is not offered as a zone.
const MIN_BESIDE: i32 = 200;

/// How far in from an output's edge a dragged pointer shows a zone, and how far along an
/// edge from a corner the corner's zone reaches.
pub const EDGE_REGION: f64 = 8.0;
pub const CORNER_REGION: f64 = 64.0;

/// The zone a drag released at `pointer` tiles to, on the output whose box is `output`.
/// `open` says which edges lead nowhere, so a pointer can rest against them: left, right,
/// top, bottom. The bottom edge gives none; its corners do.
pub fn region_zone(output: Rect, pointer: Point<f64, Logical>, open: [bool; 4]) -> Option<Zone> {
    let o = output.to_f64();
    let (left, right) = (pointer.x - o.loc.x, o.loc.x + o.size.w - pointer.x);
    let (top, bottom) = (pointer.y - o.loc.y, o.loc.y + o.size.h - pointer.y);
    let at_left = open[0] && left < EDGE_REGION;
    let at_right = open[1] && right <= EDGE_REGION;
    let at_top = open[2] && top < EDGE_REGION;
    let at_bottom = open[3] && bottom <= EDGE_REGION;
    let near_top = top < CORNER_REGION;
    let near_bottom = bottom <= CORNER_REGION;
    let near_left = left < CORNER_REGION;
    let near_right = right <= CORNER_REGION;
    Some(match () {
        _ if (at_left && near_top) || (at_top && near_left) => Zone::TopLeft,
        _ if (at_right && near_top) || (at_top && near_right) => Zone::TopRight,
        _ if (at_left && near_bottom) || (at_bottom && near_left) => Zone::BottomLeft,
        _ if (at_right && near_bottom) || (at_bottom && near_right) => Zone::BottomRight,
        _ if at_top => Zone::Fill,
        _ if at_left => Zone::LeftHalf,
        _ if at_right => Zone::RightHalf,
        _ => return None,
    })
}

/// How close an edge comes to another before a moved window's edge snaps to it.
pub const SNAP: i32 = 12;

/// `rect` moved so an edge within `SNAP` of one of `edges` meets it: x edges, then y edges.
pub fn snap(rect: Rect, xs: &[i32], ys: &[i32]) -> Point<i32, Logical> {
    let nearest = |a: i32, b: i32, lines: &[i32]| -> Option<i32> {
        lines
            .iter()
            .flat_map(|l| [(l - a), (l - b)])
            .filter(|d| d.abs() <= SNAP)
            .min_by_key(|d| d.abs())
    };
    let dx = nearest(rect.loc.x, rect.loc.x + rect.size.w, xs).unwrap_or(0);
    let dy = nearest(rect.loc.y, rect.loc.y + rect.size.h, ys).unwrap_or(0);
    rect.loc + Point::from((dx, dy))
}

/// Where a tiled window dragged away goes: its old size under the pointer, the pointer at the
/// same fraction across it and the same distance below its top.
pub fn unsnapped(
    frame: Rect,
    restore: Size<i32, Logical>,
    pointer: Point<f64, Logical>,
) -> Point<i32, Logical> {
    let across = ((pointer.x - frame.loc.x as f64) / frame.size.w.max(1) as f64).clamp(0.0, 1.0);
    let down = pointer.y - frame.loc.y as f64;
    (
        (pointer.x - across * restore.w as f64).round() as i32,
        (pointer.y - down.min(restore.h as f64 - 1.0)).round() as i32,
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

    fn zones(area: Rect, margin: i32) -> Vec<(Zone, Rect)> {
        Zone::ALL
            .into_iter()
            .filter_map(|z| Some((z, z.rect(area, margin, &[])?)))
            .collect()
    }

    #[test]
    fn zones_divide_the_ultrawide_and_the_1080p_screen() {
        let area = work_area(ultrawide(), ultrawide(), 48);
        let z = |zone: Zone| zone.rect(area, 0, &[]).unwrap();
        assert_eq!(z(Zone::LeftHalf), rect(0, 48, 1720, 1392));
        assert_eq!(z(Zone::RightHalf), rect(1720, 48, 1720, 1392));
        assert_eq!(z(Zone::LeftThird), rect(0, 48, 1147, 1392));
        assert_eq!(z(Zone::CentreThird), rect(1147, 48, 1146, 1392));
        assert_eq!(z(Zone::RightThird), rect(2293, 48, 1147, 1392));
        assert_eq!(z(Zone::LeftTwoThirds), rect(0, 48, 2293, 1392));
        assert_eq!(z(Zone::RightTwoThirds), rect(1147, 48, 2293, 1392));
        assert_eq!(z(Zone::TopLeft), rect(0, 48, 1720, 696));
        assert_eq!(z(Zone::BottomRight), rect(1720, 744, 1720, 696));
        assert_eq!(z(Zone::Centre), rect(688, 48, 2064, 1392));
        assert_eq!(z(Zone::Fill), area);
        assert!(Zone::Beside.rect(area, 0, &[]).is_none());

        let hd = rect(3440, 0, 1920, 1080);
        let area = work_area(hd, hd, 48);
        assert_eq!(
            Zone::RightHalf.rect(area, 0, &[]).unwrap(),
            rect(4400, 48, 960, 1032)
        );
        assert_eq!(
            Zone::Centre.rect(area, 0, &[]).unwrap(),
            rect(3824, 48, 1152, 1032)
        );
    }

    #[test]
    fn with_margins_tiles_keep_them_around_and_between() {
        let area = work_area(ultrawide(), ultrawide(), 48);
        let m = 12;
        for (zone, r) in zones(area, m) {
            assert!(r.loc.x >= m && r.loc.y >= 48 + m, "{zone:?} {r:?}");
            assert!(r.loc.x + r.size.w <= 3440 - m, "{zone:?} {r:?}");
            assert!(r.loc.y + r.size.h <= 1440 - m, "{zone:?} {r:?}");
        }
        let left = Zone::LeftHalf.rect(area, m, &[]).unwrap();
        let right = Zone::RightHalf.rect(area, m, &[]).unwrap();
        assert_eq!(right.loc.x - (left.loc.x + left.size.w), m);
        let third = Zone::LeftThird.rect(area, m, &[]).unwrap();
        let centre = Zone::CentreThird.rect(area, m, &[]).unwrap();
        assert_eq!(centre.loc.x - (third.loc.x + third.size.w), m);
        let top = Zone::TopLeft.rect(area, m, &[]).unwrap();
        let bottom = Zone::BottomLeft.rect(area, m, &[]).unwrap();
        assert_eq!(bottom.loc.y - (top.loc.y + top.size.h), m);
    }

    #[test]
    fn beside_takes_the_widest_gap() {
        let area = work_area(ultrawide(), ultrawide(), 48);
        let others = [rect(0, 100, 1200, 800), rect(2800, 60, 640, 900)];
        assert_eq!(
            Zone::Beside.rect(area, 0, &others).unwrap(),
            rect(1200, 48, 1600, 1392)
        );
        // Nothing wide enough.
        let full = [rect(0, 100, 3400, 800)];
        assert!(Zone::Beside.rect(area, 0, &full).is_none());
    }

    #[test]
    fn repeated_chords_cycle() {
        assert_eq!(Zone::LeftHalf.next_in_cycle(), Zone::LeftTwoThirds);
        assert_eq!(Zone::LeftTwoThirds.next_in_cycle(), Zone::LeftThird);
        assert_eq!(Zone::LeftThird.next_in_cycle(), Zone::LeftHalf);
        assert_eq!(Zone::Centre.next_in_cycle(), Zone::CentreThird);
        assert_eq!(Zone::CentreThird.next_in_cycle(), Zone::Centre);
        for zone in Zone::ALL {
            assert_eq!(Zone::from_name(zone.name()), Some(zone));
        }
    }

    #[test]
    fn edge_regions_give_halves_quarters_and_fill_but_not_the_bottom() {
        let o = ultrawide();
        let open = [true; 4];
        let at = |x: f64, y: f64| region_zone(o, (x, y).into(), open);
        assert_eq!(at(2.0, 700.0), Some(Zone::LeftHalf));
        assert_eq!(at(3439.0, 700.0), Some(Zone::RightHalf));
        assert_eq!(at(1700.0, 1.0), Some(Zone::Fill));
        assert_eq!(at(1700.0, 1439.0), None);
        assert_eq!(at(1.0, 30.0), Some(Zone::TopLeft));
        assert_eq!(at(30.0, 1.0), Some(Zone::TopLeft));
        assert_eq!(at(3439.0, 1400.0), Some(Zone::BottomRight));
        assert_eq!(at(3400.0, 1439.0), Some(Zone::BottomRight));
        assert_eq!(at(1700.0, 700.0), None);
        // An edge shared with another output is passed through, not rested on.
        assert_eq!(
            region_zone(o, (3439.0, 700.0).into(), [true, false, true, true]),
            None
        );
    }

    #[test]
    fn edges_snap_within_reach() {
        let r = rect(105, 300, 400, 300);
        assert_eq!(snap(r, &[100, 900], &[]), Point::from((100, 300)));
        // The right edge at 505 snaps to 512.
        assert_eq!(snap(r, &[512], &[]), Point::from((112, 300)));
        assert_eq!(snap(r, &[130], &[]), Point::from((105, 300)));
        assert_eq!(snap(r, &[], &[48]), Point::from((105, 300)));
        assert_eq!(
            snap(rect(105, 55, 400, 300), &[], &[48]),
            Point::from((105, 48))
        );
    }

    #[test]
    fn a_tile_dragged_away_keeps_the_pointer_where_it_held_it() {
        let frame = rect(0, 48, 1720, 1392);
        let at = unsnapped(frame, (800, 600).into(), (860.0, 60.0).into());
        assert_eq!(at, Point::from((460, 48)));
    }
}
