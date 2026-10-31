//! The squarified treemap (Bruls, Huizing and van Wijk): rows of tiles laid along the shorter
//! side, each row closed when another tile would make its worst aspect ratio worse.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }

    /// `self` placed inside `to` as it sits inside `from`.
    pub fn mapped(&self, from: Rect, to: Rect) -> Rect {
        let sx = to.w / from.w.max(1.);
        let sy = to.h / from.h.max(1.);
        Rect {
            x: to.x + (self.x - from.x) * sx,
            y: to.y + (self.y - from.y) * sy,
            w: self.w * sx,
            h: self.h * sy,
        }
    }

    pub fn lerp(a: Rect, b: Rect, p: f32) -> Rect {
        Rect {
            x: a.x + (b.x - a.x) * p,
            y: a.y + (b.y - a.y) * p,
            w: a.w + (b.w - a.w) * p,
            h: a.h + (b.h - a.h) * p,
        }
    }
}

fn worst(row: &[f64], side: f64) -> f64 {
    let sum: f64 = row.iter().sum();
    let (max, min) = row
        .iter()
        .fold((0f64, f64::MAX), |(a, b), v| (a.max(*v), b.min(*v)));
    let s2 = side * side;
    let sum2 = sum * sum;
    (s2 * max / sum2).max(sum2 / (s2 * min))
}

/// Lays `sizes` (largest first) into `area`, one rectangle each, in order.
pub fn layout(sizes: &[u64], area: Rect) -> Vec<Rect> {
    let total: f64 = sizes.iter().map(|s| *s as f64).sum();
    if total <= 0. || area.w <= 0. || area.h <= 0. {
        return vec![Rect::default(); sizes.len()];
    }
    let scale = f64::from(area.w) * f64::from(area.h) / total;
    let areas: Vec<f64> = sizes.iter().map(|s| *s as f64 * scale).collect();
    let mut out = Vec::with_capacity(sizes.len());
    let (mut x, mut y, mut w, mut h) = (
        f64::from(area.x),
        f64::from(area.y),
        f64::from(area.w),
        f64::from(area.h),
    );
    let mut i = 0;
    while i < areas.len() {
        let side = w.min(h);
        let mut end = i + 1;
        while end < areas.len() && worst(&areas[i..=end], side) <= worst(&areas[i..end], side) {
            end += 1;
        }
        let row = &areas[i..end];
        let sum: f64 = row.iter().sum();
        if w >= h {
            // A column on the left, as tall as the area.
            let cw = if h > 0. { sum / h } else { 0. };
            let mut cy = y;
            for a in row {
                let ch = if cw > 0. { a / cw } else { 0. };
                out.push(Rect {
                    x: x as f32,
                    y: cy as f32,
                    w: cw as f32,
                    h: ch as f32,
                });
                cy += ch;
            }
            x += cw;
            w -= cw;
        } else {
            let rh = if w > 0. { sum / w } else { 0. };
            let mut cx = x;
            for a in row {
                let rw = if rh > 0. { a / rh } else { 0. };
                out.push(Rect {
                    x: cx as f32,
                    y: y as f32,
                    w: rw as f32,
                    h: rh as f32,
                });
                cx += rw;
            }
            y += rh;
            h -= rh;
        }
        i = end;
    }
    out
}

/// How many of `sizes` (largest first) get tiles of their own: the rest share one, so no tile is
/// under `min` on a side. The rest are left as one when only one would be left.
pub fn kept(sizes: &[u64], area: Rect, min: f32) -> usize {
    let fits = |n: usize| {
        let mut shown: Vec<u64> = sizes[..n].to_vec();
        let rest: u64 = sizes[n..].iter().sum();
        if rest > 0 {
            shown.push(rest);
        }
        layout(&shown, area)
            .iter()
            .take(n)
            .all(|r| r.w >= min && r.h >= min)
    };
    // Fewer own tiles leave each more room, so the count that fits is found by halving.
    let (mut lo, mut hi) = (0, sizes.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(mid) { lo = mid } else { hi = mid - 1 }
    }
    if sizes.len() - lo == 1 {
        sizes.len()
    } else {
        lo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0.,
        y: 0.,
        w: 600.,
        h: 400.,
    };

    #[test]
    fn tiles_cover_the_area_in_proportion() {
        let sizes = [600, 600, 400, 400, 300, 200, 100];
        let rects = layout(&sizes, AREA);
        let total: f32 = rects.iter().map(|r| r.w * r.h).sum();
        assert!((total - 240_000.).abs() < 1.);
        let first = rects[0].w * rects[0].h;
        assert!((first / 240_000. - 600. / 2600.).abs() < 0.001);
        for r in &rects {
            assert!(
                r.x >= -0.01 && r.y >= -0.01 && r.x + r.w <= 600.01 && r.y + r.h <= 400.01,
                "{r:?}"
            );
        }
    }

    #[test]
    fn squarified_tiles_are_not_slivers() {
        let sizes: Vec<u64> = (1..=12).rev().map(|n| n * 10).collect();
        for r in layout(&sizes, AREA) {
            let aspect = (r.w / r.h).max(r.h / r.w);
            assert!(aspect < 4., "{r:?}");
        }
    }

    #[test]
    fn tiny_tiles_share_one() {
        let mut sizes = vec![1_000_000u64, 500_000];
        sizes.extend(std::iter::repeat_n(10, 200));
        let n = kept(&sizes, AREA, 8.);
        assert_eq!(n, 2, "the two large keep their own and the rest share one");
        assert_eq!(kept(&[5, 4, 3], AREA, 8.), 3);
    }
}
