//! Appearance: light or dark, the accent colour, the glass and other apps' title bars.

use freya::prelude::*;
use isle_settings::Value;
use isle_tokens::{Source, Theme};
use isle_ui::{colour, form, use_theme};

use crate::{
    nav::Location,
    ui::{Picture, Pictures, Rows},
};

const MODES: [(&str, &str); 3] = [("auto", "Auto"), ("light", "Light"), ("dark", "Dark")];

#[derive(PartialEq)]
pub struct Appearance {
    pub at: Location,
}

/// Each accent's name and its fill and mark on the theme shown.
fn accents(theme: &Theme) -> Vec<(String, Color, Color)> {
    let source = Source::shipped();
    source
        .accents()
        .into_iter()
        .filter_map(|name| {
            let appearance = isle_tokens::Appearance {
                accent: name.clone(),
                ..theme.appearance.clone()
            };
            let tokens = source.theme(&appearance).ok()?.tokens;
            let mut label = name.clone();
            label[..1].make_ascii_uppercase();
            Some((
                label,
                colour(tokens.colour_accent),
                colour(tokens.colour_on_accent),
            ))
        })
        .collect()
}

impl Component for Appearance {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let swatches = accents(&theme);
        let names: Vec<String> = Source::shipped().accents();
        let accent = r.str("appearance.accent").unwrap_or_default();
        let mode = r.str("appearance.mode").unwrap_or_default();
        let light = Picture {
            backdrop: (
                Color::from_rgb(214, 222, 235),
                Color::from_rgb(214, 222, 235),
            ),
            window: (Color::WHITE, Color::WHITE),
        };
        let dark = Picture {
            backdrop: (Color::from_rgb(38, 44, 58), Color::from_rgb(38, 44, 58)),
            window: (Color::from_rgb(30, 31, 35), Color::from_rgb(30, 31, 35)),
        };
        let auto = Picture {
            backdrop: (light.backdrop.0, dark.backdrop.1),
            window: (light.window.0, dark.window.1),
        };
        let pictures = Pictures {
            choices: vec![
                ("Auto".into(), auto),
                ("Light".into(), light),
                ("Dark".into(), dark),
            ],
            selected: MODES.iter().position(|(v, _)| *v == mode),
            alt: "Appearance".into(),
            on_select: EventHandler::new(move |i: usize| {
                desktop.set("appearance.mode", Value::Str(MODES[i].0.into()))
            }),
        };
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(
                None,
                vec![
                    r.row("appearance", "Appearance", Some("appearance.mode"))
                        .control(pictures)
                        .into_element(),
                    r.row("colour", "Colour", Some("appearance.accent"))
                        .control(form::Swatches::new(
                            swatches,
                            names.iter().position(|n| *n == accent),
                            move |i: usize| {
                                desktop.set("appearance.accent", Value::Str(names[i].clone()))
                            },
                        ))
                        .into_element(),
                    r.slider(
                        "glass",
                        "Glass",
                        "appearance.glass",
                        (0., 1.),
                        ("Clear", "Tinted"),
                    )
                    .into_element(),
                ],
            ))
            .child(form::group(
                Some("Windows".into()),
                vec![
                    r.switch(
                        "title-bars",
                        "Show title bars on other apps' windows",
                        "windows.title-bars",
                    )
                    .into_element(),
                ],
            ))
    }
}
