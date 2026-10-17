//! Spotlight: its sources, favourites, clipboard exclusions, web search and folders.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

use super::Nav;
use crate::{
    nav::Location,
    services::{Follow, use_follow},
    ui::{self, AppPicker, Ask, ListEditor, Rows},
};

/// The sources' names in `spotlight.sources-off` and as their switches read.
const SOURCES: [(&str, &str); 8] = [
    ("apps", "Apps"),
    ("actions", "Actions"),
    ("calculator", "Calculator"),
    ("files", "Files"),
    ("menu-items", "Menu items"),
    ("settings", "Settings"),
    ("windows", "Windows"),
    ("web", "Web search"),
];

const ENGINES: [(&str, &str); 4] = [
    ("DuckDuckGo", "https://duckduckgo.com/?q={query}"),
    ("Google", "https://www.google.com/search?q={query}"),
    ("Bing", "https://www.bing.com/search?q={query}"),
    ("Kagi", "https://kagi.com/search?q={query}"),
];

#[derive(PartialEq)]
pub struct Spotlight {
    pub at: Location,
}

impl Component for Spotlight {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Apps]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let at = self.at.clone();
        let sheet = self.at.sub.clone();

        let off = r.list("spotlight.sources-off");
        let results: Vec<Element> = SOURCES
            .iter()
            .map(|(id, name)| {
                let off = off.clone();
                r.switch_with(
                    &format!("source-{id}"),
                    name,
                    !off.iter().any(|o| o == id),
                    move |on| {
                        let mut next: Vec<String> =
                            off.iter().filter(|o| *o != id).cloned().collect();
                        if !on {
                            next.push((*id).to_owned());
                        }
                        desktop.set("spotlight.sources-off", Value::List(next));
                    },
                )
                .into_element()
            })
            .collect();

        let app_items = |key: &str| -> Vec<(String, Option<Option<std::path::PathBuf>>)> {
            r.list(key)
                .iter()
                .map(|id| match services.app(id) {
                    Some(app) => (app.name, Some(app.icon)),
                    None => (id.clone(), Some(None)),
                })
                .collect()
        };
        let remove = move |key: &'static str, list: Vec<String>| {
            EventHandler::new(move |i: usize| {
                let mut next = list.clone();
                if i < next.len() {
                    next.remove(i);
                }
                desktop.set(key, Value::List(next));
            })
        };
        let opener = |sub: &'static str| {
            let at = at.clone();
            EventHandler::new(move |_| super::open(nav, &at, sub))
        };

        let web = r.str("spotlight.web-search").unwrap_or_default();
        let mut engines: Vec<String> = ENGINES.iter().map(|(n, _)| (*n).to_owned()).collect();
        let known = ENGINES.iter().position(|(_, url)| *url == web);
        engines.push("Custom…".into());
        let web_row = r
            .row("web-search", "Web search", Some("spotlight.web-search"))
            .control(form::PopUp::new(
                engines.clone(),
                known.unwrap_or(ENGINES.len()),
                "Web search",
                {
                    let at = at.clone();
                    move |i: usize| match ENGINES.get(i) {
                        Some((_, url)) => {
                            desktop.set("spotlight.web-search", Value::Str((*url).into()))
                        }
                        None => super::open(nav, &at, "Custom Search…"),
                    }
                },
            ));

        let add_to = |key: &'static str, list: Vec<String>| {
            let at = at.clone();
            EventHandler::new(move |id: String| {
                let mut next = list.clone();
                if !next.contains(&id) {
                    next.push(id);
                }
                desktop.set(key, Value::List(next));
                super::close(nav, &at);
            })
        };
        let cancel = {
            let at = self.at.clone();
            EventHandler::new(move |_| super::close(nav, &at))
        };

        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(Some("Results".into()), results))
            .child(ListEditor {
                on_open: None,
                on_move: Some(EventHandler::new({
                    let list = r.list("spotlight.favourites");
                    move |(from, to): (usize, usize)| {
                        desktop.set(
                            "spotlight.favourites",
                            Value::List(ui::moved(&list, from, to)),
                        )
                    }
                })),
                title: Some("Favourites".into()),
                items: app_items("spotlight.favourites"),
                empty: "No favourites".into(),
                on_add: opener("Add Favourite…"),
                on_remove: remove("spotlight.favourites", r.list("spotlight.favourites")),
            })
            .child(ListEditor {
                on_open: None,
                on_move: None,
                title: Some("Don't keep copies from".into()),
                items: app_items("spotlight.clipboard-exclude"),
                empty: "No apps".into(),
                on_add: opener("Add Clipboard Exclusion…"),
                on_remove: remove(
                    "spotlight.clipboard-exclude",
                    r.list("spotlight.clipboard-exclude"),
                ),
            })
            .child(form::group(
                Some("Search".into()),
                vec![web_row.into_element()],
            ))
            .child(ui::foot(vec![(
                "Search Privacy…".into(),
                opener("Search Privacy…"),
            )]))
            .maybe(sheet.as_deref() == Some("Add Favourite…"), |el| {
                el.child(AppPicker {
                    title: "Add a favourite".into(),
                    on_pick: add_to("spotlight.favourites", r.list("spotlight.favourites")),
                    on_cancel: cancel.clone(),
                })
            })
            .maybe(
                sheet.as_deref() == Some("Add Clipboard Exclusion…"),
                |el| {
                    el.child(AppPicker {
                        title: "Don't keep copies from".into(),
                        on_pick: add_to(
                            "spotlight.clipboard-exclude",
                            r.list("spotlight.clipboard-exclude"),
                        ),
                        on_cancel: cancel.clone(),
                    })
                },
            )
            .maybe(sheet.as_deref() == Some("Custom Search…"), |el| {
                let at = self.at.clone();
                el.child(Ask {
                    title: "Search address".into(),
                    placeholder: "https://example.com/?q={query}".into(),
                    initial: if known.is_none() {
                        web.clone()
                    } else {
                        String::new()
                    },
                    on_done: EventHandler::new(move |url: String| {
                        if url.contains("{query}") {
                            desktop.set("spotlight.web-search", Value::Str(url));
                            super::close(nav, &at);
                        }
                    }),
                    on_cancel: cancel.clone(),
                })
            })
            .maybe(sheet.as_deref() == Some("Search Privacy…"), |el| {
                el.child(SearchPrivacy {
                    at: self.at.clone(),
                })
            })
    }
}

#[derive(PartialEq)]
struct SearchPrivacy {
    at: Location,
}

impl Component for SearchPrivacy {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let mut asking = use_state(|| None::<&'static str>);
        let folders = |key: &'static str| {
            let list = r.list(key);
            ListEditor {
                on_open: None,
                on_move: None,
                title: Some(
                    if key == "spotlight.files-include" {
                        "Also search these folders"
                    } else {
                        "Don't search these folders"
                    }
                    .into(),
                ),
                items: list.iter().map(|f| (f.clone(), None)).collect(),
                empty: "No folders".into(),
                on_add: EventHandler::new(move |_| asking.set(Some(key))),
                on_remove: EventHandler::new(move |i: usize| {
                    let mut next = list.clone();
                    if i < next.len() {
                        next.remove(i);
                    }
                    desktop.set(key, Value::List(next));
                }),
            }
            .into_element()
        };
        let at = self.at.clone();
        let asked = *asking.read();
        rect()
            .child(ui::sheet(
                520.,
                vec![
                    folders("spotlight.files-include"),
                    folders("spotlight.files-exclude"),
                ],
                vec![(
                    "Done".into(),
                    EventHandler::new(move |_| super::close(nav, &at)),
                )],
            ))
            .map(asked, |el, key| {
                let list = r.list(key);
                el.child(Ask {
                    title: "Folder".into(),
                    placeholder: "~/Projects".into(),
                    initial: String::new(),
                    on_done: EventHandler::new(move |path: String| {
                        let path = path.trim().to_owned();
                        if !path.is_empty() && !list.contains(&path) {
                            let mut next = list.clone();
                            next.push(path);
                            desktop.set(key, Value::List(next));
                        }
                        asking.set(None);
                    }),
                    on_cancel: EventHandler::new(move |_| asking.set(None)),
                })
            })
    }
}
