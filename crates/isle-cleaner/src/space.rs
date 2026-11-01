//! Space (`p1/surfaces/cleaner.md` Space): a folder's children as a squarified map or as a list,
//! the path to it above; a folder opens by growing from its tile.

use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    rc::Rc,
};

use freya::{animation::*, prelude::*};
use isle_scan::{Kind, TOP, kinds, tree::Id};
use isle_tokens::Theme;
use isle_ui::{
    Glyph,
    button::mix,
    colour,
    table::{Cell, Column, Mark, Row, Sort, Table, TableEvent},
    text, use_theme,
};

use crate::{
    basket::Basket,
    model::{Snapshot, size},
    treemap::{self, Rect},
    ui::{kind_colour, kind_glyph, kind_tile, mono},
};

/// One child of the folder shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
    pub kind: Kind,
    /// Its node, for a folder.
    pub node: Option<Id>,
    pub dev: u64,
    pub ino: u64,
}

/// The folder at `path`'s children, largest first: its folders from the scan, its files read
/// now, since the scan keeps no files.
pub fn entries(snapshot: &Snapshot, path: Option<&Path>) -> (Vec<Entry>, u64) {
    snapshot.scan.read(|tree| {
        let at = match path {
            Some(p) => tree.find(p),
            None => {
                let roots: Vec<Id> = tree.children(TOP).collect();
                if roots.len() == 1 {
                    Some(roots[0])
                } else {
                    Some(TOP)
                }
            }
        };
        let Some(at) = at else { return (Vec::new(), 0) };
        let mut out: Vec<Entry> = tree
            .children(at)
            .map(|id| {
                let p = tree.path(id);
                Entry {
                    name: if at == TOP {
                        p.display().to_string()
                    } else {
                        tree.name(id).to_string_lossy().into_owned()
                    },
                    path: p,
                    bytes: tree.node(id).bytes,
                    kind: tree.main_kind(id),
                    node: Some(id),
                    dev: tree.node(id).dev,
                    ino: tree.node(id).ino,
                }
            })
            .collect();
        if at != TOP {
            let folder = tree.path(at);
            let kind = tree.node(at).kind;
            for e in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
                let Ok(m) = e.path().symlink_metadata() else {
                    continue;
                };
                if m.is_dir() {
                    continue;
                }
                let name = e.file_name();
                out.push(Entry {
                    kind: kinds::of(kind, name.as_encoded_bytes()),
                    name: name.to_string_lossy().into_owned(),
                    path: e.path(),
                    bytes: m.blocks() * 512,
                    node: None,
                    dev: m.dev(),
                    ino: m.ino(),
                });
            }
        }
        out.sort_by_key(|e| std::cmp::Reverse(e.bytes));
        (out, tree.node(at).bytes)
    })
}

/// The folders from the scope's top to `path`, each with its name.
pub fn crumbs(roots: &[PathBuf], path: Option<&Path>) -> Vec<(Option<PathBuf>, String)> {
    let mut out = vec![(None, String::new())];
    let Some(path) = path else { return out };
    let Some(root) = roots.iter().find(|r| path.starts_with(r)) else {
        return out;
    };
    let mut at = root.clone();
    if roots.len() > 1 {
        out.push((Some(at.clone()), at.display().to_string()));
    }
    for part in path.strip_prefix(root).into_iter().flat_map(|r| r.iter()) {
        at = at.join(part);
        out.push((Some(at.clone()), part.to_string_lossy().into_owned()));
    }
    out
}

#[derive(Clone, PartialEq)]
pub enum Act {
    /// A folder to show, and the tile it grows from or shrinks into.
    Open(Option<PathBuf>, Option<(Rect, bool)>),
    Select(Option<PathBuf>),
    /// The entry asks for the basket's other state.
    Basket(Entry, bool),
    Menu(Entry, (f32, f32)),
}

#[derive(Clone)]
pub struct Space {
    pub snapshot: std::sync::Arc<Snapshot>,
    pub folder: Option<PathBuf>,
    pub scope_title: String,
    pub basket: Basket,
    pub selected: Option<PathBuf>,
    pub list: bool,
    /// The window has focus, for the selection's colour.
    pub active: bool,
    /// The tile a folder opened from, or went up into, for the zoom.
    pub zoom: Option<(Rect, bool)>,
    pub on_act: EventHandler<Act>,
}

impl PartialEq for Space {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn tile(
    theme: &Theme,
    e: Option<&Entry>,
    rest: usize,
    at: Rect,
    shown: f32,
    selected: bool,
    hovered: bool,
    in_basket: bool,
    active: bool,
) -> Element {
    let t = &theme.tokens;
    let fill = match e {
        Some(e) => mix(
            colour(t.colour_view),
            colour(kind_colour(theme, e.kind)),
            t.kind_mix,
        ),
        None => colour(t.colour_fill_strong),
    };
    let scale = theme.appearance.text_scale.max(1.);
    let labelled =
        at.w >= t.size_tile_label_width * scale && at.h >= t.size_tile_label_height * scale;
    let symbol = at.w >= t.size_tile_symbol * scale && at.h >= t.size_tile_symbol * scale;
    let band = if in_basket && !symbol {
        6.
    } else {
        t.stroke_kind
    };
    let ring = |c, w| {
        Border::new()
            .fill(colour(c))
            .width(w)
            .alignment(BorderAlignment::Inner)
    };
    rect()
        .position(Position::new_absolute().left(at.x).top(at.y))
        .width(Size::px((at.w - t.space_xxs).max(1.)))
        .height(Size::px((at.h - t.space_xxs).max(1.)))
        .corner_radius(t.radius_small)
        .overflow(Overflow::Clip)
        .background(fill)
        .maybe(theme.appearance.increase_contrast, |el| {
            el.border(ring(t.colour_hairline_strong, t.stroke_hairline))
        })
        .maybe(hovered && !selected, |el| {
            el.border(ring(t.colour_hairline_strong, t.stroke_hairline))
        })
        .maybe(selected, |el| {
            el.border(ring(
                if active {
                    t.colour_accent
                } else {
                    t.colour_hairline_strong
                },
                t.stroke_found,
            ))
        })
        .a11y_alt(match e {
            Some(e) => format!("{}, {}, {}", e.name, size(e.bytes), e.kind.name()),
            None => format!("{rest} smaller items"),
        })
        .maybe_child(e.map(|e| {
            rect()
                .position(Position::new_absolute())
                .width(Size::fill())
                .height(Size::px(band))
                .background(colour(kind_colour(theme, e.kind)))
                .into_element()
        }))
        .maybe_child((symbol || labelled).then(|| {
            rect()
                .padding(t.space_xs)
                .horizontal()
                .spacing(t.space_xs)
                .opacity(shown)
                .maybe_child(e.filter(|_| symbol).map(|e| kind_tile(theme, e.kind)))
                .maybe_child(labelled.then(|| {
                    let (name, bytes) = match e {
                        Some(e) => (e.name.clone(), size(e.bytes)),
                        None => (format!("{rest} smaller items"), String::new()),
                    };
                    rect()
                        .width(Size::px((at.w - 48.).max(20.)))
                        .child(
                            text::line(theme, text::Style::Callout, colour(t.colour_text), name)
                                .font_weight(600),
                        )
                        .child(text::line(
                            theme,
                            text::Style::Subheadline,
                            colour(t.colour_text_2),
                            bytes,
                        ))
                        .into_element()
                }))
                .into_element()
        }))
        .maybe_child(in_basket.then(|| {
            rect()
                .position(Position::new_absolute().right(t.space_xs).top(t.space_xs))
                .child(Glyph::new("check-circle-fill", 16., colour(t.colour_text)))
                .into_element()
        }))
        .into_element()
}

impl Component for Space {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let (entries, total) = entries(&self.snapshot, self.folder.as_deref());
        let mut map_size = use_state(|| (0f32, 0f32));
        let mut hover = use_state(|| None::<PathBuf>);
        let mut pointer = use_state(|| (0f32, 0f32));
        // The tile the pointer has rested on for `timing-tooltip-ms`.
        let mut tip = use_state(|| None::<PathBuf>);
        let dwell = theme.tokens.timing_tooltip_ms as u64;
        use_side_effect(move || {
            let over = hover.read().clone();
            tip.set(None);
            if let Some(path) = over {
                spawn(async move {
                    crate::sleep(std::time::Duration::from_millis(dwell)).await;
                    if hover.peek().as_ref() == Some(&path) {
                        tip.set(Some(path));
                    }
                });
            }
        });
        let controller = use_scroll_controller(ScrollConfig::default);
        let open = use_state(std::collections::BTreeSet::<PathBuf>::new);
        let reduced = theme.appearance.reduce_motion;
        let mut grow = use_animation(|_| AnimNum::new(0., 1.).time(300).function(Function::Cubic));
        let zoom_key = self.zoom.map(|(r, open)| (r.x, r.y, r.w, r.h, open));
        use_side_effect_with_deps(&zoom_key, move |z| {
            if z.is_some() {
                grow.start();
            }
        });
        let p = if self.zoom.is_some() && *grow.is_running().read() {
            grow.get().value()
        } else {
            1.
        };
        let on_act = self.on_act.clone();

        let crumbs = crumbs(&self.snapshot.roots, self.folder.as_deref());
        let crumb_count = crumbs.len();
        let path_bar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .background(colour(t.colour_view))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth {
                        top: 0.,
                        right: 0.,
                        bottom: t.stroke_hairline,
                        left: 0.,
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .child(Glyph::new("hard-drive", 16., colour(t.colour_text_2)))
            .children(
                crumbs
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, (path, name))| {
                        let last = i + 1 == crumb_count;
                        let name = if i == 0 {
                            self.scope_title.clone()
                        } else {
                            name
                        };
                        let on = on_act.clone();
                        let crumb = rect()
                            .a11y_role(AccessibilityRole::Button)
                            .a11y_alt(name.clone())
                            .on_press(move |_| on.call(Act::Open(path.clone(), None)))
                            .child(text::line(
                                &theme,
                                if last {
                                    text::Style::Headline
                                } else {
                                    text::Style::Body
                                },
                                colour(if last { t.colour_text } else { t.colour_text_2 }),
                                name,
                            ))
                            .into_element();
                        let sep = (i > 0).then(|| {
                            Glyph::new("caret-right", 12., colour(t.colour_text_2)).into_element()
                        });
                        sep.into_iter()
                            .chain(std::iter::once(crumb))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            )
            .child(rect().width(Size::flex(1.)))
            .child(mono(&theme, t.colour_text_2, size(total)));

        let body = if self.list {
            // Open folders' children follow them, a level deeper.
            let mut listed: Vec<(Entry, u16, u64)> = Vec::new();
            fn walk(
                snapshot: &Snapshot,
                open: &std::collections::BTreeSet<PathBuf>,
                items: Vec<Entry>,
                of: u64,
                depth: u16,
                out: &mut Vec<(Entry, u16, u64)>,
            ) {
                for e in items {
                    let is_open = e.node.is_some() && open.contains(&e.path);
                    let path = e.path.clone();
                    out.push((e, depth, of));
                    if is_open {
                        let (children, total) = crate::space::entries(snapshot, Some(&path));
                        walk(snapshot, open, children, total, depth + 1, out);
                    }
                }
            }
            walk(
                &self.snapshot,
                &open.read(),
                entries.clone(),
                total,
                0,
                &mut listed,
            );
            let opened = open.read().clone();
            let rows: Vec<Row> = listed
                .iter()
                .enumerate()
                .map(|(i, (e, depth, of))| Row {
                    key: i as u64,
                    depth: *depth,
                    disclosure: e.node.map(|_| opened.contains(&e.path)),
                    name: e.name.clone(),
                    mark: Some(Mark {
                        fill: kind_colour(&theme, e.kind),
                        glyph: format!("{}-fill", kind_glyph(e.kind)),
                        alt: e.kind.name().into(),
                    }),
                    check: Some(self.basket.covers(&e.path)),
                    cells: vec![
                        Cell::from(size(e.bytes)),
                        Cell {
                            text: format!("{:.0}%", e.bytes as f64 * 100. / (*of).max(1) as f64),
                            bar: Some((
                                e.bytes as f32 / (*of).max(1) as f32,
                                kind_colour(&theme, e.kind),
                            )),
                            ..Default::default()
                        },
                        Cell::from(e.kind.name().to_owned()),
                    ],
                    ..Default::default()
                })
                .collect();
            let selected: Vec<u64> = listed
                .iter()
                .position(|(e, _, _)| Some(&e.path) == self.selected.as_ref())
                .map(|i| vec![i as u64])
                .unwrap_or_default();
            let (list_entries, on) = (
                listed.into_iter().map(|(e, _, _)| e).collect::<Vec<_>>(),
                on_act.clone(),
            );
            Table {
                columns: Rc::new(vec![
                    Column::text("name", "Name", 280.),
                    Column::figures("size", "Size", 96.),
                    Column::text("share", "Share", 80.),
                    Column::text("kind", "Kind", 180.),
                ]),
                rows: Rc::new(rows),
                sort: Sort {
                    column: "size",
                    descending: true,
                },
                selected: Rc::new(selected),
                focused: self.active,
                hold: false,
                on_event: EventHandler::new(move |ev: TableEvent| {
                    let at = |k: u64| list_entries.get(k as usize).cloned();
                    match ev {
                        TableEvent::Selected(keys) => on.call(Act::Select(
                            keys.last().and_then(|k| at(*k)).map(|e| e.path),
                        )),
                        TableEvent::Open(k) => {
                            if let Some(e) = at(k).filter(|e| e.node.is_some()) {
                                on.call(Act::Open(Some(e.path), None));
                            }
                        }
                        TableEvent::Disclose(k, is_open) => {
                            if let Some(e) = at(k) {
                                let mut o = open;
                                if is_open {
                                    o.write().insert(e.path);
                                } else {
                                    o.write().remove(&e.path);
                                }
                            }
                        }
                        TableEvent::Check(k, ticked) => {
                            if let Some(e) = at(k) {
                                on.call(Act::Basket(e, ticked));
                            }
                        }
                        TableEvent::Menu(Some(k), at_point) => {
                            if let Some(e) = at(k) {
                                on.call(Act::Menu(e, at_point));
                            }
                        }
                        _ => {}
                    }
                }),
                controller,
            }
            .into_element()
        } else {
            let (w, h) = *map_size.read();
            let area = Rect { x: 0., y: 0., w, h };
            let sizes: Vec<u64> = entries.iter().map(|e| e.bytes).collect();
            let kept = treemap::kept(&sizes, area, t.size_tile_min);
            let mut shown: Vec<u64> = sizes[..kept].to_vec();
            let rest = entries.len() - kept;
            if rest > 0 {
                shown.push(sizes[kept..].iter().sum());
            }
            let rects = treemap::layout(&shown, area);
            // Opening grows the tile's rectangle to the map's; going up shrinks the map into it.
            let through = match self.zoom {
                Some((from, true)) if !reduced => Some(Rect::lerp(from, area, p)),
                Some((into, false)) if !reduced => Some(Rect::lerp(
                    Rect {
                        x: -into.x * area.w / into.w.max(1.),
                        y: -into.y * area.h / into.h.max(1.),
                        w: area.w * area.w / into.w.max(1.),
                        h: area.h * area.h / into.h.max(1.),
                    },
                    area,
                    p,
                )),
                _ => None,
            };
            let labels = if self.zoom.is_some() {
                ((p - 0.5) * 2.).clamp(0., 1.)
            } else {
                1.
            };
            let opacity = if reduced && self.zoom.is_some() {
                p
            } else {
                1.
            };
            let hovered = hover.read().clone();
            // `conventions.md`'s tooltip: the name, then size and kind. After a rest on a tile,
            // by the pointer; at once for a selected tile too small for its label, under it.
            let scale = theme.appearance.text_scale.max(1.);
            let unlabelled = |r: &Rect| {
                r.w < t.size_tile_label_width * scale || r.h < t.size_tile_label_height * scale
            };
            let placed = |path: &PathBuf| {
                entries
                    .iter()
                    .zip(&rects)
                    .take(kept)
                    .find(|(e, _)| &e.path == path)
                    .map(|(e, r)| (e.clone(), *r))
            };
            let tooltip = match (tip.read().clone(), self.selected.clone()) {
                (Some(path), _) => placed(&path).map(|(e, _)| {
                    let (x, y) = *pointer.read();
                    (e, (x + 12., y + 18.))
                }),
                (None, Some(path)) => placed(&path)
                    .filter(|(_, r)| unlabelled(r))
                    .map(|(e, r)| (e, (r.x, r.y + r.h + t.space_xs))),
                _ => None,
            }
            .map(|(e, (x, y))| {
                let (w, h) = *map_size.read();
                rect()
                    .position(
                        Position::new_absolute()
                            .left(x.min(w - 240.).max(0.))
                            .top(y.min(h - 28.).max(0.)),
                    )
                    .padding((3., t.space_s))
                    .corner_radius(t.radius_small)
                    .background(colour(t.colour_window_raised))
                    .maybe(true, |el| {
                        isle_ui::shadows(&t.shadow_menu)
                            .into_iter()
                            .fold(el, |el, s| el.shadow(s))
                    })
                    .horizontal()
                    .spacing(t.space_s)
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text),
                        e.name.clone(),
                    ))
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        format!("{} · {}", size(e.bytes), e.kind.name()),
                    ))
                    .into_element()
            });
            let tiles: Vec<Element> = rects
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let r = through.map_or(*r, |to| r.mapped(area, to));
                    let e = entries.get(i).filter(|_| i < kept);
                    let path = e.map(|e| e.path.clone());
                    let selected = path.is_some() && path == self.selected;
                    let in_basket = e.is_some_and(|e| self.basket.covers(&e.path));
                    let is_hovered = path.is_some() && path == hovered;
                    tile(
                        &theme,
                        e,
                        rest,
                        r,
                        labels,
                        selected,
                        is_hovered,
                        in_basket,
                        self.active,
                    )
                })
                .collect();
            let (hit_entries, hit_rects, on, kept_n) =
                (entries.clone(), rects.clone(), on_act.clone(), kept);
            let at = move |x: f32, y: f32| -> Option<(Entry, Rect)> {
                hit_rects
                    .iter()
                    .take(kept_n)
                    .position(|r| r.contains(x, y))
                    .and_then(|i| Some((hit_entries.get(i)?.clone(), hit_rects[i])))
            };
            let (at_press, at_move, at_menu) = (at.clone(), at.clone(), at.clone());
            let on_menu = on.clone();
            let on_key = {
                let placed: Vec<(PathBuf, Rect)> = entries
                    .iter()
                    .zip(&rects)
                    .take(kept)
                    .map(|(e, r)| (e.path.clone(), *r))
                    .collect();
                let (keyed, selected, basket, on) = (
                    entries.clone(),
                    self.selected.clone(),
                    self.basket.clone(),
                    on_act.clone(),
                );
                move |e: Event<KeyboardEventData>| {
                    if e.modifiers.contains(Modifiers::META)
                        || e.modifiers.contains(Modifiers::CONTROL)
                    {
                        return;
                    }
                    let current = selected
                        .clone()
                        .or_else(|| placed.first().map(|(p, _)| p.clone()));
                    let Some(current) = current else { return };
                    let towards = match &e.key {
                        Key::Named(NamedKey::ArrowLeft) => Some((-1., 0.)),
                        Key::Named(NamedKey::ArrowRight) => Some((1., 0.)),
                        Key::Named(NamedKey::ArrowUp) => Some((0., -1.)),
                        Key::Named(NamedKey::ArrowDown) => Some((0., 1.)),
                        _ => None,
                    };
                    if let Some((dx, dy)) = towards {
                        let next = if selected.is_none() {
                            Some(current)
                        } else {
                            nearest(&placed, &current, dx, dy)
                        };
                        if let Some(next) = next {
                            e.stop_propagation();
                            on.call(Act::Select(Some(next)));
                        }
                        return;
                    }
                    let Some(entry) = keyed.iter().find(|x| Some(&x.path) == selected.as_ref())
                    else {
                        return;
                    };
                    match &e.key {
                        Key::Character(c) if c == " " => {
                            e.stop_propagation();
                            on.call(Act::Basket(entry.clone(), !basket.covers(&entry.path)));
                        }
                        Key::Named(NamedKey::Enter) if entry.node.is_some() => {
                            let from = placed
                                .iter()
                                .find(|(p, _)| p == &entry.path)
                                .map(|(_, r)| (*r, true));
                            on.call(Act::Open(Some(entry.path.clone()), from));
                        }
                        _ => {}
                    }
                }
            };
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .padding(t.space_l)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::fill())
                        .opacity(opacity)
                        .on_sized(move |e: Event<SizedEventData>| {
                            map_size.set_if_modified((e.area.width(), e.area.height()))
                        })
                        .on_mouse_move(move |e: Event<MouseEventData>| {
                            let (x, y) = (e.element_location.x as f32, e.element_location.y as f32);
                            pointer.set((x, y));
                            hover.set_if_modified(at_move(x, y).map(|(e, _)| e.path));
                        })
                        .on_pointer_leave(move |_| hover.set_if_modified(None))
                        .on_global_key_down(on_key)
                        .on_mouse_down(move |e: Event<MouseEventData>| {
                            let (x, y) = (e.element_location.x as f32, e.element_location.y as f32);
                            tip.set(None);
                            let hit = at_press(x, y);
                            if e.button == Some(MouseButton::Right) {
                                if let Some((hit, _)) = at_menu(x, y) {
                                    on_menu.call(Act::Menu(
                                        hit,
                                        (e.global_location.x as f32, e.global_location.y as f32),
                                    ));
                                }
                                return;
                            }
                            if EventsCombos::<MapClick>::pressed(e.global_location).is_double() {
                                if let Some((hit, from)) = hit.filter(|(h, _)| h.node.is_some()) {
                                    on.call(Act::Open(Some(hit.path), Some((from, true))));
                                }
                                return;
                            }
                            on.call(Act::Select(hit.map(|(h, _)| h.path)));
                        })
                        .children(tiles)
                        .maybe_child(tooltip),
                )
                .into_element()
        };
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::flex())
            .background(colour(t.colour_view))
            .child(path_bar)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .child(body),
            )
    }
}

#[derive(Clone, Copy, PartialEq)]
struct MapClick;

/// The nearest entry from `from` in a direction, for the arrows.
pub fn nearest(rects: &[(PathBuf, Rect)], from: &Path, dx: f32, dy: f32) -> Option<PathBuf> {
    let (_, here) = rects.iter().find(|(p, _)| p == from)?;
    let (cx, cy) = here.centre();
    rects
        .iter()
        .filter(|(p, _)| p != from)
        .filter(|(_, r)| {
            let (x, y) = r.centre();
            (x - cx) * dx + (y - cy) * dy > 0.
        })
        .min_by(|(_, a), (_, b)| {
            let d = |r: &Rect| {
                let (x, y) = r.centre();
                (x - cx).powi(2) + (y - cy).powi(2)
            };
            d(a).total_cmp(&d(b))
        })
        .map(|(p, _)| p.clone())
}
