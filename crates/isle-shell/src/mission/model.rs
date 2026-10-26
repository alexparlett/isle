//! Mission Control's model: what each screen shows and where, the selection, the keys, the
//! pointer and the drag (`p1/structure/mission-control.md`, `p1/surfaces/mission-control.md`).
//! It knows no compositor: it is fed the screens and answers with effects.

/// A box in points, relative to its screen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Between `self` at 0 and `to` at 1.
    pub fn lerp(&self, to: &Rect, p: f32) -> Rect {
        let l = |a: f32, b: f32| a + (b - a) * p;
        Rect::new(
            l(self.x, to.x),
            l(self.y, to.y),
            l(self.w, to.w),
            l(self.h, to.h),
        )
    }
}

/// A window as Mission Control sees it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Window {
    pub id: u64,
    pub app: String,
    pub title: String,
    /// The app's icon file, when it has one.
    pub icon: Option<String>,
    /// Its frame on its screen, title bar included.
    pub rect: Rect,
    pub minimised: bool,
    /// Its app is hidden.
    pub hidden: bool,
    pub game: bool,
    /// Its smallest width, for Split View's half.
    pub min_width: f32,
    /// The Stage Manager set it belongs to, on a Stage Manager desktop.
    pub set: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeskKind {
    Normal,
    /// One window full screen.
    FullScreen(u64),
    /// Two windows sharing the screen, left then right.
    Split(u64, u64),
}

/// One desktop of a screen's row.
#[derive(Clone, Debug, PartialEq)]
pub struct Desk {
    pub id: u64,
    pub kind: DeskKind,
    /// Its windows, back to front.
    pub windows: Vec<u64>,
    /// Arranged by Stage Manager.
    pub stage: bool,
}

impl Desk {
    pub fn normal(id: u64, windows: Vec<u64>) -> Self {
        Self {
            id,
            kind: DeskKind::Normal,
            windows,
            stage: false,
        }
    }

    pub fn full_screen(&self) -> bool {
        !matches!(self.kind, DeskKind::Normal)
    }
}

/// One screen's row of desktops and every window on it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub output: String,
    pub width: f32,
    pub height: f32,
    pub desks: Vec<Desk>,
    pub current: usize,
    pub windows: Vec<Window>,
    /// The screen with the focused window.
    pub focused: bool,
}

impl Screen {
    pub fn window(&self, id: u64) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    fn desk_of(&self, id: u64) -> Option<usize> {
        self.desks.iter().position(|d| d.windows.contains(&id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    MissionControl,
    /// App Exposé of one app.
    Expose(String),
    /// Split View's chooser: `first` has one half, the candidates are offered in the other.
    Chooser {
        first: u64,
        left: bool,
        candidates: Vec<u64>,
    },
}

/// The gap between Split View's panes, in points.
pub const DIVIDER: f32 = 8.0;

/// Whether `window` fits half of `screen` beside a divider.
pub fn fits_half(screen: &Screen, window: &Window) -> bool {
    window.min_width <= (screen.width - DIVIDER) / 2.0
}

/// Sizes from the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub strip: f32,
    pub miniature: f32,
    pub miniature_min: f32,
    pub label: f32,
    pub tray_tile: (f32, f32),
    pub tray_heading: f32,
    pub header: f32,
    pub space_xs: f32,
    pub space_s: f32,
    pub space_m: f32,
    pub space_l: f32,
    pub space_xxl: f32,
    pub space_xxxl: f32,
    /// Under each tile: the icon's overhang, `space-xs`, the title capsule.
    pub tile_reserve: f32,
    /// Under a cluster: its name capsule and the gap above it.
    pub cluster_reserve: f32,
    pub max_desks: usize,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            strip: 48.0,
            miniature: 112.0,
            miniature_min: 64.0,
            label: 15.0,
            tray_tile: (192.0, 80.0),
            tray_heading: 14.0,
            header: 40.0,
            space_xs: 4.0,
            space_s: 8.0,
            space_m: 12.0,
            space_l: 16.0,
            space_xxl: 24.0,
            space_xxxl: 32.0,
            tile_reserve: 52.0,
            cluster_reserve: 38.0,
            max_desks: 16,
        }
    }
}

/// The settings Mission Control reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub group_by_app: bool,
}

// ---- Layout -------------------------------------------------------------------------------

/// A desktop's place in the bar, or the + tile's.
#[derive(Clone, Debug, PartialEq)]
pub struct BarItem {
    /// The desktop's index in the row; `None` for the + tile.
    pub desk: Option<usize>,
    pub miniature: Rect,
    pub label: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bar {
    pub panel: Rect,
    pub items: Vec<BarItem>,
    pub miniature: (f32, f32),
    /// The row is wider than the panel and scrolls.
    pub overflow: bool,
    pub scroll: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub window: u64,
    pub rect: Rect,
}

/// A cluster's name capsule: its group's key and where its centre sits.
#[derive(Clone, Debug, PartialEq)]
pub struct Cluster {
    pub key: String,
    pub centre_x: f32,
    pub top: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayHead {
    Minimised,
    Hidden,
    /// Windows of the app on another desktop, by its number from 1.
    Desktop(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrayGroup {
    pub head: TrayHead,
    pub heading: Rect,
    /// Each window's slot, and the thumbnail fitted inside it.
    pub items: Vec<(u64, Rect)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tray {
    pub panel: Rect,
    pub groups: Vec<TrayGroup>,
    /// The hairlines between groups.
    pub rules: Vec<Rect>,
}

/// Where everything is on one screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScreenLayout {
    pub bar: Option<Bar>,
    pub header: Option<Rect>,
    pub spread: Rect,
    pub tiles: Vec<Tile>,
    pub clusters: Vec<Cluster>,
    pub tray: Option<Tray>,
    /// App Exposé with nothing to show: where its capsule goes.
    pub empty: Option<Rect>,
}

impl ScreenLayout {
    pub fn tile(&self, window: u64) -> Option<&Tile> {
        self.tiles.iter().find(|t| t.window == window)
    }

    pub fn tray_slot(&self, window: u64) -> Option<Rect> {
        self.tray
            .as_ref()?
            .groups
            .iter()
            .flat_map(|g| &g.items)
            .find(|(id, _)| *id == window)
            .map(|(_, r)| *r)
    }
}

/// A window to lay out: its id and its real frame.
#[derive(Clone, Copy, Debug)]
struct Item {
    id: u64,
    rect: Rect,
}

/// The spread: rows by centre height, then by centre across, every row count tried and the
/// one giving the largest tiles kept; never larger than real, each row leaning toward where
/// its windows really are.
pub fn spread(items: &[(u64, Rect)], area: Rect, reserve: f32, gap: f32) -> Vec<Tile> {
    let items: Vec<Item> = items.iter().map(|&(id, rect)| Item { id, rect }).collect();
    if items.is_empty() || area.w <= 0.0 || area.h <= 0.0 {
        return Vec::new();
    }
    if let [one] = items.as_slice()
        && one.rect.w <= area.w
        && one.rect.h + reserve <= area.h
    {
        // One window that fits stays at its real size and place, kept inside the spread.
        let x = one.rect.x.clamp(area.x, area.right() - one.rect.w);
        let y = one
            .rect
            .y
            .clamp(area.y, area.bottom() - reserve - one.rect.h);
        return vec![Tile {
            window: one.id,
            rect: Rect::new(x, y, one.rect.w, one.rect.h),
        }];
    }
    let mut best: Option<(f32, Vec<Vec<Item>>, f32)> = None;
    for rows in 1..=items.len().min(10) {
        let mut by_y = items.clone();
        by_y.sort_by(|a, b| a.rect.centre().1.total_cmp(&b.rect.centre().1));
        let ideal = items.iter().map(|i| i.rect.w).sum::<f32>() / rows as f32;
        let mut out: Vec<Vec<Item>> = Vec::new();
        let mut row: Vec<Item> = Vec::new();
        let mut width = 0.0;
        for item in by_y {
            if !row.is_empty() && width + item.rect.w / 2.0 > ideal && out.len() < rows - 1 {
                out.push(std::mem::take(&mut row));
                width = 0.0;
            }
            width += item.rect.w;
            row.push(item);
        }
        out.push(row);
        for row in &mut out {
            row.sort_by(|a, b| a.rect.centre().0.total_cmp(&b.rect.centre().0));
        }
        let across = out
            .iter()
            .map(|r| {
                (area.w - gap * (r.len() as f32 - 1.0)) / r.iter().map(|i| i.rect.w).sum::<f32>()
            })
            .fold(f32::INFINITY, f32::min);
        let tall: f32 = out
            .iter()
            .map(|r| r.iter().map(|i| i.rect.h).fold(0.0, f32::max))
            .sum();
        let down = (area.h - gap * (out.len() as f32 - 1.0) - reserve * out.len() as f32) / tall;
        let scale = across.min(down).min(1.0);
        if scale <= 0.0 {
            continue;
        }
        let used = items.iter().map(|i| i.rect.w * i.rect.h).sum::<f32>() * scale * scale
            / (area.w * area.h);
        let score = scale + 0.1 * used;
        if best.as_ref().is_none_or(|(s, _, _)| score > *s) {
            best = Some((score, out, scale));
        }
    }
    let Some((_, rows, scale)) = best else {
        return Vec::new();
    };
    let heights: Vec<f32> = rows
        .iter()
        .map(|r| r.iter().map(|i| i.rect.h).fold(0.0, f32::max) * scale)
        .collect();
    let total =
        heights.iter().sum::<f32>() + reserve * rows.len() as f32 + gap * (rows.len() as f32 - 1.0);
    let mut y = area.y + (area.h - total) / 2.0;
    let mut tiles = Vec::new();
    for (row, height) in rows.iter().zip(&heights) {
        let width =
            row.iter().map(|i| i.rect.w * scale).sum::<f32>() + gap * (row.len() as f32 - 1.0);
        let slack = (area.w - width).max(0.0);
        let real = row.iter().map(|i| i.rect.centre().0).sum::<f32>() / row.len() as f32;
        let lean = ((real - area.centre().0) * 0.5).clamp(-slack / 2.0, slack / 2.0);
        let mut x = area.x + slack / 2.0 + lean;
        for item in row {
            let (w, h) = (item.rect.w * scale, item.rect.h * scale);
            tiles.push(Tile {
                window: item.id,
                rect: Rect::new(x, y + (height - h) / 2.0, w, h),
            });
            x += w + gap;
        }
        y += height + reserve + gap;
    }
    tiles
}

/// Windows clustered by `key`: each cluster's windows in a row, the clusters laid out as
/// windows are, a name capsule under each.
fn clustered(items: &[(u64, Rect, String)], area: Rect, m: &Metrics) -> (Vec<Tile>, Vec<Cluster>) {
    let mut keys: Vec<String> = Vec::new();
    for (_, _, k) in items {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    let inner = m.space_l;
    let groups: Vec<(String, Vec<(u64, Rect)>)> = keys
        .iter()
        .map(|k| {
            let members = items
                .iter()
                .filter(|(_, _, key)| key == k)
                .map(|(id, r, _)| (*id, *r))
                .collect();
            (k.clone(), members)
        })
        .collect();
    // A cluster stands in as one window: its members side by side at full size.
    let stand_ins: Vec<(u64, Rect)> = groups
        .iter()
        .enumerate()
        .map(|(i, (_, ws))| {
            let w = ws.iter().map(|(_, r)| r.w).sum::<f32>() + inner * (ws.len() as f32 - 1.0);
            let h = ws.iter().map(|(_, r)| r.h).fold(0.0, f32::max);
            let cx = ws.iter().map(|(_, r)| r.centre().0).sum::<f32>() / ws.len() as f32;
            let cy = ws.iter().map(|(_, r)| r.centre().1).sum::<f32>() / ws.len() as f32;
            (i as u64, Rect::new(cx - w / 2.0, cy - h / 2.0, w, h))
        })
        .collect();
    let placed = spread(&stand_ins, area, m.cluster_reserve, m.space_xxl * 2.0);
    let mut tiles = Vec::new();
    let mut clusters = Vec::new();
    for slot in placed {
        let (key, members) = &groups[slot.window as usize];
        let stand_in = stand_ins[slot.window as usize].1;
        let k = slot.rect.w / stand_in.w;
        let mut x = slot.rect.x;
        for (id, r) in members {
            let (w, h) = (r.w * k, r.h * k);
            tiles.push(Tile {
                window: *id,
                rect: Rect::new(x, slot.rect.y + (slot.rect.h - h) / 2.0, w, h),
            });
            x += w + inner * k;
        }
        clusters.push(Cluster {
            key: key.clone(),
            centre_x: slot.rect.centre().0,
            top: slot.rect.bottom() + m.space_s,
        });
    }
    (tiles, clusters)
}

/// The Spaces bar: miniatures shrink toward their floor to fit, then the row scrolls.
fn bar(screen: &Screen, m: &Metrics, scroll: f32) -> Bar {
    let n = screen.desks.len();
    let plus = n < m.max_desks;
    let room = screen.width - 2.0 * m.space_xxxl;
    let aspect = screen.width / screen.height.max(1.0);
    let width_at = |mh: f32| {
        let mw = (mh * aspect).round();
        2.0 * m.space_m
            + n as f32 * mw
            + (n as f32 - 1.0).max(0.0) * m.space_l
            + if plus { m.space_l + mh } else { 0.0 }
    };
    let mut mh = m.miniature;
    while mh > m.miniature_min && width_at(mh) > room {
        mh -= 1.0;
    }
    let mw = (mh * aspect).round();
    let overflow = width_at(mh) > room;
    let width = width_at(mh).min(room);
    let height = 2.0 * m.space_m + mh + m.space_xs + m.label;
    let panel = Rect::new((screen.width - width) / 2.0, m.strip, width, height);
    let scroll = if overflow {
        scroll.clamp(0.0, width_at(mh) - width)
    } else {
        0.0
    };
    let mut items = Vec::new();
    let mut x = panel.x + m.space_m - scroll;
    let top = panel.y + m.space_m;
    for i in 0..n {
        items.push(BarItem {
            desk: Some(i),
            miniature: Rect::new(x, top, mw, mh),
            label: Rect::new(x, top + mh + m.space_xs, mw, m.label),
        });
        x += mw + m.space_l;
    }
    if plus {
        items.push(BarItem {
            desk: None,
            miniature: Rect::new(x, top, mh, mh),
            label: Rect::new(x, top + mh + m.space_xs, mh, m.label),
        });
    }
    Bar {
        panel,
        items,
        miniature: (mw, mh),
        overflow,
        scroll,
    }
}

/// The tray or the set-apart row: groups of slots along the foot.
fn tray(groups: Vec<(TrayHead, Vec<&Window>)>, screen: &Screen, m: &Metrics) -> Option<Tray> {
    let groups: Vec<_> = groups
        .into_iter()
        .filter(|(_, ws)| !ws.is_empty())
        .collect();
    if groups.is_empty() {
        return None;
    }
    let (tw, th) = m.tray_tile;
    let content: f32 = groups
        .iter()
        .map(|(_, ws)| ws.len() as f32 * tw + (ws.len() as f32 - 1.0) * m.space_l)
        .sum::<f32>()
        + m.space_xxl * (groups.len() as f32 - 1.0);
    let width = (content + 2.0 * m.space_m).min(screen.width - 2.0 * m.space_xxxl);
    // Room under the slots for the icons' overhang.
    let height = 2.0 * m.space_m + m.tray_heading + m.space_xs + th + m.space_l;
    let panel = Rect::new(
        (screen.width - width) / 2.0,
        screen.height - m.space_l - height,
        width,
        height,
    );
    let mut x = panel.x + m.space_m;
    let heading_y = panel.y + m.space_m;
    let slot_y = heading_y + m.tray_heading + m.space_xs;
    let mut out = Vec::new();
    let mut rules = Vec::new();
    for (i, (head, windows)) in groups.into_iter().enumerate() {
        if i > 0 {
            rules.push(Rect::new(x + m.space_xxl / 2.0 - 0.5, slot_y, 1.0, th));
            x += m.space_xxl;
        }
        let heading = Rect::new(x, heading_y, tw, m.tray_heading);
        let mut items = Vec::new();
        for w in windows {
            items.push((w.id, Rect::new(x, slot_y, tw, th)));
            x += tw + m.space_l;
        }
        x -= m.space_l;
        out.push(TrayGroup {
            head,
            heading,
            items,
        });
    }
    Some(Tray {
        panel,
        groups: out,
        rules,
    })
}

/// `w` by `h` fitted inside `slot`, centred.
pub fn fit_in(w: f32, h: f32, slot: Rect) -> Rect {
    let k = (slot.w / w.max(1.0)).min(slot.h / h.max(1.0));
    let (fw, fh) = (w * k, h * k);
    Rect::new(
        slot.x + (slot.w - fw) / 2.0,
        slot.y + (slot.h - fh) / 2.0,
        fw,
        fh,
    )
}

fn layout_screen(
    screen: &Screen,
    mode: &Mode,
    settings: &Settings,
    m: &Metrics,
    scroll: f32,
) -> ScreenLayout {
    let desk = screen.desks.get(screen.current);
    let shown =
        |w: &&Window| !w.minimised && !w.hidden && desk.is_some_and(|d| d.windows.contains(&w.id));
    let mut out = ScreenLayout::default();
    if let Mode::Chooser {
        first,
        left,
        candidates,
    } = mode
    {
        // Only the screen the first window shares; the others show nothing of it.
        if screen.window(*first).is_none() {
            return out;
        }
        let half = (screen.width - DIVIDER) / 2.0;
        let x = if *left { half + DIVIDER } else { 0.0 };
        let area = Rect::new(
            x + m.space_xxxl,
            m.strip,
            half - 2.0 * m.space_xxxl,
            screen.height - m.strip - m.space_xxxl,
        );
        out.spread = area;
        let items: Vec<(u64, Rect)> = candidates
            .iter()
            .filter_map(|id| screen.window(*id))
            .map(|w| (w.id, w.rect))
            .collect();
        out.tiles = spread(&items, area, m.tile_reserve, m.space_xxl);
        if !candidates
            .iter()
            .filter_map(|id| screen.window(*id))
            .any(|w| fits_half(screen, w))
        {
            out.empty = Some(Rect::new(area.centre().0 - 60.0, area.centre().1 - 12.0, 120.0, 24.0));
        }
        return out;
    }
    let (top, tray_out) = match mode {
        Mode::Chooser { .. } => unreachable!(),
        Mode::MissionControl => {
            let b = bar(screen, m, scroll);
            let top = b.panel.bottom();
            out.bar = Some(b);
            let on_desk = |w: &&Window| desk.is_some_and(|d| d.windows.contains(&w.id));
            let minimised: Vec<&Window> = screen
                .windows
                .iter()
                .filter(|w| w.minimised && on_desk(w))
                .collect();
            let hidden: Vec<&Window> = screen
                .windows
                .iter()
                .filter(|w| w.hidden && !w.minimised && on_desk(w))
                .collect();
            (
                top,
                tray(
                    vec![(TrayHead::Minimised, minimised), (TrayHead::Hidden, hidden)],
                    screen,
                    m,
                ),
            )
        }
        Mode::Expose(app) => {
            let header = Rect::new(
                screen.width / 2.0 - 120.0,
                m.strip + m.space_xxl,
                240.0,
                m.header,
            );
            out.header = Some(header);
            let ours = |w: &&Window| &w.app == app;
            let groups = if screen.focused {
                let mut groups = vec![
                    (
                        TrayHead::Minimised,
                        screen
                            .windows
                            .iter()
                            .filter(|w| ours(w) && w.minimised)
                            .collect(),
                    ),
                    (
                        TrayHead::Hidden,
                        screen
                            .windows
                            .iter()
                            .filter(|w| ours(w) && w.hidden && !w.minimised)
                            .collect(),
                    ),
                ];
                for (i, d) in screen.desks.iter().enumerate() {
                    if i == screen.current {
                        continue;
                    }
                    let elsewhere: Vec<&Window> = screen
                        .windows
                        .iter()
                        .filter(|w| {
                            ours(w) && !w.minimised && !w.hidden && d.windows.contains(&w.id)
                        })
                        .collect();
                    groups.push((TrayHead::Desktop(i + 1), elsewhere));
                }
                groups
            } else {
                Vec::new()
            };
            (header.bottom(), tray(groups, screen, m))
        }
    };
    let bottom = tray_out
        .as_ref()
        .map_or(screen.height - m.space_xxxl, |t| t.panel.y - m.space_xxl);
    out.tray = tray_out;
    let area = Rect::new(
        m.space_xxxl,
        top + m.space_xxl,
        screen.width - 2.0 * m.space_xxxl,
        (bottom - top - m.space_xxl).max(0.0),
    );
    out.spread = area;
    let windows: Vec<&Window> = screen
        .windows
        .iter()
        .filter(shown)
        .filter(|w| match mode {
            Mode::Expose(app) => &w.app == app,
            Mode::MissionControl | Mode::Chooser { .. } => true,
        })
        .collect();
    let stage = desk.is_some_and(|d| d.stage);
    let by_set = matches!(mode, Mode::MissionControl) && stage;
    if by_set || (settings.group_by_app && matches!(mode, Mode::MissionControl)) {
        let items: Vec<(u64, Rect, String)> = windows
            .iter()
            .map(|w| {
                let key = match (by_set, w.set) {
                    (true, Some(set)) => format!("set:{set}"),
                    _ => w.app.clone(),
                };
                (w.id, w.rect, key)
            })
            .collect();
        let (tiles, clusters) = clustered(&items, area, m);
        out.tiles = tiles;
        out.clusters = clusters;
    } else {
        let items: Vec<(u64, Rect)> = windows.iter().map(|w| (w.id, w.rect)).collect();
        out.tiles = spread(&items, area, m.tile_reserve, m.space_xxl);
    }
    if matches!(mode, Mode::Expose(_)) && out.tiles.is_empty() && out.tray.is_none() {
        out.empty = Some(Rect::new(
            area.centre().0 - 60.0,
            area.centre().1 - 12.0,
            120.0,
            24.0,
        ));
    }
    out
}

// ---- The model ---------------------------------------------------------------------------

/// Something Mission Control can select, hover or drop on.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Tile { screen: usize, window: u64 },
    Desk { screen: usize, index: usize },
    Plus { screen: usize },
    Tray { screen: usize, window: u64 },
}

impl Target {
    pub fn screen(&self) -> usize {
        match self {
            Target::Tile { screen, .. }
            | Target::Desk { screen, .. }
            | Target::Plus { screen }
            | Target::Tray { screen, .. } => *screen,
        }
    }
}

/// The pointer's place inside a target that acts on its own: the hover badges.
#[derive(Clone, Debug, PartialEq)]
pub enum Badge {
    /// Close on a tile.
    Close { screen: usize, window: u64 },
    /// Remove a desktop, or exit a full-screen one.
    Remove { screen: usize, index: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Tab,
    BackTab,
    Return,
    Space,
    /// W or ⌘W.
    Close,
    /// Delete or Backspace: closes a window, removes a desktop.
    Delete,
    Escape,
    /// ⌃↑ or the Mission Control key: closes Mission Control, App Exposé goes to it.
    MissionControl,
    /// ⌃↓: App Exposé of the selected window's app.
    Expose,
    /// ⌃← and ⌃→.
    Desktop(i32),
    /// ⌃1…9, from 0.
    Jump(usize),
    /// ⌃⇧← and ⌃⇧→: the selected window or desktop moves.
    Carry(i32),
    /// ⌃⇧1…9, from 0.
    CarryTo(usize),
    /// ⌃⌥Space.
    WindowMenu,
}

/// Where a moved window goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    Desk {
        screen: usize,
        index: usize,
    },
    /// A new desktop at the end of the screen's row.
    New {
        screen: usize,
    },
    /// Split View with a full-screen desktop's window, on the given side.
    Split {
        screen: usize,
        index: usize,
        left: bool,
    },
}

/// What the compositor does for the model.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Close Mission Control, choosing nothing.
    Dismiss,
    /// Close it, bringing the window forward.
    Choose(u64),
    /// Close it into desktop `index` of `screen`.
    Show {
        screen: usize,
        index: usize,
    },
    /// Switch desktops while open.
    Switch {
        screen: usize,
        index: usize,
    },
    CloseWindow(u64),
    Move {
        window: u64,
        to: Place,
    },
    AddDesktop {
        screen: usize,
    },
    RemoveDesktop {
        screen: usize,
        index: usize,
    },
    /// A full-screen desktop's window leaves full screen.
    ExitFullScreen(u64),
    Reorder {
        screen: usize,
        from: usize,
        to: usize,
    },
    /// A minimised or hidden window back on the desktop, chosen.
    Restore(u64),
    /// The selected window at its real size, or back.
    Look(Option<u64>),
    WindowMenu(u64),
    /// The desktop's menu, on a secondary click on a miniature.
    DeskMenu {
        screen: usize,
        index: usize,
    },
    /// App Exposé of an app, from Mission Control or the next app's.
    Expose(String),
    /// Back to Mission Control from App Exposé.
    Overview,
}

/// A drag in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub what: Dragged,
    pub screen: usize,
    /// Where the pointer is, and where in the dragged thing it held.
    pub at: (f32, f32),
    pub grab: (f32, f32),
    /// What it would land on now.
    pub over: Option<Place>,
    /// A drop here is refused.
    pub refused: bool,
    pub started: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dragged {
    Window(u64),
    Desk(usize),
}

/// How far a press moves before it is a drag.
const DRAG_START: f32 = 4.0;

pub struct Mission {
    mode: Mode,
    screens: Vec<Screen>,
    layouts: Vec<ScreenLayout>,
    settings: Settings,
    metrics: Metrics,
    /// Apps most recent first, for Tab in App Exposé.
    apps: Vec<String>,
    selection: Option<Target>,
    /// The selection shows only after the first key (v1 D9).
    keyed: bool,
    hover: Option<Target>,
    badge: Option<Badge>,
    pressed: Option<(Target, (f32, f32), usize, u32)>,
    drag: Option<Drag>,
    looking: Option<u64>,
    scroll: Vec<f32>,
}

/// Mouse buttons by evdev code.
pub const BUTTON_LEFT: u32 = 0x110;
pub const BUTTON_RIGHT: u32 = 0x111;
pub const BUTTON_MIDDLE: u32 = 0x112;

impl Mission {
    /// Opens on `screens`, the selection on `focused` when it is shown.
    pub fn open(
        mode: Mode,
        screens: Vec<Screen>,
        apps: Vec<String>,
        focused: Option<u64>,
        settings: Settings,
        metrics: Metrics,
    ) -> Self {
        let mut this = Self {
            mode,
            scroll: vec![0.0; screens.len()],
            screens,
            layouts: Vec::new(),
            settings,
            metrics,
            apps,
            selection: None,
            keyed: false,
            hover: None,
            badge: None,
            pressed: None,
            drag: None,
            looking: None,
        };
        this.relayout();
        this.selection = focused
            .and_then(|id| this.tile_target(id))
            .or_else(|| this.first_target(this.focused_screen()));
        this
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn screens(&self) -> &[Screen] {
        &self.screens
    }

    pub fn layouts(&self) -> &[ScreenLayout] {
        &self.layouts
    }

    pub fn layout_of(&self, output: &str) -> Option<(usize, &ScreenLayout)> {
        let i = self.screens.iter().position(|s| s.output == output)?;
        Some((i, &self.layouts[i]))
    }

    /// The selection as drawn: none until a key has been pressed.
    pub fn selection(&self) -> Option<&Target> {
        self.selection.as_ref().filter(|_| self.keyed)
    }

    pub fn hover(&self) -> Option<&Target> {
        self.hover.as_ref()
    }

    pub fn badge(&self) -> Option<&Badge> {
        self.badge.as_ref()
    }

    pub fn drag(&self) -> Option<&Drag> {
        self.drag.as_ref().filter(|d| d.started)
    }

    pub fn looking(&self) -> Option<u64> {
        self.looking
    }

    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// The windows and desktops changed: the same model over the new screens.
    pub fn refresh(&mut self, screens: Vec<Screen>, apps: Vec<String>) {
        self.scroll.resize(screens.len(), 0.0);
        self.screens = screens;
        self.apps = apps;
        self.relayout();
        let alive = |t: &Option<Target>, this: &Self| t.as_ref().is_some_and(|t| this.exists(t));
        if !alive(&self.selection, self) {
            self.selection = self.first_target(self.focused_screen());
        }
        if !alive(&self.hover, self) {
            self.hover = None;
            self.badge = None;
        }
        if self
            .looking
            .is_some_and(|id| self.tile_target(id).is_none())
        {
            self.looking = None;
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.relayout();
        self.selection = self.first_target(self.focused_screen());
        self.hover = None;
        self.badge = None;
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.relayout();
    }

    fn relayout(&mut self) {
        self.layouts = self
            .screens
            .iter()
            .enumerate()
            .map(|(i, s)| {
                layout_screen(s, &self.mode, &self.settings, &self.metrics, self.scroll[i])
            })
            .collect();
    }

    fn focused_screen(&self) -> usize {
        self.screens.iter().position(|s| s.focused).unwrap_or(0)
    }

    fn exists(&self, t: &Target) -> bool {
        let Some(l) = self.layouts.get(t.screen()) else {
            return false;
        };
        match t {
            Target::Tile { window, .. } => l.tile(*window).is_some(),
            Target::Tray { window, .. } => l.tray_slot(*window).is_some(),
            Target::Desk { index, .. } => l
                .bar
                .as_ref()
                .is_some_and(|b| b.items.iter().any(|i| i.desk == Some(*index))),
            Target::Plus { .. } => l
                .bar
                .as_ref()
                .is_some_and(|b| b.items.iter().any(|i| i.desk.is_none())),
        }
    }

    fn tile_target(&self, window: u64) -> Option<Target> {
        self.layouts
            .iter()
            .enumerate()
            .find_map(|(screen, l)| l.tile(window).map(|_| Target::Tile { screen, window }))
    }

    /// Every target of a screen in reading order: the bar, the spread, the tray.
    fn targets(&self, screen: usize) -> Vec<(Target, Rect)> {
        let Some(l) = self.layouts.get(screen) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if matches!(self.mode, Mode::Chooser { .. }) {
            // The keys skip what does not fit.
            for t in &l.tiles {
                if !self.misfit(t.window) {
                    out.push((Target::Tile { screen, window: t.window }, t.rect));
                }
            }
            return out;
        }
        if let Some(bar) = &l.bar {
            for item in &bar.items {
                let t = match item.desk {
                    Some(index) => Target::Desk { screen, index },
                    None => Target::Plus { screen },
                };
                out.push((t, item.miniature));
            }
        }
        let mut tiles: Vec<&Tile> = l.tiles.iter().collect();
        tiles.sort_by(|a, b| {
            (a.rect.y, a.rect.x)
                .partial_cmp(&(b.rect.y, b.rect.x))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for t in tiles {
            out.push((
                Target::Tile {
                    screen,
                    window: t.window,
                },
                t.rect,
            ));
        }
        if let Some(tray) = &l.tray {
            for g in &tray.groups {
                for (window, r) in &g.items {
                    out.push((
                        Target::Tray {
                            screen,
                            window: *window,
                        },
                        *r,
                    ));
                }
            }
        }
        out
    }

    fn first_target(&self, screen: usize) -> Option<Target> {
        let targets = self.targets(screen);
        targets
            .iter()
            .find(|(t, _)| matches!(t, Target::Tile { .. }))
            .or(targets.first())
            .map(|(t, _)| t.clone())
    }

    fn rect_of(&self, t: &Target) -> Option<Rect> {
        self.targets(t.screen())
            .into_iter()
            .find(|(x, _)| x == t)
            .map(|(_, r)| r)
    }

    /// The nearest target from the selection in a direction; past the screen's edge, the
    /// next screen's nearest.
    fn step(&self, dx: f32, dy: f32) -> Option<Target> {
        let current = self.selection.clone()?;
        let here = self.rect_of(&current)?;
        let from = here.centre();
        let screen = current.screen();
        let best = self
            .targets(screen)
            .into_iter()
            .filter(|(t, _)| *t != current)
            // Across stays in the band it is in: a row of tiles, the bar, the tray.
            .filter(|(_, r)| dy != 0.0 || (r.y < here.bottom() && here.y < r.bottom()))
            .filter_map(|(t, r)| {
                let (x, y) = r.centre();
                let (ddx, ddy) = (x - from.0, y - from.1);
                let along = ddx * dx + ddy * dy;
                if along <= 1.0 {
                    return None;
                }
                let across = (ddx * dy - ddy * dx).abs();
                Some((along + 2.0 * across, t))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, t)| t);
        if best.is_some() || dy != 0.0 {
            return best;
        }
        // Past the edge onto the screen beside, in the order the screens sit.
        let next = if dx > 0.0 {
            screen + 1
        } else {
            screen.checked_sub(1)?
        };
        let targets = self.targets(next);
        targets
            .iter()
            .min_by(|a, b| {
                let d = |r: &Rect| (r.centre().1 - from.1).abs();
                d(&a.1).total_cmp(&d(&b.1))
            })
            .map(|(t, _)| t.clone())
    }

    fn cycle(&self, forward: bool) -> Option<Target> {
        let all: Vec<Target> = (0..self.screens.len())
            .flat_map(|s| self.targets(s).into_iter().map(|(t, _)| t))
            .collect();
        if all.is_empty() {
            return None;
        }
        let at = self
            .selection
            .as_ref()
            .and_then(|s| all.iter().position(|t| t == s));
        let n = all.len();
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        };
        Some(all[next].clone())
    }

    fn window_of(&self, t: &Target) -> Option<u64> {
        match t {
            Target::Tile { window, .. } | Target::Tray { window, .. } => Some(*window),
            _ => None,
        }
    }

    fn app_of(&self, window: u64) -> Option<String> {
        self.screens
            .iter()
            .find_map(|s| s.window(window))
            .map(|w| w.app.clone())
    }

    /// What choosing `t` does.
    /// A candidate too wide for Split View's half: shown, never chosen.
    pub fn misfit(&self, window: u64) -> bool {
        matches!(self.mode, Mode::Chooser { .. })
            && self
                .screens
                .iter()
                .find_map(|s| s.window(window).map(|w| !fits_half(s, w)))
                .unwrap_or(false)
    }

    fn choose(&self, t: &Target) -> Vec<Effect> {
        if let Target::Tile { window, .. } = t
            && self.misfit(*window)
        {
            return Vec::new();
        }
        match t {
            Target::Tile { window, .. } => vec![Effect::Choose(*window)],
            Target::Tray { window, .. } => vec![Effect::Restore(*window)],
            Target::Desk { screen, index } => vec![Effect::Show {
                screen: *screen,
                index: *index,
            }],
            Target::Plus { screen } => vec![Effect::AddDesktop { screen: *screen }],
        }
    }

    fn remove(&self, screen: usize, index: usize) -> Vec<Effect> {
        let Some(s) = self.screens.get(screen) else {
            return Vec::new();
        };
        let Some(desk) = s.desks.get(index) else {
            return Vec::new();
        };
        match desk.kind {
            DeskKind::FullScreen(w) | DeskKind::Split(w, _) => vec![Effect::ExitFullScreen(w)],
            DeskKind::Normal if s.desks.iter().filter(|d| !d.full_screen()).count() > 1 => {
                vec![Effect::RemoveDesktop { screen, index }]
            }
            DeskKind::Normal => Vec::new(),
        }
    }

    pub fn key(&mut self, key: Key) -> Vec<Effect> {
        let first = !self.keyed;
        self.keyed = true;
        self.hover = None;
        self.badge = None;
        let select = |this: &mut Self, t: Option<Target>| {
            if let Some(t) = t {
                this.keep_in_view(&t);
                this.selection = Some(t);
            }
            Vec::new()
        };
        match key {
            Key::Escape | Key::MissionControl if self.looking.is_some() => {
                self.looking = None;
                vec![Effect::Look(None)]
            }
            Key::Escape => vec![Effect::Dismiss],
            Key::MissionControl => match self.mode {
                Mode::Expose(_) => vec![Effect::Overview],
                Mode::MissionControl | Mode::Chooser { .. } => vec![Effect::Dismiss],
            },
            Key::Left | Key::Right | Key::Up | Key::Down if first => Vec::new(),
            Key::Left => {
                let t = self.step(-1.0, 0.0);
                select(self, t)
            }
            Key::Right => {
                let t = self.step(1.0, 0.0);
                select(self, t)
            }
            Key::Up => {
                let t = self.step(0.0, -1.0);
                select(self, t)
            }
            Key::Down => {
                let t = self.step(0.0, 1.0);
                select(self, t)
            }
            Key::Tab | Key::BackTab => match &self.mode {
                Mode::Expose(app) => {
                    let n = self.apps.len();
                    let at = self.apps.iter().position(|a| a == app);
                    match (at, n) {
                        (_, 0) => Vec::new(),
                        (Some(i), _) => {
                            let next = if key == Key::Tab {
                                (i + 1) % n
                            } else {
                                (i + n - 1) % n
                            };
                            vec![Effect::Expose(self.apps[next].clone())]
                        }
                        (None, _) => vec![Effect::Expose(self.apps[0].clone())],
                    }
                }
                Mode::MissionControl | Mode::Chooser { .. } => {
                    let t = self.cycle(key == Key::Tab);
                    select(self, t)
                }
            },
            Key::Return => self
                .selection
                .clone()
                .map_or(Vec::new(), |t| self.choose(&t)),
            Key::Space => {
                let id = self.selection.as_ref().and_then(|t| match t {
                    Target::Tile { window, .. } => Some(*window),
                    _ => None,
                });
                self.looking = if self.looking.is_some() { None } else { id };
                vec![Effect::Look(self.looking)]
            }
            Key::Close => self
                .selection
                .as_ref()
                .and_then(|t| self.window_of(t))
                .map_or(Vec::new(), |w| vec![Effect::CloseWindow(w)]),
            Key::Delete => match self.selection.clone() {
                Some(Target::Desk { screen, index }) => self.remove(screen, index),
                Some(t) => self
                    .window_of(&t)
                    .map_or(Vec::new(), |w| vec![Effect::CloseWindow(w)]),
                None => Vec::new(),
            },
            Key::Expose => {
                let app = self
                    .selection
                    .as_ref()
                    .and_then(|t| self.window_of(t))
                    .and_then(|w| self.app_of(w));
                app.map_or(Vec::new(), |a| vec![Effect::Expose(a)])
            }
            Key::Desktop(step) => {
                let screen = self
                    .selection
                    .as_ref()
                    .map_or(self.focused_screen(), |t| t.screen());
                let Some(s) = self.screens.get(screen) else {
                    return Vec::new();
                };
                let next = s.current as i32 + step;
                if next < 0 || next as usize >= s.desks.len() {
                    return Vec::new();
                }
                vec![Effect::Switch {
                    screen,
                    index: next as usize,
                }]
            }
            Key::Jump(index) => {
                let screen = self
                    .selection
                    .as_ref()
                    .map_or(self.focused_screen(), |t| t.screen());
                if self
                    .screens
                    .get(screen)
                    .is_some_and(|s| index < s.desks.len())
                {
                    vec![Effect::Switch { screen, index }]
                } else {
                    Vec::new()
                }
            }
            Key::Carry(step) => self.carry(|from, _| from as i32 + step),
            Key::CarryTo(index) => self.carry(|_, _| index as i32),
            Key::WindowMenu => self
                .selection
                .as_ref()
                .and_then(|t| self.window_of(t))
                .map_or(Vec::new(), |w| vec![Effect::WindowMenu(w)]),
        }
    }

    /// The selected window to another desktop, or the selected desktop along the row.
    fn carry(&mut self, to: impl Fn(usize, usize) -> i32) -> Vec<Effect> {
        match self.selection.clone() {
            Some(Target::Tile { screen, window }) => {
                let s = &self.screens[screen];
                let Some(from) = s.desk_of(window) else {
                    return Vec::new();
                };
                let index = to(from, s.desks.len());
                if index < 0 || index as usize >= s.desks.len() || index as usize == from {
                    return Vec::new();
                }
                if s.desks[index as usize].full_screen() {
                    return Vec::new();
                }
                vec![Effect::Move {
                    window,
                    to: Place::Desk {
                        screen,
                        index: index as usize,
                    },
                }]
            }
            Some(Target::Desk { screen, index }) => {
                let n = self.screens[screen].desks.len();
                let to = to(index, n);
                if to < 0 || to as usize >= n || to as usize == index {
                    return Vec::new();
                }
                self.selection = Some(Target::Desk {
                    screen,
                    index: to as usize,
                });
                vec![Effect::Reorder {
                    screen,
                    from: index,
                    to: to as usize,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// Scrolls the screen's bar so `t`, when a desktop, is in view.
    fn keep_in_view(&mut self, t: &Target) {
        let screen = t.screen();
        let Some(bar) = self.layouts.get(screen).and_then(|l| l.bar.clone()) else {
            return;
        };
        if !bar.overflow {
            return;
        }
        let item = match t {
            Target::Desk { index, .. } => bar.items.iter().find(|i| i.desk == Some(*index)),
            Target::Plus { .. } => bar.items.iter().find(|i| i.desk.is_none()),
            _ => None,
        };
        let Some(item) = item else {
            return;
        };
        let inner = Rect::new(
            bar.panel.x + self.metrics.space_m,
            0.0,
            bar.panel.w - 2.0 * self.metrics.space_m,
            1.0,
        );
        let mut scroll = self.scroll[screen];
        if item.miniature.x < inner.x {
            scroll -= inner.x - item.miniature.x;
        } else if item.miniature.right() > inner.right() {
            scroll += item.miniature.right() - inner.right();
        }
        if scroll != self.scroll[screen] {
            self.scroll[screen] = scroll;
            self.relayout();
        }
    }

    /// The bar scrolled by the wheel.
    pub fn scroll_bar(&mut self, screen: usize, by: f32) {
        if let Some(s) = self.scroll.get_mut(screen) {
            *s += by;
            self.relayout();
            self.scroll[screen] = self.layouts[screen].bar.as_ref().map_or(0.0, |b| b.scroll);
        }
    }

    /// What lies under a point of a screen.
    fn hit(&self, screen: usize, x: f32, y: f32) -> Option<Target> {
        let l = self.layouts.get(screen)?;
        if let Some(bar) = &l.bar
            && bar.panel.contains(x, y)
        {
            return bar
                .items
                .iter()
                .find(|i| {
                    let both = Rect::new(
                        i.miniature.x,
                        i.miniature.y,
                        i.miniature.w,
                        i.label.bottom() - i.miniature.y,
                    );
                    both.contains(x, y)
                })
                .map(|i| match i.desk {
                    Some(index) => Target::Desk { screen, index },
                    None => Target::Plus { screen },
                });
        }
        if let Some(tray) = &l.tray
            && tray.panel.contains(x, y)
        {
            return tray
                .groups
                .iter()
                .flat_map(|g| &g.items)
                .find(|(_, r)| r.contains(x, y))
                .map(|(window, _)| Target::Tray {
                    screen,
                    window: *window,
                });
        }
        l.tiles
            .iter()
            .rev()
            .find(|t| t.rect.contains(x, y))
            .map(|t| Target::Tile {
                screen,
                window: t.window,
            })
    }

    /// The badge under a point, on the hovered target: 20 across with a 24 hit box, centred
    /// on its top leading corner.
    fn badge_at(&self, hover: &Target, x: f32, y: f32) -> Option<Badge> {
        let r = self.rect_of(hover)?;
        let near = (x - r.x).abs() <= 12.0 && (y - r.y).abs() <= 12.0;
        if !near {
            return None;
        }
        match hover {
            Target::Tile { screen, window } => Some(Badge::Close {
                screen: *screen,
                window: *window,
            }),
            Target::Desk { screen, index } if !self.remove(*screen, *index).is_empty() => {
                Some(Badge::Remove {
                    screen: *screen,
                    index: *index,
                })
            }
            _ => None,
        }
    }

    /// Where a dragged window would land at a point of a screen.
    fn drop_place(&self, screen: usize, x: f32, y: f32, window: u64) -> (Option<Place>, bool) {
        let Some(l) = self.layouts.get(screen) else {
            return (None, false);
        };
        let from = self
            .screens
            .iter()
            .find_map(|s| s.desk_of(window).map(|d| (s, d)));
        match self.hit(screen, x, y) {
            Some(Target::Desk { index, .. }) => {
                let s = &self.screens[screen];
                let desk = &s.desks[index];
                if from.is_some_and(|(fs, d)| fs.output == s.output && d == index) {
                    return (None, false);
                }
                match desk.kind {
                    DeskKind::Split(..) => (None, true),
                    DeskKind::FullScreen(other) => {
                        let fits = self.fits_half(screen, window) && self.fits_half(screen, other);
                        if !fits {
                            return (None, true);
                        }
                        let item = l
                            .bar
                            .as_ref()
                            .and_then(|b| b.items.iter().find(|i| i.desk == Some(index)));
                        let left = item.is_some_and(|i| x < i.miniature.centre().0);
                        (
                            Some(Place::Split {
                                screen,
                                index,
                                left,
                            }),
                            false,
                        )
                    }
                    DeskKind::Normal => (Some(Place::Desk { screen, index }), false),
                }
            }
            Some(Target::Plus { .. }) => (Some(Place::New { screen }), false),
            // Over another screen's spread: its current desktop.
            _ if from.is_some_and(|(fs, _)| fs.output != self.screens[screen].output)
                && l.spread.contains(x, y) =>
            {
                let s = &self.screens[screen];
                if s.desks[s.current].full_screen() {
                    (None, true)
                } else {
                    (
                        Some(Place::Desk {
                            screen,
                            index: s.current,
                        }),
                        false,
                    )
                }
            }
            _ => (None, false),
        }
    }

    fn fits_half(&self, screen: usize, window: u64) -> bool {
        let s = &self.screens[screen];
        let half = (s.width - 8.0) / 2.0;
        self.screens
            .iter()
            .find_map(|s| s.window(window))
            .is_none_or(|w| w.min_width <= half)
    }

    pub fn pointer_move(&mut self, screen: usize, x: f32, y: f32) -> Vec<Effect> {
        if let Some((t, at, s, button)) = self.pressed.clone()
            && button == BUTTON_LEFT
        {
            let moved = ((x - at.0).powi(2) + (y - at.1).powi(2)).sqrt();
            if self.drag.is_none() && moved >= DRAG_START {
                let what = match &t {
                    Target::Tile { window, .. } | Target::Tray { window, .. } => {
                        Some(Dragged::Window(*window))
                    }
                    Target::Desk { index, .. } => Some(Dragged::Desk(*index)),
                    Target::Plus { .. } => None,
                };
                if let Some(what) = what {
                    let r = self.rect_of(&t).unwrap_or_default();
                    self.drag = Some(Drag {
                        what,
                        screen: s,
                        at: (x, y),
                        grab: (at.0 - r.x, at.1 - r.y),
                        over: None,
                        refused: false,
                        started: true,
                    });
                }
            }
        }
        if let Some(drag) = self.drag.clone() {
            let mut drag = drag;
            drag.screen = screen;
            drag.at = (x, y);
            match drag.what {
                Dragged::Window(window) => {
                    let (over, refused) = self.drop_place(screen, x, y, window);
                    drag.over = over;
                    drag.refused = refused;
                }
                Dragged::Desk(_) => {
                    drag.over = match self.hit(screen, x, y) {
                        Some(Target::Desk { index, .. }) => Some(Place::Desk { screen, index }),
                        _ => None,
                    };
                }
            }
            self.hover = None;
            self.badge = None;
            self.drag = Some(drag);
            return Vec::new();
        }
        self.hover = self.hit(screen, x, y);
        self.badge = self.hover.clone().and_then(|h| self.badge_at(&h, x, y));
        if let Some(h) = &self.hover {
            self.selection = Some(h.clone());
        }
        Vec::new()
    }

    pub fn pointer_leave(&mut self) {
        self.hover = None;
        self.badge = None;
    }

    pub fn press(&mut self, screen: usize, x: f32, y: f32, button: u32) -> Vec<Effect> {
        self.pointer_move(screen, x, y);
        match (self.hit(screen, x, y), button) {
            (Some(t), _) => {
                self.pressed = Some((t, (x, y), screen, button));
            }
            (None, _) => {
                self.pressed = None;
            }
        }
        Vec::new()
    }

    pub fn release(&mut self, screen: usize, x: f32, y: f32, button: u32) -> Vec<Effect> {
        let pressed = self.pressed.take();
        if let Some(drag) = self.drag.take() {
            return match (drag.what, drag.over) {
                (Dragged::Window(window), Some(to)) => vec![Effect::Move { window, to }],
                (Dragged::Desk(from), Some(Place::Desk { screen: s, index }))
                    if s == drag.screen && from != index =>
                {
                    vec![Effect::Reorder {
                        screen: s,
                        from,
                        to: index,
                    }]
                }
                _ => Vec::new(),
            };
        }
        let here = self.hit(screen, x, y);
        match (pressed, button) {
            (Some((t, _, _, BUTTON_LEFT)), BUTTON_LEFT) if Some(&t) == here.as_ref() => {
                if let Some(badge) = self.badge_at(&t, x, y) {
                    return match badge {
                        Badge::Close { window, .. } => vec![Effect::CloseWindow(window)],
                        Badge::Remove { screen, index } => self.remove(screen, index),
                    };
                }
                self.choose(&t)
            }
            (Some((t, _, _, BUTTON_MIDDLE)), BUTTON_MIDDLE) if Some(&t) == here.as_ref() => self
                .window_of(&t)
                .map_or(Vec::new(), |w| vec![Effect::CloseWindow(w)]),
            (Some((t, _, _, BUTTON_RIGHT)), BUTTON_RIGHT) if Some(&t) == here.as_ref() => match t {
                Target::Desk { screen, index } => vec![Effect::DeskMenu { screen, index }],
                t => self
                    .window_of(&t)
                    .map_or(Vec::new(), |w| vec![Effect::WindowMenu(w)]),
            },
            // A click on the empty wallpaper closes it.
            (None, BUTTON_LEFT) if here.is_none() && !self.on_panel(screen, x, y) => {
                vec![Effect::Dismiss]
            }
            _ => Vec::new(),
        }
    }

    fn on_panel(&self, screen: usize, x: f32, y: f32) -> bool {
        self.layouts.get(screen).is_some_and(|l| {
            l.bar.as_ref().is_some_and(|b| b.panel.contains(x, y))
                || l.tray.as_ref().is_some_and(|t| t.panel.contains(x, y))
                || l.header.is_some_and(|h| h.contains(x, y))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(id: u64, app: &str, x: f32, y: f32, w: f32, h: f32) -> Window {
        Window {
            id,
            app: app.into(),
            title: format!("{app} {id}"),
            rect: Rect::new(x, y, w, h),
            min_width: 400.0,
            ..Default::default()
        }
    }

    fn ultrawide(windows: Vec<Window>) -> Screen {
        let ids = windows
            .iter()
            .filter(|w| !w.minimised)
            .map(|w| w.id)
            .collect();
        Screen {
            output: "DP-4".into(),
            width: 3440.0,
            height: 1440.0,
            desks: vec![
                Desk::normal(1, vec![]),
                Desk::normal(2, ids),
                Desk::normal(3, vec![]),
            ],
            current: 1,
            windows,
            focused: true,
        }
    }

    fn seven() -> Vec<Window> {
        vec![
            win(1, "kitty", 80.0, 110.0, 1100.0, 680.0),
            win(2, "firefox", 760.0, 170.0, 1560.0, 1000.0),
            win(3, "zed", 2020.0, 96.0, 1340.0, 880.0),
            win(4, "files", 240.0, 720.0, 1080.0, 660.0),
            win(5, "discord", 2260.0, 650.0, 1100.0, 720.0),
            win(6, "settings", 1500.0, 560.0, 860.0, 760.0),
            win(7, "steam", 1140.0, 260.0, 1240.0, 820.0),
        ]
    }

    fn overlaps(a: &Rect, b: &Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    fn open(screens: Vec<Screen>) -> Mission {
        Mission::open(
            Mode::MissionControl,
            screens,
            vec!["kitty".into(), "firefox".into()],
            Some(2),
            Settings::default(),
            Metrics::default(),
        )
    }

    #[test]
    fn the_spread_never_overlaps_never_grows_and_stays_inside() {
        for (w, h) in [(3440.0, 1440.0), (1920.0, 1080.0)] {
            for n in 1..=40usize {
                let items: Vec<(u64, Rect)> = (0..n)
                    .map(|i| {
                        let i32_ = i as f32;
                        (
                            i as u64,
                            Rect::new(
                                (i32_ * 397.0) % (w - 900.0),
                                60.0 + (i32_ * 233.0) % (h - 600.0),
                                500.0 + (i32_ * 137.0) % 700.0,
                                400.0 + (i32_ * 89.0) % 300.0,
                            ),
                        )
                    })
                    .collect();
                let area = Rect::new(32.0, 227.0, w - 64.0, h - 227.0 - 32.0);
                let tiles = spread(&items, area, 52.0, 24.0);
                assert_eq!(tiles.len(), n, "{n} windows at {w}");
                for (i, t) in tiles.iter().enumerate() {
                    let real = items.iter().find(|(id, _)| *id == t.window).unwrap().1;
                    assert!(
                        t.rect.w <= real.w + 0.01 && t.rect.h <= real.h + 0.01,
                        "larger than real"
                    );
                    assert!(
                        t.rect.x >= area.x - 0.5 && t.rect.right() <= area.right() + 0.5,
                        "inside across: {t:?}"
                    );
                    assert!(
                        t.rect.y >= area.y - 0.5 && t.rect.bottom() + 52.0 <= area.bottom() + 0.5,
                        "inside down"
                    );
                    for u in &tiles[i + 1..] {
                        assert!(!overlaps(&t.rect, &u.rect), "{t:?} overlaps {u:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_window_on_the_left_stays_on_the_left() {
        let items = vec![
            (1, Rect::new(100.0, 300.0, 800.0, 600.0)),
            (2, Rect::new(2500.0, 300.0, 800.0, 600.0)),
        ];
        let tiles = spread(&items, Rect::new(32.0, 227.0, 3376.0, 1100.0), 52.0, 24.0);
        let x = |id| tiles.iter().find(|t| t.window == id).unwrap().rect.x;
        assert!(x(1) < x(2));
    }

    #[test]
    fn one_window_that_fits_keeps_its_place() {
        let items = vec![(1, Rect::new(400.0, 500.0, 800.0, 600.0))];
        let tiles = spread(&items, Rect::new(32.0, 227.0, 3376.0, 1100.0), 52.0, 24.0);
        assert_eq!(tiles[0].rect, Rect::new(400.0, 500.0, 800.0, 600.0));
    }

    #[test]
    fn the_bar_holds_every_desktop_and_the_plus_tile_under_the_strip() {
        let m = open(vec![ultrawide(seven())]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        assert_eq!(bar.panel.y, 48.0);
        assert_eq!(bar.panel.h, 155.0);
        assert_eq!(bar.miniature, (268.0, 112.0));
        assert_eq!(bar.items.len(), 4);
        assert!(bar.items[3].desk.is_none());
        assert!(!bar.overflow);
    }

    #[test]
    fn many_desktops_shrink_the_miniatures_then_scroll() {
        let mut s = ultrawide(seven());
        s.desks = (0..14).map(|i| Desk::normal(i, vec![])).collect();
        s.current = 0;
        let m = open(vec![s.clone()]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        assert!(
            bar.miniature.1 < 112.0 && bar.miniature.1 >= 64.0,
            "{:?}",
            bar.miniature
        );
        assert!(!bar.overflow);
        s.desks = (0..16).map(|i| Desk::normal(i, vec![])).collect();
        let m = open(vec![s.clone()]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        assert!(!bar.overflow, "sixteen fit the ultrawide shrunk");
        s.width = 1920.0;
        s.height = 1080.0;
        let m = open(vec![s]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        assert_eq!(bar.miniature.1, 64.0);
        assert!(bar.overflow);
        assert!(
            bar.items.iter().all(|i| i.desk.is_some()),
            "no + tile at the limit"
        );
    }

    #[test]
    fn the_tray_holds_minimised_then_hidden_and_clears_the_corners() {
        let mut windows = seven();
        windows[0].minimised = true;
        windows[3].hidden = true;
        let mut s = ultrawide(windows);
        s.desks[1].windows = (1..=7).collect();
        let m = open(vec![s]);
        let l = &m.layouts()[0];
        let tray = l.tray.clone().unwrap();
        assert_eq!(tray.groups[0].head, TrayHead::Minimised);
        assert_eq!(tray.groups[0].items[0].0, 1);
        assert_eq!(tray.groups[1].head, TrayHead::Hidden);
        assert_eq!(tray.panel.h, 138.0);
        assert!(tray.panel.x >= 32.0);
        assert_eq!(tray.panel.bottom(), 1440.0 - 16.0);
        assert!(l.tiles.iter().all(|t| t.window != 1 && t.window != 4));
        assert!(l.tiles.iter().all(|t| t.rect.bottom() <= tray.panel.y));
    }

    #[test]
    fn app_expose_shows_one_app_and_sets_apart_the_rest() {
        let mut windows = seven();
        windows.push(win(8, "kitty", 300.0, 300.0, 900.0, 600.0));
        let mut extra = win(9, "kitty", 0.0, 0.0, 900.0, 600.0);
        extra.minimised = true;
        windows.push(extra);
        windows.push(win(10, "kitty", 0.0, 0.0, 900.0, 600.0));
        let mut s = ultrawide(windows);
        s.desks[1].windows = (1..=9).collect();
        s.desks[2].windows = vec![10];
        let mut m = open(vec![s]);
        m.set_mode(Mode::Expose("kitty".into()));
        let l = &m.layouts()[0];
        assert!(l.bar.is_none());
        assert!(l.header.is_some());
        let shown: Vec<u64> = l.tiles.iter().map(|t| t.window).collect();
        assert_eq!(shown.len(), 2);
        assert!(shown.contains(&1) && shown.contains(&8));
        let heads: Vec<TrayHead> = l
            .tray
            .as_ref()
            .unwrap()
            .groups
            .iter()
            .map(|g| g.head.clone())
            .collect();
        assert_eq!(heads, vec![TrayHead::Minimised, TrayHead::Desktop(3)]);
    }

    #[test]
    fn app_expose_with_nothing_says_so() {
        let mut m = open(vec![ultrawide(seven())]);
        m.set_mode(Mode::Expose("mail".into()));
        assert!(m.layouts()[0].empty.is_some());
    }

    #[test]
    fn grouping_by_application_clusters_each_app() {
        let mut windows = seven();
        windows.push(win(8, "kitty", 2000.0, 900.0, 900.0, 500.0));
        let mut s = ultrawide(windows);
        s.desks[1].windows = (1..=8).collect();
        let m = Mission::open(
            Mode::MissionControl,
            vec![s],
            vec![],
            None,
            Settings { group_by_app: true },
            Metrics::default(),
        );
        let l = &m.layouts()[0];
        assert_eq!(l.clusters.len(), 7);
        let a = l.tile(1).unwrap().rect;
        let b = l.tile(8).unwrap().rect;
        assert!(
            (a.centre().1 - b.centre().1).abs() < 1.0,
            "a cluster is one row"
        );
    }

    #[test]
    fn the_selection_shows_after_the_first_key_and_arrows_move_it() {
        let mut m = open(vec![ultrawide(seven())]);
        assert!(m.selection().is_none());
        m.key(Key::Right);
        assert_eq!(
            m.selection(),
            Some(&Target::Tile {
                screen: 0,
                window: 2
            })
        );
        m.key(Key::Right);
        assert!(
            matches!(m.selection(), Some(Target::Tile { .. })),
            "{:?}",
            m.selection()
        );
        assert_ne!(
            m.selection(),
            Some(&Target::Tile {
                screen: 0,
                window: 2
            })
        );
        for _ in 0..6 {
            m.key(Key::Up);
        }
        assert!(matches!(
            m.selection(),
            Some(Target::Desk { .. } | Target::Plus { .. })
        ));
        assert_eq!(m.key(Key::Return).len(), 1);
    }

    #[test]
    fn keys_choose_close_and_look() {
        let mut m = open(vec![ultrawide(seven())]);
        assert_eq!(m.key(Key::Return), vec![Effect::Choose(2)]);
        assert_eq!(m.key(Key::Close), vec![Effect::CloseWindow(2)]);
        assert_eq!(m.key(Key::Space), vec![Effect::Look(Some(2))]);
        assert_eq!(m.key(Key::Escape), vec![Effect::Look(None)]);
        assert_eq!(m.key(Key::Escape), vec![Effect::Dismiss]);
        assert_eq!(m.key(Key::Expose), vec![Effect::Expose("firefox".into())]);
        assert_eq!(
            m.key(Key::Desktop(1)),
            vec![Effect::Switch {
                screen: 0,
                index: 2
            }]
        );
        assert_eq!(m.key(Key::Desktop(5)), vec![]);
        assert_eq!(
            m.key(Key::Carry(-1)),
            vec![Effect::Move {
                window: 2,
                to: Place::Desk {
                    screen: 0,
                    index: 0
                }
            }]
        );
    }

    #[test]
    fn the_last_desktop_is_never_removed_and_a_full_screen_one_exits() {
        let mut s = ultrawide(seven());
        s.desks = vec![
            Desk::normal(1, (1..=7).collect()),
            Desk {
                id: 2,
                kind: DeskKind::FullScreen(9),
                windows: vec![9],
                stage: false,
            },
        ];
        s.current = 0;
        let mut m = open(vec![s]);
        assert!(m.remove(0, 0).is_empty());
        assert_eq!(m.remove(0, 1), vec![Effect::ExitFullScreen(9)]);
        m.selection = Some(Target::Desk {
            screen: 0,
            index: 1,
        });
        m.keyed = true;
        assert_eq!(m.key(Key::Delete), vec![Effect::ExitFullScreen(9)]);
    }

    #[test]
    fn a_click_chooses_a_middle_click_closes_and_the_wallpaper_dismisses() {
        let mut m = open(vec![ultrawide(seven())]);
        let r = m.layouts()[0].tile(3).unwrap().rect;
        let (x, y) = r.centre();
        m.press(0, x, y, BUTTON_LEFT);
        assert_eq!(m.release(0, x, y, BUTTON_LEFT), vec![Effect::Choose(3)]);
        m.press(0, x, y, BUTTON_MIDDLE);
        assert_eq!(
            m.release(0, x, y, BUTTON_MIDDLE),
            vec![Effect::CloseWindow(3)]
        );
        m.press(0, x, y, BUTTON_RIGHT);
        assert_eq!(
            m.release(0, x, y, BUTTON_RIGHT),
            vec![Effect::WindowMenu(3)]
        );
        m.press(0, 5.0, 1430.0, BUTTON_LEFT);
        assert_eq!(
            m.release(0, 5.0, 1430.0, BUTTON_LEFT),
            vec![Effect::Dismiss]
        );
    }

    #[test]
    fn the_close_badge_closes_and_a_desktops_remove_removes() {
        let mut m = open(vec![ultrawide(seven())]);
        let r = m.layouts()[0].tile(3).unwrap().rect;
        m.pointer_move(0, r.x + 2.0, r.y + 2.0);
        assert!(matches!(m.badge(), Some(Badge::Close { window: 3, .. })));
        m.press(0, r.x + 2.0, r.y + 2.0, BUTTON_LEFT);
        assert_eq!(
            m.release(0, r.x + 2.0, r.y + 2.0, BUTTON_LEFT),
            vec![Effect::CloseWindow(3)]
        );
        let mini = m.layouts()[0].bar.as_ref().unwrap().items[0].miniature;
        m.pointer_move(0, mini.x + 1.0, mini.y + 1.0);
        m.press(0, mini.x + 1.0, mini.y + 1.0, BUTTON_LEFT);
        assert_eq!(
            m.release(0, mini.x + 1.0, mini.y + 1.0, BUTTON_LEFT),
            vec![Effect::RemoveDesktop {
                screen: 0,
                index: 0
            }]
        );
    }

    #[test]
    fn a_tile_dragged_to_a_desktop_or_the_plus_tile_moves_its_window() {
        let mut m = open(vec![ultrawide(seven())]);
        let r = m.layouts()[0].tile(6).unwrap().rect;
        let (x, y) = r.centre();
        let bar = m.layouts()[0].bar.clone().unwrap();
        m.press(0, x, y, BUTTON_LEFT);
        let (dx, dy) = bar.items[2].miniature.centre();
        m.pointer_move(0, x + 10.0, y + 10.0);
        m.pointer_move(0, dx, dy);
        assert_eq!(
            m.drag().unwrap().over,
            Some(Place::Desk {
                screen: 0,
                index: 2
            })
        );
        assert_eq!(
            m.release(0, dx, dy, BUTTON_LEFT),
            vec![Effect::Move {
                window: 6,
                to: Place::Desk {
                    screen: 0,
                    index: 2
                }
            }]
        );
        m.press(0, x, y, BUTTON_LEFT);
        let (px, py) = bar.items[3].miniature.centre();
        m.pointer_move(0, px, py);
        assert_eq!(
            m.release(0, px, py, BUTTON_LEFT),
            vec![Effect::Move {
                window: 6,
                to: Place::New { screen: 0 }
            }]
        );
    }

    #[test]
    fn a_drop_on_a_full_screen_desktop_makes_a_split_and_a_split_refuses() {
        let mut s = ultrawide(seven());
        s.windows.push(win(9, "firefox", 0.0, 0.0, 3440.0, 1440.0));
        s.windows.push(win(10, "zed", 0.0, 0.0, 1716.0, 1440.0));
        s.windows
            .push(win(11, "kitty", 1724.0, 0.0, 1716.0, 1440.0));
        s.desks.push(Desk {
            id: 4,
            kind: DeskKind::FullScreen(9),
            windows: vec![9],
            stage: false,
        });
        s.desks.push(Desk {
            id: 5,
            kind: DeskKind::Split(10, 11),
            windows: vec![10, 11],
            stage: false,
        });
        let mut m = open(vec![s]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        let (x, y) = m.layouts()[0].tile(6).unwrap().rect.centre();
        m.press(0, x, y, BUTTON_LEFT);
        let fs = bar.items[3].miniature;
        m.pointer_move(0, fs.x + fs.w * 0.75, fs.centre().1);
        assert_eq!(
            m.drag().unwrap().over,
            Some(Place::Split {
                screen: 0,
                index: 3,
                left: false
            })
        );
        let split = bar.items[4].miniature;
        m.pointer_move(0, split.centre().0, split.centre().1);
        let d = m.drag().unwrap();
        assert!(d.over.is_none() && d.refused);
        assert_eq!(
            m.release(0, split.centre().0, split.centre().1, BUTTON_LEFT),
            vec![]
        );
    }

    #[test]
    fn a_desktop_dragged_along_the_bar_reorders() {
        let mut m = open(vec![ultrawide(seven())]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        let (x, y) = bar.items[0].miniature.centre();
        let (tx, ty) = bar.items[2].miniature.centre();
        m.press(0, x, y, BUTTON_LEFT);
        m.pointer_move(0, x + 20.0, y);
        m.pointer_move(0, tx, ty);
        assert_eq!(
            m.release(0, tx, ty, BUTTON_LEFT),
            vec![Effect::Reorder {
                screen: 0,
                from: 0,
                to: 2
            }]
        );
    }

    #[test]
    fn arrows_cross_onto_the_next_screen() {
        let mut hd = ultrawide(vec![win(20, "mail", 200.0, 200.0, 1200.0, 700.0)]);
        hd.output = "HDMI-1".into();
        hd.width = 1920.0;
        hd.height = 1080.0;
        hd.focused = false;
        hd.desks = vec![Desk::normal(9, vec![20])];
        hd.current = 0;
        let mut m = open(vec![ultrawide(seven()), hd]);
        m.key(Key::Right);
        for _ in 0..8 {
            m.key(Key::Right);
        }
        assert_eq!(m.selection().map(|t| t.screen()), Some(1));
    }

    #[test]
    fn tab_in_app_expose_moves_to_the_next_app() {
        let mut m = open(vec![ultrawide(seven())]);
        m.set_mode(Mode::Expose("kitty".into()));
        assert_eq!(m.key(Key::Tab), vec![Effect::Expose("firefox".into())]);
        assert_eq!(m.key(Key::MissionControl), vec![Effect::Overview]);
    }

    #[test]
    fn the_chooser_offers_the_free_half_and_skips_misfits() {
        let mut s = ultrawide(seven());
        s.windows[0].rect = Rect::new(0.0, 0.0, 1716.0, 1440.0);
        s.windows[6].min_width = 2000.0;
        let mut m = open(vec![s]);
        m.set_mode(Mode::Chooser {
            first: 1,
            left: true,
            candidates: vec![2, 3, 7],
        });
        let l = &m.layouts()[0];
        assert!(l.bar.is_none() && l.tray.is_none());
        assert_eq!(l.tiles.len(), 3);
        assert!(l.tiles.iter().all(|t| t.rect.x >= 1724.0));
        assert!(m.misfit(7) && !m.misfit(2));
        m.key(Key::Right);
        for _ in 0..4 {
            m.key(Key::Tab);
            assert!(!matches!(m.selection(), Some(Target::Tile { window: 7, .. })));
        }
        let r = m.layouts()[0].tile(7).unwrap().rect;
        let (x, y) = r.centre();
        m.press(0, x, y, BUTTON_LEFT);
        assert_eq!(m.release(0, x, y, BUTTON_LEFT), vec![]);
    }

    #[test]
    fn layouts_hold_at_the_smaller_screen() {
        let mut s = ultrawide(
            seven()
                .into_iter()
                .map(|mut w| {
                    w.rect = Rect::new(
                        w.rect.x * 0.558,
                        w.rect.y * 0.75,
                        w.rect.w * 0.558,
                        w.rect.h * 0.75,
                    );
                    w
                })
                .collect(),
        );
        s.width = 1920.0;
        s.height = 1080.0;
        let m = open(vec![s]);
        let bar = m.layouts()[0].bar.clone().unwrap();
        assert_eq!(bar.miniature, (199.0, 112.0));
        assert_eq!(m.layouts()[0].tiles.len(), 7);
    }
}
