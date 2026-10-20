//! Appearance: light or dark, the accent colour, the glass, other apps' title bars, and the
//! Advanced… sheet for the tokens file.

use freya::prelude::*;
use isle_settings::Value;
use isle_tokens::{Source, Theme};
use isle_ui::{colour, form, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Services},
    ui::{self, Picture, Pictures, Rows},
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
        // Eight themes resolved: kept until the mode or contrast changes, not per glass step.
        type Swatches = Vec<(String, Color, Color)>;
        let cache = use_hook(|| {
            std::rc::Rc::new(std::cell::RefCell::new(None::<((isle_tokens::Mode, bool), Swatches)>))
        });
        let key = (theme.appearance.mode, theme.appearance.increase_contrast);
        let swatches = {
            let mut cached = cache.borrow_mut();
            match cached.as_ref() {
                Some((k, list)) if *k == key => list.clone(),
                _ => {
                    let list = accents(&theme);
                    *cached = Some((key, list.clone()));
                    list
                }
            }
        };
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
        let (nav, at) = (Nav::get(), self.at.clone());
        let sheet = self.at.sub.clone();
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
            .child(ui::foot(vec![(
                "Advanced…".into(),
                EventHandler::new({
                    let at = at.clone();
                    move |_| super::open(nav, &at, "Advanced…")
                }),
            )]))
            .maybe(sheet.as_deref() == Some("Advanced…"), |el| {
                el.child(Advanced { at: at.clone() })
            })
    }
}

/// The tokens file: where it is, whether it is in force, and the editor to change it in.
#[derive(PartialEq)]
struct Advanced {
    at: Location,
}

impl Component for Advanced {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let services = Services::get();
        let status = r.desktop.tokens_status.read().clone();
        let text = r.desktop.tokens_text.read().clone();
        let path = status.as_ref().map(|s| s.file.clone()).unwrap_or_default();
        let shown = path.replacen(&std::env::var("HOME").unwrap_or_default(), "~", 1);
        let (state, detail, refused) = match &status {
            Some(s) if !s.ok => (
                "Not loaded",
                None,
                Some(format!(
                    "Line {}: {}. The shipped tokens are in use until the file reads.",
                    s.line, s.message
                )),
            ),
            _ if text.trim().is_empty() => (
                "None",
                Some("The shipped tokens are in use.".to_owned()),
                None,
            ),
            _ => {
                let n = isle_tokens::Source::changed(&text);
                let tokens = if n == 1 { "token" } else { "tokens" };
                ("Loaded", Some(format!("Changes {n} {tokens}.")), None)
            }
        };
        let none = state == "None";
        let mut status_row = r
            .info("tokens-status", "Status", state.to_owned())
            .refused(refused);
        if let Some(detail) = detail {
            status_row = status_row.description(detail);
        }
        let at = self.at.clone();
        let editor = path.clone();
        ui::sheet(
            520.,
            vec![
                ui::alert_text(
                    "Advanced",
                    "The look's sizes, colours and timings are design tokens. A file of your \
                     own changes any of them, and is read each time you save it.",
                ),
                form::group(
                    None,
                    vec![
                        r.info("tokens-file", "Tokens file", shown).into_element(),
                        status_row.into_element(),
                    ],
                )
                .into_element(),
            ],
            vec![
                (
                    "Show Shipped Tokens".into(),
                    EventHandler::new(move |_| {
                        services.act(Act::EditTokens {
                            path: String::new(),
                            shipped: true,
                        })
                    }),
                ),
                (
                    if none { "Create in Editor…" } else { "Open in Editor" }.into(),
                    EventHandler::new(move |_| {
                        services.act(Act::EditTokens {
                            path: editor.clone(),
                            shipped: false,
                        })
                    }),
                ),
                (
                    "Done".into(),
                    EventHandler::new(move |_| super::close(nav, &at)),
                ),
            ],
        )
    }
}
