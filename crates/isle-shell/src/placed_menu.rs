//! Menus and panels the compositor places by a window: the window menu, and Isle's About
//! panel for an app whose menus have no About of their own.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_tokens::Theme;
use isle_ui::{
    Button, Glyph, Menu, MenuEntry, button::Kind, colour, menu::Material, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Action, Actions, Layer, Scope, SurfaceDef};

/// The window menu's entries and how it was opened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuModel {
    pub entries: Rc<Vec<MenuEntry>>,
    pub by_key: bool,
    pub generation: u64,
}

#[derive(Default)]
pub struct WindowMenu {
    model: Rc<RefCell<MenuModel>>,
    state: Rc<RefCell<Option<State<MenuModel>>>>,
}

impl WindowMenu {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, model: MenuModel) {
        *self.model.borrow_mut() = model;
    }

    /// Puts `model` in force; call it in the tree's root scope.
    pub fn set(&self, model: MenuModel) {
        *self.model.borrow_mut() = model.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(model);
        }
    }

    pub fn by_key(&self) -> bool {
        self.model.borrow().by_key
    }
}

impl SurfaceDef for WindowMenu {
    fn name(&self) -> &'static str {
        "window-menu"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size((1, 1).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(WindowMenuApp {
            model: self.model.clone(),
            state: self.state.clone(),
        })
    }

    fn anchor(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn hit_shapes(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct WindowMenuApp {
    model: Rc<RefCell<MenuModel>>,
    state: Rc<RefCell<Option<State<MenuModel>>>>,
}

impl App for WindowMenuApp {
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
        let model = state.read().clone();
        let actions = Actions::get();
        rect().expanded().child(
            rect().key(model.generation).child(isle_ui::Hit).child(
                Menu::new(model.entries, move |e| actions.send(Action::Menu(e)))
                    .material(Material::Glass)
                    .first_selected(model.by_key),
            ),
        )
    }
}

/// The size a window menu's surface needs: the menu and one submenu beside it.
pub fn window_menu_size(theme: &Theme, rows: usize) -> SSize<i32, Logical> {
    let t = &theme.tokens;
    let row = (text::metrics(theme, text::Style::Body).1 + 6.0).max(t.size_menu_row);
    let h = rows as f32 * row + 2.0 * (t.radius_card - t.radius_row) + 24.0;
    let w = 2.0 * t.size_menu_max;
    (w.ceil() as i32, h.ceil() as i32).into()
}

/// What Isle's About panel shows of an app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct About {
    pub name: String,
    pub icon: Option<PathBuf>,
    pub version: Option<String>,
    pub package: Option<String>,
    pub comment: Option<String>,
}

#[derive(Default)]
pub struct AboutPanel {
    model: Rc<RefCell<About>>,
    state: Rc<RefCell<Option<State<About>>>>,
}

impl AboutPanel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_model(&self, about: About) {
        *self.model.borrow_mut() = about;
    }

    /// Puts `about` in force; call it in the tree's root scope.
    pub fn set(&self, about: About) {
        *self.model.borrow_mut() = about.clone();
        if let Some(mut state) = *self.state.borrow() {
            state.set_if_modified(about);
        }
    }

    pub fn size() -> SSize<i32, Logical> {
        (320, 260).into()
    }
}

impl SurfaceDef for AboutPanel {
    fn name(&self) -> &'static str {
        "about"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size(Self::size())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(AboutApp {
            model: self.model.clone(),
            state: self.state.clone(),
        })
    }

    fn corner_radius(&self, theme: &Theme) -> f32 {
        theme.tokens.radius_panel
    }
}

#[derive(Clone)]
struct AboutApp {
    model: Rc<RefCell<About>>,
    state: Rc<RefCell<Option<State<About>>>>,
}

impl App for AboutApp {
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
        AboutView { about: state }
    }
}

#[derive(PartialEq)]
struct AboutView {
    about: State<About>,
}

impl Component for AboutView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let about = self.about.read().clone();
        let actions = Actions::get();
        let close = actions.clone();
        let ink = colour(t.colour_text);
        let ink2 = colour(t.colour_text_2);
        rect()
            .expanded()
            .corner_radius(t.radius_panel)
            .background(colour(t.material_solid))
            .map(Some(shadows(&t.shadow_panel)), |el, list| {
                list.into_iter().fold(el, |el, s| el.shadow(s))
            })
            .padding(t.space_l)
            .spacing(t.space_s)
            .cross_align(Alignment::Center)
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                if e.key == Key::Named(NamedKey::Escape) {
                    actions.send(Action::Close);
                }
            })
            .child(match about.icon {
                Some(path) => ImageViewer::new(path)
                    .width(Size::px(64.))
                    .height(Size::px(64.))
                    .into_element(),
                None => Glyph::new("app-window", 64.0, ink2).into_element(),
            })
            .child(text::line(
                &theme,
                text::Style::Title,
                ink,
                about.name.clone(),
            ))
            .map(about.version.clone(), |el, v| {
                el.child(text::line(
                    &theme,
                    text::Style::Callout,
                    ink2,
                    format!("Version {v}"),
                ))
            })
            .map(about.package.clone(), |el, p| {
                el.child(text::line(
                    &theme,
                    text::Style::Callout,
                    ink2,
                    format!("From {p}"),
                ))
            })
            .map(about.comment.clone(), |el, c| {
                el.child(text::line(&theme, text::Style::Body, ink, c))
            })
            .child(rect().height(Size::flex(1.)))
            .child(
                Button::new("Close")
                    .kind(Kind::Default)
                    .on_press(move |_| close.send(Action::Close)),
            )
    }
}
