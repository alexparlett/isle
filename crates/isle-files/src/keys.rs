//! Moving and typing in a view, shared by List, Icons, Columns and the panels
//! (`p1/structure/files.md` § Keyboard, Moving in a view).

use std::time::{Duration, Instant};

use freya::prelude::*;

/// How a view lays its items out, as the keys move through them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// Items across a row: one in List and Columns.
    pub columns: usize,
    /// Items a page holds.
    pub page: usize,
}

impl Default for Layout {
    fn default() -> Self {
        Self { columns: 1, page: 10 }
    }
}

/// Where a key moves the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
    Left,
    Right,
    First,
    Last,
    PageUp,
    PageDown,
}

impl Move {
    /// The move a key makes; ← and → only where a row has several items.
    pub fn of(key: &Key, modifiers: Modifiers, layout: Layout) -> Option<Self> {
        let option = modifiers.contains(Modifiers::ALT);
        Some(match key {
            Key::Named(NamedKey::ArrowUp) if option => Move::First,
            Key::Named(NamedKey::ArrowDown) if option => Move::Last,
            Key::Named(NamedKey::ArrowUp) => Move::Up,
            Key::Named(NamedKey::ArrowDown) => Move::Down,
            Key::Named(NamedKey::ArrowLeft) if layout.columns > 1 => Move::Left,
            Key::Named(NamedKey::ArrowRight) if layout.columns > 1 => Move::Right,
            Key::Named(NamedKey::Home) => Move::First,
            Key::Named(NamedKey::End) => Move::Last,
            Key::Named(NamedKey::PageUp) => Move::PageUp,
            Key::Named(NamedKey::PageDown) => Move::PageDown,
            _ => return None,
        })
    }
}

/// The place in a view's order of `len` items the cursor moves to from `from`; with no
/// cursor, a move down starts at the first and a move up at the last.
pub fn target(len: usize, from: Option<usize>, by: Move, layout: Layout) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let last = len - 1;
    let Some(at) = from else {
        return Some(match by {
            Move::Up | Move::Left | Move::Last | Move::PageUp => last,
            _ => 0,
        });
    };
    let columns = layout.columns.max(1);
    Some(match by {
        Move::Up => at.checked_sub(columns).unwrap_or(at),
        // Into the next row, its last item when the row is short of this column.
        Move::Down if at / columns < last / columns => (at + columns).min(last),
        Move::Down => at,
        Move::Left => at.saturating_sub(1),
        Move::Right => (at + 1).min(last),
        Move::First => 0,
        Move::Last => last,
        Move::PageUp => at.saturating_sub(layout.page.max(1)),
        Move::PageDown => (at + layout.page.max(1)).min(last),
    })
}

/// Names typed to select by, reset after a second without a key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Typed {
    text: String,
    last: Option<Instant>,
}

const RESET: Duration = Duration::from_secs(1);

impl Typed {
    /// Adds a character; what has been typed so far, lower-cased.
    pub fn push(&mut self, c: &str, now: Instant) -> String {
        if self.last.is_none_or(|t| now.duration_since(t) > RESET) {
            self.text.clear();
        }
        self.text.push_str(&c.to_lowercase());
        self.last = Some(now);
        self.text.clone()
    }

    /// Typing is under way: a Space joins the name rather than opening Quick Look.
    pub fn going(&self, now: Instant) -> bool {
        !self.text.is_empty() && self.last.is_some_and(|t| now.duration_since(t) <= RESET)
    }
}

/// The name `typed` selects: the first starting with it, else the first after it, else the
/// last, as Finder's.
pub fn by_name<'a>(names: impl Iterator<Item = &'a str>, typed: &str) -> Option<usize> {
    let mut after = None;
    let mut count = 0;
    for (i, name) in names.enumerate() {
        count = i + 1;
        let name = name.to_lowercase();
        if name.starts_with(typed) {
            return Some(i);
        }
        if after.is_none() && name.as_str() > typed {
            after = Some(i);
        }
    }
    after.or(count.checked_sub(1))
}

/// A key that moves the cursor or types a name in `tab`'s view; whether it was one.
pub fn act(mut tab: State<crate::tab::Tab>, e: &KeyboardEventData, layout: Layout, mut typed: State<Typed>) -> bool {
    let m = e.modifiers;
    if m.intersects(Modifiers::META | Modifiers::CONTROL) {
        return false;
    }
    let pick = |t: &crate::tab::Tab, to: usize, order: &[usize]| t.folder.entries()[order[to]].path.clone();
    if let Some(by) = Move::of(&e.key, m, layout) {
        let mut t = tab.write();
        let order = crate::view::order(t.folder.entries(), t.shown.group);
        let from = t.cursor.as_deref().and_then(|c| t.folder.position(c)).and_then(|p| order.iter().position(|&i| i == p));
        let Some(to) = target(order.len(), from, by, layout) else { return true };
        let path = pick(&t, to, &order);
        if m.contains(Modifiers::SHIFT) {
            if !t.is_selected(&path) {
                t.selection.push(path.clone());
            }
            t.cursor = Some(path);
        } else {
            t.select_only(path);
        }
        return true;
    }
    let Key::Character(c) = &e.key else { return false };
    let now = Instant::now();
    if m.contains(Modifiers::ALT) || c.chars().any(char::is_control) || (c == " " && !typed.peek().going(now)) {
        return false;
    }
    let text = typed.write().push(c, now);
    let mut t = tab.write();
    let order = crate::view::order(t.folder.entries(), t.shown.group);
    let found = by_name(order.iter().map(|&i| t.folder.entries()[i].name.as_str()), &text);
    if let Some(to) = found {
        let path = pick(&t, to, &order);
        t.select_only(path);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: Layout = Layout { columns: 4, page: 12 };

    #[test]
    fn moves_go_by_rows_in_a_grid_and_stop_at_the_ends() {
        assert_eq!(target(10, Some(5), Move::Up, GRID), Some(1));
        assert_eq!(target(10, Some(1), Move::Up, GRID), Some(1));
        assert_eq!(target(10, Some(5), Move::Down, GRID), Some(9));
        assert_eq!(target(10, Some(7), Move::Down, GRID), Some(9), "a short last row takes the last item");
        assert_eq!(target(10, Some(9), Move::Down, GRID), Some(9));
        assert_eq!(target(10, Some(5), Move::Left, GRID), Some(4));
        assert_eq!(target(10, Some(9), Move::Right, GRID), Some(9));
        assert_eq!(target(30, Some(5), Move::PageDown, GRID), Some(17));
        assert_eq!(target(30, Some(5), Move::PageUp, GRID), Some(0));
        assert_eq!(target(10, None, Move::Down, GRID), Some(0));
        assert_eq!(target(10, None, Move::Up, GRID), Some(9));
        assert_eq!(target(0, None, Move::Down, GRID), None);
    }

    #[test]
    fn typing_selects_by_name_and_resets_after_a_second() {
        let names = ["Applications", "Desktop", "Documents", "Downloads", "notes.md"];
        let mut typed = Typed::default();
        let t0 = Instant::now();
        assert_eq!(typed.push("D", t0), "d");
        assert_eq!(by_name(names.into_iter(), "d"), Some(1));
        assert_eq!(typed.push("o", t0 + Duration::from_millis(300)), "do");
        assert_eq!(typed.push("w", t0 + Duration::from_millis(600)), "dow");
        assert_eq!(by_name(names.into_iter(), "dow"), Some(3));
        assert_eq!(by_name(names.into_iter(), "m"), Some(4), "none starts with it: the next after");
        assert_eq!(by_name(names.into_iter(), "z"), Some(4), "past every name: the last");
        assert_eq!(typed.push("n", t0 + Duration::from_secs(3)), "n", "a pause starts again");
    }
}
