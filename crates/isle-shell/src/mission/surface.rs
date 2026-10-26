//! Mission Control's two surfaces on each screen, either side of the windows the compositor
//! draws as tiles: the bar, tray and header behind them, the icons, rings, titles and badges
//! in front (`p1/surfaces/mission-control.md`; P16 choice 9).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Glass, Glyph, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::model::{Badge, Place, Rect, ScreenLayout, Target, TrayHead};
use crate::{
    Here,
    surface::{Layer, Scope, SurfaceDef},
    switcher::surface::PreviewImage,
};

/// A desktop as the bar names it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeskFace {
    pub label: String,
    pub current: bool,
    /// The icons leading the label: a full-screen desktop's app, a Split View's two.
    pub icons: Vec<Option<String>>,
    /// A game's desktop leads with a cover.
    pub game: bool,
}

/// A window as the tiles, tray and capsules name it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowFace {
    pub id: u64,
    pub title: String,
    pub app: String,
    pub icon: Option<String>,
    pub game: bool,
}

/// What one screen's two surfaces show now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScreenFrame {
    pub layout: ScreenLayout,
    pub desks: Vec<DeskFace>,
    pub windows: Vec<WindowFace>,
    /// Each desktop's miniature, by its index.
    pub miniatures: Vec<(usize, PreviewImage)>,
    /// Each tray window's thumbnail.
    pub thumbs: Vec<(u64, PreviewImage)>,
    pub selection: Option<Target>,
    pub hover: Option<Target>,
    pub badge: Option<Badge>,
    /// Where a drag would land, on this screen.
    pub drop: Option<Place>,
    pub dragging: Option<u64>,
    /// App Exposé's app: its name and icon.
    pub header: Option<(String, Option<String>)>,
    /// Each cluster's name and icon, by its key.
    pub clusters: Vec<(String, String, Option<String>)>,
    /// Solid and still: Game Mode or Reduce Transparency.
    pub solid: bool,
    pub looking: Option<u64>,
    /// Split View's chooser is open: candidates too wide for the half, and no closing.
    pub chooser: bool,
    pub misfits: Vec<u64>,
}

impl ScreenFrame {
    fn window(&self, id: u64) -> Option<&WindowFace> {
        self.windows.iter().find(|w| w.id == id)
    }

    fn miniature(&self, desk: usize) -> Option<&PreviewImage> {
        self.miniatures
            .iter()
            .find(|(d, _)| *d == desk)
            .map(|(_, p)| p)
    }

    fn thumb(&self, id: u64) -> Option<&PreviewImage> {
        self.thumbs.iter().find(|(w, _)| *w == id).map(|(_, p)| p)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Back,
    Front,
}

/// One of the two surfaces; each screen's instance draws that screen's frame.
pub struct MissionSurface {
    part: Part,
    frames: Rc<RefCell<HashMap<String, ScreenFrame>>>,
    states: Rc<RefCell<HashMap<String, State<ScreenFrame>>>>,
    still: std::cell::Cell<bool>,
}

impl MissionSurface {
    pub fn back() -> Rc<Self> {
        Rc::new(Self::new(Part::Back))
    }

    pub fn front() -> Rc<Self> {
        Rc::new(Self::new(Part::Front))
    }

    fn new(part: Part) -> Self {
        Self {
            part,
            frames: Rc::default(),
            states: Rc::default(),
            still: Default::default(),
        }
    }

    /// Keeps `frame` for `output`'s next tree; safe anywhere.
    pub fn set_model(&self, output: &str, frame: ScreenFrame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
    }

    /// Puts `frame` in force on `output`; call it in that instance's root scope.
    pub fn set(&self, output: &str, frame: ScreenFrame) {
        self.frames
            .borrow_mut()
            .insert(output.to_owned(), frame.clone());
        if let Some(mut state) = self.states.borrow().get(output).copied() {
            state.set_if_modified(frame);
        }
    }

    pub fn clear(&self) {
        self.frames.borrow_mut().clear();
    }

    pub fn set_still(&self, still: bool) {
        self.still.set(still);
    }
}

impl SurfaceDef for MissionSurface {
    fn name(&self) -> &'static str {
        match self.part {
            Part::Back => "mission-back",
            Part::Front => "mission-front",
        }
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    /// The front takes the keyboard so typing reaches no window; the compositor reads the
    /// keys before it does.
    fn takes_keyboard(&self) -> bool {
        self.part == Part::Front
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(MissionApp {
            part: self.part,
            frames: self.frames.clone(),
            states: self.states.clone(),
        })
    }

    /// The pointer never reaches them: the compositor reads it for the model.
    fn hit_shapes(&self) -> bool {
        true
    }

    fn fades(&self) -> bool {
        true
    }

    fn transitions(&self) -> bool {
        !self.still.get()
    }
}

#[derive(Clone)]
struct MissionApp {
    part: Part,
    frames: Rc<RefCell<HashMap<String, ScreenFrame>>>,
    states: Rc<RefCell<HashMap<String, State<ScreenFrame>>>>,
}

impl App for MissionApp {
    fn render(&self) -> impl IntoElement {
        let here = use_consume::<Here>().0;
        use_drop({
            let (states, here) = (self.states.clone(), here.clone());
            move || {
                states.borrow_mut().remove(&here);
            }
        });
        let state = use_hook(|| {
            let frame = self.frames.borrow().get(&here).cloned().unwrap_or_default();
            let state = State::create(frame);
            self.states.borrow_mut().insert(here.clone(), state);
            state
        });
        Screen {
            part: self.part,
            frame: state,
        }
    }
}

#[derive(PartialEq)]
struct Screen {
    part: Part,
    frame: State<ScreenFrame>,
}

impl Component for Screen {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let frame = self.frame.read().clone();
        let root = rect().expanded();
        match self.part {
            Part::Back => back(&theme, &frame, root),
            Part::Front => front(&theme, &frame, root),
        }
    }
}

/// Over the miniatures and thumbnails, which draw in a layer of their own for their clip.
const ABOVE: freya::prelude::Layer = freya::prelude::Layer::Relative(2);

/// A rect placed at `r` in the screen.
fn at(r: &Rect) -> freya::prelude::Rect {
    rect()
        .position(Position::new_absolute().left(r.x).top(r.y))
        .width(Size::px(r.w))
        .height(Size::px(r.h))
}

fn grown(r: &Rect, by: f32) -> Rect {
    Rect::new(r.x - by, r.y - by, r.w + 2.0 * by, r.h + 2.0 * by)
}

/// A glass panel, or solid in Game Mode and under Reduce Transparency.
fn panel(theme: &Theme, r: &Rect, radius: f32, solid: bool) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let contrast = theme.appearance.increase_contrast;
    at(r)
        .corner_radius(radius)
        .background(if solid {
            colour(t.material_solid)
        } else if contrast {
            colour(t.material_glass_tinted)
        } else {
            Color::TRANSPARENT
        })
        .map(Some(shadows(&t.shadow_panel)), |el, list| {
            list.into_iter().fold(el, |el, s| el.shadow(s))
        })
        .maybe(contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(1.0)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .maybe(!solid, |el| el.child(Glass::new(radius)))
}

/// An app's icon at `size`, a game's cover in its place, a generic glyph without one.
fn icon(theme: &Theme, face: Option<&WindowFace>, size: f32) -> Element {
    let t = &theme.tokens;
    match face {
        Some(f) if f.game => rect()
            .width(Size::px((size * 2.0 / 3.0).round()))
            .height(Size::px(size))
            .corner_radius(t.radius_small)
            .background(colour(t.colour_fill_strong))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(1.0)
                    .alignment(BorderAlignment::Inner),
            )
            .maybe_child(f.icon.clone().map(|p| AppIcon::new(p, size * 2.0 / 3.0)))
            .into_element(),
        Some(WindowFace {
            icon: Some(path), ..
        }) => AppIcon::new(path.clone(), size).into_element(),
        _ => Glyph::new("app-window", size * 0.75, colour(t.colour_text_2)).into_element(),
    }
}

/// The capsule a title or a name sits in.
fn capsule(
    theme: &Theme,
    cx: f32,
    y: f32,
    h: f32,
    max: f32,
    solid: bool,
    kids: Vec<Element>,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    // A full-width row centres the capsule on `cx` whatever its width.
    let span = max.max(1.0);
    rect()
        .position(Position::new_absolute().left(cx - span / 2.0).top(y))
        .width(Size::px(span))
        .height(Size::px(h))
        .direction(Direction::Horizontal)
        .main_align(Alignment::Center)
        .child(
            rect()
                .height(Size::px(h))
                .padding((0.0, t.space_s))
                .corner_radius(h / 2.0)
                .direction(Direction::Horizontal)
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .background(if solid {
                    colour(t.material_solid)
                } else {
                    Color::TRANSPARENT
                })
                .map(Some(shadows(&t.shadow_control)), |el, list| {
                    list.into_iter().fold(el, |el, s| el.shadow(s))
                })
                .maybe(!solid, |el| el.child(Glass::new(h / 2.0).front()))
                .children(kids),
        )
}

/// A badge's circle centred on `(x, y)`.
fn badge(theme: &Theme, x: f32, y: f32, glyph: &str) -> freya::prelude::Rect {
    let t = &theme.tokens;
    rect()
        .layer(ABOVE)
        .position(Position::new_absolute().left(x - 10.0).top(y - 10.0))
        .width(Size::px(20.0))
        .height(Size::px(20.0))
        .corner_radius(10.0)
        .background(colour(t.material_solid))
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(1.0)
                .alignment(BorderAlignment::Inner),
        )
        .map(Some(shadows(&t.shadow_control)), |el, list| {
            list.into_iter().fold(el, |el, s| el.shadow(s))
        })
        .center()
        .child(Glyph::new(glyph, 12.0, colour(t.colour_text)).fill(true))
}

fn image_in(handle: &PreviewImage, r: &Rect, radius: f32, alpha: f32) -> freya::prelude::Rect {
    let fitted = super::model::fit_in(
        handle.0.image.width() as f32,
        handle.0.image.height() as f32,
        *r,
    );
    at(&fitted)
        .corner_radius(radius)
        .overflow(Overflow::Clip)
        .opacity(alpha)
        .child(
            image(handle.0.clone())
                .width(Size::px(fitted.w))
                .height(Size::px(fitted.h)),
        )
}

fn back(theme: &Theme, frame: &ScreenFrame, mut root: freya::prelude::Rect) -> Element {
    let t = &theme.tokens;
    let l = &frame.layout;
    let solid = frame.solid;
    if let Some(bar) = &l.bar {
        let mut p = panel(theme, &bar.panel, t.radius_panel, solid).overflow(Overflow::Clip);
        // Children are placed in screen points; the panel's origin is taken off.
        let local = |r: &Rect| Rect::new(r.x - bar.panel.x, r.y - bar.panel.y, r.w, r.h);
        for item in &bar.items {
            let m = local(&item.miniature);
            let label = local(&item.label);
            let target = match item.desk {
                Some(index) => Target::Desk { screen: 0, index },
                None => Target::Plus { screen: 0 },
            };
            let picked = frame.selection.as_ref().is_some_and(|s| same(s, &target))
                || frame.hover.as_ref().is_some_and(|s| same(s, &target));
            let dropping = match (&frame.drop, item.desk) {
                (Some(Place::Desk { index, .. }), Some(d)) => *index == d,
                (Some(Place::New { .. }), None) => true,
                _ => false,
            };
            if (picked || dropping) && item.desk.is_some() {
                let plate = Rect::new(m.x, m.y, m.w, label.bottom() - m.y);
                p = p.child(
                    at(&grown(&plate, t.space_xs))
                        .corner_radius(t.radius_card)
                        .background(colour(t.colour_fill_strong)),
                );
            }
            match item.desk {
                Some(index) => {
                    let face = frame.desks.get(index).cloned().unwrap_or_default();
                    let mut mini = at(&m)
                        .corner_radius(t.radius_control)
                        .overflow(Overflow::Clip)
                        .background(colour(t.colour_window_sunken));
                    if let Some(img) = frame.miniature(index) {
                        mini = mini.child(image_in(img, &Rect::new(0.0, 0.0, m.w, m.h), 0.0, 1.0));
                    }
                    if let Some(Place::Split { index: d, left, .. }) = &frame.drop
                        && *d == index
                    {
                        let half = Rect::new(
                            if *left { 2.0 } else { m.w / 2.0 },
                            2.0,
                            m.w / 2.0 - 2.0,
                            m.h - 4.0,
                        );
                        mini = mini.child(
                            at(&half)
                                .corner_radius(t.radius_small)
                                .background(colour(t.colour_fill_strong))
                                .border(
                                    Border::new()
                                        .fill(colour(t.colour_hairline_strong))
                                        .width(1.0)
                                        .alignment(BorderAlignment::Inner),
                                ),
                        );
                    }
                    p = p.child(mini);
                    if face.current {
                        p = p.child(
                            at(&grown(&m, t.stroke_focus_offset + t.stroke_found))
                                .layer(ABOVE)
                                .corner_radius(
                                    t.radius_control + t.stroke_focus_offset + t.stroke_found,
                                )
                                .border(
                                    Border::new()
                                        .fill(colour(t.colour_text))
                                        .width(t.stroke_found)
                                        .alignment(BorderAlignment::Inner),
                                ),
                        );
                    }
                    let ink = if face.current {
                        t.colour_text
                    } else {
                        t.colour_text_2
                    };
                    let mut row = at(&label)
                        .direction(Direction::Horizontal)
                        .main_align(Alignment::Center)
                        .cross_align(Alignment::Center)
                        .spacing(t.space_xs);
                    if face.game {
                        row = row.child(
                            rect()
                                .width(Size::px(11.0))
                                .height(Size::px(16.0))
                                .corner_radius(2.0)
                                .background(colour(t.colour_fill_strong)),
                        );
                    }
                    for path in face.icons.iter().flatten() {
                        row = row.child(AppIcon::new(path.clone(), 16.0));
                    }
                    p = p.child(row.child(text::line(
                        theme,
                        text::Style::Callout,
                        colour(ink),
                        face.label.clone(),
                    )));
                    if let Some(Badge::Remove { index: b, .. }) = &frame.badge
                        && *b == index
                    {
                        let exit = !matches!(face.icons.len(), 0) || face.game;
                        p = p.child(badge(
                            theme,
                            m.x,
                            m.y,
                            if exit { "arrows-in-simple" } else { "x" },
                        ));
                    } else if frame.hover.as_ref().is_some_and(|h| same(h, &target))
                        && frame
                            .desks
                            .iter()
                            .filter(|d| d.icons.is_empty() && !d.game)
                            .count()
                            > 1
                    {
                        let exit = !face.icons.is_empty() || face.game;
                        p = p.child(badge(
                            theme,
                            m.x,
                            m.y,
                            if exit { "arrows-in-simple" } else { "x" },
                        ));
                    }
                }
                None => {
                    let on = picked || dropping;
                    p = p.child(
                        at(&m)
                            .corner_radius(t.radius_control)
                            .background(colour(if on {
                                t.colour_fill_strong
                            } else {
                                t.colour_fill
                            }))
                            .center()
                            .child(Glyph::new(
                                "plus",
                                t.size_glyph_large,
                                colour(if on { t.colour_text } else { t.colour_text_2 }),
                            )),
                    );
                }
            }
        }
        if bar.overflow {
            let track = Rect::new(
                t.space_m,
                bar.panel.h - 6.0,
                bar.panel.w - 2.0 * t.space_m,
                3.0,
            );
            p = p.child(
                at(&track)
                    .corner_radius(1.5)
                    .background(colour(t.colour_fill)),
            );
        }
        root = root.child(p);
    }
    if let (Some(h), Some((name, path))) = (&l.header, &frame.header) {
        let face = WindowFace {
            icon: path.clone(),
            ..Default::default()
        };
        root = root.child(capsule(
            theme,
            h.centre().0,
            h.y,
            h.h,
            h.w * 3.0,
            solid,
            vec![
                icon(theme, Some(&face), 32.0),
                text::line(
                    theme,
                    text::Style::Title3,
                    colour(t.colour_text),
                    name.clone(),
                )
                .into_element(),
            ],
        ));
    }
    if let Some(tray) = &l.tray {
        let mut p = panel(theme, &tray.panel, t.radius_panel, solid);
        let local = |r: &Rect| Rect::new(r.x - tray.panel.x, r.y - tray.panel.y, r.w, r.h);
        for rule in &tray.rules {
            p = p.child(at(&local(rule)).background(colour(
                if theme.appearance.increase_contrast {
                    t.colour_hairline_strong
                } else {
                    t.colour_hairline
                },
            )));
        }
        for g in &tray.groups {
            let head = match &g.head {
                TrayHead::Minimised => "Minimised".to_owned(),
                TrayHead::Hidden => "Hidden".to_owned(),
                TrayHead::Desktop(n) => format!("Desktop {n}"),
            };
            p = p.child(
                at(&local(&g.heading)).child(
                    text::line(
                        theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        head,
                    )
                    .font_weight(600),
                ),
            );
            let hidden = g.head == TrayHead::Hidden;
            for (id, slot) in &g.items {
                let s = local(slot);
                let mut well = at(&s);
                if hidden {
                    well = well
                        .corner_radius(t.radius_small)
                        .background(colour(t.colour_fill));
                }
                p = p.child(well);
                if let Some(img) = frame.thumb(*id) {
                    p = p.child(image_in(
                        img,
                        &s,
                        t.radius_small,
                        if hidden { 0.5 } else { 1.0 },
                    ));
                }
                let target = Target::Tray {
                    screen: 0,
                    window: *id,
                };
                if frame.selection.as_ref().is_some_and(|x| same(x, &target))
                    || frame.hover.as_ref().is_some_and(|x| same(x, &target))
                {
                    let fitted = frame.thumb(*id).map_or(s, |img| {
                        super::model::fit_in(
                            img.0.image.width() as f32,
                            img.0.image.height() as f32,
                            s,
                        )
                    });
                    p = p.child(ring(theme, &fitted, t.radius_small));
                }
                p = p.child(
                    rect()
                        .layer(ABOVE)
                        .position(
                            Position::new_absolute()
                                .left(s.x + s.w / 2.0 - 16.0)
                                .top(s.bottom() - 16.0),
                        )
                        .child(icon(theme, frame.window(*id), 32.0)),
                );
            }
        }
        root = root.child(p);
    }
    root.into_element()
}

/// The selection's ring: the screen's one accent mark.
fn ring(theme: &Theme, r: &Rect, radius: f32) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let by = t.stroke_focus_offset + t.stroke_focus_ring;
    at(&grown(r, by))
        .layer(ABOVE)
        .corner_radius(radius + by)
        .border(
            Border::new()
                .fill(colour(t.colour_accent))
                .width(t.stroke_focus_ring)
                .alignment(BorderAlignment::Inner),
        )
}

/// Targets compared without their screen: each screen's frame holds only its own.
fn same(a: &Target, b: &Target) -> bool {
    match (a, b) {
        (Target::Tile { window: x, .. }, Target::Tile { window: y, .. }) => x == y,
        (Target::Tray { window: x, .. }, Target::Tray { window: y, .. }) => x == y,
        (Target::Desk { index: x, .. }, Target::Desk { index: y, .. }) => x == y,
        (Target::Plus { .. }, Target::Plus { .. }) => true,
        _ => false,
    }
}

fn front(theme: &Theme, frame: &ScreenFrame, mut root: freya::prelude::Rect) -> Element {
    let t = &theme.tokens;
    let l = &frame.layout;
    let grouped = !l.clusters.is_empty();
    for tile in &l.tiles {
        if frame.dragging == Some(tile.window) || frame.looking == Some(tile.window) {
            continue;
        }
        let r = tile.rect;
        let face = frame.window(tile.window);
        let target = Target::Tile {
            screen: 0,
            window: tile.window,
        };
        let misfit = frame.misfits.contains(&tile.window);
        let picked = frame.picked_any(&target) && !misfit;
        let hovered_misfit = misfit && frame.hover.as_ref().is_some_and(|h| same(h, &target));
        let radius = (t.radius_window * (r.w / 1000.0)).max(t.radius_small);
        if picked {
            root = root.child(ring(theme, &r, radius));
        }
        if !grouped {
            let w = if face.is_some_and(|f| f.game) {
                32.0
            } else {
                48.0
            };
            root = root.child(
                rect()
                    .position(
                        Position::new_absolute()
                            .left(r.centre().0 - w / 2.0)
                            .top(r.bottom() - 24.0),
                    )
                    .opacity(if misfit { 0.4 } else { 1.0 })
                    .child(icon(theme, face, 48.0)),
            );
        }
        if hovered_misfit {
            let y = r.bottom() + 24.0 + t.space_xs;
            root = root.child(capsule(
                theme,
                r.centre().0,
                y,
                t.size_control,
                r.w.max(t.size_menu_max),
                frame.solid,
                vec![
                    text::line(theme, text::Style::Body, colour(t.colour_text_2), "Doesn't fit in Split View")
                        .into_element(),
                ],
            ));
        }
        if picked && let Some(face) = face {
            let y = r.bottom()
                + if grouped {
                    t.space_xs
                } else {
                    24.0 + t.space_xs
                };
            root = root.child(capsule(
                theme,
                r.centre().0,
                y,
                t.size_control,
                r.w.max(t.size_menu_max),
                frame.solid,
                vec![
                    text::line(
                        theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        face.title.clone(),
                    )
                    .into_element(),
                ],
            ));
        }
        if frame.chooser {
            continue;
        }
        if let Some(Badge::Close { window, .. }) = &frame.badge
            && *window == tile.window
        {
            root = root.child(badge(theme, r.x, r.y, "x"));
        } else if frame.hover.as_ref().is_some_and(|h| same(h, &target)) {
            root = root.child(badge(theme, r.x, r.y, "x"));
        }
    }
    for cluster in &l.clusters {
        let (name, path) = frame
            .clusters
            .iter()
            .find(|(k, _, _)| *k == cluster.key)
            .map(|(_, n, p)| (n.clone(), p.clone()))
            .unwrap_or_else(|| (cluster.key.clone(), None));
        let face = WindowFace {
            icon: path,
            ..Default::default()
        };
        root = root.child(capsule(
            theme,
            cluster.centre_x,
            cluster.top,
            t.size_control_large,
            t.size_menu_max,
            frame.solid,
            vec![
                icon(theme, Some(&face), 16.0),
                text::line(theme, text::Style::Headline, colour(t.colour_text), name)
                    .into_element(),
            ],
        ));
    }
    if let Some(e) = &l.empty {
        let words = if frame.header.is_some() {
            "No windows"
        } else {
            "No other windows fit"
        };
        root = root.child(capsule(
            theme,
            e.centre().0,
            e.y,
            t.size_control,
            t.size_menu_max,
            frame.solid,
            vec![
                text::line(theme, text::Style::Body, colour(t.colour_text_2), words).into_element(),
            ],
        ));
    }
    // A tray thumbnail's title sits above the tray, never on its glass.
    if let Some(tray) = &l.tray {
        for g in &tray.groups {
            for (id, slot) in &g.items {
                let target = Target::Tray {
                    screen: 0,
                    window: *id,
                };
                if frame.picked_any(&target)
                    && let Some(face) = frame.window(*id)
                {
                    root = root.child(capsule(
                        theme,
                        slot.centre().0,
                        tray.panel.y - t.space_xs - t.size_control,
                        t.size_control,
                        t.size_menu_max,
                        frame.solid,
                        vec![
                            text::line(
                                theme,
                                text::Style::Body,
                                colour(t.colour_text),
                                face.title.clone(),
                            )
                            .into_element(),
                        ],
                    ));
                }
            }
        }
    }
    root.into_element()
}

impl ScreenFrame {
    fn picked_any(&self, t: &Target) -> bool {
        self.selection.as_ref().is_some_and(|s| same(s, t))
            || self.hover.as_ref().is_some_and(|s| same(s, t))
    }
}
