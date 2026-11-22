//! Store's window (`p14e-store.md`, board `p14e/store-1.html`): the sidebar, a toolbar with
//! Back and search, then the page.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use isle_app::{Chord, Command, Desktop, Menu, ToolbarExt};
use isle_catalog::catalogue::{Kind as Offered, Offer};
use isle_tokens::{Colour, Theme};
use isle_ui::{AppIcon, Button, Glyph, SearchField, button::Kind, colour, scroll_area, text, use_theme};
use serde_json::Value;

use crate::{
    data::{self, Ask, Event},
    installed::{App, Catalogue},
    pages::{self, Ctx, Sheet},
    plan::Plan,
};

/// The compositor's lights over the toolbar's leading end.
const LIGHTS: f32 = 68.;

#[derive(Clone, PartialEq)]
pub enum Page {
    Discover,
    Browse,
    Category(&'static str),
    Installed,
    Updates,
    Catalogue(Catalogue),
    App(Box<Offer>),
}

#[derive(Clone, PartialEq)]
pub struct StoreWindow;

pub fn hue(theme: &Theme, name: &str) -> Colour {
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
pub fn badge(theme: &Theme, catalogue: Catalogue) -> Element {
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

pub fn section(theme: &Theme, title: &str, trailing: Option<Element>) -> Element {
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

pub fn note(theme: &Theme, line: impl Into<String>) -> Element {
    let t = &theme.tokens;
    text::line(theme, text::Style::Callout, colour(t.colour_text_2), line).into_element()
}

/// The part of a run that is Store's: Flathub's and the AUR's, not the system's.
fn store_run(status: &Value) -> Option<&Value> {
    let run = &status["progress"];
    let part = run["part"].as_str().unwrap_or_default();
    (["flathub", "aur", "install", "remove", "appimage"].iter().any(|p| part.starts_with(p)))
        .then_some(run)
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
        let mut page = use_state(|| Page::Discover);
        let mut back = use_state(Vec::<Page>::new);
        let mut status = use_state(|| Value::Null);
        let mut apps = use_state(Vec::<App>::new);
        let mut failed = use_state(|| None::<String>);
        let mut offers = use_state(|| Rc::new(Vec::<Offer>::new()));
        let mut packages = use_state(|| Rc::new(HashSet::<String>::new()));
        let mut icons = use_state(|| Rc::new(HashMap::<PathBuf, Option<PathBuf>>::new()));
        let mut shots = use_state(|| Rc::new(HashMap::<String, Option<PathBuf>>::new()));
        let mut aur_found = use_state(|| Rc::new(None::<(String, Result<Vec<Offer>, String>)>));
        let mut plans = use_state(|| Rc::new(HashMap::<String, Plan>::new()));
        let mut sheet = use_state(|| None::<Sheet>);
        let mut images = use_state(HashMap::<String, data::Image>::new);
        let limit = use_state(|| pages::PAGE);
        let narrow = use_state(|| None::<Offered>);
        let narrowing = use_state(|| false);
        let mut query = use_state(String::new);
        let opened_from = use_state(|| None::<String>);
        let reviewing = use_state(|| None::<String>);
        let asked = use_hook(|| Rc::new(RefCell::new(HashSet::<String>::new())));
        let requested = use_hook(|| Rc::new(RefCell::new(HashSet::<PathBuf>::new())));
        let shots_requested = use_hook(|| Rc::new(RefCell::new(HashSet::<String>::new())));
        let pending_get = use_hook(|| Rc::new(RefCell::new(None::<String>)));
        // Update Everything's parts still to run, one after another.
        let queue = use_hook(|| Rc::new(RefCell::new(VecDeque::<String>::new())));
        let (data, incoming) = use_hook(move || {
            let (data, rx) = data::start(&worker, bus);
            (data, Rc::new(RefCell::new(Some(rx))))
        });
        use_hook({
            let data = data.clone();
            let queue = queue.clone();
            let asked = asked.clone();
            let pending_get = pending_get.clone();
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
                                    plans.set(Rc::new(HashMap::new()));
                                    asked.borrow_mut().clear();
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
                            Event::Packages(set) => packages.set(Rc::new(set)),
                            Event::Images(found) => images.set(found),
                            Event::Offers(list) => offers.set(Rc::new(list)),
                            Event::Icons(made) => {
                                let mut all = (**icons.read()).clone();
                                all.extend(made);
                                icons.set(Rc::new(all));
                            }
                            Event::Shot(url, path) => {
                                let mut all = (**shots.read()).clone();
                                all.insert(url, path);
                                shots.set(Rc::new(all));
                            }
                            Event::Aur(term, got) => aur_found.set(Rc::new(Some((term, got)))),
                            Event::Plan(key, plan) => {
                                let waiting = pending_get.borrow().as_deref() == Some(key.as_str());
                                if waiting {
                                    *pending_get.borrow_mut() = None;
                                    let system_waiting = status.read()["pending"]
                                        .as_array()
                                        .is_some_and(|l| l.iter().any(|p| p["source"] == "Repositories"));
                                    let shown = sheet.read().clone();
                                    if let Some(Sheet::Get(o)) = shown
                                        && pages::get_key(&o) == key
                                        && pages::straight(&o, &plan, system_waiting)
                                    {
                                        sheet.set(None);
                                        data.ask(Ask::Install(pages::install_part(&o)));
                                    }
                                }
                                let mut all = (**plans.read()).clone();
                                all.insert(key, plan);
                                plans.set(Rc::new(all));
                            }
                            Event::Failed(why) => failed.set(Some(why)),
                        }
                    }
                });
            }
        });

        let can_back = !back.read().is_empty();
        isle_app::use_commands(vec![
            Command::new("check", "Check for Updates", Menu::File, 1).chord(Chord::cmd('r')),
            Command::new("back", "Back", Menu::Go, 0)
                .chord(Chord::cmd('['))
                .enabled(can_back),
            Command::new("discover", "Discover", Menu::Go, 1).chord(Chord::cmd('1')),
            Command::new("browse", "Browse", Menu::Go, 1).chord(Chord::cmd('2')),
            Command::new("installed", "Installed", Menu::Go, 1).chord(Chord::cmd('3')),
            Command::new("updates", "Updates", Menu::Go, 1).chord(Chord::cmd('4')),
        ]);
        let desktop = Desktop::get();
        use_side_effect({
            let data = data.clone();
            move || {
                if desktop.commands.read().is_empty() {
                    return;
                }
                let mut queue = desktop.commands;
                for command in std::mem::take(&mut *queue.write()) {
                    let to = match command.as_str() {
                        "check" => {
                            data.ask(Ask::Check);
                            None
                        }
                        "back" => {
                            let last = back.write().pop();
                            if let Some(last) = last {
                                page.set(last);
                            }
                            None
                        }
                        "discover" => Some(Page::Discover),
                        "browse" => Some(Page::Browse),
                        "installed" => Some(Page::Installed),
                        "updates" => Some(Page::Updates),
                        _ => None,
                    };
                    if let Some(to) = to {
                        query.set(String::new());
                        back.set(Vec::new());
                        page.set(to);
                    }
                }
            }
        });

        let typed = query.read().trim().to_string();
        use_side_effect_with_deps(&typed, {
            let data = data.clone();
            move |typed: &String| {
                if typed.len() >= 2 {
                    data.ask(Ask::SearchAur(typed.clone()));
                }
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
        let image_states = images.read().clone();
        let newer_images: Vec<(String, String)> = image_states
            .iter()
            .filter(|(_, i)| i.newer == Some(true))
            .map(|(id, i)| (id.clone(), format!("appimage:{}", i.path.display())))
            .collect();
        let store_count = flathub.len() + aur.len() + newer_images.len();

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

        let ctx = Ctx {
            theme: theme.clone(),
            offers: offers.read().clone(),
            icons: icons.read().clone(),
            wanted: Rc::new(RefCell::new(Vec::new())),
            shots_wanted: Rc::new(RefCell::new(Vec::new())),
            apps: Rc::new(installed.clone()),
            packages: packages.read().clone(),
            status: Rc::new(s.clone()),
            plans: plans.read().clone(),
            shots: shots.read().clone(),
            aur: aur_found.read().clone(),
            data: data.clone(),
            page,
            back,
            sheet,
            limit,
            failed,
            pending_get: pending_get.clone(),
            narrow,
            narrowing,
            opened_from,
            reviewing,
            query: typed.clone(),
            system_waiting,
        };
        let current = page.read().clone();
        let searching = !typed.is_empty()
            && !(matches!(current, Page::App(_))
                && opened_from.read().as_deref() == Some(typed.as_str()));
        let body: Vec<Element> = match current.clone() {
            _ if searching => ctx.search(&typed),
            Page::Discover => ctx.discover(),
            Page::Browse => ctx.browse(),
            Page::Category(key) => ctx.category(key),
            Page::Catalogue(c) => ctx.catalogue(c),
            Page::App(o) => ctx.app(&o, &asked),
            Page::Installed => ctx.installed(),
            Page::Updates => {
                let mut out = Vec::new();
                let everything: Vec<String> = [
                    (!flathub.is_empty()).then(|| "flathub".to_owned()),
                    (!aur.is_empty() && !system_waiting).then(|| "aur".to_owned()),
                ]
                .into_iter()
                .flatten()
                .chain(newer_images.iter().map(|(_, part)| part.clone()))
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
                        let trailing = match image_states.get(&a.id) {
                            Some(i) if i.newer == Some(true) => update_button(
                                "Update",
                                format!("appimage:{}", i.path.display()),
                                true,
                                true,
                            ),
                            Some(i) if i.newer == Some(false) => note(&theme, "Up to date"),
                            Some(i) if i.info.is_some() => {
                                note(&theme, "Put appimageupdatetool's AppImage in Applications to update it here")
                            }
                            _ => note(&theme, "Updates from its developer"),
                        };
                        out.push(update_row(
                            &theme,
                            Some(a),
                            &a.name,
                            Catalogue::AppImages,
                            a.summary.clone(),
                            trailing,
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
        };

        // The icons and screenshots this render drew without, asked for once each.
        let icons_wanted: Vec<PathBuf> = ctx
            .wanted
            .borrow_mut()
            .drain(..)
            .filter(|p| requested.borrow_mut().insert(p.clone()))
            .collect();
        for chunk in icons_wanted.chunks(8) {
            data.ask(Ask::Icons(chunk.to_vec()));
        }
        let shots_wanted: Vec<String> = ctx
            .shots_wanted
            .borrow_mut()
            .drain(..)
            .filter(|u| shots_requested.borrow_mut().insert(u.clone()))
            .collect();
        if !shots_wanted.is_empty() {
            data.ask(Ask::Shots(shots_wanted));
        }

        let lit = match &current {
            Page::Category(_) => Some(Page::Browse),
            Page::App(_) => None,
            p => Some(p.clone()),
        };
        let side_row = |p: Page, label: &'static str, glyph: &'static str, count: usize| {
            let selected = !searching && lit.as_ref() == Some(&p);
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
                .on_press(move |_| {
                    query.set(String::new());
                    back.set(Vec::new());
                    let mut limit = limit;
                    limit.set(pages::PAGE);
                    page.set(p.clone());
                })
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
        let heading = |words: &'static str| {
            rect()
                .padding((t.space_l, t.space_s, t.space_xxs, t.space_s))
                .child(text::line(
                    &theme,
                    text::Style::Footnote,
                    colour(t.colour_text_2),
                    words,
                ))
                .into_element()
        };
        let sidebar = rect()
            .width(Size::px(t.size_sidebar))
            .height(Size::fill())
            .background(colour(t.colour_sidebar))
            .padding((t.size_toolbar, t.space_s, t.space_s, t.space_s))
            .spacing(t.space_xxs)
            .a11y_role(AccessibilityRole::List)
            .child(side_row(Page::Discover, "Discover", "diamond", 0))
            .child(side_row(Page::Browse, "Browse", "squares-four", 0))
            .child(side_row(Page::Installed, "Installed", "app-window", 0))
            .child(side_row(
                Page::Updates,
                "Updates",
                "download-simple",
                store_count,
            ))
            .child(heading("Catalogues"))
            .child(side_row(
                Page::Catalogue(Catalogue::Cachyos),
                "CachyOS",
                "package",
                0,
            ))
            .child(side_row(
                Page::Catalogue(Catalogue::Flathub),
                "Flathub",
                "stack",
                0,
            ))
            .child(side_row(Page::Catalogue(Catalogue::Aur), "AUR", "users", 0))
            .child(side_row(
                Page::Catalogue(Catalogue::AppImages),
                "AppImages",
                "folder",
                0,
            ));
        let can_go_back = !back.read().is_empty();
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .child(
                Button::icon("caret-left", "Back")
                    .enabled(can_go_back)
                    .on_press(move |_| {
                        let last = back.write().pop();
                        if let Some(last) = last {
                            page.set(last);
                        }
                    }),
            )
            .child(rect().width(Size::flex(1.)))
            .child(SearchField {
                value: query.into_writable(),
                placeholder: "Search".into(),
                width: 240.,
                focus: false,
            })
            .toolbar();
        let shown_sheet = sheet.read().clone().map(|sh| ctx.sheet(&sh));
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
            .maybe_child(shown_sheet)
    }
}
