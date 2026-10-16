//! A surface drawn once offscreen at startup, holding one of each thing the shell draws, so
//! Skia compiles its programs before the first menu opens rather than as it does.

use std::rc::Rc;

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_ui::{
    Button, Glass, Glyph, Menu, MenuEntry, MenuItem, SearchField, Slider, Switch, button::Kind,
    colour, menu::Hint, menu::Material, shadows, text, use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{Layer, Scope, SurfaceDef};

pub struct WarmUp;

impl SurfaceDef for WarmUp {
    fn name(&self) -> &'static str {
        "warm-up"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::Placed
    }

    fn takes_keyboard(&self) -> bool {
        false
    }

    fn geometry(&self, _output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        Rectangle::from_size((1200, 700).into())
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(Scene)
    }

    fn transitions(&self) -> bool {
        false
    }
}

#[derive(Clone, PartialEq)]
struct Scene;

fn entries() -> Rc<Vec<MenuEntry>> {
    let hint = Some(Hint {
        modifiers: "⌘".into(),
        key: "Q".into(),
    });
    Rc::new(vec![
        MenuEntry::Section("Section".into()),
        MenuEntry::Item(MenuItem::new(1, "Item").hint(hint)),
        MenuEntry::Item(MenuItem::new(2, "Disabled").enabled(false)),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new(3, "Submenu").submenu(vec![MenuEntry::Item(MenuItem::new(4, "Inner"))])),
    ])
}

impl App for Scene {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let query = use_state(|| "Query".to_string());
        let ink = colour(t.colour_text);
        let panel = |list: &[isle_tokens::Shadow], solid: bool| {
            rect()
                .width(Size::px(160.))
                .height(Size::px(90.))
                .corner_radius(t.radius_card)
                .background(if solid {
                    colour(t.material_solid)
                } else {
                    Color::TRANSPARENT
                })
                .map(Some(shadows(list)), |el, list| list.into_iter().fold(el, |el, s| el.shadow(s)))
                .maybe(!solid, |el| el.child(Glass::new(t.radius_card)))
        };
        let styles = [
            text::Style::Body,
            text::Style::Callout,
            text::Style::Clock,
            text::Style::Footnote,
            text::Style::Headline,
            text::Style::Subheadline,
            text::Style::Title,
            text::Style::Title3,
        ];
        rect()
            .expanded()
            .padding(40.)
            .spacing(24.)
            .child(
                rect()
                    .horizontal()
                    .spacing(24.)
                    .child(Menu::new(entries(), |_| {}).material(Material::Glass).first_selected(true))
                    .child(Menu::new(entries(), |_| {}).material(Material::Solid))
                    .child(Menu::new(entries(), |_| {}).material(Material::Window))
                    .child(SearchField {
                        value: query.into_writable(),
                        placeholder: "Search".into(),
                        width: t.size_menu_search,
                    }),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(24.)
                    .child(panel(&t.shadow_menu, false))
                    .child(panel(&t.shadow_panel, false))
                    .child(panel(&t.shadow_bar, true))
                    .child(panel(&t.shadow_window, true))
                    .child(panel(&t.shadow_control, true)),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(16.)
                    .child(Button::new("Regular"))
                    .child(Button::new("Default").kind(Kind::Default))
                    .child(Button::new("Plain").kind(Kind::Plain))
                    .child(Button::new("Destructive").kind(Kind::Destructive))
                    .child(Switch::new(true, "Switch", |_| {}))
                    .child(Slider::new(0.5, "Slider", |_| {}))
                    .child(Glyph::new("isle-mark", t.size_glyph, ink)),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(16.)
                    .children(styles.into_iter().map(|style| text::line(&theme, style, ink, "Text").into_element())),
            )
    }
}
