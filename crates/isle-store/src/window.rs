//! Store's window (`p14e-store.md`, board `p14e/store-1.html`): the sidebar, then Updates by
//! catalogue or the installed apps.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use freya::prelude::*;
use isle_app::ToolbarExt;
use isle_tokens::{Colour, Theme};
use isle_ui::{AppIcon, Button, Glyph, button::Kind, colour, scroll_area, text, use_theme};
use serde_json::Value;

use crate::{
    data::{self, Ask, Event},
    installed::{App, Catalogue},
};

/// The compositor's lights over the toolbar's leading end.
const LIGHTS: f32 = 68.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Updates,
    Installed,
}

#[derive(Clone, PartialEq)]
pub struct StoreWindow;

fn hue(theme: &Theme, name: &str) -> Colour {
    let t = &theme.tokens;
    match name {
        "teal" => t.pane_teal,
        "blue" => t.pane_blue,
        "indigo" => t.pane_indigo,
        "purple" => t.pane_purple,
        _ => t.pane_grey,
    }
}

/// A catalogue's name in its hue, as every app and package wears it.
fn badge(theme: &Theme, catalogue: Catalogue) -> Element {
    let t = &theme.tokens;
    let ink = colour(hue(theme, catalogue.hue()));
    rect()
        .padding((1., t.space_xs))
        .corner_radius(999.)
        .border(
            Border::new()
                .fill(ink)
                .width(t.stroke_hairline)
                .alignment(BorderAlignment::Inner),
        )
        .child(
            text::line(theme, text::Style::Footnote, ink, catalogue.name())
                .font_weight(t.type_weight_semibold as i32),
        )
        .into_element()
}

fn icon(theme: &Theme, app: Option<&App>, size: f32) -> Element {
    let t = &theme.tokens;
    match app.and_then(|a| a.icon.clone()) {
        Some(path) => AppIcon::new(path, size).into_element(),
        None => rect()
            .width(Size::px(size))
            .height(Size::px(size))
            .corner_radius(size / 4.)
            .background(colour(t.colour_fill_strong))
            .center()
            .child(Glyph::new(
                "app-window",
                size * 0.5,
                colour(t.colour_text_2),
            ))
            .into_element(),
    }
}

/// One update: the app's icon, name and catalogue, the versions, and what can be done.
fn update_row(
    theme: &Theme,
    app: Option<&App>,
    name: &str,
    catalogue: Catalogue,
    versions: String,
    trailing: Element,
) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding((t.space_m, 0.))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: t.stroke_hairline,
                    left: 0.,
                }),
        )
        .child(icon(theme, app, 48.))
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t.space_xxs)
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(text::line(
                            theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            name,
                        ))
                        .child(badge(theme, catalogue)),
                )
                .child(text::line(
                    theme,
                    text::Style::Callout,
                    colour(t.colour_text_2),
                    versions,
                )),
        )
        .child(trailing)
        .into_element()
}

fn section(theme: &Theme, title: &str, trailing: Option<Element>) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .padding((t.space_l, 0., t.space_xs, 0.))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: t.stroke_hairline,
                    left: 0.,
                }),
        )
        .child(rect().width(Size::flex(1.)).child(text::line(
            theme,
            text::Style::Title3,
            colour(t.colour_text),
            title,
        )))
        .maybe_child(trailing)
        .into_element()
}

fn note(theme: &Theme, line: impl Into<String>) -> Element {
    let t = &theme.tokens;
    text::line(theme, text::Style::Callout, colour(t.colour_text_2), line).into_element()
}

/// The part of a run that is Store's: Flathub's and the AUR's, not the system's.
fn store_run(status: &Value) -> Option<&Value> {
    let run = &status["progress"];
    let part = run["part"].as_str().unwrap_or_default();
    (part.starts_with("flathub") || part.starts_with("aur")).then_some(run)
}

fn pending<'a>(status: &'a Value, source: &str) -> Vec<&'a Value> {
    status["pending"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["source"] == source)
        .collect()
}

impl freya::prelude::App for StoreWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let mut page = use_state(|| Page::Updates);
        let mut status = use_state(|| Value::Null);
        let mut apps = use_state(Vec::<App>::new);
        let mut failed = use_state(|| None::<String>);
        // Update Everything's parts still to run, one after another.
        let queue = use_hook(|| Rc::new(RefCell::new(VecDeque::<String>::new())));
        let (data, incoming) = use_hook(move || {
            let (data, rx) = data::start(&worker, bus);
            (data, Rc::new(RefCell::new(Some(rx))))
        });
        use_hook({
            let data = data.clone();
            let queue = queue.clone();
            move || {
                let Some(mut rx) = incoming.borrow_mut().take() else {
                    return;
                };
                spawn(async move {
                    use futures_util::StreamExt;
                    let mut was_running = false;
                    while let Some(event) = rx.next().await {
                        match event {
                            Event::Status(next) => {
                                let running = next["progress"]["state"] == "Running";
                                if was_running && !running {
                                    data.ask(Ask::Rescan);
                                    if next["progress"]["state"] == "Done"
                                        && let Some(part) = queue.borrow_mut().pop_front()
                                    {
                                        data.ask(Ask::Install(part));
                                    } else {
                                        queue.borrow_mut().clear();
                                    }
                                }
                                was_running = running;
                                status.set(next);
                            }
                            Event::Installed(list) => apps.set(list),
                            Event::Failed(why) => failed.set(Some(why)),
                        }
                    }
                });
            }
        });

        let s = status.read().clone();
        let installed = apps.read().clone();
        let by_id = |id: &str| {
            installed
                .iter()
                .find(|a| a.id == id || a.source == id)
                .cloned()
        };
        let flathub = pending(&s, "Flathub");
        let aur = pending(&s, "Aur");
        let system_waiting = !pending(&s, "Repositories").is_empty();
        let running = s["progress"]["state"] == "Running";
        let store_count = flathub.len() + aur.len();

        let ask = {
            let data = data.clone();
            move |a: Ask| {
                let mut failed = failed;
                failed.set(None);
                data.ask(a);
            }
        };
        let update_button = |label: &'static str, part: String, enabled: bool, small: bool| {
            let ask = ask.clone();
            let b = Button::new(label).enabled(enabled && !running);
            let b = if small { b.small() } else { b };
            b.on_press(move |_| ask(Ask::Install(part.clone())))
                .into_element()
        };

        let body: Vec<Element> = match *page.read() {
            Page::Updates => {
                let mut out = Vec::new();
                let everything: Vec<String> = [
                    (!flathub.is_empty()).then(|| "flathub".to_owned()),
                    (!aur.is_empty() && !system_waiting).then(|| "aur".to_owned()),
                ]
                .into_iter()
                .flatten()
                .collect();
                let queue_e = queue.clone();
                let ask_e = ask.clone();
                out.push(
                    rect()
                        .width(Size::fill())
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .child(rect().width(Size::flex(1.)).child(text::line(
                            &theme,
                            text::Style::Title,
                            colour(t.colour_text),
                            "Updates",
                        )))
                        .child(
                            Button::new("Update Everything")
                                .enabled(!everything.is_empty() && !running)
                                .on_press(move |_| {
                                    let mut parts: VecDeque<String> = everything.clone().into();
                                    if let Some(first) = parts.pop_front() {
                                        *queue_e.borrow_mut() = parts;
                                        ask_e(Ask::Install(first));
                                    }
                                }),
                        )
                        .into_element(),
                );
                out.push(note(
                    &theme,
                    match s["checked_at"].as_u64() {
                        Some(_) if s["checking"] == true => "Checking…".to_owned(),
                        Some(_) => "Flathub, the AUR and AppImages".to_owned(),
                        None => "Not checked yet".to_owned(),
                    },
                ));
                if let Some(why) = failed.read().clone() {
                    out.push(
                        text::line(
                            &theme,
                            text::Style::Callout,
                            colour(t.colour_warn_text),
                            why,
                        )
                        .into_element(),
                    );
                }
                if let Some(run) = store_run(&s).filter(|r| r["state"] == "Running") {
                    let (done, total) = (
                        run["done"].as_u64().unwrap_or(0),
                        run["total"].as_u64().unwrap_or(0),
                    );
                    let ask_s = ask.clone();
                    out.push(
                        rect()
                            .width(Size::fill())
                            .spacing(t.space_xs)
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .content(Content::flex())
                                    .cross_align(Alignment::Center)
                                    .child(rect().width(Size::flex(1.)).child(text::line(
                                        &theme,
                                        text::Style::Headline,
                                        colour(t.colour_text),
                                        if total > 0 {
                                            format!("Updating {} of {total}", done.min(total))
                                        } else {
                                            "Updating".to_owned()
                                        },
                                    )))
                                    .child(
                                        Button::new("Stop")
                                            .enabled(s["can_stop"] == true)
                                            .on_press(move |_| ask_s(Ask::Stop)),
                                    ),
                            )
                            .child(isle_ui::LevelBar {
                                level: if total > 0 {
                                    done as f32 / total as f32
                                } else {
                                    0.
                                },
                                track: colour(t.colour_fill_strong),
                                fill: colour(t.colour_accent),
                            })
                            .child(note(&theme, run["phase"].as_str().unwrap_or_default()))
                            .into_element(),
                    );
                }

                // CachyOS: its apps update with the system, in Software Update.
                let system_apps = s["system"]["apps"].as_array().cloned().unwrap_or_default();
                if !system_apps.is_empty() {
                    let ask_su = ask.clone();
                    out.push(section(
                        &theme,
                        "CachyOS",
                        Some(
                            Button::new("Software Update…")
                                .kind(Kind::Plain)
                                .on_press(move |_| ask_su(Ask::SoftwareUpdate))
                                .into_element(),
                        ),
                    ));
                    out.push(note(
                        &theme,
                        "Apps from CachyOS update with the system, its kernel and drivers, in Settings › Software Update.",
                    ));
                    for a in &system_apps {
                        let found = by_id(a["desktop"].as_str().unwrap_or_default())
                            .or_else(|| by_id(a["package"].as_str().unwrap_or_default()));
                        let name = found.as_ref().map(|f| f.name.clone()).unwrap_or_else(|| {
                            a["package"].as_str().unwrap_or_default().to_owned()
                        });
                        out.push(update_row(
                            &theme,
                            found.as_ref(),
                            &name,
                            Catalogue::Cachyos,
                            format!(
                                "{} → {}",
                                a["from"].as_str().unwrap_or_default(),
                                a["to"].as_str().unwrap_or_default()
                            ),
                            note(&theme, "With the system"),
                        ));
                    }
                }

                for (title, items, catalogue, part) in [
                    ("Flathub", &flathub, Catalogue::Flathub, "flathub"),
                    ("AUR", &aur, Catalogue::Aur, "aur"),
                ] {
                    if items.is_empty() {
                        continue;
                    }
                    let blocked = catalogue == Catalogue::Aur && system_waiting;
                    out.push(section(
                        &theme,
                        title,
                        Some(update_button("Update All", part.to_owned(), !blocked, true)),
                    ));
                    if blocked {
                        out.push(note(&theme, "Packages from the AUR are built against the system: update CachyOS first, in Software Update."));
                    } else if catalogue == Catalogue::Aur {
                        out.push(note(
                            &theme,
                            "Built on this computer from community scripts.",
                        ));
                    }
                    for p in items {
                        let id = p["name"].as_str().unwrap_or_default();
                        let found = by_id(id);
                        let name = found
                            .as_ref()
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| id.to_owned());
                        out.push(update_row(
                            &theme,
                            found.as_ref(),
                            &name,
                            catalogue,
                            format!(
                                "{} → {}",
                                p["from"].as_str().unwrap_or_default(),
                                p["to"].as_str().unwrap_or_default()
                            ),
                            update_button("Update", format!("{part}:{id}"), !blocked, true),
                        ));
                    }
                }

                let images: Vec<App> = installed
                    .iter()
                    .filter(|a| a.catalogue == Catalogue::AppImages)
                    .cloned()
                    .collect();
                if !images.is_empty() {
                    out.push(section(&theme, "AppImages", None));
                    for a in &images {
                        out.push(update_row(
                            &theme,
                            Some(a),
                            &a.name,
                            Catalogue::AppImages,
                            a.summary.clone(),
                            note(&theme, "Updates from its developer"),
                        ));
                    }
                }

                if store_count == 0 && system_apps.is_empty() && !running {
                    let ask_c = ask.clone();
                    out.push(
                        rect()
                            .width(Size::fill())
                            .padding((t.space_xxl, 0.))
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(Glyph::new("check", 40., colour(t.colour_ok)))
                            .child(text::line(
                                &theme,
                                text::Style::Title2,
                                colour(t.colour_text),
                                "Everything is up to date",
                            ))
                            .child(Button::new("Check Now").on_press(move |_| ask_c(Ask::Check)))
                            .into_element(),
                    );
                }
                out
            }
            Page::Installed => {
                let mut out = vec![
                    text::line(
                        &theme,
                        text::Style::Title,
                        colour(t.colour_text),
                        "Installed",
                    )
                    .into_element(),
                    note(
                        &theme,
                        format!(
                            "{} apps · the system's packages are not listed here",
                            installed.len()
                        ),
                    ),
                ];
                for a in &installed {
                    let ask_o = ask.clone();
                    let id = a.id.clone();
                    out.push(update_row(
                        &theme,
                        Some(a),
                        &a.name,
                        a.catalogue,
                        a.summary.clone(),
                        Button::new("Open")
                            .small()
                            .on_press(move |_| ask_o(Ask::Launch(id.clone())))
                            .into_element(),
                    ));
                }
                out
            }
        };

        let current = *page.read();
        let side_row = |p: Page, label: &'static str, glyph: &'static str, count: usize| {
            let selected = current == p;
            let (fill, ink) = if selected {
                (Some(t.colour_accent), t.colour_on_accent)
            } else {
                (None, t.colour_text)
            };
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .corner_radius(t.radius_row)
                .map(fill, |el, f| el.background(colour(f)))
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(label)
                .on_press(move |_| page.set(p))
                .child(Glyph::new(
                    glyph,
                    t.size_glyph,
                    colour(if selected { ink } else { t.colour_accent }),
                ))
                .child(rect().width(Size::flex(1.)).child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(ink),
                    label,
                )))
                .maybe_child((count > 0).then(|| {
                    text::line(
                        &theme,
                        text::Style::Subheadline,
                        colour(ink),
                        count.to_string(),
                    )
                    .into_element()
                }))
                .into_element()
        };
        let sidebar = rect()
            .width(Size::px(t.size_sidebar))
            .height(Size::fill())
            .background(colour(t.colour_sidebar))
            .padding((t.size_toolbar, t.space_s, t.space_s, t.space_s))
            .spacing(t.space_xxs)
            .a11y_role(AccessibilityRole::List)
            .child(side_row(
                Page::Updates,
                "Updates",
                "download-simple",
                store_count,
            ))
            .child(side_row(Page::Installed, "Installed", "app-window", 0));
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS))
            .toolbar();
        let content = rect()
            .width(Size::flex(1.))
            .height(Size::fill())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(
                scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((0., t.space_xxl, t.space_xxl, t.space_xxl))
                            .spacing(t.space_s)
                            .children(body),
                    ),
            );
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .horizontal()
            .content(Content::flex())
            .child(sidebar)
            .child(content)
    }
}
