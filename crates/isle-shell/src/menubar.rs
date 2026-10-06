//! The menu bar, on demand: a band across the top of one screen with the system menu, the
//! front app's name and menus, Window and Help, and the open menu under its title
//! (`p1/structure/menu-bar.md`, `p1/surfaces/menus.md`).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    Glass, Glyph, Hit, Menu, MenuEntry, MenuEvent, SearchField, colour, focus_ring, menu::Material, shadows,
    text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Here, Layer, Scope, SurfaceDef};

/// A title in the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Title {
    pub label: String,
    /// The system menu shows Isle's mark; the app menu its name in the headline weight.
    pub kind: TitleKind,
    pub enabled: bool,
    pub menu: Rc<Vec<MenuEntry>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleKind {
    System,
    App,
    Menu,
    Overflow,
}

/// One frame of a screen's bar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarFrame {
    /// How far the band has slid down: 0 hidden above the screen, 1 in place.
    pub shown: f32,
    pub titles: Vec<Title>,
    /// The title whose menu is open.
    pub open: Option<usize>,
    /// The title the keyboard is on with no menu open.
    pub focused: Option<usize>,
    /// The open menu was opened from the keyboard: its first row is selected.
    pub by_key: bool,
    /// Where the island's reserve starts and ends, in points from the left.
    pub reserve: (f32, f32),
    pub solid: bool,
    /// Bumped on every opening, so a menu reopened starts fresh.
    pub generation: u64,
}

/// What the bar asks of the compositor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarAction {
    /// A click on a title: its menu, or closed again.
    Title(usize),
    /// The pointer moved onto a title while a menu is open.
    Hover(usize),
    Menu(MenuEvent),
    /// Left or Right on the titles from the keyboard.
    Step(i32),
    /// Escape with no menu open.
    Close,
}

/// Each title cell's left edge and width, and where the menus that do not fit before the
/// island's reserve start folding into the overflow.
pub fn layout(
    theme: &Theme,
    titles: &[(String, TitleKind)],
    width: f32,
    reserve: (f32, f32),
) -> (Vec<(f32, f32)>, Option<usize>) {
    let t = &theme.tokens;
    let body = text::metrics(theme, text::Style::Body).0;
    let cell = |label: &str, kind: TitleKind| {
        let content = match kind {
            TitleKind::System => t.size_glyph,
            TitleKind::Overflow => t.size_glyph_small,
            TitleKind::App => label.chars().count() as f32 * body * 0.6,
            TitleKind::Menu => label.chars().count() as f32 * body * 0.56,
        };
        let content = if kind == TitleKind::App {
            content.min(t.size_bar_name)
        } else {
            content
        };
        (content + 2.0 * t.space_s).ceil()
    };
    let _ = width;
    let mut x = t.space_s;
    let mut out = Vec::with_capacity(titles.len());
    for (label, kind) in titles {
        let w = cell(label, *kind);
        out.push((x, w));
        x += w + t.space_xxs;
    }
    // Menus folded keep the system menu, the app menu, Window and Help; the » takes the
    // place of the first that would reach the reserve.
    let limit = reserve.0;
    let fold = out.iter().position(|(x, w)| x + w > limit);
    (out, fold)
}

#[derive(Default)]
pub struct MenuBarSurface {
    frames: Rc<RefCell<HashMap<String, BarFrame>>>,
    states: Rc<RefCell<HashMap<String, State<BarFrame>>>>,
}

impl MenuBarSurface {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Keeps `frame` for `output`'s next tree; safe anywhere.
    pub fn set_model(&self, output: &str, frame: BarFrame) {
        self.frames.borrow_mut().insert(output.to_owned(), frame);
    }

    /// Puts `frame` in force on `output`; call it in that instance's root scope.
    pub fn set(&self, output: &str, frame: BarFrame) {
        self.frames
            .borrow_mut()
            .insert(output.to_owned(), frame.clone());
        if let Some(mut state) = self.states.borrow().get(output).copied() {
            state.set_if_modified(frame);
        }
    }
}

impl SurfaceDef for MenuBarSurface {
    fn name(&self) -> &'static str {
        "menu-bar"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::EveryOutput
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(output)
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(BarApp {
            frames: self.frames.clone(),
            states: self.states.clone(),
        })
    }

    fn hit_shapes(&self) -> bool {
        true
    }

    fn transitions(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct BarApp {
    frames: Rc<RefCell<HashMap<String, BarFrame>>>,
    states: Rc<RefCell<HashMap<String, State<BarFrame>>>>,
}

impl App for BarApp {
    fn render(&self) -> impl IntoElement {
        let here = use_consume::<Here>().0;
        // A closed surface's tree goes; the compositor must not write to its state after.
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
        Bar { frame: state }
    }
}

#[derive(PartialEq)]
struct Bar {
    frame: State<BarFrame>,
}

fn send(actions: &Actions, action: BarAction) {
    actions.send(Action::MenuBar(action));
}

impl Component for Bar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let frame = self.frame.read().clone();
        let actions = Actions::get();
        let height = t.size_bar_height;
        let offset = -(height + 24.0) * (1.0 - frame.shown);
        let titles: Vec<(String, TitleKind)> = frame
            .titles
            .iter()
            .map(|t| (t.label.clone(), t.kind))
            .collect();
        let (cells, _) = layout(&theme, &titles, 0.0, frame.reserve);
        let ink = colour(t.colour_text);
        let band_fill = if frame.solid {
            colour(t.material_solid)
        } else {
            Color::TRANSPARENT
        };

        let title_cells =
            frame
                .titles
                .iter()
                .zip(&cells)
                .enumerate()
                .map(|(i, (title, (x, w)))| {
                    let open = frame.open == Some(i);
                    let focused = frame.focused == Some(i) && frame.open.is_none();
                    let press = actions.clone();
                    let hover = actions.clone();
                    let enabled = title.enabled;
                    let content = match title.kind {
                        TitleKind::System => Glyph::new("isle-mark", t.size_glyph, ink)
                            .alt("System menu")
                            .into_element(),
                        TitleKind::Overflow => {
                            Glyph::new("caret-double-right", t.size_glyph_small, ink)
                                .alt("More menus")
                                .into_element()
                        }
                        TitleKind::App => {
                            text::line(&theme, text::Style::Headline, ink, title.label.clone())
                                .into_element()
                        }
                        TitleKind::Menu => text::line(
                            &theme,
                            text::Style::Body,
                            if enabled {
                                ink
                            } else {
                                colour(t.colour_text_disabled)
                            },
                            title.label.clone(),
                        )
                        .into_element(),
                    };
                    rect()
                        .position(Position::new_absolute().left(*x).top(0.))
                        .width(Size::px(*w))
                        .height(Size::px(height))
                        .center()
                        .a11y_role(AccessibilityRole::MenuItem)
                        .a11y_alt(title.label.clone())
                        .on_press(move |e: Event<PressEventData>| {
                            e.stop_propagation();
                            send(&press, BarAction::Title(i));
                        })
                        .on_pointer_enter(move |_| send(&hover, BarAction::Hover(i)))
                        .child(
                            rect()
                                .width(Size::px(*w))
                                .height(Size::px(t.size_control))
                                .corner_radius(t.radius_control)
                                .background(if open || focused {
                                    colour(t.colour_fill_strong)
                                } else {
                                    Color::TRANSPARENT
                                })
                                .maybe(focused, |el| el.border(focus_ring(&theme)))
                                .center()
                                .child(content),
                        )
                        .into_element()
                });

        let menu = frame.open.and_then(|i| {
            let title = frame.titles.get(i)?;
            let (x, _) = cells.get(i)?;
            let on_menu = actions.clone();
            let material = if frame.solid {
                Material::Solid
            } else {
                Material::Glass
            };
            let help = title.kind == TitleKind::Menu && title.label == "Help" && i + 1 == frame.titles.len();
            Some(
                rect()
                    .position(Position::new_absolute().left(*x).top(height + t.space_xs))
                    .key(frame.generation)
                    .child(Hit)
                    .child(if help {
                        HelpMenu {
                            titles: frame.titles.clone(),
                            material,
                        }
                        .into_element()
                    } else {
                        Menu::new(title.menu.clone(), move |e| send(&on_menu, BarAction::Menu(e)))
                            .material(material)
                            .first_selected(frame.by_key)
                            .into_element()
                    }),
            )
        });

        let keys = actions.clone();
        let by_key = frame.focused.is_some() && frame.open.is_none();
        rect()
            .expanded()
            .maybe(by_key, |el| {
                el.on_global_key_down(move |e: Event<KeyboardEventData>| match &e.key {
                    Key::Named(NamedKey::ArrowLeft) => send(&keys, BarAction::Step(-1)),
                    Key::Named(NamedKey::ArrowRight) => send(&keys, BarAction::Step(1)),
                    Key::Named(NamedKey::Enter) | Key::Named(NamedKey::ArrowDown) => {
                        send(&keys, BarAction::Menu(MenuEvent::Forward))
                    }
                    Key::Named(NamedKey::Escape) => send(&keys, BarAction::Close),
                    _ => {}
                })
            })
            .child(
                rect()
                    .position(Position::new_absolute().left(0.).top(offset))
                    .width(Size::fill())
                    .height(Size::px(height))
                    .background(band_fill)
                    .map(Some(shadows(&t.shadow_bar)), |el, list| {
                        list.into_iter().fold(el, |el, s| el.shadow(s))
                    })
                    .maybe(!frame.solid, |el| el.child(Glass::new(0.0)))
                    .child(Hit)
                    .child(
                        rect()
                            .position(
                                Position::new_absolute()
                                    .left(0.)
                                    .top(height - t.stroke_hairline.max(1.0)),
                            )
                            .width(Size::fill())
                            .height(Size::px(t.stroke_hairline.max(1.0)))
                            .background(colour(t.colour_hairline)),
                    )
                    .children(title_cells),
            )
            .maybe_child(menu)
    }
}

/// Help with menu search: a field over the menu; as a query stands, the items of the app's
/// menus whose labels hold it, each with its path, then Help's own items.
#[derive(PartialEq)]
struct HelpMenu {
    titles: Vec<Title>,
    material: Material,
}

impl Component for HelpMenu {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let actions = Actions::get();
        let query = use_state(String::new);
        let q = query.read().clone();
        let Some(help) = self.titles.last() else {
            return rect().into_element();
        };
        // The app's menus, folded ones included; not the system menu, not Help itself.
        let last = self.titles.len().saturating_sub(1);
        let menus: Vec<(String, Rc<Vec<MenuEntry>>)> = self
            .titles
            .iter()
            .enumerate()
            .filter(|(i, title)| *i != last && title.kind != TitleKind::System)
            .map(|(_, title)| (title.label.clone(), title.menu.clone()))
            .collect();
        let mut entries: Vec<MenuEntry> = Vec::new();
        if !q.trim().is_empty() {
            let found = isle_ui::menu::search(&menus, &q);
            if found.is_empty() {
                entries.push(MenuEntry::Section(format!("No results for “{}”", q.trim())));
            }
            for (path, item) in found {
                let mut item = item;
                item.label = format!("{} › {}", path.join(" › "), item.label);
                entries.push(MenuEntry::Item(item));
            }
            entries.push(MenuEntry::Separator);
        }
        entries.extend(help.menu.iter().cloned());
        let width = if q.trim().is_empty() {
            t.size_menu_min
        } else {
            t.size_menu_search
        };
        rect()
            .corner_radius(t.radius_card)
            .spacing(t.space_xs)
            .child(SearchField {
                value: query.into_writable(),
                placeholder: "Search".into(),
                width,
            })
            .child(
                rect().key(q.clone()).child(
                    Menu::new(Rc::new(entries), move |e| send(&actions, BarAction::Menu(e)))
                        .material(self.material)
                        .type_select(false)
                        .first_selected(!q.trim().is_empty()),
                ),
            )
            .into_element()
    }
}
