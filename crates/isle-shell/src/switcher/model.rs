//! The switcher's entries, selection, keys and layout (`p1/structure/switcher.md`,
//! `p1/surfaces/switcher.md`), with no drawing and no clock of its own, so they are tested
//! alone.

use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

/// One window in an app's window row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Window {
    pub id: u64,
    pub title: String,
    pub minimised: bool,
}

/// One app: a plate in the row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub app: String,
    pub name: String,
    pub icon: Option<PathBuf>,
    /// Its toplevels in the order Cmd+` walks; empty for a background app.
    pub windows: Vec<Window>,
    /// The window its card previews and a commit raises: its most recent.
    pub recent: Option<u64>,
    pub hidden: bool,
    /// Every window it has is minimised.
    pub minimised: bool,
    pub background: bool,
    pub not_responding: bool,
    pub attention: bool,
    pub urgent: bool,
    pub badge: Option<i64>,
    pub progress: Option<f64>,
    /// Its icon is a game's cover.
    pub cover: bool,
}

/// A key while the chord is held, already read from its keysym.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Tab, or Cmd+Tab pressed again.
    Next,
    /// Shift+Tab.
    Previous,
    Left,
    Right,
    Down,
    Up,
    /// ` and Shift+`.
    Windows {
        forward: bool,
    },
    Quit,
    Hide,
    Close,
    Return,
    Escape,
}

/// What the compositor is to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Bring the app forward with its most recent window.
    Activate(String),
    /// Bring this window forward.
    ActivateWindow(u64),
    /// Start a background app again, which shows its window.
    Relaunch(String),
    Quit(String),
    /// The polite signal to a background app.
    Stop(String),
    Hide(String, bool),
    Close(u64),
}

/// The switcher from the press of its chord to the commit or cancel.
#[derive(Clone, Debug)]
pub struct Switcher {
    entries: Vec<Entry>,
    selected: usize,
    /// The window row: the selected window's place in the selected app's windows.
    row: Option<usize>,
    /// Apps Q was pressed on, kept with a spinner until they have gone.
    quitting: HashSet<String>,
    opened: Instant,
    delay: Duration,
    drawn: bool,
    done: bool,
}

impl Switcher {
    /// Opened by Cmd+Tab (`backwards` for Shift+Cmd+Tab) over `entries` in recency order,
    /// background apps last. `front` is the app with focus, which the selection skips.
    pub fn open(
        entries: Vec<Entry>,
        front: Option<&str>,
        backwards: bool,
        delay: Duration,
        now: Instant,
    ) -> Self {
        let with_windows = entries.iter().filter(|e| !e.background).count();
        let front_first = with_windows > 0 && front == Some(entries[0].app.as_str());
        let selected = if backwards {
            with_windows.saturating_sub(1)
        } else if front_first && with_windows > 1 {
            1
        } else {
            0
        };
        Self {
            entries,
            selected,
            row: None,
            quitting: HashSet::new(),
            opened: now,
            delay,
            drawn: false,
            done: false,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// The selected window's place in the window row, while it is open.
    pub fn row(&self) -> Option<usize> {
        self.row
    }

    pub fn quitting(&self, app: &str) -> bool {
        self.quitting.contains(app)
    }

    /// Whether the panel shows: the delay has passed, or Tab came again inside it.
    pub fn drawn(&self) -> bool {
        self.drawn
    }

    pub fn done(&self) -> bool {
        self.done
    }

    /// When the panel is due to show, while it is not yet.
    pub fn next_change(&self) -> Option<Instant> {
        (!self.drawn && !self.done).then_some(self.opened + self.delay)
    }

    /// Shows the panel once its delay has passed; true when that changed it.
    pub fn tick(&mut self, now: Instant) -> bool {
        if !self.drawn && !self.done && now >= self.opened + self.delay {
            self.drawn = true;
            return true;
        }
        false
    }

    /// The apps changed while the chord is held: the selection follows its app, a quit app
    /// stays until it has gone, and one gone takes the selection to the next.
    pub fn refresh(&mut self, entries: Vec<Entry>) {
        let app = self.selected().map(|e| e.app.clone());
        let window = self.row_window();
        let at = self.selected;
        self.quitting
            .retain(|a| entries.iter().any(|e| &e.app == a));
        self.entries = entries;
        self.selected = match app.and_then(|a| self.entries.iter().position(|e| e.app == a)) {
            Some(i) => i,
            None => at.min(self.entries.len().saturating_sub(1)),
        };
        self.row = match (self.row, window) {
            (Some(_), Some(id)) => {
                let windows = &self.entries.get(self.selected).map(|e| &e.windows);
                windows
                    .and_then(|w| w.iter().position(|w| w.id == id))
                    .or_else(|| windows.filter(|w| !w.is_empty()).map(|_| 0))
            }
            _ => None,
        };
    }

    fn row_window(&self) -> Option<u64> {
        let entry = self.selected()?;
        Some(entry.windows.get(self.row?)?.id)
    }

    fn select(&mut self, index: usize) {
        if index != self.selected {
            self.selected = index;
            self.row = None;
        }
    }

    fn step(&mut self, forward: bool) {
        let n = self.entries.len();
        if n == 0 {
            return;
        }
        let next = if forward {
            (self.selected + 1) % n
        } else {
            (self.selected + n - 1) % n
        };
        self.select(next);
    }

    fn step_window(&mut self, forward: bool) {
        let Some(entry) = self.selected() else {
            return;
        };
        let n = entry.windows.len();
        if n == 0 {
            return;
        }
        let recent = entry
            .recent
            .and_then(|id| entry.windows.iter().position(|w| w.id == id))
            .unwrap_or(0);
        // From the window in front when the row is not open yet, so ` moves at once.
        let at = self.row.unwrap_or(recent);
        self.row = Some(if forward {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        });
    }

    /// A key while the chord is held.
    pub fn key(&mut self, key: Key, now: Instant) -> Vec<Effect> {
        if self.done {
            return Vec::new();
        }
        match key {
            Key::Next | Key::Previous => {
                // A second Tab inside the delay draws at once, so it never selects blind.
                if !self.drawn && now < self.opened + self.delay {
                    self.drawn = true;
                }
                self.step(key == Key::Next);
            }
            Key::Left | Key::Right => match self.row {
                Some(_) => self.step_window(key == Key::Right),
                None => self.step(key == Key::Right),
            },
            Key::Down => {
                if self.row.is_none()
                    && let Some(entry) = self.selected()
                    && !entry.windows.is_empty()
                {
                    let recent = entry
                        .recent
                        .and_then(|id| entry.windows.iter().position(|w| w.id == id))
                        .unwrap_or(0);
                    self.row = Some(recent);
                }
            }
            Key::Up => self.row = None,
            Key::Windows { forward } => self.step_window(forward),
            Key::Quit => {
                let Some(entry) = self.selected() else {
                    return Vec::new();
                };
                let app = entry.app.clone();
                if entry.background {
                    return vec![Effect::Stop(app)];
                }
                self.quitting.insert(app.clone());
                return vec![Effect::Quit(app)];
            }
            Key::Hide => {
                if let Some(entry) = self.selected().filter(|e| !e.background) {
                    return vec![Effect::Hide(entry.app.clone(), !entry.hidden)];
                }
            }
            Key::Close => {
                let id = self.row_window().or_else(|| {
                    let entry = self.selected().filter(|e| !e.background)?;
                    entry.recent.or(entry.windows.first().map(|w| w.id))
                });
                if let Some(id) = id {
                    return vec![Effect::Close(id)];
                }
            }
            Key::Return => return self.commit(),
            Key::Escape => self.done = true,
        }
        Vec::new()
    }

    /// The release of the chord, Return, or a click: the selection comes forward. A quick
    /// tap never commits to a background app.
    pub fn commit(&mut self) -> Vec<Effect> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        if let Some(id) = self.row_window() {
            return vec![Effect::ActivateWindow(id)];
        }
        let Some(entry) = self.selected() else {
            return Vec::new();
        };
        if entry.background {
            return if self.drawn {
                vec![Effect::Relaunch(entry.app.clone())]
            } else {
                Vec::new()
            };
        }
        vec![Effect::Activate(entry.app.clone())]
    }

    /// A click on a plate commits it; a click in the window row commits that window.
    pub fn click(&mut self, index: usize, window: Option<usize>) -> Vec<Effect> {
        if index >= self.entries.len() {
            return Vec::new();
        }
        self.select(index);
        if window.is_some() {
            self.row = window;
        }
        self.drawn = true;
        self.commit()
    }

    /// The pointer moved onto a plate, or a window of the row.
    pub fn hover(&mut self, index: usize, window: Option<usize>) {
        if index >= self.entries.len() {
            return;
        }
        match window {
            Some(w) if index == self.selected && self.row.is_some() => self.row = Some(w),
            _ => self.select(index),
        }
    }

    pub fn cancel(&mut self) {
        self.done = true;
    }
}

/// The sizes the layout reads, in points; `tokens.kdl`'s values by default. An app's plate
/// holds its icon inset by `space_s`; a window's plate in the window row holds its preview's
/// card inset the same way, and its title under it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// A plate's width at full size and its floor.
    pub plate_width: f32,
    pub plate_min_width: f32,
    /// The card's top inset and height together at full size: the card is this less
    /// `space_s` twice.
    pub card_height: f32,
    /// A background app's plate at full size.
    pub background_width: f32,
    pub space_xs: f32,
    pub space_s: f32,
    /// Each side of the divider before the background group.
    pub divider_margin: f32,
    /// The icon's half below the card, and the room under the icon.
    pub icon_overhang: f32,
    pub icon_drop: f32,
    /// The selected app's name line, and a window title's in the window row.
    pub name: f32,
    pub title: f32,
    /// The window row's plate width and card height at full size, as `plate_width` and
    /// `card_height` are the app row's.
    pub row_plate_width: f32,
    pub row_card_height: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            plate_width: 144.0,
            plate_min_width: 80.0,
            card_height: 144.0,
            background_width: 144.0,
            space_xs: 4.0,
            space_s: 8.0,
            divider_margin: 12.0,
            icon_overhang: 0.0,
            icon_drop: 8.0,
            name: 18.0,
            title: 16.0,
            row_plate_width: 256.0,
            row_card_height: 160.0,
        }
    }
}

impl Metrics {
    /// The card inside a plate `plate` points wide.
    fn card(&self, plate: f32) -> (f32, f32) {
        self.card_of(plate, self.plate_width, self.card_height)
    }

    /// The card inside a plate `plate` wide, of a row whose plates are `full_plate` wide
    /// with cards `full_card` tall less their insets at full size.
    fn card_of(&self, plate: f32, full_plate: f32, full_card: f32) -> (f32, f32) {
        let full_w = full_plate - 2.0 * self.space_s;
        let full_h = full_card - 2.0 * self.space_s;
        let w = plate - 2.0 * self.space_s;
        (w, (w * full_h / full_w).round())
    }

    fn plate_height(&self, card_h: f32) -> f32 {
        self.space_s + card_h + self.icon_overhang + self.icon_drop
    }
}

/// A box in the panel, from its top left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Where everything goes, for the entries as they are now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    /// The panel without its window row: where it starts, so the row grows it downward.
    pub rows_height: f32,
    /// The card inside each app's plate, inset `space_s` from its top and sides.
    pub card_width: f32,
    pub card_height: f32,
    /// Each entry's plate, in entry order.
    pub plates: Vec<Rect>,
    /// The name line under each row of plates.
    pub bands: Vec<Rect>,
    /// The hairline between the apps and the background group, when they share a row.
    pub divider: Option<Rect>,
    /// The rule above the window row, and its plates, while it is open; each window plate
    /// holds a card of `window_card` and its title under it.
    pub rule: Option<Rect>,
    pub windows: Vec<Rect>,
    pub window_card: (f32, f32),
}

/// Lays out `entries` for an output `screen` points wide, with the window row open
/// (`row_of`) holding `windows` plates.
pub fn layout(
    entries: &[Entry],
    screen: f32,
    m: &Metrics,
    row_of: Option<usize>,
    windows: usize,
) -> Layout {
    let max = (screen * 0.75).floor();
    let apps = entries.iter().filter(|e| !e.background).count();
    let background = entries.len() - apps;
    let gap = m.space_s;

    // Plates shrink together, keeping their cards' aspect, until the apps and the
    // background group fit in one row; past the floor the apps wrap into rows of equal
    // count.
    let bg_of = |plate: f32| (m.background_width * plate / m.plate_width).round();
    let row_width = |n: usize, nb: usize, plate: f32| {
        let plates = n as f32 * plate + nb as f32 * bg_of(plate);
        let gaps = (n + nb).saturating_sub(1) as f32 * gap;
        let divider = if n > 0 && nb > 0 {
            2.0 * m.divider_margin + 1.0 + gap
        } else {
            0.0
        };
        2.0 * m.space_s + plates + gaps + divider
    };
    let mut plate = m.plate_width;
    if row_width(apps, background, plate) > max {
        let fixed = row_width(apps, background, 0.0);
        let per = apps as f32 + background as f32 * m.background_width / m.plate_width;
        plate = ((max - fixed) / per)
            .floor()
            .clamp(m.plate_min_width, m.plate_width);
    }
    let (card_w, card_h) = m.card(plate);
    let bg_w = bg_of(plate);
    let plate_h = m.plate_height(card_h);

    // Rows: apps at the floor wrap into rows of equal count; the background group joins
    // the last row when it fits, else takes its own.
    let per_row = {
        let mut n = apps.max(1);
        while n > 1 && row_width(n, 0, plate) > max {
            n -= 1;
        }
        n
    };
    let row_count = apps.div_ceil(per_row).max(usize::from(apps > 0));
    let equal = if row_count > 0 {
        apps.div_ceil(row_count)
    } else {
        0
    };
    let mut rows: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
    let mut index = 0;
    for r in 0..row_count {
        let left = apps - index;
        let n = if r + 1 == row_count {
            left
        } else {
            equal.min(left)
        };
        rows.push(((index..index + n).collect(), Vec::new()));
        index += n;
    }
    if background > 0 {
        let group: Vec<usize> = (apps..entries.len()).collect();
        match rows.last_mut() {
            Some((last, bg)) if row_width(last.len(), background, plate) <= max => *bg = group,
            _ => rows.push((Vec::new(), group)),
        }
    }

    let widths: Vec<f32> = rows
        .iter()
        .map(|(a, b)| row_width(a.len(), b.len(), plate))
        .collect();
    let mut width = widths.iter().cloned().fold(0.0, f32::max);
    let mut plates = vec![Rect::default(); entries.len()];
    let mut bands = Vec::new();
    let mut divider = None;
    let mut y = m.space_s;
    for ((apps_in, bg_in), w) in rows.iter().zip(&widths) {
        let mut x = (width - w) / 2.0 + m.space_s;
        for (k, &i) in apps_in.iter().enumerate() {
            if k > 0 {
                x += gap;
            }
            plates[i] = Rect {
                x,
                y,
                w: plate,
                h: plate_h,
            };
            x += plate;
        }
        if !apps_in.is_empty() && !bg_in.is_empty() {
            divider = Some(Rect {
                x: x + gap + m.divider_margin,
                y: y + m.space_s,
                w: 1.0,
                h: card_h,
            });
            x += gap + 2.0 * m.divider_margin + 1.0;
        }
        for (k, &i) in bg_in.iter().enumerate() {
            if k > 0 || !apps_in.is_empty() {
                x += gap;
            }
            plates[i] = Rect {
                x,
                y,
                w: bg_w,
                h: plate_h,
            };
            x += bg_w;
        }
        // The name sits just under the plates, the icon's room above it shared.
        y += plate_h + m.space_s - m.space_xs;
        bands.push(Rect {
            x: m.space_s,
            y,
            w: width - 2.0 * m.space_s,
            h: m.name,
        });
        y += m.name;
    }
    let rows_height = y + m.space_s;
    let mut height = rows_height;

    let mut rule = None;
    let mut window_plates = Vec::new();
    let mut window_card = (0.0, 0.0);
    if row_of.is_some() && windows > 0 {
        // The window row, centred under a rule; it shrinks on its own count, and the panel
        // widens about its centre if the row needs it.
        let row_w = |n: usize, p: f32| n as f32 * p + n.saturating_sub(1) as f32 * gap;
        let mut wplate = m.row_plate_width;
        if row_w(windows, wplate) + 2.0 * m.space_s > max {
            let room = max - 2.0 * m.space_s - windows.saturating_sub(1) as f32 * gap;
            wplate = (room / windows as f32).floor().max(m.plate_min_width);
        }
        let (wc_w, wc_h) = m.card_of(wplate, m.row_plate_width, m.row_card_height);
        window_card = (wc_w, wc_h);
        let total = row_w(windows, wplate);
        if total + 2.0 * m.space_s > width {
            let grow = total + 2.0 * m.space_s - width;
            width += grow;
            for p in plates.iter_mut() {
                p.x += grow / 2.0;
            }
            if let Some(d) = divider.as_mut() {
                d.x += grow / 2.0;
            }
            for b in bands.iter_mut() {
                b.w = width - 2.0 * m.space_s;
            }
        }
        let rule_y = rows_height - m.space_s + m.space_xs;
        rule = Some(Rect {
            x: m.space_s,
            y: rule_y,
            w: width - 2.0 * m.space_s,
            h: 1.0,
        });
        let top = rule_y + 1.0 + gap;
        let start = (width - total) / 2.0;
        let wplate_h = m.space_s + wc_h + m.space_xs + m.title + m.space_s;
        window_plates = (0..windows)
            .map(|k| Rect {
                x: start + k as f32 * (wplate + gap),
                y: top,
                w: wplate,
                h: wplate_h,
            })
            .collect();
        height = top + wplate_h + m.space_s;
    }

    Layout {
        width,
        height,
        rows_height,
        card_width: card_w,
        card_height: card_h,
        plates,
        bands,
        divider,
        rule,
        windows: window_plates,
        window_card,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, windows: u64) -> Entry {
        Entry {
            app: name.into(),
            name: name.into(),
            windows: (1..=windows)
                .map(|i| Window {
                    id: i + 10 * name.len() as u64,
                    title: format!("{name} {i}"),
                    minimised: false,
                })
                .collect(),
            recent: (windows > 0).then(|| 1 + 10 * name.len() as u64),
            ..Default::default()
        }
    }

    fn background(name: &str) -> Entry {
        Entry {
            background: true,
            ..app(name, 0)
        }
    }

    fn apps(n: usize) -> Vec<Entry> {
        (0..n).map(|i| app(&format!("app{i:02}"), 1)).collect()
    }

    const DELAY: Duration = Duration::from_millis(150);

    #[test]
    fn a_quick_tap_commits_to_the_previous_app_with_nothing_drawn() {
        let t0 = Instant::now();
        let mut s = Switcher::open(apps(3), Some("app00"), false, DELAY, t0);
        assert_eq!(s.next_change(), Some(t0 + DELAY));
        assert!(!s.tick(t0 + Duration::from_millis(100)));
        assert_eq!(s.commit(), vec![Effect::Activate("app01".into())]);
        assert!(!s.drawn());
        assert!(s.done());
        assert!(s.commit().is_empty(), "one commit per press");
    }

    #[test]
    fn held_past_the_delay_it_draws_and_a_second_tab_inside_it_draws_at_once() {
        let t0 = Instant::now();
        let mut s = Switcher::open(apps(4), Some("app00"), false, DELAY, t0);
        assert!(s.tick(t0 + DELAY));
        assert!(s.drawn());

        let mut s = Switcher::open(apps(4), Some("app00"), false, DELAY, t0);
        s.key(Key::Next, t0 + Duration::from_millis(40));
        assert!(s.drawn());
        assert_eq!(s.selected().unwrap().app, "app02");
    }

    #[test]
    fn shift_cmd_tab_opens_on_the_last_app_with_a_window_and_selection_wraps() {
        let t0 = Instant::now();
        let mut entries = apps(3);
        entries.push(background("steam"));
        let mut s = Switcher::open(entries, Some("app00"), true, DELAY, t0);
        assert_eq!(s.selected().unwrap().app, "app02");
        s.key(Key::Next, t0);
        assert_eq!(s.selected().unwrap().app, "steam");
        s.key(Key::Next, t0);
        assert_eq!(s.selected().unwrap().app, "app00");
        s.key(Key::Previous, t0);
        assert_eq!(s.selected().unwrap().app, "steam");
    }

    #[test]
    fn with_no_window_in_front_the_first_app_is_selected() {
        let t0 = Instant::now();
        let s = Switcher::open(apps(3), None, false, DELAY, t0);
        assert_eq!(s.selected().unwrap().app, "app00");
        let s = Switcher::open(apps(1), Some("app00"), false, DELAY, t0);
        assert_eq!(s.selected().unwrap().app, "app00");
    }

    #[test]
    fn a_quick_tap_never_commits_to_a_background_app_but_a_drawn_one_relaunches() {
        let t0 = Instant::now();
        let mut s = Switcher::open(vec![background("steam")], None, false, DELAY, t0);
        assert!(s.commit().is_empty());
        let mut s = Switcher::open(vec![background("steam")], None, false, DELAY, t0);
        s.tick(t0 + DELAY);
        assert_eq!(s.commit(), vec![Effect::Relaunch("steam".into())]);
    }

    #[test]
    fn q_quits_and_stops_h_hides_w_closes_the_front_window_or_the_rows() {
        let t0 = Instant::now();
        let mut entries = vec![app("front", 1), app("kitty", 3), background("steam")];
        entries[1].hidden = true;
        let mut s = Switcher::open(entries, Some("front"), false, DELAY, t0);
        assert_eq!(
            s.key(Key::Hide, t0),
            vec![Effect::Hide("kitty".into(), false)]
        );
        let recent = s.selected().unwrap().recent.unwrap();
        assert_eq!(s.key(Key::Close, t0), vec![Effect::Close(recent)]);
        s.key(Key::Windows { forward: true }, t0);
        let second = s.selected().unwrap().windows[1].id;
        assert_eq!(s.row(), Some(1));
        assert_eq!(s.key(Key::Close, t0), vec![Effect::Close(second)]);
        assert_eq!(s.key(Key::Quit, t0), vec![Effect::Quit("kitty".into())]);
        assert!(s.quitting("kitty"));
        s.key(Key::Next, t0);
        assert!(
            s.key(Key::Hide, t0).is_empty(),
            "nothing to hide in the background"
        );
        assert!(s.key(Key::Close, t0).is_empty());
        assert_eq!(s.key(Key::Quit, t0), vec![Effect::Stop("steam".into())]);
    }

    #[test]
    fn the_window_row_opens_on_the_recent_window_and_commits_the_chosen_one() {
        let t0 = Instant::now();
        let mut s = Switcher::open(
            vec![app("front", 1), app("kitty", 3)],
            Some("front"),
            false,
            DELAY,
            t0,
        );
        s.key(Key::Down, t0);
        assert_eq!(s.row(), Some(0));
        s.key(Key::Right, t0);
        s.key(Key::Right, t0);
        assert_eq!(s.row(), Some(2));
        s.key(Key::Right, t0);
        assert_eq!(s.row(), Some(0), "wraps");
        s.key(Key::Windows { forward: false }, t0);
        let last = s.selected().unwrap().windows[2].id;
        s.key(Key::Up, t0);
        assert_eq!(s.row(), None);
        s.key(Key::Windows { forward: false }, t0);
        assert_eq!(s.commit(), vec![Effect::ActivateWindow(last)]);
    }

    #[test]
    fn escape_cancels_and_nothing_follows() {
        let t0 = Instant::now();
        let mut s = Switcher::open(apps(2), Some("app00"), false, DELAY, t0);
        s.key(Key::Escape, t0);
        assert!(s.done());
        assert!(s.commit().is_empty());
        assert!(s.key(Key::Quit, t0).is_empty());
    }

    #[test]
    fn the_selection_follows_its_app_as_the_apps_change() {
        let t0 = Instant::now();
        let mut s = Switcher::open(apps(4), Some("app00"), false, DELAY, t0);
        s.key(Key::Next, t0);
        assert_eq!(s.selected().unwrap().app, "app02");
        s.key(Key::Quit, t0);
        // An app before it went; a new one joined at the end.
        let mut next = apps(4);
        next.remove(1);
        next.push(app("new", 1));
        s.refresh(next.clone());
        assert_eq!(s.selected().unwrap().app, "app02");
        assert!(s.quitting("app02"));
        // The quit app went: the selection moves to the next, its mark goes.
        next.retain(|e| e.app != "app02");
        s.refresh(next);
        assert_eq!(s.selected().unwrap().app, "app03");
        assert!(!s.quitting("app02"));
    }

    #[test]
    fn hover_selects_and_a_click_commits() {
        let t0 = Instant::now();
        let mut s = Switcher::open(apps(4), Some("app00"), false, DELAY, t0);
        s.hover(3, None);
        assert_eq!(s.selected().unwrap().app, "app03");
        assert_eq!(s.click(2, None), vec![Effect::Activate("app02".into())]);
    }

    #[test]
    fn six_apps_make_a_row_of_icons() {
        let l = layout(&apps(6), 3440.0, &Metrics::default(), None, 0);
        // 8 + 6 × 144 + 5 × 8 + 8 wide; a plate is a 128-point icon inside 8; the name 4 under.
        assert_eq!(l.width, 920.0);
        assert_eq!(
            l.plates[0],
            Rect {
                x: 8.0,
                y: 8.0,
                w: 144.0,
                h: 144.0
            }
        );
        assert_eq!(l.plates[1].x, 160.0);
        assert_eq!(l.bands[0].y, 156.0);
        assert_eq!(l.height, 156.0 + 18.0 + 8.0);
    }

    #[test]
    fn many_apps_shrink_to_the_floor_then_wrap_into_rows_of_equal_count() {
        let m = Metrics::default();
        // Sixteen fit the ultrawide's three quarters at full size; the seventeenth shrinks them.
        let l = layout(&apps(16), 3440.0, &m, None, 0);
        assert_eq!(l.plates[0].w, 144.0);
        let l = layout(&apps(17), 3440.0, &m, None, 0);
        let w = l.plates[0].w;
        assert!((80.0..144.0).contains(&w), "{w}");
        assert!(l.width <= 2580.0);
        assert_eq!(l.bands.len(), 1);

        // At the floor, (2580 - 16 + 8) / (80 + 8) is twenty-nine a row.
        let l = layout(&apps(29), 3440.0, &m, None, 0);
        assert_eq!(l.bands.len(), 1);
        let l = layout(&apps(30), 3440.0, &m, None, 0);
        assert_eq!(l.bands.len(), 2);
        let first_row = l.plates.iter().filter(|p| p.y == l.plates[0].y).count();
        assert_eq!(first_row, 15, "thirty as fifteen and fifteen");

        let l = layout(&apps(9), 1920.0, &m, None, 0);
        assert_eq!(l.plates[0].w, 144.0, "nine at full size on the 1920 screen");
        let l = layout(&apps(17), 1920.0, &m, None, 0);
        assert_eq!(l.bands.len(), 2);
        assert!(l.width <= 1440.0);
    }

    #[test]
    fn the_background_group_ends_the_last_row_after_a_divider_or_takes_its_own() {
        let m = Metrics::default();
        let mut entries = apps(3);
        entries.push(background("steam"));
        entries.push(background("sync"));
        let l = layout(&entries, 3440.0, &m, None, 0);
        let divider = l.divider.unwrap();
        assert!(divider.x > l.plates[2].x + l.plates[2].w);
        assert!(l.plates[3].x > divider.x);
        assert_eq!(l.plates[3].w, 144.0);
        assert_eq!(l.plates[3].y, l.plates[0].y);

        let mut entries = apps(29);
        entries.push(background("steam"));
        let l = layout(&entries, 3440.0, &m, None, 0);
        assert!(l.divider.is_none());
        assert!(l.plates[29].y > l.plates[0].y, "its own row");

        let l = layout(&[background("steam")], 3440.0, &m, None, 0);
        assert!(l.divider.is_none());
        assert_eq!(l.plates[0].x, 8.0);
    }

    #[test]
    fn the_window_row_grows_the_panel_downward_under_its_app() {
        let m = Metrics::default();
        let entries = vec![app("a", 1), app("kitty", 3)];
        let closed = layout(&entries, 3440.0, &m, None, 0);
        let open = layout(&entries, 3440.0, &m, Some(1), 3);
        assert_eq!(open.rows_height, closed.height);
        assert!(open.height > closed.height);
        assert_eq!(open.windows.len(), 3);
        assert_eq!(open.windows[0].w, 256.0);
        assert_eq!(open.window_card, (240.0, 144.0));
        // Wider than the app row: the panel widens about its centre and the row is inside.
        assert!(open.width >= 3.0 * 256.0 + 2.0 * 8.0 + 16.0);
        assert!(open.windows[0].x >= 8.0);
        assert!(open.windows[2].x + open.windows[2].w <= open.width - 8.0 + 0.01);
    }
}
