//! Every `isle-ui` component in its states on one surface, to check them against the
//! design's boards in the preview and in test builds.

use std::rc::Rc;

use freya::prelude::*;
use freya_embed::AppComponent;
use isle_ui::{
    Button, Glyph, LevelBar, LevelGlyph, Menu, MenuEntry, MenuItem, Slider, Spinner, Switch,
    button::Kind,
    colour,
    menu::{Hint, Mark, Material},
    text::{self, Style},
    use_theme,
};
use smithay::utils::{Logical, Rectangle, Size as SSize};

use crate::{
    Layer, Scope, SurfaceDef,
    island::rows::{Choice, Lead, PageRow, Rows, SliderRow},
};

#[derive(Default)]
pub struct Gallery;

impl SurfaceDef for Gallery {
    fn name(&self) -> &'static str {
        "gallery"
    }

    fn layer(&self) -> Layer {
        Layer::Top
    }

    fn scope(&self) -> Scope {
        Scope::MainOutput
    }

    fn takes_keyboard(&self) -> bool {
        true
    }

    fn geometry(&self, output: SSize<i32, Logical>) -> Rectangle<i32, Logical> {
        let size = SSize::from((1200.min(output.w), 720.min(output.h)));
        Rectangle::new(
            ((output.w - size.w) / 2, (output.h - size.h) / 2).into(),
            size,
        )
    }

    fn app(&self) -> AppComponent {
        AppComponent::new(GalleryApp)
    }
}

#[derive(Clone)]
struct GalleryApp;

impl App for GalleryApp {
    fn render(&self) -> impl IntoElement {
        Components
    }
}

fn hint(modifiers: &str, key: &str) -> Option<Hint> {
    Some(Hint {
        modifiers: modifiers.into(),
        key: key.into(),
    })
}

fn sample_menu() -> Rc<Vec<MenuEntry>> {
    let sort = vec![
        MenuEntry::Item(MenuItem::new(20, "Name").mark(Mark::Check)),
        MenuEntry::Item(MenuItem::new(21, "Kind")),
        MenuEntry::Item(MenuItem::new(22, "Date Modified")),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new(23, "Folders first").mark(Mark::Mixed)),
    ];
    Rc::new(vec![
        MenuEntry::Section("View".into()),
        MenuEntry::Item(
            MenuItem::new(1, "as Icons")
                .mark(Mark::Check)
                .hint(hint("⌘", "1")),
        ),
        MenuEntry::Item(MenuItem::new(2, "as List").hint(hint("⌘", "2"))),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new(3, "Sort By").submenu(sort)),
        MenuEntry::Item(MenuItem::new(4, "Show Hidden Files").hint(hint("⇧⌘", "."))),
        MenuEntry::Item(MenuItem::new(5, "Clean Up").enabled(false)),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new(6, "Enter Full Screen").hint(hint("⌃⌘", "F"))),
    ])
}

fn sample_rows() -> Vec<PageRow> {
    let choice = |id: &str, label: &str| Choice {
        id: id.into(),
        label: label.into(),
        enabled: true,
        ..Default::default()
    };
    vec![
        PageRow::Heading("Known networks".into()),
        PageRow::Choice(Choice {
            checked: true,
            trailing: Some("wifi-high".into()),
            lead: Lead::Glyph("lock-simple".into()),
            ..choice("home", "Home")
        }),
        PageRow::Choice(Choice {
            busy: true,
            detail: Some("Connecting".into()),
            ..choice("cafe", "Café")
        }),
        PageRow::Choice(Choice {
            error: Some("Couldn't connect to “Office”. Check the password and try again.".into()),
            ..choice("office", "Office")
        }),
        PageRow::Choice(Choice {
            battery: Some(64),
            detail: Some("Connected".into()),
            sub_page: true,
            ..choice("mouse", "MX Master 3S")
        }),
        PageRow::Group {
            id: "other".into(),
            label: "Other networks".into(),
            open: false,
            rows: vec![],
        },
        PageRow::Slider(SliderRow {
            id: "firefox".into(),
            label: "Firefox".into(),
            value: 0.7,
            enabled: true,
            ..Default::default()
        }),
        PageRow::Switch {
            id: "contrast".into(),
            label: "Increase contrast".into(),
            on: false,
            enabled: true,
        },
        PageRow::Status("No apps playing sound".into()),
    ]
}

#[derive(PartialEq)]
struct Components;

impl Component for Components {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let ink = colour(t.colour_text);
        let glyphs = [
            "wifi-high",
            "bluetooth-connected",
            "speaker-slash",
            "isle-vpn",
            "moon",
            "arrows-clockwise",
            "battery-warning",
            "isle-keyboard-slash",
            "warning-circle",
            "isle-mark",
            "isle-equaliser",
        ];
        let menu = use_hook(sample_menu);
        let mut level = use_state(|| 0.6_f32);
        let mut toggled = use_state(|| true);
        rect()
            .expanded()
            .padding(t.space_xl)
            .spacing(t.space_l)
            .background(colour(t.colour_window_sunken))
            .child(text::line(&theme, Style::Title, ink, "Components"))
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_m)
                    .cross_align(Alignment::Center)
                    .children(
                        glyphs
                            .iter()
                            .map(|g| Glyph::new(*g, t.size_glyph, ink).alt(*g).into_element()),
                    )
                    .child(Glyph::new("isle-vpn", t.size_glyph, ink).fill(true))
                    .child(LevelGlyph {
                        whole: "wifi-high".into(),
                        level: "wifi-low".into(),
                        size: t.size_glyph,
                        colour: ink,
                        dimmed: colour(t.colour_text_disabled),
                        alt: "Wi-Fi, signal low".into(),
                    }),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_m)
                    .cross_align(Alignment::Center)
                    .child(Button::new("Regular"))
                    .child(Button::new("Default").kind(Kind::Default))
                    .child(Button::new("Plain").kind(Kind::Plain))
                    .child(Button::new("Disabled").enabled(false))
                    .child(Button::new("Small").small())
                    .child(Button::new("Stop").small().kind(Kind::Default))
                    .child(Button::icon("caret-left", "Back").kind(Kind::Plain)),
            )
            .child(rect().width(Size::px(300.)).child(LevelBar {
                level: 0.4,
                track: colour(t.colour_fill_strong),
                fill: colour(t.colour_text),
            }))
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_l)
                    .cross_align(Alignment::Center)
                    .child(rect().width(Size::px(200.)).child(Slider::new(
                        *level.read(),
                        "Volume",
                        move |v| level.set(v),
                    )))
                    .child(
                        rect()
                            .width(Size::px(200.))
                            .child(Slider::new(0.4, "Brightness", |_| {}).glass(true)),
                    )
                    .child(
                        rect()
                            .width(Size::px(200.))
                            .child(Slider::new(0.4, "Muted", |_| {}).glass(true).muted(true)),
                    )
                    .child(
                        rect()
                            .width(Size::px(200.))
                            .child(Slider::new(0.3, "Position", |_| {}).glass(true).scrubber()),
                    )
                    .child(Switch::new(*toggled.read(), "Wi-Fi", move |on| toggled.set(on)))
                    .child(Switch::new(false, "Off", |_| {}))
                    .child(Switch::new(true, "Disabled", |_| {}).enabled(false))
                    .child(Spinner::new(t.size_glyph, "Working")),
            )
            .child(
                rect()
                    .width(Size::px(t.size_panel))
                    .corner_radius(t.radius_card)
                    .background(colour(t.colour_fill))
                    .padding((t.space_xs, 0.))
                    .child(Rows {
                        rows: sample_rows(),
                        on_event: (|e| tracing::info!(?e, "row")).into(),
                    }),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(t.space_xxl)
                    .child(
                        Menu::new(menu.clone(), |e| tracing::info!(?e, "menu"))
                            .first_selected(true),
                    )
                    .child(
                        Menu::new(menu, |e| tracing::info!(?e, "menu")).material(Material::Window),
                    ),
            )
    }
}
