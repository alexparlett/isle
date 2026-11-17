//! General › Software Update: the system, CachyOS and Isle, each a card with what is waiting,
//! the run, and a restart owed (`p14e-store.md` choice 1, board `p14e/store-1.html`).

use freya::prelude::*;
use isle_ui::{Button, Glyph, button::Kind, colour, form, text, use_theme};
use serde_json::Value;

use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, Rows},
};

/// A time today as hours and minutes, local.
fn clock(secs: u64) -> String {
    let t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return String::new();
    }
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// What the card says and offers, from `Updates1.Status`.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub glyph: &'static str,
    pub tone: Tone,
    pub title: String,
    pub sub: String,
    /// By source: name, count or note.
    pub sources: Vec<(String, String)>,
    pub progress: Option<f32>,
    pub lines: Vec<String>,
    pub note: Option<String>,
    pub buttons: Vec<CardButton>,
    /// More Info… and Show Details at the leading end.
    pub more: bool,
    pub details: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tone {
    Plain,
    Accent,
    Good,
    Warn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardButton {
    pub label: &'static str,
    pub act: Option<Act>,
    pub default: bool,
}

fn button(label: &'static str, act: Option<Act>, default: bool) -> CardButton {
    CardButton {
        label,
        act,
        default,
    }
}

impl Card {
    pub fn from_status(s: &Value) -> Card {
        let run = &s["progress"];
        let state = run["state"].as_str().unwrap_or_default();
        let pending = s["pending"].as_array().cloned().unwrap_or_default();
        let restart: Vec<String> = s["restart"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| r.as_str().map(str::to_owned))
            .collect();
        let lines: Vec<String> = run["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l.as_str().map(str::to_owned))
            .collect();
        let tail = lines[lines.len().saturating_sub(8)..].to_vec();
        let at = |key: &str| run[key].as_u64().map(clock).unwrap_or_default();
        let base = Card {
            glyph: "arrow-circle-up",
            tone: Tone::Plain,
            title: String::new(),
            sub: String::new(),
            sources: Vec::new(),
            progress: None,
            lines: Vec::new(),
            note: None,
            buttons: Vec::new(),
            more: false,
            details: false,
        };
        match state {
            "Running" => {
                let (done, total) = (
                    run["done"].as_u64().unwrap_or(0),
                    run["total"].as_u64().unwrap_or(0),
                );
                let can_stop = s["can_stop"] == true;
                Card {
                    tone: Tone::Accent,
                    title: if total > 0 {
                        format!("Installing {} of {total}", done.min(total))
                    } else {
                        "Installing".into()
                    },
                    sub: format!(
                        "{} · Started {}",
                        run["phase"].as_str().unwrap_or_default(),
                        at("started_at")
                    ),
                    progress: Some(if total > 0 {
                        done as f32 / total as f32
                    } else {
                        0.0
                    }),
                    lines: tail,
                    note: (!can_stop)
                        .then(|| "Can't stop while packages are being installed.".into()),
                    buttons: vec![CardButton {
                        label: "Stop",
                        act: can_stop.then_some(Act::StopUpdates),
                        default: false,
                    }],
                    details: true,
                    ..base
                }
            }
            "Failed" => Card {
                glyph: "warning",
                tone: Tone::Warn,
                title: "The update stopped".into(),
                sub: run["reason"].as_str().unwrap_or_default().into(),
                lines: tail,
                buttons: vec![button(
                    "Try Again",
                    Some(Act::InstallUpdates(String::new())),
                    true,
                )],
                details: true,
                ..base
            },
            "Stopped" => Card {
                title: "The update was stopped".into(),
                sub: "Nothing more was installed.".into(),
                lines: tail,
                buttons: vec![
                    button("Check Now", Some(Act::CheckUpdates), false),
                    button("Update Now", Some(Act::InstallUpdates(String::new())), true),
                ],
                details: true,
                ..base
            },
            "Refused" => Card {
                glyph: "lock",
                title: "The update didn't start".into(),
                sub: "Permission was not given. Nothing was changed.".into(),
                buttons: vec![button(
                    "Try Again",
                    Some(Act::InstallUpdates(String::new())),
                    true,
                )],
                ..base
            },
            "Done" if restart.is_empty() => Card {
                glyph: "check",
                tone: Tone::Good,
                title: "Updates installed".into(),
                sub: format!(
                    "{} · Finished {}",
                    plural(
                        run["total"].as_u64().unwrap_or(0) as usize,
                        "package",
                        "packages"
                    ),
                    at("finished_at")
                ),
                buttons: vec![button("Check Now", Some(Act::CheckUpdates), false)],
                ..base
            },
            _ if s["checking"] == true => Card {
                title: "Checking for updates…".into(),
                sub: "Repositories and Isle".into(),
                buttons: vec![button("Check Now", None, false)],
                ..base
            },
            _ if !pending.is_empty() => Card {
                title: String::new(),
                ..base
            },
            _ if !restart.is_empty() => Card {
                glyph: "arrows-clockwise",
                tone: Tone::Accent,
                title: "Restart to finish updating".into(),
                sub: "Until you restart, the computer keeps running what it started with.".into(),
                sources: restart.into_iter().map(|r| (r, String::new())).collect(),
                buttons: vec![
                    button("Check Now", Some(Act::CheckUpdates), false),
                    button("Restart…", Some(Act::RestartForUpdates), true),
                ],
                ..base
            },
            _ => Card {
                glyph: "check",
                tone: if s["check_error"].is_string() {
                    Tone::Plain
                } else {
                    Tone::Good
                },
                title: if s["check_error"].is_string() {
                    "Couldn't check for updates".into()
                } else {
                    "CachyOS and Isle are up to date".into()
                },
                sub: s["check_error"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| checked(s)),
                buttons: vec![button("Check Now", Some(Act::CheckUpdates), false)],
                ..base
            },
        }
    }
}

/// The cards the page shows: the run's or the restart's when there is one, then a card each
/// for CachyOS and Isle while they have updates waiting.
pub fn cards(s: &Value) -> Vec<Card> {
    let first = Card::from_status(s);
    let running = s["progress"]["state"] == "Running";
    let mut out = Vec::new();
    if !first.title.is_empty() {
        let ends = !matches!(first.title.as_str(), "Restart to finish updating");
        out.push(first);
        if ends || running || s["checking"] == true {
            return out;
        }
    }
    if s["checking"] == true {
        return out;
    }
    // With updates waiting, the status says nothing of a restart still owed; ask without them.
    if out.is_empty() {
        let mut owed = s.clone();
        owed["pending"] = Value::Array(Vec::new());
        let restart = Card::from_status(&owed);
        if restart.title == "Restart to finish updating" {
            out.push(restart);
        }
    }
    let pending = s["pending"].as_array().cloned().unwrap_or_default();
    let system = &s["system"];
    let repo = pending
        .iter()
        .filter(|p| p["source"] == "Repositories")
        .count();
    let base = Card {
        glyph: "arrow-circle-up",
        tone: Tone::Accent,
        title: String::new(),
        sub: String::new(),
        sources: Vec::new(),
        progress: None,
        lines: Vec::new(),
        note: None,
        buttons: Vec::new(),
        more: false,
        details: false,
    };
    if repo > 0 {
        let named = |key: &str, label: &str| {
            let n = &system[key];
            n["package"].as_str().map(|p| {
                (
                    label.to_owned(),
                    format!(
                        "{p} {} → {}",
                        n["from"].as_str().unwrap_or_default(),
                        n["to"].as_str().unwrap_or_default()
                    ),
                )
            })
        };
        let mut sources: Vec<(String, String)> = [
            named("kernel", "Kernel"),
            named("driver", "Graphics driver"),
            named("mesa", "Graphics libraries"),
        ]
        .into_iter()
        .flatten()
        .collect();
        let tools: Vec<&str> = system["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !tools.is_empty() {
            sources.push(("CachyOS tools".into(), tools.join(", ")));
        }
        let apps: Vec<&str> = system["apps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| a["package"].as_str())
            .collect();
        if !apps.is_empty() {
            sources.push((
                format!(
                    "Also updates {} installed from CachyOS",
                    plural(apps.len(), "app", "apps")
                ),
                apps.join(", "),
            ));
        }
        let restart = system["kernel"].is_object() || system["driver"].is_object();
        out.push(Card {
            title: "CachyOS".into(),
            sub: format!(
                "{}{} · {}",
                plural(repo, "package", "packages"),
                if restart { " · Restart required" } else { "" },
                checked(s)
            ),
            sources,
            buttons: vec![
                button("Check Now", Some(Act::CheckUpdates), false),
                button(
                    "Update Now",
                    Some(Act::InstallUpdates("system".into())),
                    true,
                ),
            ],
            more: true,
            ..base.clone()
        });
    }
    if let Some(isle) = pending.iter().find(|p| p["source"] == "Isle") {
        let from = isle["from"].as_str().unwrap_or_default();
        let from = from.rsplit(".g").next().unwrap_or(from);
        let from: String = from.chars().take(7).collect();
        let to: String = isle["to"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .take(7)
            .collect();
        out.push(Card {
            glyph: "sun-horizon",
            title: "Isle".into(),
            sub: format!("{from} → {to} · Built on this computer"),
            buttons: vec![button(
                "Update Now",
                Some(Act::InstallUpdates("isle".into())),
                true,
            )],
            ..base
        });
    }
    out
}

fn checked(s: &Value) -> String {
    match s["checked_at"].as_u64() {
        Some(at) => format!("Checked {}", clock(at)),
        None => "Not checked yet".into(),
    }
}

#[derive(PartialEq)]
pub struct SoftwareUpdatePage {
    pub at: Location,
}

impl Component for SoftwareUpdatePage {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let services = use_follow(&[Follow::Updates]);
        let r = Rows::new(&self.at);
        let details = use_state(|| false);
        let mut more = use_state(|| false);
        let status = services.updates.read().state.clone();
        let mut groups: Vec<Element> = Vec::new();
        for card in cards(&status) {
            groups.push(
                form::group(
                    None,
                    vec![card_view(&theme, &card, services, details, more)],
                )
                .into_element(),
            );
        }
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::sub_header(
                "arrow-circle-up",
                "Software Update",
                "CachyOS, its kernel and drivers, and Isle.",
            ))
            .children(groups)
            .child(text::line(
                &theme,
                text::Style::Callout,
                colour(t.colour_text_2),
                "Apps from Flathub, the AUR and AppImages update in Store.",
            ))
            .child(form::group(
                None,
                vec![
                    r.switch(
                        "updates-auto",
                        "Check for updates automatically",
                        "updates.check",
                    )
                    .description("Looks once an hour; nothing is installed until you choose.")
                    .into_element(),
                ],
            ))
            .maybe(*more.read(), |el| {
                el.child(MoreInfo {
                    status: status.clone(),
                    on_done: EventHandler::new(move |_| more.set(false)),
                })
            })
    }
}

/// One card: its glyph, title and line, what it lists, its progress and lines, its buttons.
fn card_view(
    theme: &isle_tokens::Theme,
    card: &Card,
    services: crate::services::Services,
    mut details: State<bool>,
    mut more: State<bool>,
) -> Element {
    let t = &theme.tokens;

    let (fill, ink) = match card.tone {
        Tone::Plain => (t.colour_fill_strong, t.colour_text),
        Tone::Accent => (t.colour_accent, t.colour_on_accent),
        Tone::Good => (t.colour_ok, t.colour_on_danger),
        Tone::Warn => (t.colour_warn, t.colour_on_danger),
    };
    let circle = rect()
        .width(Size::px(40.))
        .height(Size::px(40.))
        .corner_radius(20.)
        .background(colour(fill))
        .center()
        .child(Glyph::new(card.glyph, 22., colour(ink)).fill(card.tone != Tone::Plain));
    let head = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_m)
        .child(circle)
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t.space_xxs)
                .child(text::line(
                    &theme,
                    text::Style::Title3,
                    colour(t.colour_text),
                    card.title.clone(),
                ))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    card.sub.clone(),
                )),
        );
    let sources = (!card.sources.is_empty()).then(|| {
        rect()
            .width(Size::fill())
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill))
            .padding((t.space_xxs, 0.))
            .children(card.sources.iter().map(|(name, note)| {
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .padding((0., t.space_m))
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        name.clone(),
                    )))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        note.clone(),
                    ))
                    .into_element()
            }))
    });
    let bar = card.progress.map(|level| isle_ui::LevelBar {
        level,
        track: colour(t.colour_fill_strong),
        fill: colour(t.colour_accent),
    });
    let shown_lines = card.details && *details.read() && !card.lines.is_empty();
    let log = shown_lines.then(|| {
        rect()
            .width(Size::fill())
            .corner_radius(t.radius_control)
            .background(colour(t.colour_window_sunken))
            .padding((t.space_s, t.space_m))
            .children(card.lines.iter().map(|l| {
                label()
                    .font_family("JetBrains Mono")
                    .font_size(t.type_style_callout_size)
                    .color(colour(t.colour_text_2))
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .text(l.clone())
                    .into_element()
            }))
    });
    let leading: Option<Element> = if card.more {
        Some(
            Button::new("Show All")
                .on_press(move |_| more.set(true))
                .into_element(),
        )
    } else if card.details && !card.lines.is_empty() {
        let open = *details.read();
        Some(
            Button::new(if open { "Hide Details" } else { "Show Details" })
                .on_press(move |_| details.set(!open))
                .into_element(),
        )
    } else {
        None
    };
    let buttons = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_s)
        .cross_align(Alignment::Center)
        .child(rect().width(Size::flex(1.)).maybe_child(leading))
        .children(card.buttons.iter().map(|b| {
            let act = b.act.clone();
            Button::new(b.label)
                .kind(if b.default {
                    Kind::Default
                } else {
                    Kind::Regular
                })
                .enabled(act.is_some())
                .on_press(move |_| {
                    if let Some(act) = act.clone() {
                        services.act(act);
                    }
                })
                .into_element()
        }));
    let body = rect()
        .width(Size::fill())
        .padding(t.space_l)
        .spacing(t.space_m)
        .child(head)
        .maybe_child(sources)
        .maybe_child(bar)
        .maybe_child(log)
        .maybe_child(
            card.note
                .clone()
                .map(|n| text::line(&theme, text::Style::Callout, colour(t.colour_text_2), n)),
        )
        .child(buttons);
    body.into_element()
}

/// Every package waiting, by source, with a search field.
#[derive(PartialEq)]
struct MoreInfo {
    status: Value,
    on_done: EventHandler<()>,
}

impl Component for MoreInfo {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let query = use_state(String::new);
        let q = query.read().to_lowercase();
        let pending = self.status["pending"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut sections: Vec<Element> = Vec::new();
        for (source, name) in [("Repositories", "Repositories"), ("Isle", "Isle")] {
            let rows: Vec<Element> = pending
                .iter()
                .filter(|p| p["source"] == source)
                .filter(|p| q.is_empty() || p["name"].as_str().is_some_and(|n| n.contains(&q)))
                .map(|p| {
                    let cell = |s: &str, ink| {
                        rect().width(Size::flex(1.)).child(
                            label()
                                .font_family("JetBrains Mono")
                                .font_size(t.type_style_callout_size)
                                .color(colour(ink))
                                .max_lines(1)
                                .text_overflow(TextOverflow::Ellipsis)
                                .text(s.to_owned()),
                        )
                    };
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.size_row))
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_m)
                        .child(rect().width(Size::flex(2.)).child(text::line(
                            &theme,
                            text::Style::Body,
                            colour(t.colour_text),
                            p["name"].as_str().unwrap_or_default(),
                        )))
                        .child(cell(
                            p["from"].as_str().unwrap_or_default(),
                            t.colour_text_2,
                        ))
                        .child(cell(p["to"].as_str().unwrap_or_default(), t.colour_text))
                        .into_element()
                })
                .collect();
            if !rows.is_empty() {
                sections.push(
                    text::line(
                        &theme,
                        text::Style::Headline,
                        colour(t.colour_text),
                        format!("{name} · {}", rows.len()),
                    )
                    .into_element(),
                );
                sections.extend(rows);
            }
        }
        isle_ui::sheet(
            560.,
            vec![
                text::line(
                    &theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    plural(pending.len(), "update", "updates"),
                )
                .into_element(),
                isle_ui::TextField::new(query, "Search packages")
                    .placeholder("Search packages")
                    .into_element(),
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::px(360.))
                    .children(sections)
                    .into_element(),
            ],
            vec![("Done".into(), self.on_done.clone())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_system_and_isle_each_have_a_card() {
        let up = cards(&status(r#"{"pending":[],"checked_at":0,"restart":[]}"#));
        assert_eq!(up.len(), 1);
        assert_eq!(up[0].title, "CachyOS and Isle are up to date");
        let waiting = cards(&status(
            r#"{"pending":[{"name":"linux-cachyos","from":"7.2.10-1","to":"7.3.4-1","source":"Repositories"},
                {"name":"blender","from":"4.5.3-1","to":"4.5.4-1","source":"Repositories"},
                {"name":"isle-git","from":"r725.g0f8209c46951-1","to":"7a1b2c3d4e5f","source":"Isle"}],
              "system":{"packages":2,"kernel":{"package":"linux-cachyos","from":"7.2.10","to":"7.3.4"},
                "driver":null,"mesa":null,"tools":[],"apps":[{"package":"blender","desktop":"blender","from":"4.5.3","to":"4.5.4"}]},
              "restart":[]}"#,
        ));
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0].title, "CachyOS");
        assert!(
            waiting[0].sub.starts_with("2 packages · Restart required"),
            "{}",
            waiting[0].sub
        );
        assert_eq!(
            waiting[0].sources,
            vec![
                ("Kernel".into(), "linux-cachyos 7.2.10 → 7.3.4".into()),
                (
                    "Also updates 1 app installed from CachyOS".into(),
                    "blender".into()
                )
            ]
        );
        assert_eq!(
            waiting[0].buttons[1].act,
            Some(Act::InstallUpdates("system".into()))
        );
        assert_eq!(waiting[1].title, "Isle");
        assert_eq!(waiting[1].sub, "0f8209c → 7a1b2c3 · Built on this computer");
        assert_eq!(
            waiting[1].buttons[0].act,
            Some(Act::InstallUpdates("isle".into()))
        );
        let running = cards(&status(
            r#"{"progress":{"state":"Running","done":112,"total":469,"phase":"Upgrading libuv","lines":[],"started_at":0},"can_stop":false,
                "pending":[{"name":"glibc","from":"1","to":"2","source":"Repositories"}],"restart":[]}"#,
        ));
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].title, "Installing 112 of 469");
        assert_eq!(running[0].buttons[0].act, None);
        let owed = cards(&status(
            r#"{"progress":{"state":"Done","total":3},"restart":["The kernel was updated"],
                "pending":[{"name":"isle-git","from":"r1.gabc-1","to":"def","source":"Isle"}]}"#,
        ));
        assert_eq!(owed[0].title, "Restart to finish updating");
        assert_eq!(owed[1].title, "Isle");
        let refused = cards(&status(r#"{"progress":{"state":"Refused"},"restart":[]}"#));
        assert_eq!(refused[0].title, "The update didn't start");
    }
}
