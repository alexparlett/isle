//! The dashboard's grid in cells: where it sits on a screen, the layout, the soft push,
//! resizing within a widget's sizes, and the free place for an added widget
//! (`p1/structure/widgets.md` The grid, Edit mode).

/// A box in whole cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cells {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Cells {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn overlaps(&self, other: &Cells) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The grid on one screen: its cells and where its top left sits, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub cols: i32,
    pub rows: i32,
    pub x: f32,
    pub y: f32,
    pub cell: f32,
    pub gap: f32,
}

/// What the extent is measured from: the tokens it names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub cell: f32,
    pub gap: f32,
    pub side: f32,
    pub top: f32,
    /// From the screen's bottom to the grid's: the controls, their foot and the space over them.
    pub bottom: f32,
}

impl Extent {
    /// Columns as many as fit the width less `side` each side, rows as many as fit between
    /// `top` and `bottom`, the grid centred in both.
    pub fn of(width: f32, height: f32, m: Metrics) -> Extent {
        let pitch = m.cell + m.gap;
        let fit = |room: f32| (((room + m.gap) / pitch).floor() as i32).max(0);
        let room_w = width - 2.0 * m.side;
        let room_h = height - m.top - m.bottom;
        let cols = fit(room_w);
        let rows = fit(room_h);
        let span = |n: i32| (n as f32 * pitch - m.gap).max(0.0);
        Extent {
            cols,
            rows,
            x: m.side + ((room_w - span(cols)) / 2.0).floor(),
            y: m.top + ((room_h - span(rows)) / 2.0).floor(),
            cell: m.cell,
            gap: m.gap,
        }
    }

    pub fn pitch(&self) -> f32 {
        self.cell + self.gap
    }

    /// A box of cells in points on its screen.
    pub fn rect(&self, c: Cells) -> (f32, f32, f32, f32) {
        let p = self.pitch();
        (
            self.x + c.x as f32 * p,
            self.y + c.y as f32 * p,
            c.w as f32 * p - self.gap,
            c.h as f32 * p - self.gap,
        )
    }

    /// The cell under a point, which may lie outside the grid.
    pub fn cell_at(&self, x: f32, y: f32) -> (i32, i32) {
        let p = self.pitch();
        (
            ((x - self.x) / p).floor() as i32,
            ((y - self.y) / p).floor() as i32,
        )
    }

    pub fn holds(&self, c: Cells) -> bool {
        c.x >= 0 && c.y >= 0 && c.x + c.w <= self.cols && c.y + c.h <= self.rows
    }
}

/// A placed widget: its instance, as `clock#1`, and its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub id: String,
    pub at: Cells,
}

impl Placed {
    /// The widget it is an instance of: `clock` for `clock#1`.
    pub fn kind(&self) -> &str {
        kind_of(&self.id)
    }
}

pub fn kind_of(id: &str) -> &str {
    id.split_once('#').map_or(id, |(kind, _)| kind)
}

/// The sizes a widget supports, in cells, the default among them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sizes {
    pub all: Vec<(i32, i32)>,
    pub default: (i32, i32),
}

impl Sizes {
    pub fn new(all: &[(i32, i32)], default: (i32, i32)) -> Self {
        Self {
            all: all.to_vec(),
            default,
        }
    }

    pub fn allows(&self, w: i32, h: i32) -> bool {
        self.all.contains(&(w, h))
    }

    /// The size a resize by `dw` and `dh` cells lands on: the nearest allowed size that
    /// changes in that direction and keeps the other side where it can.
    pub fn step(&self, from: (i32, i32), dw: i32, dh: i32) -> Option<(i32, i32)> {
        self.all
            .iter()
            .copied()
            .filter(|&(w, h)| {
                (dw == 0 || (w - from.0).signum() == dw.signum())
                    && (dh == 0 || (h - from.1).signum() == dh.signum())
                    && (w, h) != from
            })
            .min_by_key(|&(w, h)| {
                let off_w = if dw == 0 { (w - from.0).abs() * 100 } else { (w - from.0).abs() };
                let off_h = if dh == 0 { (h - from.1).abs() * 100 } else { (h - from.1).abs() };
                (off_w + off_h, w * h)
            })
    }

    /// The allowed size nearest a dragged frame of `w` by `h` cells.
    pub fn nearest(&self, w: i32, h: i32) -> (i32, i32) {
        self.all
            .iter()
            .copied()
            .min_by_key(|&(sw, sh)| ((sw - w).abs() + (sh - h).abs(), sw * sh))
            .unwrap_or(self.default)
    }
}

/// The way a push travels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl Dir {
    /// The direction of a move from one place to another, by its larger side; none when it
    /// does not move.
    pub fn of(dx: i32, dy: i32) -> Option<Dir> {
        if dx == 0 && dy == 0 {
            None
        } else if dx.abs() >= dy.abs() {
            Some(if dx > 0 { Dir::Right } else { Dir::Left })
        } else {
            Some(if dy > 0 { Dir::Down } else { Dir::Up })
        }
    }
}

/// One screen's widgets. Nothing overlaps and nothing moves but by an edit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub widgets: Vec<Placed>,
}

impl Layout {
    pub fn get(&self, id: &str) -> Option<&Placed> {
        self.widgets.iter().find(|p| p.id == id)
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.widgets.iter().position(|p| p.id == id)
    }

    /// Whether `at` is free of every widget but `except`.
    pub fn free(&self, at: Cells, except: Option<&str>) -> bool {
        self.widgets
            .iter()
            .filter(|p| Some(p.id.as_str()) != except)
            .all(|p| !p.at.overlaps(&at))
    }

    /// The layout as `settings.kdl` keeps it: `clock#1 11 6 2 2` per widget.
    pub fn to_lines(&self) -> Vec<String> {
        self.widgets
            .iter()
            .map(|p| format!("{} {} {} {} {}", p.id, p.at.x, p.at.y, p.at.w, p.at.h))
            .collect()
    }

    /// Reads `to_lines`' form. A line that does not read is skipped and named in the second
    /// list; a widget that overlaps one before it or lies off the grid is moved to the
    /// nearest free place, or skipped when there is none.
    pub fn from_lines(lines: &[String], extent: &Extent) -> (Layout, Vec<String>) {
        let mut layout = Layout::default();
        let mut skipped = Vec::new();
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let nums: Option<Vec<i32>> =
                parts.get(1..).and_then(|n| n.iter().map(|s| s.parse().ok()).collect());
            let (id, at) = match (parts.first(), nums.as_deref()) {
                (Some(id), Some(&[x, y, w, h])) if w > 0 && h > 0 && layout.get(id).is_none() => {
                    (id.to_string(), Cells::new(x, y, w, h))
                }
                _ => {
                    skipped.push(line.clone());
                    continue;
                }
            };
            if extent.holds(at) && layout.free(at, None) {
                layout.widgets.push(Placed { id, at });
            } else if let Some(at) = layout.nearest_free(at.w, at.h, (at.x, at.y), extent) {
                layout.widgets.push(Placed { id, at });
            } else {
                skipped.push(line.clone());
            }
        }
        (layout, skipped)
    }

    /// The free place of `w` by `h` nearest the cell `to`, by its top left; ties go up, then
    /// left.
    pub fn nearest_free(&self, w: i32, h: i32, to: (i32, i32), extent: &Extent) -> Option<Cells> {
        let mut best: Option<(i64, Cells)> = None;
        for y in 0..=(extent.rows - h) {
            for x in 0..=(extent.cols - w) {
                let at = Cells::new(x, y, w, h);
                if !self.free(at, None) {
                    continue;
                }
                let (dx, dy) = ((x - to.0) as i64, (y - to.1) as i64);
                let d = dx * dx + dy * dy;
                if best.is_none_or(|(b, c)| (d, y, x) < (b, c.y, c.x)) {
                    best = Some((d, at));
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// Where a widget added by a click goes: the free place nearest the top centre.
    pub fn place_for(&self, size: (i32, i32), extent: &Extent) -> Option<Cells> {
        let (w, h) = size;
        let mut best: Option<((i64, i32, i32), Cells)> = None;
        for y in 0..=(extent.rows - h) {
            for x in 0..=(extent.cols - w) {
                let at = Cells::new(x, y, w, h);
                if !self.free(at, None) {
                    continue;
                }
                // Twice the distance from the widget's top centre to the grid's.
                let dx = (2 * x + w - extent.cols) as i64;
                let dy = 2 * y as i64;
                let key = (dx * dx + dy * dy, y, x);
                if best.is_none_or(|(b, _)| key < b) {
                    best = Some((key, at));
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// `id` put at `to`, the widgets it lands on pushed away along `dir` and those they land
    /// on after them. Computed from `self`, the layout before the drag, so dragging back
    /// gives it back. None where a push would leave the grid.
    pub fn push(&self, id: &str, to: Cells, dir: Option<Dir>, extent: &Extent) -> Option<Layout> {
        if !extent.holds(to) {
            return None;
        }
        let mut out = self.clone();
        let moving = match out.index(id) {
            Some(i) => {
                out.widgets[i].at = to;
                i
            }
            None => {
                out.widgets.push(Placed {
                    id: id.to_owned(),
                    at: to,
                });
                out.widgets.len() - 1
            }
        };
        let Some(dir) = dir else {
            return out.free(to, Some(id)).then_some(out);
        };
        let mut queue = vec![moving];
        let mut steps = 0;
        while let Some(i) = queue.pop() {
            steps += 1;
            if steps > 64 * out.widgets.len().max(1) {
                return None;
            }
            let by = out.widgets[i].at;
            for j in 0..out.widgets.len() {
                if j == i || j == moving || !out.widgets[j].at.overlaps(&by) {
                    continue;
                }
                let o = &mut out.widgets[j].at;
                match dir {
                    Dir::Right => o.x = by.x + by.w,
                    Dir::Left => o.x = by.x - o.w,
                    Dir::Down => o.y = by.y + by.h,
                    Dir::Up => o.y = by.y - o.h,
                }
                if !extent.holds(*o) {
                    return None;
                }
                queue.push(j);
            }
        }
        Some(out)
    }

    /// `id` resized to `size`, its top left kept, pushing right or down as it grows.
    pub fn resize(&self, id: &str, size: (i32, i32), extent: &Extent) -> Option<Layout> {
        let at = self.get(id)?.at;
        let to = Cells::new(at.x, at.y, size.0, size.1);
        let dir = if size.0 > at.w {
            Some(Dir::Right)
        } else if size.1 > at.h {
            Some(Dir::Down)
        } else {
            None
        };
        let grown = self.push(id, to, dir, extent)?;
        // Growing both ways pushes along the second side too.
        if size.0 > at.w && size.1 > at.h {
            return grown.push(id, to, Some(Dir::Down), extent);
        }
        Some(grown)
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.widgets.len();
        self.widgets.retain(|p| p.id != id);
        self.widgets.len() != before
    }

    /// The next free instance of `kind`: `clock#3` when `clock#1` and `clock#2` are placed.
    pub fn next_id(&self, kind: &str, taken: impl Fn(&str) -> bool) -> String {
        (1..)
            .map(|n| format!("{kind}#{n}"))
            .find(|id| self.get(id).is_none() && !taken(id))
            .unwrap()
    }
}

/// The default layout (`p1/surfaces/widgets.md` The grid): one block centred on the screen,
/// or where it does not fit, each widget at the free place nearest the top centre.
pub fn default_layout(extent: &Extent) -> Layout {
    const BLOCK: [(&str, Cells); 9] = [
        ("clock#1", Cells::new(0, 0, 2, 2)),
        ("calendar#1", Cells::new(2, 0, 4, 2)),
        ("weather#1", Cells::new(6, 0, 2, 2)),
        ("media#1", Cells::new(8, 0, 4, 1)),
        ("storage#1", Cells::new(8, 1, 4, 1)),
        ("system#1", Cells::new(12, 0, 2, 2)),
        ("devices#1", Cells::new(14, 0, 2, 1)),
        ("notifications#1", Cells::new(14, 1, 2, 1)),
        ("", Cells::new(0, 0, 16, 2)),
    ];
    let (block, parts) = BLOCK.split_last().unwrap();
    let mut layout = Layout::default();
    if extent.cols >= block.1.w && extent.rows >= block.1.h {
        let (ox, oy) = ((extent.cols - block.1.w) / 2, (extent.rows - block.1.h) / 2);
        for (id, at) in parts {
            layout.widgets.push(Placed {
                id: (*id).to_owned(),
                at: Cells::new(at.x + ox, at.y + oy, at.w, at.h),
            });
        }
    } else {
        for (id, at) in parts {
            if let Some(at) = layout.place_for((at.w, at.h), extent) {
                layout.widgets.push(Placed {
                    id: (*id).to_owned(),
                    at,
                });
            }
        }
    }
    layout
}

/// Every change made since edit mode began, to step back and forth through.
#[derive(Clone, Debug, Default)]
pub struct History {
    back: Vec<Layout>,
    forward: Vec<Layout>,
}

impl History {
    /// Records `before`, the layout an edit replaced.
    pub fn did(&mut self, before: Layout) {
        self.back.push(before);
        self.forward.clear();
    }

    pub fn undo(&mut self, now: &Layout) -> Option<Layout> {
        let before = self.back.pop()?;
        self.forward.push(now.clone());
        Some(before)
    }

    pub fn redo(&mut self, now: &Layout) -> Option<Layout> {
        let after = self.forward.pop()?;
        self.back.push(now.clone());
        Some(after)
    }

    pub fn clear(&mut self) {
        self.back.clear();
        self.forward.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> Metrics {
        Metrics {
            cell: 72.0,
            gap: 16.0,
            side: 32.0,
            top: 48.0 + 16.0,
            bottom: 32.0 + 30.0 + 24.0,
        }
    }

    fn ultrawide() -> Extent {
        Extent::of(3440.0, 1440.0, metrics())
    }

    fn small() -> Extent {
        Extent::of(1920.0, 1080.0, metrics())
    }

    fn layout(items: &[(&str, i32, i32, i32, i32)]) -> Layout {
        Layout {
            widgets: items
                .iter()
                .map(|&(id, x, y, w, h)| Placed {
                    id: id.into(),
                    at: Cells::new(x, y, w, h),
                })
                .collect(),
        }
    }

    #[test]
    fn the_grid_fills_each_screen_as_p1_measured_it() {
        let e = ultrawide();
        assert_eq!((e.cols, e.rows, e.x, e.y), (38, 14, 56.0, 101.0));
        assert_eq!(e.rect(Cells::new(37, 13, 1, 1)).0 + 72.0, 3440.0 - 56.0);
        let e = small();
        assert_eq!((e.cols, e.rows), (21, 10));
        assert_eq!(e.rect(Cells::new(0, 0, 21, 10)).2, 1832.0);
        assert_eq!(e.rect(Cells::new(0, 0, 21, 10)).3, 864.0);
    }

    #[test]
    fn the_default_block_is_centred_where_p1_put_it() {
        let l = default_layout(&ultrawide());
        assert_eq!(l.get("clock#1").unwrap().at, Cells::new(11, 6, 2, 2));
        assert_eq!(l.get("notifications#1").unwrap().at, Cells::new(25, 7, 2, 1));
        let (x, y, _, _) = ultrawide().rect(Cells::new(11, 6, 1, 1));
        assert_eq!((x, y), (1024.0, 629.0));
        let l = default_layout(&small());
        assert_eq!(l.get("clock#1").unwrap().at, Cells::new(2, 4, 2, 2));
        let tiny = Extent::of(1280.0, 720.0, metrics());
        let l = default_layout(&tiny);
        assert!(l.widgets.len() >= 6, "a narrow screen still gets what fits");
        assert!(l.widgets.iter().all(|p| tiny.holds(p.at)));
    }

    #[test]
    fn a_drag_pushes_neighbours_on_and_dragging_back_restores_them() {
        let e = ultrawide();
        let before = layout(&[("a", 0, 0, 2, 2), ("b", 2, 0, 2, 2), ("c", 4, 0, 2, 1)]);
        let mid = before.push("a", Cells::new(1, 0, 2, 2), Some(Dir::Right), &e).unwrap();
        assert_eq!(mid.get("b").unwrap().at, Cells::new(3, 0, 2, 2));
        assert_eq!(mid.get("c").unwrap().at, Cells::new(5, 0, 2, 1), "the push cascades");
        let back = before.push("a", Cells::new(0, 0, 2, 2), Some(Dir::Left), &e).unwrap();
        assert_eq!(back, before);
    }

    #[test]
    fn a_push_past_the_edge_is_refused() {
        let e = ultrawide();
        let before = layout(&[("a", 34, 0, 2, 2), ("b", 36, 0, 2, 2)]);
        assert!(before.push("a", Cells::new(35, 0, 2, 2), Some(Dir::Right), &e).is_none());
        assert!(before.push("a", Cells::new(37, 0, 2, 2), Some(Dir::Right), &e).is_none());
        let up = layout(&[("a", 0, 1, 2, 1), ("b", 0, 0, 2, 1)]);
        assert!(up.push("a", Cells::new(0, 0, 2, 1), Some(Dir::Up), &e).is_none());
    }

    #[test]
    fn a_drop_without_a_direction_needs_a_free_place() {
        let e = ultrawide();
        let before = layout(&[("a", 0, 0, 2, 2)]);
        assert!(before.push("new", Cells::new(1, 1, 2, 1), None, &e).is_none());
        let added = before.push("new", Cells::new(2, 0, 2, 1), None, &e).unwrap();
        assert_eq!(added.widgets.len(), 2);
    }

    #[test]
    fn resizing_steps_through_the_sizes_and_pushes() {
        let clock = Sizes::new(&[(2, 1), (2, 2), (4, 2)], (2, 2));
        assert_eq!(clock.step((2, 2), 1, 0), Some((4, 2)));
        assert_eq!(clock.step((2, 2), 0, -1), Some((2, 1)));
        assert_eq!(clock.step((2, 1), 0, 1), Some((2, 2)));
        assert_eq!(clock.step((4, 2), 1, 0), None);
        assert_eq!(clock.step((2, 1), 1, 0), Some((4, 2)), "growing wider may grow taller");
        assert_eq!(clock.nearest(3, 2), (2, 2));

        let e = ultrawide();
        let before = layout(&[("clock#1", 0, 0, 2, 2), ("b", 2, 0, 2, 2)]);
        let grown = before.resize("clock#1", (4, 2), &e).unwrap();
        assert_eq!(grown.get("b").unwrap().at.x, 4);
        let shrunk = grown.resize("clock#1", (2, 1), &e).unwrap();
        assert_eq!(shrunk.get("b").unwrap().at.x, 4, "nothing moves back by itself");
    }

    #[test]
    fn an_added_widget_goes_nearest_the_top_centre() {
        let e = ultrawide();
        let empty = Layout::default();
        assert_eq!(empty.place_for((2, 2), &e), Some(Cells::new(18, 0, 2, 2)));
        let taken = layout(&[("a", 18, 0, 2, 2)]);
        let at = taken.place_for((2, 2), &e).unwrap();
        assert_eq!(at.y, 0);
        assert!(at.x == 16 || at.x == 20);
        let full = layout(&[("a", 0, 0, 38, 14)]);
        assert_eq!(full.place_for((2, 1), &e), None);
    }

    #[test]
    fn a_hand_edited_layout_is_read_without_losing_anything_it_can_keep() {
        let e = small();
        let lines: Vec<String> = [
            "clock#1 0 0 2 2",
            "media#1 1 1 4 1",
            "weather#1 30 0 2 2",
            "nonsense",
            "clock#1 5 5 2 2",
        ]
        .map(String::from)
        .to_vec();
        let (l, skipped) = Layout::from_lines(&lines, &e);
        assert_eq!(skipped, vec!["nonsense".to_owned(), "clock#1 5 5 2 2".to_owned()]);
        assert_eq!(l.widgets.len(), 3);
        assert!(l.widgets.iter().all(|p| e.holds(p.at) && l.free(p.at, Some(&p.id))));
        let (again, _) = Layout::from_lines(&l.to_lines(), &e);
        assert_eq!(again, l);
    }

    #[test]
    fn undo_and_redo_walk_the_edits() {
        let a = layout(&[("a", 0, 0, 2, 2)]);
        let b = layout(&[("a", 2, 0, 2, 2)]);
        let c = layout(&[("a", 4, 0, 2, 2)]);
        let mut h = History::default();
        h.did(a.clone());
        h.did(b.clone());
        assert_eq!(h.undo(&c), Some(b.clone()));
        assert_eq!(h.undo(&b), Some(a.clone()));
        assert_eq!(h.undo(&a), None);
        assert_eq!(h.redo(&a), Some(b.clone()));
        h.did(b.clone());
        assert_eq!(h.redo(&a), None, "a new edit drops what was undone");
    }

    #[test]
    fn instances_are_numbered_from_their_kind() {
        let l = layout(&[("clock#1", 0, 0, 2, 2), ("clock#2", 2, 0, 2, 2)]);
        assert_eq!(l.next_id("clock", |_| false), "clock#3");
        assert_eq!(l.next_id("clock", |id| id == "clock#3"), "clock#4");
        assert_eq!(kind_of("clock#2"), "clock");
    }
}
