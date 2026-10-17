//! Focus: the modes, each mode's page and Add Focus….

use freya::prelude::*;
use isle_focus::{DO_NOT_DISTURB, GAME_MODE};
use isle_settings::Value;
use isle_ui::{Glyph, colour, form, text, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Follow, Services, use_follow},
    ui::{self, AppPicker, ListEditor, Rows},
};

/// The symbols a new mode can take.
const SYMBOLS: [&str; 12] = [
    "moon",
    "briefcase",
    "book-open",
    "code",
    "barbell",
    "car",
    "bed",
    "headphones",
    "music-note",
    "leaf",
    "users",
    "game-controller",
];

/// The user's modes, by name, from the settings' entries.
pub fn user_modes(r: &Rows) -> Vec<String> {
    let mut names: Vec<String> = r
        .desktop
        .values
        .read()
        .keys()
        .filter_map(|k| {
            k.strip_prefix("focus-mode[")?
                .split_once("].")
                .map(|(n, _)| n.to_owned())
        })
        .filter(|n| n != DO_NOT_DISTURB && n != GAME_MODE)
        .collect();
    names.dedup();
    names
}

/// A schedule as its row reads: "Weekdays 09:00–17:00".
pub fn describe_schedule(text: &str) -> String {
    let mut words = text.split_whitespace();
    let span = words.next().unwrap_or_default().replace('-', "–");
    let days = match words.next().unwrap_or("every-day") {
        "every-day" => "Every day".to_owned(),
        "weekdays" => "Weekdays".to_owned(),
        "weekends" => "Weekends".to_owned(),
        list => list
            .split(',')
            .map(|d| {
                let mut d = d.to_owned();
                d[..1].make_ascii_uppercase();
                d
            })
            .collect::<Vec<_>>()
            .join(", "),
    };
    format!("{days} {span}")
}

fn field(name: &str, f: &str) -> String {
    format!("focus-mode[{name}].{f}")
}

fn symbol(r: &Rows, name: &str) -> String {
    r.str(&field(name, "symbol")).unwrap_or_else(|| {
        match name {
            DO_NOT_DISTURB => "moon",
            GAME_MODE => "game-controller",
            _ => "circle",
        }
        .to_owned()
    })
}

/// A mode's symbol in a circle, the accent's while it is on.
fn badge(glyph: &str, on: bool, size: f32) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .corner_radius(size / 2.)
        .background(colour(if on {
            t.colour_accent
        } else {
            t.colour_fill_strong
        }))
        .center()
        .child(
            Glyph::new(
                glyph,
                size * 0.55,
                colour(if on {
                    t.colour_on_accent
                } else {
                    t.colour_text
                }),
            )
            .fill(true),
        )
        .into_element()
}

#[derive(PartialEq)]
pub struct Focus {
    pub at: Location,
}

impl Component for Focus {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Focus, Follow::Apps]);
        match self.at.sub.as_deref() {
            Some(sub) if sub.starts_with("mode/") => {
                return ModePage {
                    at: self.at.clone(),
                    name: sub["mode/".len()..].to_owned(),
                }
                .into_element();
            }
            _ => {}
        }
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let active = services.focus.read().state.clone();
        let mut names = vec![DO_NOT_DISTURB.to_owned(), GAME_MODE.to_owned()];
        names.extend(user_modes(&r));
        let rows: Vec<Element> = names
            .iter()
            .map(|name| {
                let on = active.as_deref() == Some(name.as_str());
                let schedules = r.list(&field(name, "schedules"));
                let value = if on {
                    "On".to_owned()
                } else if name == GAME_MODE && r.bool("focus.game-mode-auto") {
                    "When a game is in front".to_owned()
                } else if let Some(s) = schedules.first() {
                    describe_schedule(s)
                } else {
                    "Off".to_owned()
                };
                let at = self.at.clone();
                let sub = format!("mode/{name}");
                r.row(name, name.clone(), None)
                    .leading(badge(&symbol(&r, name), on, 28.))
                    .opens(Some(value), move |_| super::open(nav, &at, &sub))
                    .into_element()
            })
            .collect();
        let at = self.at.clone();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(None, rows))
            .child(ui::foot(vec![(
                "Add Focus…".into(),
                EventHandler::new({
                    let at = at.clone();
                    move |_| super::open(nav, &at, "Add Focus…")
                }),
            )]))
            .maybe(self.at.sub.as_deref() == Some("Add Focus…"), |el| {
                el.child(AddFocus { at: at.clone() })
            })
            .into_element()
    }
}

#[derive(PartialEq)]
struct ModePage {
    at: Location,
    name: String,
}

impl Component for ModePage {
    fn render(&self) -> impl IntoElement {
        let services = Services::get();
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let name = self.name.clone();
        let mut adding = use_state(|| None::<&'static str>);
        let app_label = |id: &String| match services.app(id) {
            Some(app) => (app.name, Some(app.icon)),
            None => (id.clone(), Some(None)),
        };
        let allow_key = field(&name, "allow-apps");
        let allowed = r.list(&allow_key);
        let list_remove = |key: String, list: Vec<String>| {
            EventHandler::new(move |i: usize| {
                let mut next = list.clone();
                if i < next.len() {
                    next.remove(i);
                }
                desktop.set(key.clone(), Value::List(next));
            })
        };

        let automatic: Element = if name == GAME_MODE {
            let off = r.list("focus.games-off");
            let items = off
                .iter()
                .map(|id| match services.app(id) {
                    Some(app) => (app.name, Some(app.icon)),
                    None => (format!("{id} — Not installed"), Some(None)),
                })
                .collect();
            rect()
                .width(Size::fill())
                .spacing(20.)
                .child(form::group(
                    Some("Turn on automatically".into()),
                    vec![
                        r.switch(
                            "game-mode-auto",
                            "When a game is in front",
                            "focus.game-mode-auto",
                        )
                        .into_element(),
                    ],
                ))
                .child(ListEditor {
                    title: Some("Off for these games".into()),
                    items,
                    empty: "No games".into(),
                    on_add: EventHandler::new(move |_| adding.set(Some("games-off"))),
                    on_remove: list_remove("focus.games-off".into(), off),
                })
                .into_element()
        } else {
            let schedules = r.list(&field(&name, "schedules"));
            let whiles = r.list(&field(&name, "while-apps"));
            let mut items: Vec<(String, Option<Option<std::path::PathBuf>>)> = schedules
                .iter()
                .map(|s| (describe_schedule(s), None))
                .collect();
            items.extend(
                whiles
                    .iter()
                    .map(|id| (format!("While {} is in front", app_label(id).0), None)),
            );
            let (skey, wkey) = (field(&name, "schedules"), field(&name, "while-apps"));
            let count = schedules.len();
            ListEditor {
                title: Some("Turn on automatically".into()),
                items,
                empty: "Never".into(),
                on_add: EventHandler::new(move |_| adding.set(Some("automatic"))),
                on_remove: EventHandler::new(move |i: usize| {
                    if i < count {
                        let mut next = schedules.clone();
                        next.remove(i);
                        desktop.set(skey.clone(), Value::List(next));
                    } else {
                        let mut next = whiles.clone();
                        next.remove(i - count);
                        desktop.set(wkey.clone(), Value::List(next));
                    }
                }),
            }
            .into_element()
        };

        let builtin = name == DO_NOT_DISTURB || name == GAME_MODE;
        let adding_now = *adding.read();
        let picked = move |key: String| {
            EventHandler::new(move |id: String| {
                let mut next = r_list(desktop, &key);
                if !next.contains(&id) {
                    next.push(id);
                }
                desktop.set(key.clone(), Value::List(next));
                adding.set(None);
            })
        };
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::sub_header(
                &symbol(&r, &name),
                &name,
                &format!("What gets through while {name} is on."),
            ))
            .child(ListEditor {
                title: Some("Allowed notifications".into()),
                items: allowed.iter().map(app_label).collect(),
                empty: "No apps".into(),
                on_add: EventHandler::new(move |_| adding.set(Some("allow"))),
                on_remove: list_remove(allow_key.clone(), allowed.clone()),
            })
            .child(form::group(
                None,
                vec![
                    r.switch_key(
                        "allow-critical",
                        "Allow time-sensitive notifications",
                        field(&name, "allow-critical"),
                        true,
                        true,
                    )
                    .into_element(),
                ],
            ))
            .child(automatic)
            .maybe(!builtin, |el| {
                el.child(ui::foot(vec![(
                    "Delete Focus".into(),
                    EventHandler::new(move |_| adding.set(Some("delete"))),
                )]))
            })
            .map(adding_now, |el, what| match what {
                "allow" => el.child(AppPicker {
                    title: "Allow notifications from".into(),
                    on_pick: picked(field(&name, "allow-apps")),
                    on_cancel: EventHandler::new(move |_| adding.set(None)),
                }),
                "games-off" => el.child(AppPicker {
                    title: "Game Mode stays off for".into(),
                    on_pick: picked("focus.games-off".into()),
                    on_cancel: EventHandler::new(move |_| adding.set(None)),
                }),
                "automatic" => el.child(AddAutomatic {
                    name: name.clone(),
                    on_done: EventHandler::new(move |_| adding.set(None)),
                }),
                _ => el.child(ui::sheet(
                    360.,
                    vec![ui::alert_text(
                        &format!("Delete {name}?"),
                        "Its rules go with it. Notifications it held come through again.",
                    )],
                    vec![
                        (
                            "Cancel".into(),
                            EventHandler::new(move |_| adding.set(None)),
                        ),
                        (
                            "Delete".into(),
                            EventHandler::new({
                                let name = name.clone();
                                move |_| {
                                    desktop.remove(format!("focus-mode[{name}]"));
                                    adding.set(None);
                                    nav.back();
                                }
                            }),
                        ),
                    ],
                )),
            })
    }
}

/// A list's value read outside a render, from the values the desktop last gave.
fn r_list(desktop: isle_app::Desktop, key: &str) -> Vec<String> {
    match desktop.values.peek().get(key) {
        Some(Value::List(l)) => l.clone(),
        _ => Vec::new(),
    }
}

/// The sheet behind Turn on automatically's +: a time, or an app in front.
#[derive(PartialEq)]
struct AddAutomatic {
    name: String,
    on_done: EventHandler<()>,
}

impl Component for AddAutomatic {
    fn render(&self) -> impl IntoElement {
        let desktop = isle_app::Desktop::get();
        let mut kind = use_state(|| 0usize);
        let from = use_state(|| "09:00".to_owned());
        let to = use_state(|| "17:00".to_owned());
        let mut days = use_state(|| 1usize);
        let name = self.name.clone();
        let done = self.on_done.clone();
        if *kind.read() == 1 {
            let done = done.clone();
            let key = field(&name, "while-apps");
            return AppPicker {
                title: "Turn on while this app is in front".into(),
                on_pick: EventHandler::new(move |id: String| {
                    let mut next = r_list(desktop, &key);
                    if !next.contains(&id) {
                        next.push(id);
                    }
                    desktop.set(key.clone(), Value::List(next));
                    done.call(());
                }),
                on_cancel: self.on_done.clone(),
            }
            .into_element();
        }
        const DAYS: [(&str, &str); 3] = [
            ("every-day", "Every day"),
            ("weekdays", "Weekdays"),
            ("weekends", "Weekends"),
        ];
        let key = field(&name, "schedules");
        ui::sheet(
            400.,
            vec![
                form::Segmented::new(
                    vec!["Time".into(), "App".into()],
                    0,
                    "Turn on",
                    move |i: usize| kind.set(i),
                )
                .into_element(),
                form::group(
                    None,
                    vec![
                        form::Row::new("From")
                            .control(isle_ui::TextField::new(from, "From").width(Size::px(80.)))
                            .into_element(),
                        form::Row::new("To")
                            .control(isle_ui::TextField::new(to, "To").width(Size::px(80.)))
                            .into_element(),
                        form::Row::new("Days")
                            .control(form::PopUp::new(
                                DAYS.iter().map(|(_, l)| (*l).to_owned()).collect(),
                                *days.read(),
                                "Days",
                                move |i: usize| days.set(i),
                            ))
                            .into_element(),
                    ],
                )
                .into_element(),
            ],
            vec![
                ("Cancel".into(), self.on_done.clone()),
                (
                    "Add".into(),
                    EventHandler::new(move |_| {
                        let schedule = format!(
                            "{}-{} {}",
                            from.peek().trim(),
                            to.peek().trim(),
                            DAYS[*days.peek()].0
                        );
                        if isle_focus::Schedule::parse(&schedule).is_ok() {
                            let mut next = r_list(desktop, &key);
                            next.push(schedule);
                            desktop.set(key.clone(), Value::List(next));
                            done.call(());
                        }
                    }),
                ),
            ],
        )
    }
}

#[derive(PartialEq)]
struct AddFocus {
    at: Location,
}

impl Component for AddFocus {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = isle_app::Desktop::get();
        let nav = Nav::get();
        let name = use_state(String::new);
        let mut chosen = use_state(|| 0usize);
        let at = self.at.clone();
        let cancel = {
            let at = at.clone();
            EventHandler::new(move |_| super::close(nav, &at))
        };
        let grid = rect()
            .width(Size::fill())
            .horizontal()
            .spacing(t.space_s)
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt("Symbol")
            .children(SYMBOLS.iter().enumerate().map(|(i, glyph)| {
                rect()
                    .key(i)
                    .a11y_role(AccessibilityRole::RadioButton)
                    .a11y_alt(*glyph)
                    .on_press(move |_| chosen.set(i))
                    .child(badge(glyph, *chosen.read() == i, 24.))
                    .into_element()
            }));
        let named = !name.read().trim().is_empty();
        ui::sheet(
            400.,
            vec![
                text::line(
                    &theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    "Add Focus",
                )
                .into_element(),
                form::group(
                    None,
                    vec![
                        form::Row::new("Name")
                            .control(
                                isle_ui::TextField::new(name, "Name")
                                    .placeholder("Reading")
                                    .width(Size::px(200.)),
                            )
                            .into_element(),
                    ],
                )
                .into_element(),
                grid.into_element(),
            ],
            vec![
                ("Cancel".into(), cancel),
                (
                    "Add".into(),
                    EventHandler::new(move |_| {
                        let new = name.peek().trim().to_owned();
                        if !named || new.contains(['[', ']', '"']) {
                            return;
                        }
                        desktop.set(
                            field(&new, "symbol"),
                            Value::Str(SYMBOLS[*chosen.peek()].to_owned()),
                        );
                        nav.replace(Location {
                            pane: at.pane.clone(),
                            sub: Some(format!("mode/{new}")),
                            row: None,
                        });
                    }),
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_symbol_is_bundled() {
        for symbol in SYMBOLS {
            assert!(isle_ui::glyph_svg(symbol, true).is_some(), "{symbol}");
        }
    }

    #[test]
    fn schedules_read_as_their_rows_say() {
        assert_eq!(
            describe_schedule("09:00-17:00 weekdays"),
            "Weekdays 09:00–17:00"
        );
        assert_eq!(describe_schedule("22:00-07:00"), "Every day 22:00–07:00");
        assert_eq!(
            describe_schedule("10:00-12:00 mon,wed"),
            "Mon, Wed 10:00–12:00"
        );
    }
}
