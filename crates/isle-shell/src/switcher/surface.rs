//! The switcher's panel: plates, previews, marks, the name band and the window row, drawn
//! from the model's layout (`p1/surfaces/switcher.md`).

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub use freya::elements::image::ImageHandle;
use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Glass, Glyph, colour, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use super::model::{Entry, Layout, Rect};
use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

/// What the panel shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub row: Option<usize>,
    pub quitting: Vec<String>,
    pub layout: Layout,
    /// Each previewed window's image, by window id.
    pub previews: Vec<(u64, PreviewImage)>,
    /// Solid and still: Game Mode or Reduce Transparency.
    pub solid: bool,
}

/// A window's preview, compared by its image.
#[derive(Clone)]
pub struct PreviewImage(pub ImageHandle);

impl PartialEq for PreviewImage {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Debug for PreviewImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreviewImage({})", self.0.image.unique_id())
    }
}

impl Frame {
    fn preview(&self, id: Option<u64>) -> Option<&ImageHandle> {
        let id = id?;
        self.previews
            .iter()
            .find(|(w, _)| *w == id)
            .map(|(_, p)| &p.0)
    }
}

/// What the pointer does in the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitcherAction {
    /// The pointer moved onto a plate, or a window of the row.
    Hover {
        index: usize,
        window: Option<usize>,
    },
    Click {
        index: usize,
        window: Option<usize>,
    },
}

#[derive(Default)]
pub struct SwitcherSurface {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
    size: Cell<SSize<i32, Logical>>,
    /// Game Mode turns its fades off.
    still: Cell<bool>,
}

impl SwitcherSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Keeps `frame` for the next tree; safe anywhere.
    pub fn set_model(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame;
    }

    /// Puts `frame` in force; call it in the tree's root scope.
    pub fn set(&self, frame: Frame) {
        *self.frame.borrow_mut() = frame.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(frame);
        }
    }

    /// The room round the panel for its shadow.
    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_panel).ceil()
    }

    /// The surface's size for a layout, with its margin.
    pub fn size_for(layout: &Layout, theme: &Theme) -> SSize<i32, Logical> {
        let m = Self::margin(theme);
        let size = (
            (layout.width + 2.0 * m).ceil() as i32,
            (layout.height + 2.0 * m).ceil() as i32,
        )
            .into();
        size
    }

    pub fn set_size(&self, size: SSize<i32, Logical>) {
        self.size.set(size);
    }

    pub fn set_still(&self, still: bool) {
        self.still.set(still);
    }

    /// Six apps with every mark, and a background app, for the preview.
    pub fn sample() -> Rc<Self> {
        let app = |name: &str, windows: u64| Entry {
            app: name.to_lowercase(),
            name: name.into(),
            windows: (0..windows)
                .map(|i| super::model::Window {
                    id: i,
                    title: format!("{name} {}", i + 1),
                    minimised: false,
                })
                .collect(),
            recent: Some(0),
            ..Default::default()
        };
        let entries = vec![
            app("Firefox", 1),
            Entry {
                attention: true,
                ..app("kitty", 3)
            },
            Entry {
                hidden: true,
                ..app("Steam", 1)
            },
            Entry {
                badge: Some(4),
                urgent: true,
                ..app("Discord", 1)
            },
            Entry {
                progress: Some(0.4),
                minimised: true,
                ..app("Files", 1)
            },
            Entry {
                not_responding: true,
                ..app("Zed", 2)
            },
            Entry {
                background: true,
                windows: Vec::new(),
                recent: None,
                ..app("Sync", 0)
            },
        ];
        let layout = super::model::layout(&entries, 1920.0, &Default::default(), None, 0);
        let this = Self::new();
        let theme = isle_tokens::Source::shipped()
            .theme(&Default::default())
            .expect("the shipped tokens resolve");
        this.set_size(Self::size_for(&layout, &theme));
        this.set_model(Frame {
            entries,
            selected: 1,
            layout,
            ..Default::default()
        });
        this
    }
}

impl SurfaceDef for SwitcherSurface {
    fn name(&self) -> &'static str {
        "switcher"
    }

    fn layer(&self) -> Layer {
        Layer::Overlay
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(self.size.get())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(SwitcherApp {
            frame: self.frame.clone(),
            state: self.state.clone(),
        })
    }

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
struct SwitcherApp {
    frame: Rc<RefCell<Frame>>,
    state: Rc<RefCell<Option<State<Frame>>>>,
}

impl App for SwitcherApp {
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
        Panel { frame: state }
    }
}

#[derive(PartialEq)]
struct Panel {
    frame: State<Frame>,
}

/// Over a card's preview, which draws in a layer of its own for its clip.
const ABOVE: freya::prelude::Layer = freya::prelude::Layer::Relative(2);

/// A rect placed at `r` in its parent.
fn at(r: &Rect) -> freya::prelude::Rect {
    rect()
        .position(Position::new_absolute().left(r.x).top(r.y))
        .width(Size::px(r.w))
        .height(Size::px(r.h))
}

/// `w` by `h` fitted inside `bw` by `bh`, keeping its aspect.
fn fit(w: f32, h: f32, bw: f32, bh: f32) -> (f32, f32) {
    let s = (bw / w.max(1.0)).min(bh / h.max(1.0));
    ((w * s).round(), (h * s).round())
}

/// The states named after the app, for its accessible name and its band.
fn states(entry: &Entry, quitting: bool) -> Vec<String> {
    let mut out = Vec::new();
    if entry.windows.len() > 1 {
        out.push(format!("{} windows", entry.windows.len()));
    }
    if let Some(n) = entry.badge {
        out.push(format!("{n} notifications"));
    }
    if let Some(p) = entry.progress {
        out.push(format!("{:.0}% done", p * 100.0));
    }
    if entry.urgent {
        out.push("needs attention urgently".into());
    } else if entry.attention {
        out.push("needs attention".into());
    }
    for (on, word) in [
        (entry.hidden, "hidden"),
        (entry.minimised, "minimised"),
        (entry.not_responding, "not responding"),
        (entry.background, "running in background"),
        (quitting, "quitting"),
    ] {
        if on {
            out.push(word.into());
        }
    }
    out
}

/// The word the band shows after the selected app's name.
fn state_word(entry: &Entry) -> Option<&'static str> {
    if entry.not_responding {
        Some("Not responding")
    } else if entry.background {
        Some("Running in background")
    } else if entry.hidden {
        Some("Hidden")
    } else if entry.minimised {
        Some("Minimised")
    } else {
        None
    }
}

impl Component for Panel {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let layout = &frame.layout;
        let margin = SwitcherSurface::margin(&theme);
        let contrast = theme.appearance.increase_contrast;
        let edge = if contrast {
            colour(t.colour_hairline_strong)
        } else {
            colour(t.colour_hairline)
        };

        let mut panel = rect()
            .width(Size::px(layout.width))
            .height(Size::px(layout.height))
            .corner_radius(t.radius_panel)
            .background(if frame.solid {
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
                        .fill(edge)
                        .width(1.0)
                        .alignment(BorderAlignment::Inner),
                )
            })
            .maybe(!frame.solid, |el| el.child(Glass::new(t.radius_panel)))
            .child(isle_ui::Hit);

        if let Some(d) = &layout.divider {
            panel = panel.child(at(d).background(edge));
        }
        let row_open = frame.row.is_some();
        for (i, (entry, plate)) in frame.entries.iter().zip(&layout.plates).enumerate() {
            let selected = i == frame.selected;
            let quitting = frame.quitting.contains(&entry.app);
            panel = panel.child(plate_of(
                &theme,
                &frame,
                entry,
                plate,
                layout,
                PlateState {
                    selected,
                    unfocused: selected && row_open,
                    quitting,
                },
                i,
                &actions,
            ));
        }

        // The selected app's name, under its plate, clamped inside the panel.
        if let (Some(entry), Some(plate)) = (
            frame.entries.get(frame.selected),
            layout.plates.get(frame.selected),
        ) && let Some(band) = layout.bands.iter().find(|b| b.y >= plate.y + plate.h - 0.5)
        {
            let w = band.w.min(560.0);
            let centre = plate.x + plate.w / 2.0;
            let x = (centre - w / 2.0).clamp(band.x, band.x + band.w - w);
            let word = state_word(entry);
            panel = panel.child(
                at(&Rect {
                    x,
                    y: band.y,
                    w,
                    h: band.h,
                })
                .direction(Direction::Horizontal)
                .main_align(Alignment::Center)
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(text::line(
                    &theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    entry.name.clone(),
                ))
                .map(word, |el, word| {
                    el.child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        word,
                    ))
                }),
            );
        }

        // The window row, under a rule.
        if let (Some(rule), Some(entry)) = (&layout.rule, frame.entries.get(frame.selected)) {
            panel = panel.child(at(rule).background(edge));
            let card_w = layout
                .windows
                .first()
                .map_or(0.0, |p| p.w - 2.0 * t.space_s);
            let card_h = layout
                .windows
                .first()
                .map_or(0.0, |p| p.h - 2.0 * t.space_s);
            for (k, (window, plate)) in entry.windows.iter().zip(&layout.windows).enumerate() {
                let selected = frame.row == Some(k);
                let hover = actions.clone();
                let click = actions.clone();
                let index = frame.selected;
                let mut well = rect()
                    .position(Position::new_absolute().left(t.space_s).top(t.space_s))
                    .width(Size::px(card_w))
                    .height(Size::px(card_h))
                    .corner_radius(t.radius_small)
                    .background(colour(t.colour_fill))
                    .center()
                    .child(preview(
                        frame.preview(Some(window.id)),
                        card_w,
                        card_h,
                        t.radius_small,
                        false,
                    ));
                if window.minimised {
                    well = well.child(tags(&theme, &[Tag::Minimised]));
                }
                panel = panel.child(
                    at(plate)
                        .corner_radius(t.radius_card)
                        .background(if selected {
                            colour(t.colour_fill_strong)
                        } else {
                            Color::TRANSPARENT
                        })
                        .a11y_alt(window.title.clone())
                        .on_pointer_move(move |_| {
                            hover.send(Action::Switcher(SwitcherAction::Hover {
                                index,
                                window: Some(k),
                            }))
                        })
                        .on_press(move |_| {
                            click.send(Action::Switcher(SwitcherAction::Click {
                                index,
                                window: Some(k),
                            }))
                        })
                        .child(well),
                );
            }
            if let (Some(band), Some(window)) = (
                &layout.window_band,
                frame.row.and_then(|k| entry.windows.get(k)),
            ) {
                panel = panel.child(
                    at(band)
                        .main_align(Alignment::Center)
                        .cross_align(Alignment::Center)
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text),
                            window.title.clone(),
                        )),
                );
            }
        }

        rect()
            .expanded()
            .padding(margin)
            .a11y_role(AccessibilityRole::List)
            .a11y_alt("App switcher")
            .child(panel)
    }
}

struct PlateState {
    selected: bool,
    /// Selected, with the window row open below it.
    unfocused: bool,
    quitting: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Attention,
    Urgent,
    Hidden,
    Minimised,
    NotResponding,
    Background,
    Quitting,
}

#[allow(clippy::too_many_arguments)]
fn plate_of(
    theme: &Theme,
    frame: &Frame,
    entry: &Entry,
    plate: &Rect,
    layout: &Layout,
    state: PlateState,
    index: usize,
    actions: &Actions,
) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let card_w = plate.w - 2.0 * t.space_s;
    let card_h = layout.card_height;
    let hover = actions.clone();
    let click = actions.clone();

    let mut marks = Vec::new();
    if entry.urgent {
        marks.push(Tag::Urgent);
    } else if entry.attention {
        marks.push(Tag::Attention);
    }
    if state.quitting {
        marks.push(Tag::Quitting);
    }
    if entry.not_responding {
        marks.push(Tag::NotResponding);
    }
    if entry.background {
        marks.push(Tag::Background);
    } else if entry.hidden {
        marks.push(Tag::Hidden);
    } else if entry.minimised {
        marks.push(Tag::Minimised);
    }

    let mut well = rect()
        .position(Position::new_absolute().left(t.space_s).top(t.space_s))
        .width(Size::px(card_w))
        .height(Size::px(card_h))
        .corner_radius(t.radius_small)
        .background(colour(t.colour_fill))
        .center();
    if !entry.background {
        well = well.child(preview(
            frame.preview(entry.recent),
            card_w,
            card_h,
            t.radius_small,
            entry.hidden,
        ));
    }
    if !marks.is_empty() {
        well = well.child(tags(theme, &marks));
    }

    let icon_size = t.size_app_icon_medium;
    let icon_top = t.space_s + card_h - icon_size / 2.0;
    let (icon_w, icon_h) = if entry.cover {
        ((icon_size * 2.0 / 3.0).round(), icon_size)
    } else {
        (icon_size, icon_size)
    };
    let icon_left = t.space_s + (card_w - icon_w) / 2.0;
    let icon = rect()
        .layer(ABOVE)
        .position(Position::new_absolute().left(icon_left).top(icon_top))
        .width(Size::px(icon_w))
        .height(Size::px(icon_h))
        .map(entry.icon.clone(), |el, path| {
            if entry.cover {
                el.corner_radius(t.radius_small)
                    .overflow(Overflow::Clip)
                    .border(
                        Border::new()
                            .fill(colour(t.colour_hairline))
                            .width(1.0)
                            .alignment(BorderAlignment::Inner),
                    )
                    .child(
                        ImageViewer::new(path)
                            .width(Size::px(icon_w))
                            .height(Size::px(icon_h)),
                    )
            } else {
                el.child(AppIcon::new(path, icon_size))
            }
        })
        .maybe(entry.icon.is_none(), |el| {
            el.center().child(Glyph::new(
                "app-window",
                icon_size * 0.75,
                colour(t.colour_text_2),
            ))
        });

    let mut out = at(plate)
        .corner_radius(t.radius_card)
        .background(if state.unfocused {
            colour(t.colour_selection_unfocused)
        } else if state.selected {
            if theme.appearance.increase_contrast {
                colour(t.colour_control_pressed)
            } else {
                colour(t.colour_fill_strong)
            }
        } else {
            Color::TRANSPARENT
        })
        .maybe(state.selected && theme.appearance.increase_contrast, |el| {
            el.border(
                Border::new()
                    .fill(colour(t.colour_hairline_strong))
                    .width(1.0)
                    .alignment(BorderAlignment::Inner),
            )
        })
        .a11y_role(AccessibilityRole::ListItem)
        .a11y_alt({
            let mut name = vec![entry.name.clone()];
            name.extend(states(entry, state.quitting));
            name.join(", ")
        })
        .on_pointer_move(move |_| {
            hover.send(Action::Switcher(SwitcherAction::Hover {
                index,
                window: None,
            }))
        })
        .on_press(move |_| {
            click.send(Action::Switcher(SwitcherAction::Click {
                index,
                window: None,
            }))
        })
        .child(well)
        .child(icon);

    // The window count, quiet, past the icon's trailing edge on its lower half.
    if entry.windows.len() > 1 {
        out = out.child(
            rect()
                .layer(ABOVE)
                .position(
                    Position::new_absolute()
                        .left(icon_left + icon_w + t.space_xs)
                        .top(icon_top + icon_h * 0.75 - 8.0),
                )
                .height(Size::px(16.0))
                .padding((0.0, t.space_xs))
                .corner_radius(8.0)
                .background(colour(t.colour_fill_strong))
                .center()
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text),
                    entry.windows.len().to_string(),
                )),
        );
    }
    // The badge, centred on the icon's top trailing corner, inset into it.
    if let Some(count) = entry.badge {
        out = out.child(
            rect()
                .layer(ABOVE)
                .position(
                    Position::new_absolute()
                        .left(icon_left + icon_w - 4.0 - 9.0)
                        .top(icon_top + 4.0 - 9.0),
                )
                .height(Size::px(18.0))
                .min_width(Size::px(18.0))
                .padding((0.0, 5.0))
                .corner_radius(9.0)
                .background(colour(t.colour_danger))
                .center()
                .child(text::line(
                    theme,
                    text::Style::Footnote,
                    colour(t.colour_on_danger),
                    isle_badge_text(count),
                )),
        );
    }
    // Progress along the plate's lower edge; its room is kept when there is none.
    if let Some(p) = entry.progress {
        let y = t.space_s + card_h + icon_size / 2.0 + t.space_s;
        out = out.child(
            rect()
                .position(Position::new_absolute().left(t.space_s).top(y))
                .width(Size::px(card_w))
                .height(Size::px(6.0))
                .corner_radius(3.0)
                .background(colour(t.colour_fill_strong))
                .child(
                    rect()
                        .width(Size::px((card_w * p as f32).round()))
                        .height(Size::px(6.0))
                        .corner_radius(3.0)
                        .background(colour(t.colour_accent)),
                ),
        );
    }
    out
}

fn isle_badge_text(count: i64) -> String {
    if count > 999 {
        "999+".into()
    } else {
        count.to_string()
    }
}

/// A window's preview fitted inside its card, or nothing until it arrives.
fn preview(
    image_handle: Option<&ImageHandle>,
    w: f32,
    h: f32,
    radius: f32,
    hidden: bool,
) -> Element {
    let Some(handle) = image_handle else {
        return rect().into_element();
    };
    let (iw, ih) = (handle.image.width() as f32, handle.image.height() as f32);
    let (pw, ph) = fit(iw, ih, w, h);
    rect()
        .width(Size::px(pw))
        .height(Size::px(ph))
        .corner_radius(radius)
        .overflow(Overflow::Clip)
        .opacity(if hidden { 0.5 } else { 1.0 })
        .child(
            image(handle.clone())
                .width(Size::px(pw))
                .height(Size::px(ph)),
        )
        .into_element()
}

/// The marks at a card's top leading corner, attention first.
fn tags(theme: &Theme, marks: &[Tag]) -> freya::prelude::Rect {
    let t = &theme.tokens;
    let mut row = rect()
        .layer(ABOVE)
        .position(Position::new_absolute().left(t.space_xs).top(t.space_xs))
        .direction(Direction::Horizontal)
        .spacing(t.space_xxs);
    for tag in marks {
        let (glyph, fill, ink, ground, alt) = match tag {
            Tag::Attention => (
                "bell-simple-ringing",
                false,
                t.colour_warn_text,
                t.material_solid,
                "Needs attention",
            ),
            Tag::Urgent => (
                "bell-simple-ringing",
                true,
                t.colour_on_danger,
                t.colour_danger,
                "Needs attention urgently",
            ),
            Tag::Hidden => (
                "eye-slash",
                false,
                t.colour_text_2,
                t.material_solid,
                "Hidden",
            ),
            Tag::Minimised => (
                "minus",
                false,
                t.colour_text_2,
                t.material_solid,
                "Minimised",
            ),
            Tag::NotResponding => (
                "hourglass-medium",
                false,
                t.colour_text_2,
                t.material_solid,
                "Not responding",
            ),
            Tag::Background => (
                "tray",
                false,
                t.colour_text_2,
                t.material_solid,
                "Running in background",
            ),
            Tag::Quitting => (
                "circle-notch",
                false,
                t.colour_text_2,
                t.material_solid,
                "Quitting",
            ),
        };
        row = row.child(
            rect()
                .height(Size::px(t.size_control_small))
                .width(Size::px(t.size_control_small + t.space_xs))
                .corner_radius(t.size_control_small / 2.0)
                .background(colour(ground))
                .map(Some(shadows(&t.shadow_control)), |el, list| {
                    list.into_iter().fold(el, |el, s| el.shadow(s))
                })
                .maybe(theme.appearance.increase_contrast, |el| {
                    el.border(
                        Border::new()
                            .fill(colour(t.colour_hairline_strong))
                            .width(1.0)
                            .alignment(BorderAlignment::Inner),
                    )
                })
                .center()
                .child(
                    Glyph::new(glyph, t.size_glyph_small, colour(ink))
                        .fill(fill)
                        .alt(alt),
                ),
        );
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_fit_their_card_keeping_aspect() {
        assert_eq!(fit(3440.0, 1440.0, 256.0, 160.0), (256.0, 107.0));
        assert_eq!(fit(1146.0, 1440.0, 256.0, 160.0), (127.0, 160.0));
    }

    #[test]
    fn the_band_names_the_state_and_badges_cap() {
        let entry = Entry {
            name: "Steam".into(),
            background: true,
            ..Default::default()
        };
        assert_eq!(state_word(&entry), Some("Running in background"));
        assert_eq!(isle_badge_text(1200), "999+");
        let entry = Entry {
            name: "kitty".into(),
            windows: vec![Default::default(), Default::default()],
            badge: Some(4),
            ..Default::default()
        };
        assert_eq!(states(&entry, false), vec!["2 windows", "4 notifications"]);
    }
}
