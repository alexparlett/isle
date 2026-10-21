//! Wallpaper: Frost, a colour or a picture, per screen or for all, and Dark Mode's.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{colour, form};

use crate::{
    nav::Location,
    services::{Follow, use_follow},
    ui::{Ask, Picture, Pictures, Rows},
};

/// The colours offered; the last swatch opens a field for any.
const COLOURS: [(&str, &str); 7] = [
    ("Black", "#000000"),
    ("Graphite", "#2a2b30"),
    ("Slate", "#3c4a5c"),
    ("Navy", "#1f3a68"),
    ("Teal", "#1d6b73"),
    ("Moss", "#3d5a3a"),
    ("Sand", "#c8b58f"),
];

/// What a source is: Frost, a colour or a picture (or folder).
pub fn kind(source: &str) -> usize {
    if source == "frost" {
        0
    } else if source.starts_with('#') {
        1
    } else {
        2
    }
}

/// `#rrggbb` as a colour.
pub fn hex(text: &str) -> Option<Color> {
    let h = text.strip_prefix('#').filter(|h| h.len() == 6)?;
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn expand(path: &str) -> std::path::PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => std::env::home_dir().unwrap_or_default().join(rest),
        None => path.into(),
    }
}

#[derive(PartialEq)]
pub struct Wallpaper {
    pub at: Location,
}

impl Component for Wallpaper {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Displays]);
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        let mut asking = use_state(|| None::<&'static str>);
        let (app, window) = (isle_app::app(), isle_app::this_window());
        // Files' panel through the portal; with no portal, the path is typed.
        let choose = move |what: &'static str, target: String| {
            let mut asking = asking;
            let Some(bus) = app.bus() else {
                asking.set(Some(what));
                return;
            };
            let parent = app.export(window).unwrap_or_default();
            spawn(async move {
                let wanted = isle_app::portal::Wanted {
                    title: if what == "dark" { "Choose a Dark Mode picture".into() } else { "Choose a picture".into() },
                    types: vec![("Images".into(), vec!["image/*".into()])],
                    folder: dirs_pictures(),
                    ..Default::default()
                };
                match isle_app::portal::choose_file(&bus, &parent, wanted).await {
                    Ok(Some(path)) => desktop.set(target, Value::Str(path.display().to_string())),
                    Ok(None) => {}
                    Err(err) => {
                        tracing::warn!("no file chooser: {err}");
                        asking.set(Some(what));
                    }
                }
            });
        };
        let displays: Vec<isle_ipc::DisplayInfo> = services
            .displays
            .read()
            .state
            .iter()
            .filter(|d| d.enabled)
            .cloned()
            .collect();
        let identities: Vec<String> = displays
            .iter()
            .map(|d| format!("{} {} {}", d.make, d.model, d.serial))
            .collect();
        let per_screen = r
            .desktop
            .values
            .read()
            .keys()
            .any(|k| k.starts_with("wallpaper-output["));
        let mut screen = use_state(|| 0usize);
        let shown = (*screen.read()).min(identities.len().saturating_sub(1));
        let section = match identities.get(shown) {
            Some(id) if per_screen => format!("wallpaper-output[{id}]"),
            _ => "wallpaper".to_owned(),
        };
        let key = |f: &str| format!("{section}.{f}");
        let source = r.str(&key("source")).unwrap_or_else(|| "frost".into());
        let dark = r.str(&key("dark-source"));
        let fit = r.str(&key("fit")).unwrap_or_else(|| "fill".into());
        let fill = r.str(&key("colour"));
        let current = kind(&source);
        let is_folder = current == 2 && expand(&source).is_dir();
        let missing = current == 2 && !expand(&source).exists();

        let preview = {
            let base = rect()
                .width(Size::px(240.))
                .height(Size::px(150.))
                .corner_radius(t.radius_card)
                .overflow(Overflow::Clip);
            match current {
                1 => base
                    .background(hex(&source).unwrap_or(Color::BLACK))
                    .into_element(),
                2 if !is_folder && !missing => base
                    .background(fill.as_deref().and_then(hex).unwrap_or(Color::BLACK))
                    .child(
                        ImageViewer::new(expand(&source))
                            .width(Size::fill())
                            .height(Size::fill()),
                    )
                    .into_element(),
                _ => base.background(colour(t.colour_fill_strong)).into_element(),
            }
        };

        let frost = Picture {
            backdrop: (
                Color::from_rgb(110, 140, 190),
                Color::from_rgb(160, 120, 190),
            ),
            window: (
                Color::from_rgb(200, 215, 235),
                Color::from_rgb(215, 200, 235),
            ),
        };
        let solid = Picture {
            backdrop: (Color::from_rgb(31, 58, 104), Color::from_rgb(31, 58, 104)),
            window: (Color::from_rgb(31, 58, 104), Color::from_rgb(31, 58, 104)),
        };
        let photo = Picture {
            backdrop: (Color::from_rgb(70, 110, 80), Color::from_rgb(190, 160, 110)),
            window: (
                Color::from_rgb(120, 160, 200),
                Color::from_rgb(120, 160, 200),
            ),
        };
        let source_key = key("source");
        let pictures = Pictures {
            choices: vec![
                ("Frost".into(), frost),
                ("Colour".into(), solid),
                ("Picture".into(), photo),
            ],
            selected: Some(current),
            alt: "Wallpaper".into(),
            on_select: EventHandler::new({
                let source_key = source_key.clone();
                move |i: usize| match i {
                    0 => desktop.set(source_key.clone(), Value::Str("frost".into())),
                    1 => desktop.set(source_key.clone(), Value::Str(COLOURS[3].1.into())),
                    _ => asking.set(Some("source")),
                }
            }),
        };

        let mut rows = vec![
            r.switch_with("all-screens", "Show on all screens", !per_screen, {
                let identities = identities.clone();
                move |on| {
                    if on {
                        for id in &identities {
                            desktop.remove(format!("wallpaper-output[{id}]"));
                        }
                    } else if let Some(id) = identities.first() {
                        desktop.set(
                            format!("wallpaper-output[{id}].source"),
                            Value::Str("frost".into()),
                        );
                    }
                }
            })
            .into_element(),
        ];
        if per_screen && !displays.is_empty() {
            rows.push(
                r.row("screen", "Screen", None)
                    .control(form::PopUp::new(
                        displays
                            .iter()
                            .map(|d| {
                                if d.model.is_empty() {
                                    d.name.clone()
                                } else {
                                    d.model.clone()
                                }
                            })
                            .collect(),
                        shown,
                        "Screen",
                        move |i: usize| screen.set(i),
                    ))
                    .into_element(),
            );
        }
        rows.push(
            r.row("wallpaper", "Wallpaper", Some(&source_key))
                .control(pictures)
                .into_element(),
        );
        if current == 2 {
            rows.push(
                r.row(
                    "picture",
                    if is_folder { "Folder" } else { "Picture" },
                    Some(&source_key),
                )
                .description(if missing {
                    "Missing".to_owned()
                } else {
                    source.clone()
                })
                .control(
                    isle_ui::Button::new("Choose…").on_press({
                        let (choose, target) = (choose.clone(), source_key.clone());
                        move |_| choose("source", target.clone())
                    }),
                )
                .into_element(),
            );
            let fit_key = key("fit");
            const FITS: [(&str, &str); 3] = [
                ("fill", "Fill screen"),
                ("fit", "Fit to screen"),
                ("centre", "Centre"),
            ];
            rows.push(
                r.row("fit", "Fit", Some(&fit_key))
                    .control(form::PopUp::new(
                        FITS.iter().map(|(_, l)| (*l).to_owned()).collect(),
                        FITS.iter().position(|(v, _)| *v == fit).unwrap_or(0),
                        "Fit",
                        move |i: usize| desktop.set(fit_key.clone(), Value::Str(FITS[i].0.into())),
                    ))
                    .into_element(),
            );
        }
        if current == 1 || (current == 2 && fit != "fill") {
            let colour_key = if current == 1 {
                source_key.clone()
            } else {
                key("colour")
            };
            let chosen = if current == 1 {
                Some(source.clone())
            } else {
                fill.clone()
            };
            let mut swatches: Vec<(String, Color, Color)> = COLOURS
                .iter()
                .map(|(n, h)| ((*n).to_owned(), hex(h).unwrap(), Color::WHITE))
                .collect();
            swatches.push((
                "Other…".into(),
                colour(t.colour_fill_strong),
                colour(t.colour_text),
            ));
            let selected = chosen
                .as_deref()
                .and_then(|c| COLOURS.iter().position(|(_, h)| h.eq_ignore_ascii_case(c)));
            rows.push(
                r.row("colour", "Colour", Some(&colour_key))
                    .control(form::Swatches::new(
                        swatches,
                        selected,
                        move |i: usize| match COLOURS.get(i) {
                            Some((_, h)) => {
                                desktop.set(colour_key.clone(), Value::Str((*h).into()))
                            }
                            None => asking.set(Some("colour")),
                        },
                    ))
                    .into_element(),
            );
        }

        let dark_key = key("dark-source");
        let mut dark_rows = vec![
            r.switch_with(
                "dark-mode",
                "Use a different picture in Dark Mode",
                dark.is_some(),
                {
                    let (dark_key, source) = (dark_key.clone(), source.clone());
                    move |on| {
                        if on {
                            desktop.set(dark_key.clone(), Value::Str(source.clone()))
                        } else {
                            desktop.reset(dark_key.clone())
                        }
                    }
                },
            )
            .into_element(),
        ];
        if let Some(dark) = &dark {
            dark_rows.push(
                r.row("dark-picture", "Dark Mode picture", Some(&dark_key))
                    .description(dark.clone())
                    .control(
                        isle_ui::Button::new("Choose…").on_press({
                            let (choose, target) = (choose.clone(), dark_key.clone());
                            move |_| choose("dark", target.clone())
                        }),
                    )
                    .into_element(),
            );
        }

        let mut groups = rect()
            .width(Size::fill())
            .spacing(20.)
            .child(
                rect()
                    .width(Size::fill())
                    .cross_align(Alignment::Center)
                    .child(preview),
            )
            .child(form::group(None, rows))
            .child(form::group(Some("Dark Mode".into()), dark_rows));
        if is_folder {
            let change_key = key("change");
            const CHANGES: [(&str, &str); 4] = [
                ("hourly", "Every hour"),
                ("daily", "Every day"),
                ("login", "When logging in"),
                ("never", "Never"),
            ];
            let change = r.str(&change_key).unwrap_or_else(|| "daily".into());
            groups = groups.child(form::group(
                Some("Folder".into()),
                vec![
                    r.row("change", "Change picture", Some(&change_key))
                        .control(form::PopUp::new(
                            CHANGES.iter().map(|(_, l)| (*l).to_owned()).collect(),
                            CHANGES.iter().position(|(v, _)| *v == change).unwrap_or(1),
                            "Change picture",
                            move |i: usize| {
                                desktop.set(change_key.clone(), Value::Str(CHANGES[i].0.into()))
                            },
                        ))
                        .into_element(),
                    r.switch_key("shuffle", "Random order", key("shuffle"), false, true)
                        .into_element(),
                ],
            ));
        }
        let asked = *asking.read();
        groups
            .map(asked, |el, what| {
                let (title, placeholder, target) = match what {
                    "colour" => (
                        "Colour",
                        "#1f3a68",
                        if current == 1 {
                            source_key.clone()
                        } else {
                            key("colour")
                        },
                    ),
                    "dark" => (
                        "Dark Mode picture",
                        "~/Pictures/night.jpg",
                        dark_key.clone(),
                    ),
                    _ => (
                        "Picture or folder",
                        "~/Pictures/Wallpapers",
                        source_key.clone(),
                    ),
                };
                el.child(Ask {
                    title: title.into(),
                    placeholder: placeholder.into(),
                    initial: String::new(),
                    on_done: EventHandler::new(move |text: String| {
                        let text = text.trim().to_owned();
                        let ok = if what == "colour" {
                            hex(&text).is_some()
                        } else {
                            !text.is_empty()
                        };
                        if ok {
                            let text = if what == "colour" {
                                text
                            } else {
                                expand(&text).display().to_string()
                            };
                            desktop.set(target.clone(), Value::Str(text));
                            asking.set(None);
                        }
                    }),
                    on_cancel: EventHandler::new(move |_| asking.set(None)),
                })
            })
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_is_frost_a_colour_or_a_picture() {
        assert_eq!(kind("frost"), 0);
        assert_eq!(kind("#1f3a68"), 1);
        assert_eq!(kind("~/Pictures/a.jpg"), 2);
        assert_eq!(hex("#ff0000"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(hex("red"), None);
    }
}

/// The user's Pictures folder, where a picture is first looked for.
fn dirs_pictures() -> Option<std::path::PathBuf> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    Some(home.join("Pictures")).filter(|p| p.is_dir())
}
