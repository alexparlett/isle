//! The screen-share picker a ScreenCast request opens: what to share with the app, each
//! source a live tile (`p1/surfaces/capture.md`, The screen-share picker).

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use freya::{elements::image::ImageHandle, prelude::*};
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{AppIcon, Button, button::Kind, colour, focus_ring, shadows, text, use_theme};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::surface::{Action, Actions, Layer, Scope, SurfaceDef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Screens,
    Windows,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Screens => "Screens",
            Tab::Windows => "Windows",
        }
    }
}

/// One source the app may have.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub key: String,
    pub title: String,
    /// A screen's size, or the window's app.
    pub subtitle: String,
    pub icon: Option<PathBuf>,
    pub preview: Option<Preview>,
}

/// A tile's live picture, compared by its image.
#[derive(Clone)]
pub struct Preview(pub ImageHandle);

impl PartialEq for Preview {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Preview")
    }
}

/// What the picker shows now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub app: String,
    pub icon: Option<PathBuf>,
    pub tabs: Vec<Tab>,
    pub tab: usize,
    /// The current tab's tiles.
    pub tiles: Vec<Tile>,
    pub chosen: Vec<String>,
    pub focus: Option<usize>,
    pub multiple: bool,
    /// "Let {app} share this again without asking", when the app asks to keep it.
    pub remember: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerAction {
    Tab(usize),
    /// A tile clicked; a second click soon after shares it alone.
    Choose(String),
    /// The keyboard's tile moves by a step; nought chooses it.
    Step(i32),
    Remember(bool),
    Share,
    Cancel,
}

#[derive(Default)]
pub struct PickerSurface {
    model: Rc<RefCell<Model>>,
    state: Rc<RefCell<Option<State<Model>>>>,
    /// Attached to the asking app's window rather than placed on the screen.
    sheet: bool,
}

pub const TILE_WIDTH: f32 = 200.0;
pub const TILE_PREVIEW: f32 = 125.0;

impl PickerSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The picker as a sheet on a window, one per ask.
    pub fn sheet() -> Rc<Self> {
        Rc::new(Self {
            sheet: true,
            ..Self::default()
        })
    }

    pub fn set(&self, model: Model) {
        *self.model.borrow_mut() = model.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(model);
        }
    }

    pub fn set_model(&self, model: Model) {
        *self.model.borrow_mut() = model;
    }

    pub fn margin(theme: &Theme) -> f32 {
        isle_ui::shadow_reach(&theme.tokens.shadow_window).ceil()
    }

    /// The well's height: two rows of tiles; more scroll.
    pub fn grid_height(theme: &Theme) -> f32 {
        let t = &theme.tokens;
        let (_, body, _) = text::metrics(theme, text::Style::Body);
        let (_, sub, _) = text::metrics(theme, text::Style::Subheadline);
        let tile = TILE_PREVIEW + t.space_s + body + sub;
        2.0 * t.space_s + 2.0 * tile + t.space_m
    }

    /// The surface's size: the panel at its width, two rows of tiles, and its shadow's room.
    pub fn size_for(theme: &Theme) -> SSize<i32, Logical> {
        let t = &theme.tokens;
        let m = Self::margin(theme);
        let (_, body, _) = text::metrics(theme, text::Style::Body);
        let (_, headline, _) = text::metrics(theme, text::Style::Headline);
        let height = 2.0 * t.space_l
            + t.size_app_icon.max(headline)
            + t.space_m
            + t.size_control
            + t.space_m
            + Self::grid_height(theme)
            + t.space_m
            + body.max(16.0)
            + t.space_l
            + t.size_control
            + t.space_s;
        (
            (t.size_picker + 2.0 * m).ceil() as i32,
            (height + 2.0 * m).ceil() as i32,
        )
            .into()
    }
}

impl SurfaceDef for PickerSurface {
    fn name(&self) -> &'static str {
        if self.sheet { "share-sheet" } else { "share-picker" }
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        if self.sheet { Scope::Attached } else { Scope::Placed }
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(PickerApp {
            model: self.model.clone(),
            state: self.state.clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct PickerApp {
    model: Rc<RefCell<Model>>,
    state: Rc<RefCell<Option<State<Model>>>>,
}

fn send(actions: &Actions, action: PickerAction) {
    actions.send(Action::Picker(action));
}

impl App for PickerApp {
    fn render(&self) -> impl IntoElement {
        use_drop({
            let slot = self.state.clone();
            move || {
                *slot.borrow_mut() = None;
            }
        });
        let state = use_hook(|| {
            let state = State::create(self.model.borrow().clone());
            *self.state.borrow_mut() = Some(state);
            state
        });
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let model = state.read().clone();
        let margin = PickerSurface::margin(&theme);
        let inner = t.size_picker - 2.0 * t.space_xl;

        let mut tabs = rect()
            .horizontal()
            .height(Size::px(t.size_control))
            .padding(t.space_xxs)
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill));
        for (i, tab) in model.tabs.iter().enumerate() {
            let chosen = i == model.tab;
            let actions = actions.clone();
            tabs = tabs.child(
                rect()
                    .height(Size::fill())
                    .padding((0., t.space_m))
                    .center()
                    .corner_radius(t.radius_small)
                    .maybe(chosen, |el| el.background(colour(t.material_solid)))
                    .on_press(move |_| send(&actions, PickerAction::Tab(i)))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(if chosen {
                            t.colour_text
                        } else {
                            t.colour_text_2
                        }),
                        tab.label(),
                    )),
            );
        }

        let columns = ((inner - 2.0 * t.space_s + t.space_m) / (TILE_WIDTH + t.space_m))
            .floor()
            .max(1.0) as usize;
        let mut grid = rect()
            .width(Size::fill())
            .height(Size::fill())
            .overflow(Overflow::Clip)
            .padding(t.space_s)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_sunken))
            .spacing(t.space_m);
        if model.tiles.is_empty() {
            grid = grid.center().child(text::line(
                &theme,
                text::Style::Body,
                colour(t.colour_text_2),
                "No windows",
            ));
        }
        for row in model.tiles.chunks(columns).enumerate() {
            let (r, tiles) = row;
            let mut line = rect().horizontal().spacing(t.space_m);
            for (c, tile) in tiles.iter().enumerate() {
                let index = r * columns + c;
                let chosen = model.chosen.contains(&tile.key);
                let focused = model.focus == Some(index);
                let press = actions.clone();
                let key = tile.key.clone();
                let picture = match &tile.preview {
                    Some(Preview(handle)) => {
                        let (iw, ih) = (handle.image.width() as f32, handle.image.height() as f32);
                        let f = (TILE_WIDTH / iw.max(1.0)).min(TILE_PREVIEW / ih.max(1.0));
                        rect().child(
                            image(handle.clone())
                                .width(Size::px((iw * f).round()))
                                .height(Size::px((ih * f).round())),
                        )
                    }
                    None => match &tile.icon {
                        Some(icon) => {
                            rect().child(AppIcon::new(icon.clone(), t.size_app_icon_medium))
                        }
                        None => rect().child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text_2),
                            tile.title.clone(),
                        )),
                    },
                };
                line = line.child(
                    rect()
                        .width(Size::px(TILE_WIDTH))
                        .a11y_alt(tile.title.clone())
                        .on_press(move |_| send(&press, PickerAction::Choose(key.clone())))
                        .child(
                            rect()
                                .width(Size::px(TILE_WIDTH))
                                .height(Size::px(TILE_PREVIEW))
                                .corner_radius(t.radius_small)
                                .overflow(Overflow::Clip)
                                .background(colour(t.colour_fill))
                                .center()
                                .maybe(chosen, |el| {
                                    el.border(
                                        Border::new()
                                            .fill(colour(t.colour_accent))
                                            .width(t.stroke_found)
                                            .alignment(BorderAlignment::Inner),
                                    )
                                })
                                .maybe(focused && !chosen, |el| el.border(focus_ring(&theme)))
                                .child(picture),
                        )
                        .child(rect().height(Size::px(t.space_s)))
                        .child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text),
                            tile.title.clone(),
                        ))
                        .child(text::line(
                            &theme,
                            text::Style::Subheadline,
                            colour(t.colour_text_2),
                            tile.subtitle.clone(),
                        )),
                );
            }
            grid = grid.child(line);
        }

        let mut options = rect().spacing(t.space_s);
        if let Some(on) = model.remember {
            let actions = actions.clone();
            options = options.child(
                rect()
                    .horizontal()
                    .spacing(t.space_s)
                    .cross_align(Alignment::Center)
                    .on_press(move |_| send(&actions, PickerAction::Remember(!on)))
                    .child(
                        rect()
                            .width(Size::px(16.0))
                            .height(Size::px(16.0))
                            .corner_radius(4.0)
                            .background(colour(if on { t.colour_accent } else { t.colour_fill })),
                    )
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        format!("Let {} share this again without asking", model.app),
                    )),
            );
        }

        let share_label = if model.chosen.len() > 1 {
            format!("Share {} sources", model.chosen.len())
        } else {
            "Share".to_owned()
        };
        let (cancel, share, keys) = (actions.clone(), actions.clone(), actions.clone());
        let can_share = !model.chosen.is_empty();
        let tab_count = model.tabs.len();
        let tab_now = model.tab;
        rect()
            .expanded()
            .padding(margin)
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                let action = match &e.key {
                    Key::Named(NamedKey::Escape) => Some(PickerAction::Cancel),
                    Key::Named(NamedKey::Enter) => Some(PickerAction::Share),
                    Key::Named(NamedKey::Tab) if tab_count > 1 => {
                        Some(PickerAction::Tab((tab_now + 1) % tab_count))
                    }
                    Key::Named(NamedKey::ArrowLeft) => Some(PickerAction::Step(-1)),
                    Key::Named(NamedKey::ArrowRight) => Some(PickerAction::Step(1)),
                    Key::Named(NamedKey::ArrowUp) => Some(PickerAction::Step(-(columns as i32))),
                    Key::Named(NamedKey::ArrowDown) => Some(PickerAction::Step(columns as i32)),
                    Key::Character(c) if c == " " => Some(PickerAction::Step(0)),
                    _ => None,
                };
                if let Some(action) = action {
                    send(&keys, action);
                }
            })
            .child(
                rect()
                    .width(Size::px(t.size_picker))
                    .height(Size::fill())
                    .background(colour(t.colour_window))
                    .corner_radius(t.radius_window)
                    .map(Some(shadows(&t.shadow_window)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .padding((t.space_l, t.space_xl))
                    .child(isle_ui::Hit)
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_m)
                            .maybe(model.icon.is_some(), |el| {
                                el.child(AppIcon::new(model.icon.clone().unwrap(), t.size_app_icon))
                            })
                            .child(text::line(
                                &theme,
                                text::Style::Headline,
                                colour(t.colour_text),
                                format!("Choose what to share with {}", model.app),
                            )),
                    )
                    .child(rect().height(Size::px(t.space_m)))
                    .child(rect().width(Size::fill()).center().child(tabs))
                    .child(rect().height(Size::px(t.space_m)))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::px(PickerSurface::grid_height(&theme)))
                            .child(grid),
                    )
                    .child(rect().height(Size::px(t.space_m)))
                    .child(options)
                    .child(rect().height(Size::px(t.space_l)))
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::End)
                            .spacing(t.space_s)
                            .child(
                                Button::new("Cancel")
                                    .on_press(move |_| send(&cancel, PickerAction::Cancel)),
                            )
                            .child(
                                Button::new(share_label)
                                    .kind(Kind::Default)
                                    .enabled(can_share)
                                    .on_press(move |_| send(&share, PickerAction::Share)),
                            ),
                    ),
            )
    }
}
