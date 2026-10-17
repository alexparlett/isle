//! Apps: every app and library game, each with its own page.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{Glyph, colour, form};

use super::Nav;
use crate::{
    nav::Location,
    services::{App, Follow, Services, use_follow},
    ui::{self, Rows},
};

/// `list` with `id` in it or not.
pub fn toggled(list: &[String], id: &str, member: bool) -> Vec<String> {
    let mut next: Vec<String> = list.iter().filter(|e| *e != id).cloned().collect();
    if member {
        next.push(id.to_owned());
    }
    next
}

/// What the row's value says when something is set for the app.
fn value(r: &Rows, app: &App) -> Option<&'static str> {
    let has = |key: &str| r.list(key).iter().any(|e| *e == app.id);
    if has("windows.games") {
        Some("Game")
    } else if has("windows.not-games") {
        Some("Not a game")
    } else if has("windows.keep-running") {
        Some("Keeps running")
    } else if has("keyboard.untranslated-apps") {
        Some("Sends keys as typed")
    } else {
        None
    }
}

#[derive(PartialEq)]
pub struct Apps {
    pub at: Location,
}

impl Component for Apps {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Apps, Follow::NotifyApps]);
        if let Some(id) = self.at.sub.as_deref().and_then(|s| s.strip_prefix("app/")) {
            return AppPage {
                at: self.at.clone(),
                id: id.to_owned(),
            }
            .into_element();
        }
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let theme = isle_ui::theme();
        let t = &theme.tokens;
        let query = use_state(String::new);
        let q = query.read().to_lowercase();
        let apps: Vec<App> = services
            .apps
            .read()
            .state
            .iter()
            .filter(|a| {
                (a.listed || a.game) && (q.is_empty() || a.name.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        let rows: Vec<Element> = apps
            .iter()
            .map(|app| {
                let at = self.at.clone();
                let sub = format!("app/{}", app.id);
                r.row(&app.id, app.name.clone(), None)
                    .leading(ui::app_icon(app.icon.clone(), 24.))
                    .opens(value(&r, app).map(str::to_owned), move |_| {
                        super::open(nav, &at, &sub)
                    })
                    .into_element()
            })
            .collect();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(
                isle_ui::TextField::new(query, "Search apps")
                    .placeholder("Search apps")
                    .leading(Glyph::new(
                        "magnifying-glass",
                        t.size_glyph,
                        colour(t.colour_text_2),
                    )),
            )
            .child(if rows.is_empty() {
                ui::missing("No apps")
            } else {
                form::group(None, rows).into_element()
            })
            .into_element()
    }
}

#[derive(PartialEq)]
struct AppPage {
    at: Location,
    id: String,
}

impl Component for AppPage {
    fn render(&self) -> impl IntoElement {
        let services = Services::get();
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let id = self.id.clone();
        let app = services.app(&id).unwrap_or(App {
            id: id.clone(),
            name: id.clone(),
            icon: None,
            listed: false,
            game: false,
        });
        let name = app.name.clone();
        let list = |key: &str| r.list(key);
        let keep = list("windows.keep-running");
        let untranslated = list("keyboard.untranslated-apps");
        let (games, not_games) = (list("windows.games"), list("windows.not-games"));
        let translate = r.bool("keyboard.translate");
        let game_choice = if games.contains(&id) {
            1
        } else if not_games.contains(&id) {
            2
        } else {
            0
        };
        let posted = services
            .notify_apps
            .read()
            .state
            .iter()
            .any(|(k, _)| *k == id);

        let mut rows = Vec::new();
        if !app.game {
            let (id, keep) = (id.clone(), keep.clone());
            rows.push(
                r.switch_with(
                    "keep-running",
                    "Keep running after last window",
                    keep.contains(&id),
                    move |on| {
                        desktop.set("windows.keep-running", Value::List(toggled(&keep, &id, on)))
                    },
                )
                .into_element(),
            );
        }
        {
            let (id, untranslated) = (id.clone(), untranslated.clone());
            rows.push(
                r.row(
                    "mac-shortcuts",
                    "Use Mac shortcuts",
                    Some("keyboard.untranslated-apps"),
                )
                .description(format!("Cmd shortcuts become {name}'s own."))
                .enabled(translate)
                .control(
                    isle_ui::Switch::new(
                        !untranslated.contains(&id),
                        "Use Mac shortcuts",
                        move |on: bool| {
                            desktop.set(
                                "keyboard.untranslated-apps",
                                Value::List(toggled(&untranslated, &id, !on)),
                            )
                        },
                    )
                    .enabled(translate),
                )
                .into_element(),
            );
        }
        {
            let id = id.clone();
            rows.push(
                r.row("game", "Treat as a game", Some("windows.games"))
                    .control(form::Segmented::new(
                        vec!["Automatic".into(), "Always".into(), "Never".into()],
                        game_choice,
                        "Treat as a game",
                        move |i: usize| {
                            desktop.set("windows.games", Value::List(toggled(&games, &id, i == 1)));
                            desktop.set(
                                "windows.not-games",
                                Value::List(toggled(&not_games, &id, i == 2)),
                            );
                        },
                    ))
                    .into_element(),
            );
        }
        if posted {
            let summary = super::notifications::summary(&r, &id);
            let id = id.clone();
            rows.push(
                r.row("notifications", "Notifications", None)
                    .opens(Some(summary), move |_| {
                        nav.go(Location {
                            pane: "notifications".into(),
                            sub: Some(format!("app/{id}")),
                            row: None,
                        })
                    })
                    .into_element(),
            );
        }
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::app_header(
                app.icon.clone(),
                &name,
                &format!("What the desktop does with {name}'s windows and keys."),
            ))
            .child(form::group(None, rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_joins_and_leaves_a_list_once() {
        let list = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(toggled(&list, "c", true), ["a", "b", "c"]);
        assert_eq!(toggled(&list, "a", false), ["b"]);
        assert_eq!(toggled(&list, "a", true), ["b", "a"]);
    }
}
