//! Spotlight's field and the panel under it: the results list, the Apps view's grid and the
//! emoji grid, drawn from a frame the compositor builds from the model (`p1/surfaces/spotlight.md`).

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    AppIcon, Glass, Glyph, Hit, Menu, MenuEntry, MenuEvent, colour, menu::Material, shadows, text,
    use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// A row's leading slot, resolved by the compositor.
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    /// An app's icon file.
    Icon(PathBuf),
    /// A game's cover or a file's thumbnail, kept in its own shape.
    Picture(PathBuf),
    Glyph(&'static str),
    /// An emoji or symbol, drawn as text.
    Text(String),
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowView {
    pub key: String,
    pub title: String,
    pub detail: Option<String>,
    pub trailing: Option<String>,
    pub slot: Slot,
    /// The title in the top hit's and the calculation's heavier style.
    pub strong: bool,
    pub badge: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LineView {
    Header(String),
    Row(RowView),
}

/// How the panel lays out its lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    List,
    /// The Apps view: tiles, this many columns.
    Apps(usize),
    /// The emoji grid, this many columns.
    Emoji(usize),
}

/// Where an open menu hangs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAnchor {
    /// Under the selected row or tile.
    Selected,
    /// Under the Apps view's category filter.
    Filter,
}

/// A menu open in Spotlight: a row's or tile's, or the category filter's.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuView {
    pub entries: Rc<Vec<MenuEntry>>,
    pub anchor: MenuAnchor,
    /// Opened from the keyboard, so its first row is selected.
    pub by_key: bool,
}

/// What Spotlight shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub text: String,
    /// The caret, in characters.
    pub caret: usize,
    /// The whole query is selected.
    pub all: bool,
    pub placeholder: String,
    /// The scope's token: its name, and an app's icon in the one-app scope.
    pub token: Option<(String, Option<PathBuf>)>,
    /// The quick key whose scope is on, 1 to 4.
    pub quick: Option<u8>,
    pub lines: Vec<LineView>,
    pub selected: Option<String>,
    pub searching: bool,
    /// The panel is up, at its full height.
    pub panel: bool,
    pub layout: Layout,
    /// The Files scope's switch: Some(contents).
    pub files: Option<bool>,
    /// The line shown in place of rows: "No results for …".
    pub empty: Option<String>,
    /// A system prompt holds the keyboard over Spotlight.
    pub unfocused: bool,
    /// Solid and still: Game Mode or Reduce Transparency.
    pub solid: bool,
    /// The quick keys' chords as the keyboard names them: "⌘1" or "Cmd+1".
    pub quick_chords: [String; 4],
    /// The Apps view's panel, fitted to the screen: width and height.
    pub apps_panel: (f32, f32),
    /// The Apps view's top bar: its title and the category filter's choice.
    pub apps_title: String,
    pub filter: String,
    pub menu: Option<MenuView>,
    /// The tile being dragged.
    pub dragging: Option<String>,
}

impl Frame {
    fn selected_index(&self) -> Option<usize> {
        let key = self.selected.as_ref()?;
        self.lines
            .iter()
            .position(|l| matches!(l, LineView::Row(r) if &r.key == key))
    }
}

/// What the pointer does in Spotlight.
#[derive(Clone, Debug, PartialEq)]
pub enum SpotlightAction {
    /// The pointer moved onto a row or tile.
    Hover(String),
    Click(String),
    /// Secondary click: the row's or tile's menu.
    Secondary(String),
    /// A quick-key button.
    Quick(u8),
    /// The Files scope's Names / Contents switch.
    Contents(bool),
    /// A press on a tile, which may become a drag.
    DragFrom(String),
    /// A release on a tile.
    DragTo(String),
    /// The Apps view's category filter.
    Filter,
    Menu(MenuEvent),
}

#[derive(Default)]
pub struct SpotlightSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
}

impl SpotlightSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Puts `frame` in force.
    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The Apps view's panel, which is the widest and tallest Spotlight gets, under a field
    /// at `top` on a screen `screen_height` tall.
    pub fn apps_panel(theme: &Theme, columns: usize, top: f32, screen_height: f32) -> (f32, f32) {
        let t = &theme.tokens;
        let width = columns as f32 * t.size_spotlight_tile + 2.0 * t.space_l;
        let below = top + t.size_spotlight_field + t.space_s;
        let room = (screen_height - t.space_xxxl - below - t.size_row_large - t.space_s)
            .max(t.size_spotlight_tile);
        let rows =
            ((room / t.size_spotlight_tile).floor() as usize).clamp(1, t.limit_apps_rows as usize);
        (
            width,
            t.size_row_large + rows as f32 * t.size_spotlight_tile + t.space_s,
        )
    }

    /// The surface's size: the field and the largest panel, with their shadows' reach.
    pub fn size_for(
        theme: &Theme,
        columns: usize,
        top: f32,
        screen_height: f32,
    ) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let (apps_w, apps_h) = Self::apps_panel(theme, columns, top, screen_height);
        let width = apps_w.max(t.size_spotlight_width);
        let height = t.size_spotlight_field + t.space_s + apps_h;
        (
            (width + 2.0 * m).ceil() as i32,
            (height + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for SpotlightSurface {
    fn name(&self) -> &'static str {
        "spotlight"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(self.size.get())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(SpotlightApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.5, 0.0)
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct SpotlightApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for SpotlightApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.frame.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        Root { frame: state }
    }
}

fn send(actions: &Actions, action: SpotlightAction) {
    actions.send(Action::Spotlight(action));
}

/// A glass shape with its shadow, or the solid material where glass is off.
fn shape(theme: &Theme, solid: bool, width: f32, height: f32) -> Rect {
    let t = &theme.tokens;
    rect()
        .width(Size::px(width))
        .height(Size::px(height))
        .corner_radius(t.radius_panel)
        .background(if solid {
            colour(t.material_solid)
        } else {
            Color::TRANSPARENT
        })
        .maybe(theme.appearance.increase_contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(1.)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .map(Some(shadows(&t.shadow_panel)), |el, list| {
            list.into_iter().fold(el, |el, s| el.shadow(s))
        })
        .maybe(!solid, |el| el.child(Glass::new(t.radius_panel)))
        .child(Hit)
}

#[derive(PartialEq)]
struct Root {
    frame: State<Frame>,
}

/// Where the selected row and the filter button are in the surface, for a menu to hang
/// under: their bottom-left corners.
#[derive(Clone, Copy, PartialEq)]
struct Anchors {
    selected: State<(f32, f32)>,
    filter: State<(f32, f32)>,
}

impl Component for Root {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let solid = frame.solid || theme.appearance.reduce_transparency;
        let anchors = use_hook(|| Anchors {
            selected: State::create((0., 0.)),
            filter: State::create((0., 0.)),
        });
        let field = Field {
            frame: frame.clone(),
            solid,
        };
        let menu = frame.menu.clone().map(|m| {
            let actions = Actions::get();
            let (x, y) = match m.anchor {
                MenuAnchor::Selected => *anchors.selected.read(),
                MenuAnchor::Filter => *anchors.filter.read(),
            };
            rect()
                .position(Position::new_absolute().left(x).top(y))
                .layer(freya::prelude::Layer::Overlay)
                .child(
                    Menu::new(m.entries, move |e| send(&actions, SpotlightAction::Menu(e)))
                        .material(Material::Solid)
                        .first_selected(m.by_key),
                )
        });
        let mut root = rect()
            .expanded()
            .padding(SpotlightSurface::margin(&theme))
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .a11y_role(AccessibilityRole::Dialog)
            .a11y_alt("Spotlight")
            .child(field);
        if frame.panel {
            root = root.child(match frame.layout {
                Layout::List => ListPanel {
                    frame,
                    solid,
                    anchors,
                }
                .into_element(),
                Layout::Apps(columns) | Layout::Emoji(columns) => GridPanel {
                    frame,
                    solid,
                    columns,
                    anchors,
                }
                .into_element(),
            });
        }
        rect().expanded().child(root).maybe_child(menu)
    }
}

#[derive(PartialEq)]
struct Field {
    frame: Frame,
    solid: bool,
}

impl Component for Field {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = &self.frame;
        let actions = Actions::get();
        let ink = colour(t.colour_text);
        let ink_2 = colour(t.colour_text_2);
        let size = t.type_style_title_2_size * theme.appearance.text_scale.min(t.type_scale_max);
        let line = t.type_style_title_2_line * theme.appearance.text_scale.min(t.type_scale_max);
        let query = |s: String, c: Color| {
            label()
                .font_size(size)
                .line_height(line / size)
                .color(c)
                .max_lines(1)
                .text(s)
        };
        let chars: Vec<char> = frame.text.chars().collect();
        let caret = frame.caret.min(chars.len());
        let before: String = chars[..caret].iter().collect();
        let after: String = chars[caret..].iter().collect();
        let caret_bar = rect()
            .width(Size::px(2.))
            .height(Size::px(line))
            .background(ink);
        let text: Element = if frame.text.is_empty() {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .child(caret_bar)
                .child(query(frame.placeholder.clone(), ink_2))
                .into_element()
        } else if frame.all {
            let mix = colour(t.colour_accent).with_a((t.selection_mix * 255.) as u8);
            rect()
                .background(mix)
                .corner_radius(t.radius_small)
                .child(query(frame.text.clone(), ink))
                .into_element()
        } else {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .child(query(before, ink))
                .child(caret_bar)
                .child(query(after, ink))
                .into_element()
        };
        let token = frame.token.as_ref().map(|(name, icon)| {
            rect()
                .height(Size::px(t.size_control))
                .padding((0., t.space_s))
                .corner_radius(t.size_control / 2.)
                .background(colour(t.colour_fill_strong))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .maybe_child(icon.clone().map(|i| AppIcon::new(i, t.size_app_icon_small)))
                .child(text::line(&theme, text::Style::Body, ink, name.clone()).font_weight(500))
        });
        let quick = frame.text.is_empty().then(|| {
            const KEYS: [(&str, &str); 4] = [
                ("squares-four", "Apps"),
                ("folder", "Files"),
                ("command", "Actions"),
                ("clipboard-text", "Clipboard"),
            ];
            rect()
                .horizontal()
                .spacing(t.space_xs)
                .children(KEYS.iter().enumerate().map(|(i, (glyph, name))| {
                    let n = i as u8 + 1;
                    let on = frame.quick == Some(n);
                    let press = actions.clone();
                    rect()
                        .width(Size::px(32.))
                        .height(Size::px(32.))
                        .corner_radius(t.radius_card)
                        .background(colour(if on {
                            t.colour_control_pressed
                        } else {
                            t.colour_fill
                        }))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt(format!("{name} {}", frame.quick_chords[i]))
                        .on_press(move |_| send(&press, SpotlightAction::Quick(n)))
                        .child(Glyph::new(*glyph, t.size_glyph_large, ink_2))
                        .into_element()
                }))
        });
        let activity = (frame.searching && !frame.text.is_empty())
            .then(|| Glyph::new("dots-three", t.size_glyph, ink_2));
        shape(
            &theme,
            self.solid,
            t.size_spotlight_width,
            t.size_spotlight_field,
        )
        .child(
            rect()
                .expanded()
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .padding((
                    0.,
                    t.space_s,
                    0.,
                    if token.is_some() {
                        t.space_m
                    } else {
                        t.space_l
                    },
                ))
                .spacing(t.space_s)
                .a11y_role(AccessibilityRole::SearchInput)
                .a11y_alt(if frame.text.is_empty() {
                    frame.placeholder.clone()
                } else {
                    frame.text.clone()
                })
                .maybe_child(token)
                .child(rect().width(Size::flex(1.)).child(text))
                .maybe_child(quick)
                .maybe_child(activity.map(|a| rect().padding((0., t.space_s)).child(a))),
        )
    }
}

/// Each line's height in the list: a header is a short row.
fn line_height(theme: &Theme, line: &LineView) -> f32 {
    match line {
        LineView::Header(_) => theme.tokens.size_row,
        LineView::Row(_) => theme.tokens.size_row_large,
    }
}

#[derive(PartialEq)]
struct ListPanel {
    frame: Frame,
    solid: bool,
    anchors: Anchors,
}

#[derive(Clone, PartialEq)]
struct ListData {
    lines: Rc<Vec<LineView>>,
    selected: Option<String>,
    unfocused: bool,
    anchors: Anchors,
}

impl Component for ListPanel {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = &self.frame;
        let strip = frame.files.map(|contents| FilesStrip { contents });
        let strip_h = if strip.is_some() {
            t.size_control + t.space_s
        } else {
            0.
        };
        let height = t.limit_results_rows as f32 * t.size_row_large + 2.0 * t.space_s + strip_h;
        let heights: Rc<Vec<f32>> =
            Rc::new(frame.lines.iter().map(|l| line_height(&theme, l)).collect());
        let item_size = {
            let heights = heights.clone();
            ItemSize::from(move |i: usize| heights.get(i).copied().unwrap_or(0.))
        };
        let mut controller = use_scroll_controller(ScrollConfig::default);
        if let Some(index) = frame.selected_index() {
            controller.scroll_to_index(index, &item_size, Direction::Vertical);
        }
        let data = ListData {
            lines: Rc::new(frame.lines.clone()),
            selected: frame.selected.clone(),
            unfocused: frame.unfocused,
            anchors: self.anchors,
        };
        let length = frame.lines.len();
        let body: Element = match &frame.empty {
            Some(empty) if frame.lines.is_empty() => rect()
                .padding((t.space_s, t.space_m))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    empty.clone(),
                ))
                .into_element(),
            _ => VirtualScrollView::new_with_data_controlled(
                data,
                |item: VirtualItem, data: &ListData| match &data.lines[item.index] {
                    LineView::Header(h) => Header { text: h.clone() }.into_element(),
                    LineView::Row(row) => ResultRow {
                        row: row.clone(),
                        selected: data.selected.as_ref() == Some(&row.key),
                        unfocused: data.unfocused,
                        anchors: data.anchors,
                    }
                    .into_element(),
                },
                controller,
            )
            .length(length)
            .item_size(item_size)
            .into_element(),
        };
        shape(&theme, self.solid, t.size_spotlight_width, height).child(
            rect()
                .expanded()
                .padding(t.space_s)
                .maybe_child(strip)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::flex(1.))
                        .corner_radius(t.radius_card)
                        .overflow(Overflow::Clip)
                        .child(body),
                ),
        )
    }
}

#[derive(PartialEq)]
struct FilesStrip {
    contents: bool,
}

impl Component for FilesStrip {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let segment = |label: &'static str, on: bool, contents: bool| {
            let press = actions.clone();
            rect()
                .height(Size::px(t.size_control))
                .padding((0., t.space_m))
                .corner_radius(t.radius_small)
                .background(if on {
                    colour(t.material_solid)
                } else {
                    Color::TRANSPARENT
                })
                .center()
                .a11y_role(AccessibilityRole::RadioButton)
                .on_press(move |_| send(&press, SpotlightAction::Contents(contents)))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    label,
                ))
        };
        rect().padding((0., t.space_s, t.space_s, t.space_s)).child(
            rect()
                .horizontal()
                .corner_radius(t.radius_small)
                .background(colour(t.colour_fill))
                .child(segment("Names", !self.contents, false))
                .child(segment("Contents", self.contents, true)),
        )
    }
}

#[derive(PartialEq)]
struct Header {
    text: String,
}

impl Component for Header {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .padding((0., t.space_s))
            .main_align(Alignment::End)
            .a11y_role(AccessibilityRole::Heading)
            .child(
                text::line(
                    &theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    self.text.clone(),
                )
                .font_weight(600),
            )
    }
}

#[derive(PartialEq)]
struct Leading {
    slot: Slot,
    size: f32,
    ink: Color,
}

impl Component for Leading {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let inner: Element = match &self.slot {
            Slot::Icon(path) => AppIcon::new(path.clone(), self.size).into_element(),
            Slot::Picture(path) => ImageViewer::new(path.clone())
                .height(Size::px(self.size))
                .width(Size::px(self.size))
                .aspect_ratio(AspectRatio::Min)
                .corner_radius(t.radius_small)
                .into_element(),
            Slot::Glyph(name) => Glyph::new(*name, t.size_glyph_large, self.ink).into_element(),
            Slot::Text(s) => label()
                .font_size(self.size * 0.7)
                .text(s.clone())
                .into_element(),
            Slot::None => rect().into_element(),
        };
        rect()
            .width(Size::px(self.size))
            .height(Size::px(self.size))
            .center()
            .child(inner)
    }
}

#[derive(PartialEq)]
struct ResultRow {
    row: RowView,
    selected: bool,
    unfocused: bool,
    anchors: Anchors,
}

impl Component for ResultRow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let row = &self.row;
        let (fill, ink, ink_2) = match (self.selected, self.unfocused) {
            (true, false) => (
                colour(t.colour_accent),
                colour(t.colour_on_accent),
                colour(t.colour_on_accent),
            ),
            (true, true) => (
                colour(t.colour_selection_unfocused),
                colour(t.colour_text),
                colour(t.colour_text_2),
            ),
            _ => (
                Color::TRANSPARENT,
                colour(t.colour_text),
                colour(t.colour_text_2),
            ),
        };
        let style = if row.strong {
            text::Style::Headline
        } else {
            text::Style::Body
        };
        let (hover, click, secondary) = (actions.clone(), actions.clone(), actions);
        let (k1, k2, k3) = (row.key.clone(), row.key.clone(), row.key.clone());
        let mut anchor = self.anchors.selected;
        let pad = t.space_s;
        rect()
            .maybe(self.selected, |el| {
                el.on_sized(move |e: Event<SizedEventData>| {
                    anchor.set_if_modified((e.area.min_x() + pad, e.area.max_y()))
                })
            })
            .width(Size::fill())
            .height(Size::px(t.size_row_large))
            .corner_radius(t.radius_card)
            .background(fill)
            .padding((0., t.space_m, 0., t.space_s))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .a11y_role(AccessibilityRole::ListItem)
            .a11y_alt(match &row.detail {
                Some(d) => format!("{}, {d}", row.title),
                None => row.title.clone(),
            })
            .on_pointer_move(move |_| send(&hover, SpotlightAction::Hover(k1.clone())))
            .on_press(move |_| send(&click, SpotlightAction::Click(k2.clone())))
            .on_secondary_down(move |_| send(&secondary, SpotlightAction::Secondary(k3.clone())))
            .child(Leading {
                slot: row.slot.clone(),
                size: t.size_app_icon,
                ink,
            })
            .child(
                rect()
                    .width(Size::flex(1.))
                    .child(text::line(&theme, style, ink, row.title.clone()))
                    .maybe_child(
                        row.detail
                            .clone()
                            .map(|d| text::line(&theme, text::Style::Subheadline, ink_2, d)),
                    ),
            )
            .maybe_child(
                row.trailing
                    .clone()
                    .map(|tr| text::line(&theme, text::Style::Body, ink_2, tr)),
            )
    }
}

/// One line of the grid: a section header or a row of cells.
#[derive(Clone, PartialEq)]
enum GridLine {
    Header(String),
    Cells(Vec<RowView>),
}

fn grid_lines(lines: &[LineView], columns: usize) -> Vec<GridLine> {
    let mut out = Vec::new();
    let mut cells: Vec<RowView> = Vec::new();
    for line in lines {
        match line {
            LineView::Header(h) => {
                if !cells.is_empty() {
                    out.push(GridLine::Cells(std::mem::take(&mut cells)));
                }
                out.push(GridLine::Header(h.clone()));
            }
            LineView::Row(r) => {
                if cells.len() == columns {
                    out.push(GridLine::Cells(std::mem::take(&mut cells)));
                }
                cells.push(r.clone());
            }
        }
    }
    if !cells.is_empty() {
        out.push(GridLine::Cells(cells));
    }
    out
}

#[derive(PartialEq)]
struct GridPanel {
    frame: Frame,
    solid: bool,
    columns: usize,
    anchors: Anchors,
}

#[derive(Clone, PartialEq)]
struct GridData {
    lines: Rc<Vec<GridLine>>,
    selected: Option<String>,
    unfocused: bool,
    apps: bool,
    anchors: Anchors,
    dragging: Option<String>,
}

impl Component for GridPanel {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = &self.frame;
        let apps = matches!(frame.layout, Layout::Apps(_));
        let cell = if apps {
            t.size_spotlight_tile
        } else {
            t.size_emoji_cell
        };
        let mut lines = grid_lines(&frame.lines, self.columns);
        // The top bar names the first section.
        if apps && matches!(lines.first(), Some(GridLine::Header(_))) {
            lines.remove(0);
        }
        let heights: Rc<Vec<f32>> = Rc::new(
            lines
                .iter()
                .map(|l| match l {
                    GridLine::Header(_) => t.size_row,
                    GridLine::Cells(_) => cell,
                })
                .collect(),
        );
        let item_size = {
            let heights = heights.clone();
            ItemSize::from(move |i: usize| heights.get(i).copied().unwrap_or(0.))
        };
        let mut controller = use_scroll_controller(ScrollConfig::default);
        if let Some(sel) = &frame.selected
            && let Some(index) = lines
                .iter()
                .position(|l| matches!(l, GridLine::Cells(c) if c.iter().any(|r| &r.key == sel)))
        {
            controller.scroll_to_index(index, &item_size, Direction::Vertical);
        }
        let (width, height) = if apps {
            frame.apps_panel
        } else {
            (
                t.size_spotlight_width,
                7.0 * cell + t.size_row + 2.0 * t.space_s,
            )
        };
        let length = lines.len();
        let selected_name = frame
            .selected
            .as_ref()
            .and_then(|k| {
                frame
                    .lines
                    .iter()
                    .find_map(|l| matches!(l, LineView::Row(r) if &r.key == k).then(|| l.clone()))
            })
            .and_then(|l| match l {
                LineView::Row(r) => Some(r.detail.unwrap_or(r.title)),
                LineView::Header(_) => None,
            });
        let data = GridData {
            lines: Rc::new(lines),
            selected: frame.selected.clone(),
            unfocused: frame.unfocused,
            apps,
            anchors: self.anchors,
            dragging: frame.dragging.clone(),
        };
        let grid = VirtualScrollView::new_with_data_controlled(
            data,
            |item: VirtualItem, data: &GridData| match &data.lines[item.index] {
                GridLine::Header(h) => Header { text: h.clone() }.into_element(),
                GridLine::Cells(cells) => rect()
                    .horizontal()
                    .children(cells.iter().map(|r| {
                        Tile {
                            row: r.clone(),
                            selected: data.selected.as_ref() == Some(&r.key),
                            unfocused: data.unfocused,
                            apps: data.apps,
                            anchors: data.anchors,
                            dragged: data.dragging.as_ref() == Some(&r.key),
                        }
                        .into_element()
                    }))
                    .into_element(),
            },
            controller,
        )
        .length(length)
        .item_size(item_size);
        let body = match &frame.empty {
            Some(empty) if frame.lines.is_empty() => rect()
                .padding((t.space_s, t.space_m))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    empty.clone(),
                ))
                .into_element(),
            _ => grid.into_element(),
        };
        let top_bar = apps.then(|| AppsBar {
            title: frame.apps_title.clone(),
            filter: frame.filter.clone(),
            anchor: self.anchors.filter,
        });
        let side = if apps { t.space_l } else { t.space_s };
        shape(&theme, self.solid, width, height).child(
            rect()
                .expanded()
                .padding((if apps { 0. } else { t.space_s }, side, t.space_s, side))
                .maybe_child(top_bar)
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::flex(1.))
                        .overflow(Overflow::Clip)
                        .child(body),
                )
                .maybe_child((!apps).then(|| {
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.size_row))
                        .border(
                            Border::new()
                                .fill(colour(t.colour_hairline))
                                .width(BorderWidth {
                                    top: 1.,
                                    right: 0.,
                                    bottom: 0.,
                                    left: 0.,
                                }),
                        )
                        .main_align(Alignment::Center)
                        .padding((0., t.space_s))
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text),
                            selected_name.unwrap_or_default(),
                        ))
                })),
        )
    }
}

/// The Apps view's fixed bar: the section's title and the category filter.
#[derive(PartialEq)]
struct AppsBar {
    title: String,
    filter: String,
    anchor: State<(f32, f32)>,
}

impl Component for AppsBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let mut anchor = self.anchor;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row_large))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .child(
                rect().width(Size::flex(1.)).child(
                    text::line(
                        &theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        self.title.clone(),
                    )
                    .font_weight(600),
                ),
            )
            .child(
                rect()
                    .height(Size::px(t.size_control))
                    .padding((0., t.space_s))
                    .corner_radius(t.radius_small)
                    .background(colour(t.colour_control))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .a11y_role(AccessibilityRole::ComboBox)
                    .a11y_alt(self.filter.clone())
                    .on_sized(move |e: Event<SizedEventData>| {
                        anchor.set_if_modified((e.area.min_x(), e.area.max_y()))
                    })
                    .on_press(move |_| send(&actions, SpotlightAction::Filter))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        self.filter.clone(),
                    ))
                    .child(Glyph::new(
                        "caret-up-down",
                        t.size_glyph_small,
                        colour(t.colour_text_2),
                    )),
            )
    }
}

#[derive(PartialEq)]
struct Tile {
    row: RowView,
    selected: bool,
    unfocused: bool,
    apps: bool,
    anchors: Anchors,
    dragged: bool,
}

impl Component for Tile {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let row = &self.row;
        let (hover, click, secondary) = (actions.clone(), actions.clone(), actions.clone());
        let (k1, k2, k3) = (row.key.clone(), row.key.clone(), row.key.clone());
        let (down, up) = (actions.clone(), actions);
        let (k4, k5) = (row.key.clone(), row.key.clone());
        let fill = if self.unfocused {
            t.colour_selection_unfocused
        } else {
            t.colour_accent
        };
        let mut anchor = self.anchors.selected;
        let base = rect()
            .maybe(self.selected, |el| {
                el.on_sized(move |e: Event<SizedEventData>| {
                    anchor.set_if_modified((e.area.min_x(), e.area.max_y()))
                })
            })
            .maybe(self.dragged, |el| el.opacity(0.4))
            .on_mouse_down(move |_| send(&down, SpotlightAction::DragFrom(k4.clone())))
            .on_mouse_up(move |_| send(&up, SpotlightAction::DragTo(k5.clone())))
            .on_pointer_move(move |_| send(&hover, SpotlightAction::Hover(k1.clone())))
            .on_press(move |_| send(&click, SpotlightAction::Click(k2.clone())))
            .on_secondary_down(move |_| send(&secondary, SpotlightAction::Secondary(k3.clone())))
            .a11y_role(AccessibilityRole::ListItem)
            .a11y_alt(row.title.clone());
        if !self.apps {
            return base
                .width(Size::px(t.size_emoji_cell))
                .height(Size::px(t.size_emoji_cell))
                .corner_radius(t.radius_card)
                .background(if self.selected {
                    colour(fill)
                } else {
                    Color::TRANSPARENT
                })
                .center()
                .child(match &row.slot {
                    Slot::Text(s) => label().font_size(28.).text(s.clone()).into_element(),
                    other => Leading {
                        slot: other.clone(),
                        size: 28.,
                        ink: colour(t.colour_text),
                    }
                    .into_element(),
                })
                .into_element();
        }
        let icon = t.size_app_icon_large;
        let badge = row.badge.map(|n| {
            let text = if n > 999 {
                "999+".to_owned()
            } else {
                n.to_string()
            };
            rect()
                .position(Position::new_absolute().top(-5.).right(-5.))
                .min_width(Size::px(18.))
                .height(Size::px(18.))
                .padding((0., 5.))
                .corner_radius(9.)
                .background(colour(t.colour_danger))
                .center()
                .child(
                    label()
                        .font_size(11.)
                        .font_weight(600)
                        .color(colour(t.colour_on_danger))
                        .text(text),
                )
        });
        let (name_fill, name_ink) = if self.selected {
            (
                colour(fill),
                colour(if self.unfocused {
                    t.colour_text
                } else {
                    t.colour_on_accent
                }),
            )
        } else {
            (Color::TRANSPARENT, colour(t.colour_text))
        };
        base.width(Size::px(t.size_spotlight_tile))
            .height(Size::px(t.size_spotlight_tile))
            .padding((t.space_m, t.space_xs, 0., t.space_xs))
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(
                rect()
                    .width(Size::px(icon + 2.0 * t.space_xs))
                    .height(Size::px(icon + 2.0 * t.space_xs))
                    .corner_radius(t.radius_card)
                    .background(if self.selected {
                        colour(t.colour_fill_strong)
                    } else {
                        Color::TRANSPARENT
                    })
                    .center()
                    .child(
                        rect()
                            .child(Leading {
                                slot: row.slot.clone(),
                                size: icon,
                                ink: colour(t.colour_text),
                            })
                            .maybe_child(badge),
                    ),
            )
            .child(
                rect()
                    .max_width(Size::fill())
                    .padding((1., t.space_xs))
                    .corner_radius(t.radius_small)
                    .background(name_fill)
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        name_ink,
                        row.title.clone(),
                    )),
            )
            .into_element()
    }
}
