//! Keyboard: key repeat, each keyboard's kind, input sources, Caps Lock and the shortcuts.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{CheckRow, Glyph, colour, form, text, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Follow, Services, use_follow},
    ui::{self, Rows},
};

const RATES: &[i64] = &[2, 5, 10, 15, 20, 25, 30, 40, 60];
const DELAYS: &[i64] = &[2000, 1500, 1000, 800, 600, 400, 250, 150];

/// Caps Lock's choices: the option it writes, none for the key as it is.
const CAPS: [(Option<&str>, &str); 4] = [
    (None, "Caps Lock"),
    (Some("caps:ctrl_modifier"), "Control"),
    (Some("caps:escape"), "Escape"),
    (Some("caps:none"), "No Action"),
];

/// The shortcut sheet's categories and their actions, as `shortcuts.<id>` and their names.
pub const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "Windows",
        &[
            ("close-window", "Close window"),
            ("minimise", "Minimise"),
            ("minimise-app", "Minimise all of the app's windows"),
            ("hide", "Hide"),
            ("hide-others", "Hide others"),
            ("quit-app", "Quit"),
            ("full-screen", "Enter or leave full screen"),
            ("next-window", "Next window of the app"),
            ("previous-window", "Previous window of the app"),
            ("window-menu", "Window menu"),
            ("zoom-menu", "Zoom menu"),
        ],
    ),
    (
        "Tiling",
        &[
            ("tile-left", "Left half"),
            ("tile-right", "Right half"),
            ("tile-centre", "Centre"),
            ("fill", "Fill"),
            ("restore", "Restore"),
            ("tile-top-left", "Top left"),
            ("tile-top-right", "Top right"),
            ("tile-bottom-left", "Bottom left"),
            ("tile-bottom-right", "Bottom right"),
            ("tile-beside", "Fill beside"),
            ("next-display", "Move to next display"),
            ("previous-display", "Move to previous display"),
        ],
    ),
    (
        "Desktops",
        &[
            ("desktop-left", "Desktop to the left"),
            ("desktop-right", "Desktop to the right"),
            ("carry-left", "Carry window left"),
            ("carry-right", "Carry window right"),
        ],
    ),
    (
        "Spotlight and apps",
        &[
            ("spotlight", "Spotlight"),
            ("apps", "Apps"),
            ("emoji", "Emoji"),
            ("switcher", "App switcher"),
            ("switcher-back", "App switcher, from the end"),
            ("terminal", "Open a terminal"),
            ("settings", "Open System Settings"),
            ("force-quit", "Force Quit"),
        ],
    ),
    (
        "Menus and panels",
        &[
            ("control-centre", "Control Centre"),
            ("notification-centre", "Notification Centre"),
            ("menu-bar", "Menu bar"),
            ("help-search", "Search menus"),
        ],
    ),
    (
        "Screenshots",
        &[
            ("screenshot-screen", "Screenshot of the screen"),
            (
                "screenshot-screen-copy",
                "Screenshot of the screen to the clipboard",
            ),
            ("screenshot-pick", "Screenshot of a portion"),
            (
                "screenshot-pick-copy",
                "Screenshot of a portion to the clipboard",
            ),
            ("capture-toolbar", "Capture toolbar"),
            ("capture-toolbar-copy", "Capture toolbar to the clipboard"),
            ("colour-pick", "Pick a colour"),
            ("stop-recording", "Stop recording"),
        ],
    ),
    (
        "Focus",
        &[
            ("do-not-disturb", "Do Not Disturb"),
            ("game-mode", "Game Mode"),
        ],
    ),
    (
        "Session",
        &[
            ("lock", "Lock Screen"),
            ("log-out", "Log Out…"),
            ("log-out-now", "Log Out"),
            ("session-chooser", "Restart, sleep or shut down"),
        ],
    ),
    (
        "Keyboard and displays",
        &[
            ("next-layout", "Next input source"),
            ("take-keys-back", "Take keys back"),
            ("redetect-displays", "Detect Displays"),
        ],
    ),
    (
        "Media",
        &[
            ("volume-up", "Volume up"),
            ("volume-down", "Volume down"),
            ("mute", "Mute"),
            ("mic-mute", "Microphone mute"),
            ("brightness-up", "Brightness up"),
            ("brightness-down", "Brightness down"),
            ("play-pause", "Play or pause"),
            ("next-track", "Next track"),
            ("previous-track", "Previous track"),
        ],
    ),
];

/// The modifiers that with 1 to 9 jump to a desktop.
const JUMPS: &[(&str, &str)] = &[
    ("Ctrl", "⌃ Control"),
    ("Option", "⌥ Option"),
    ("Cmd", "⌘ Command"),
    ("Ctrl+Option", "⌃⌥ Control-Option"),
    ("Ctrl+Cmd", "⌃⌘ Control-Command"),
];

/// The modifiers that with a press anywhere on a window move it.
const DRAGS: &[(&str, &str)] = &[
    ("Ctrl+Cmd", "⌃⌘ Control-Command"),
    ("Option+Cmd", "⌥⌘ Option-Command"),
    ("Ctrl+Option", "⌃⌥ Control-Option"),
    ("Cmd", "⌘ Command"),
    ("Option", "⌥ Option"),
];

/// A source's name: "British – Mac" for `gb` and `mac`.
pub fn source_name(catalogue: &[isle_keys::keymap::Layout], layout: &str, variant: &str) -> String {
    let Some(found) = catalogue.iter().find(|l| l.name == layout) else {
        return if variant.is_empty() {
            layout.to_owned()
        } else {
            format!("{layout} ({variant})")
        };
    };
    match found.variants.iter().find(|(v, _)| v == variant) {
        Some((_, description)) => description
            .split_once(": ")
            .map_or(description.as_str(), |(_, d)| d)
            .to_owned(),
        None => found.description.clone(),
    }
}

/// The settings' layouts paired with their variants.
fn sources(r: &Rows) -> Vec<(String, String)> {
    let variants = r.list("keyboard.variants");
    r.list("keyboard.layouts")
        .into_iter()
        .enumerate()
        .map(|(i, l)| (l, variants.get(i).cloned().unwrap_or_default()))
        .collect()
}

fn write_sources(desktop: isle_app::Desktop, list: &[(String, String)]) {
    desktop.set(
        "keyboard.layouts",
        Value::List(list.iter().map(|(l, _)| l.clone()).collect()),
    );
    desktop.set(
        "keyboard.variants",
        Value::List(list.iter().map(|(_, v)| v.clone()).collect()),
    );
}

#[derive(PartialEq)]
pub struct Keyboard {
    pub at: Location,
}

impl Component for Keyboard {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Keyboards]);
        if let Some(name) = self
            .at
            .sub
            .as_deref()
            .and_then(|s| s.strip_prefix("device/"))
        {
            return Device {
                at: self.at.clone(),
                name: name.to_owned(),
            }
            .into_element();
        }
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let catalogue = use_memo(isle_keys::keymap::catalogue);
        let mut detecting = use_state(|| false);
        let at = self.at.clone();

        let keyboards: Vec<Element> = services
            .keyboards
            .read()
            .state
            .iter()
            .filter(|k| !k.remapper)
            .map(|k| {
                let mut value = if k.profile == "mac" { "Mac" } else { "PC" }.to_owned();
                if k.guessed {
                    value.push_str(" (automatic)");
                }
                let at = at.clone();
                let sub = format!("device/{}", k.name);
                r.row(&k.name, k.name.clone(), None)
                    .leading(Glyph::new(
                        "keyboard",
                        20.,
                        colour(isle_ui::theme().tokens.colour_text_2),
                    ))
                    .opens(Some(value), move |_| super::open(nav, &at, &sub))
                    .into_element()
            })
            .collect();
        let detect = EventHandler::new({
            let at = at.clone();
            move |_| {
                let Some(bus) = bus.clone() else { return };
                detecting.set(true);
                let job = worker.spawn(async move {
                    let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await.ok()?;
                    proxy.detect().await.ok()
                });
                let at = at.clone();
                spawn(async move {
                    let found = job.await.ok().flatten();
                    detecting.set(false);
                    if let Some(found) = found.filter(|f| !f.keyboard.is_empty()) {
                        super::open(nav, &at, &format!("device/{}", found.keyboard));
                    }
                });
            }
        });
        let list = sources(&r);
        let shown = list
            .iter()
            .map(|(l, v)| source_name(&catalogue.read(), l, v))
            .collect::<Vec<_>>()
            .join(", ");
        let opener = |sub: &'static str| {
            let at = at.clone();
            EventHandler::new(move |_| super::open(nav, &at, sub))
        };
        let sheet = self.at.sub.clone();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(
                None,
                vec![
                    r.slider_steps(
                        "repeat-rate",
                        "Key repeat rate",
                        "keyboard.repeat-rate",
                        RATES,
                        ("Slow", "Fast"),
                    )
                    .into_element(),
                    r.slider_steps(
                        "repeat-delay",
                        "Delay until repeat",
                        "keyboard.repeat-delay",
                        DELAYS,
                        ("Long", "Short"),
                    )
                    .into_element(),
                ],
            ))
            .child(form::group(
                Some("Keyboards".into()),
                if keyboards.is_empty() {
                    vec![form::Row::new("No keyboard is connected.").enabled(false).into_element()]
                } else {
                    keyboards
                },
            ))
            .child(ui::foot(vec![(
                if *detecting.read() {
                    "Press any key…"
                } else {
                    "Detect…"
                }
                .into(),
                detect,
            )]))
            .child(form::group(
                Some("Text input".into()),
                vec![
                    r.row("input-sources", "Input sources", Some("keyboard.layouts"))
                        .description(if shown.is_empty() {
                            "The system's layout".to_owned()
                        } else {
                            shown
                        })
                        .control(isle_ui::Button::new("Edit…").on_press(opener("Input Sources…")))
                        .into_element(),
                ],
            ))
            .child(form::group(
                None,
                vec![
                    r.switch("numlock", "Turn on Num Lock at login", "keyboard.numlock")
                        .into_element(),
                    r.switch(
                        "translate",
                        "Use Mac shortcuts in other apps",
                        "keyboard.translate",
                    )
                    .description("Cmd shortcuts become each app's own, as Cmd+C does Ctrl+C.")
                    .into_element(),
                ],
            ))
            .child(ui::foot(vec![
                ("Modifier Keys…".into(), opener("Modifier Keys…")),
                ("Keyboard Shortcuts…".into(), opener("Keyboard Shortcuts…")),
            ]))
            .maybe(sheet.as_deref() == Some("Input Sources…"), |el| {
                el.child(InputSources { at: at.clone() })
            })
            .maybe(sheet.as_deref() == Some("Modifier Keys…"), |el| {
                el.child(ModifierKeys { at: at.clone() })
            })
            .maybe(sheet.as_deref() == Some("Keyboard Shortcuts…"), |el| {
                el.child(Shortcuts { at: at.clone() })
            })
            .into_element()
    }
}

#[derive(PartialEq)]
struct Device {
    at: Location,
    name: String,
}

impl Component for Device {
    fn render(&self) -> impl IntoElement {
        let services = Services::get();
        let r = Rows::new(&self.at);
        let name = self.name.clone();
        let field = |f: &str| format!("keyboard-device[{name}].{f}");
        let desktop = r.desktop;
        let profile = r.str(&field("profile")).unwrap_or_else(|| "auto".into());
        let is_mac = match profile.as_str() {
            "mac" => true,
            "pc" => false,
            _ => services
                .keyboards
                .read()
                .state
                .iter()
                .any(|k| k.name == name && k.profile == "mac"),
        };
        let model = r.str(&field("model")).unwrap_or_else(|| "auto".into());
        const PROFILES: [&str; 3] = ["auto", "mac", "pc"];
        const MODELS: [&str; 3] = ["auto", "iso", "ansi"];
        let (pkey, mkey) = (field("profile"), field("model"));
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::sub_header(
                "keyboard",
                &name,
                "How this keyboard's modifier keys work.",
            ))
            .child(form::group(
                None,
                vec![
                    r.row("kind", "Kind", Some(&pkey))
                        .control(form::Segmented::new(
                            vec!["Automatic".into(), "Mac".into(), "PC".into()],
                            PROFILES.iter().position(|p| *p == profile).unwrap_or(0),
                            "Kind",
                            move |i: usize| {
                                desktop.set(pkey.clone(), Value::Str(PROFILES[i].into()))
                            },
                        ))
                        .into_element(),
                    r.switch_key(
                        "swap-alt-super",
                        "Swap Alt and Super",
                        field("swap-alt-super"),
                        true,
                        !is_mac,
                    )
                    .description("Cmd sits beside the space bar.")
                    .into_element(),
                    r.row("model", "Keyboard type", Some(&mkey))
                        .description("ISO has a key left of Z.")
                        .control(form::PopUp::new(
                            vec!["Automatic".into(), "ISO".into(), "ANSI".into()],
                            MODELS.iter().position(|m| *m == model).unwrap_or(0),
                            "Keyboard type",
                            move |i: usize| desktop.set(mkey.clone(), Value::Str(MODELS[i].into())),
                        ))
                        .into_element(),
                ],
            ))
            .into_element()
    }
}

#[derive(PartialEq)]
struct InputSources {
    at: Location,
}

impl Component for InputSources {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let catalogue = use_memo(isle_keys::keymap::catalogue);
        let mut adding = use_state(|| false);
        let query = use_state(String::new);
        let list = sources(&r);
        let at = self.at.clone();
        let done = EventHandler::new(move |_| super::close(nav, &at));
        if *adding.read() {
            let q = query.read().to_lowercase();
            let mut choices: Vec<(String, String, String)> = Vec::new();
            for layout in catalogue.read().iter() {
                choices.push((
                    layout.name.clone(),
                    String::new(),
                    layout.description.clone(),
                ));
                for (v, d) in &layout.variants {
                    let d = d.split_once(": ").map_or(d.as_str(), |(_, d)| d).to_owned();
                    choices.push((layout.name.clone(), v.clone(), d));
                }
            }
            choices.retain(|(_, _, d)| q.is_empty() || d.to_lowercase().contains(&q));
            choices.truncate(200);
            return ui::sheet(
                480.,
                vec![
                    isle_ui::TextField::new(query, "Search layouts")
                        .placeholder("Search")
                        .into_element(),
                    isle_ui::scroll_area(&theme)
                        .width(Size::fill())
                        .height(Size::px(320.))
                        .child(rect().width(Size::fill()).children(choices.into_iter().map(
                            |(l, v, d)| {
                                let list = list.clone();
                                rect()
                                    .key(format!("{l}({v})"))
                                    .width(Size::fill())
                                    .padding((t.space_xs, t.space_s))
                                    .corner_radius(t.radius_row)
                                    .a11y_role(AccessibilityRole::ListItem)
                                    .a11y_alt(d.clone())
                                    .on_press(move |_| {
                                        let mut next = list.clone();
                                        if !next.iter().any(|(a, b)| *a == l && *b == v) {
                                            next.push((l.clone(), v.clone()));
                                        }
                                        write_sources(desktop, &next);
                                        adding.set(false);
                                    })
                                    .child(text::line(
                                        &theme,
                                        text::Style::Body,
                                        colour(t.colour_text),
                                        d,
                                    ))
                                    .into_element()
                            },
                        )))
                        .into_element(),
                ],
                vec![("Back".into(), EventHandler::new(move |_| adding.set(false)))],
            );
        }
        let names: Vec<(String, Option<Option<std::path::PathBuf>>)> = list
            .iter()
            .enumerate()
            .map(|(i, (l, v))| {
                let name = source_name(&catalogue.read(), l, v);
                (
                    if i == 0 {
                        format!("{name} (default)")
                    } else {
                        name
                    },
                    None,
                )
            })
            .collect();
        let chord = r
            .list("shortcuts.next-layout")
            .first()
            .cloned()
            .unwrap_or_default();
        let mut body = vec![
            ui::ListEditor {
                on_open: None,
                on_move: Some(EventHandler::new({
                    let list = list.clone();
                    move |(from, to): (usize, usize)| {
                        write_sources(desktop, &ui::moved(&list, from, to))
                    }
                })),
                title: None,
                items: names,
                empty: "The system's layout".into(),
                on_add: EventHandler::new(move |_| adding.set(true)),
                on_remove: EventHandler::new({
                    let list = list.clone();
                    move |i: usize| {
                        let mut next = list.clone();
                        if i < next.len() {
                            next.remove(i);
                        }
                        write_sources(desktop, &next);
                    }
                }),
            }
            .into_element(),
        ];
        if list.len() > 1 && !chord.is_empty() {
            body.push(
                form::group(
                    None,
                    vec![r.info("switch", "Switch with", chord).into_element()],
                )
                .into_element(),
            );
        }
        ui::sheet(480., body, vec![("Done".into(), done)])
    }
}

#[derive(PartialEq)]
struct ModifierKeys {
    at: Location,
}

impl Component for ModifierKeys {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let options = r.list("keyboard.options");
        let selected = CAPS
            .iter()
            .position(|(o, _)| o.is_some_and(|o| options.iter().any(|x| x == o)))
            .unwrap_or(0);
        let at = self.at.clone();
        ui::sheet(
            440.,
            vec![
                form::group(
                    None,
                    vec![
                        r.row("caps-lock", "Caps Lock key", Some("keyboard.options"))
                            .control(form::PopUp::new(
                                CAPS.iter().map(|(_, l)| (*l).to_owned()).collect(),
                                selected,
                                "Caps Lock key",
                                move |i: usize| {
                                    let mut next: Vec<String> = options
                                        .iter()
                                        .filter(|o| !o.starts_with("caps:"))
                                        .cloned()
                                        .collect();
                                    if let Some(option) = CAPS[i].0 {
                                        next.push(option.to_owned());
                                    }
                                    desktop.set("keyboard.options", Value::List(next));
                                },
                            ))
                            .into_element(),
                    ],
                )
                .into_element(),
            ],
            vec![(
                "Done".into(),
                EventHandler::new(move |_| super::close(nav, &at)),
            )],
        )
    }
}

#[derive(PartialEq)]
struct Shortcuts {
    at: Location,
}

impl Component for Shortcuts {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let mut category = use_state(|| 0usize);
        let mut recording = use_state(|| None::<&'static str>);
        let mut clash = use_state(|| None::<String>);
        let (name, actions) = SHORTCUTS[*category.read()];
        let categories = rect().width(Size::px(180.)).spacing(t.space_xxs).children(
            SHORTCUTS.iter().enumerate().map(|(i, (label, _))| {
                let on = *category.read() == i;
                rect()
                    .key(i)
                    .width(Size::fill())
                    .padding((t.space_xs, t.space_s))
                    .corner_radius(t.radius_row)
                    .maybe(on, |el| el.background(colour(t.colour_accent)))
                    .a11y_role(AccessibilityRole::ListItem)
                    .a11y_alt(*label)
                    .on_press(move |_| category.set(i))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(if on {
                            t.colour_on_accent
                        } else {
                            t.colour_text
                        }),
                        *label,
                    ))
                    .into_element()
            }),
        );
        // The category's settings that are not chords.
        let terminal = use_state(String::new);
        let extra: Vec<Element> = match name {
            "Desktops" => vec![
                r.choice(
                    "desktop-jump",
                    "Switch to a desktop with",
                    "shortcuts.desktop-jump",
                    JUMPS,
                )
                .into_element(),
                r.choice(
                    "window-drag",
                    "Move and resize windows with",
                    "shortcuts.window-drag",
                    DRAGS,
                )
                .into_element(),
            ],
            "Spotlight and apps" => {
                let current = r.str("shortcuts.terminal-command").unwrap_or_default();
                vec![
                    r.row(
                        "terminal-command",
                        "Terminal",
                        Some("shortcuts.terminal-command"),
                    )
                    .control(
                        isle_ui::TextField::new(terminal, "Terminal")
                            .placeholder(current)
                            .width(Size::px(200.))
                            .on_submit(move |text: String| {
                                let text = text.trim().to_owned();
                                if !text.is_empty() {
                                    desktop.set("shortcuts.terminal-command", Value::Str(text));
                                }
                            }),
                    )
                    .into_element(),
                ]
            }
            _ => Vec::new(),
        };
        let rows: Vec<Element> = actions
            .iter()
            .map(|(id, label)| {
                let key = format!("shortcuts.{id}");
                let chords = r.list(&key);
                let on = !chords.is_empty();
                let shown = if *recording.read() == Some(*id) {
                    "Type a shortcut…".to_owned()
                } else if on {
                    chords.join(", ")
                } else {
                    "None".to_owned()
                };
                let record = {
                    let (bus, worker, key) = (bus.clone(), worker.clone(), key.clone());
                    let id: &'static str = id;
                    move |_| {
                        let Some(bus) = bus.clone() else { return };
                        recording.set(Some(id));
                        let job = worker.spawn(async move {
                            let proxy = isle_ipc::keyboard::KeyboardProxy::new(&bus).await.ok()?;
                            proxy.record().await.ok()
                        });
                        let key = key.clone();
                        spawn(async move {
                            let got = job.await.ok().flatten();
                            recording.set(None);
                            if let Some(got) = got.filter(|g| !g.cancelled) {
                                desktop.set(
                                    key,
                                    Value::List(if got.chord.is_empty() {
                                        Vec::new()
                                    } else {
                                        vec![got.chord]
                                    }),
                                );
                                clash.set(Some(got.holder).filter(|h| !h.is_empty()));
                            }
                        });
                    }
                };
                rect()
                    .key(*id)
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .padding((t.space_xs, 0.))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(CheckRow::new(*label, on).on_toggle({
                                let key = key.clone();
                                move |checked: bool| {
                                    if checked {
                                        desktop.reset(key.clone())
                                    } else {
                                        desktop.set(key.clone(), Value::List(Vec::new()))
                                    }
                                }
                            })),
                    )
                    .child(isle_ui::Button::new(shown).small().on_press(record))
                    .into_element()
            })
            .collect();
        let at = self.at.clone();
        ui::sheet(
            720.,
            vec![
                rect()
                    .width(Size::fill())
                    .height(Size::px(400.))
                    .horizontal()
                    .content(Content::flex())
                    .spacing(t.space_l)
                    .child(categories)
                    .child(
                        isle_ui::scroll_area(&theme)
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .child(text::line(
                                        &theme,
                                        text::Style::Headline,
                                        colour(t.colour_text),
                                        name,
                                    ))
                                    .children(rows)
                                    .children(extra)
                                    .map(clash.read().clone(), |el, holder| {
                                        el.child(text::line(
                                            &theme,
                                            text::Style::Callout,
                                            colour(t.colour_warn_text),
                                            format!("That shortcut is also {holder}'s."),
                                        ))
                                    }),
                            ),
                    )
                    .into_element(),
            ],
            vec![
                (
                    "Restore Defaults".into(),
                    EventHandler::new(move |_| desktop.reset("shortcuts")),
                ),
                (
                    "Done".into(),
                    EventHandler::new(move |_| super::close(nav, &at)),
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shortcut_is_a_setting() {
        let mut keys = isle_settings::schema_of::<isle_settings::schema::Settings>();
        keys.retain(|k| k.key.starts_with("shortcuts."));
        for (_, actions) in SHORTCUTS {
            for (id, _) in *actions {
                assert!(
                    keys.iter().any(|k| k.key == format!("shortcuts.{id}")),
                    "{id}"
                );
            }
        }
    }

    #[test]
    fn a_source_reads_by_its_description() {
        let catalogue = isle_keys::keymap::parse_catalogue(
            "! layout\n  gb              English (UK)\n! variant\n  mac             gb: English (UK, Macintosh)\n",
        );
        assert_eq!(source_name(&catalogue, "gb", ""), "English (UK)");
        assert_eq!(
            source_name(&catalogue, "gb", "mac"),
            "English (UK, Macintosh)"
        );
        assert_eq!(source_name(&catalogue, "xx", "y"), "xx (y)");
    }
}
