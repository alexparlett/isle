//! Desktop: how windows tile and fill, Mission Control and Stage Manager, and the
//! Shortcuts… and Hot Corners… sheets.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{colour, form, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::Rows,
};

/// Each corner's name, its action's key and its modifiers' key.
const CORNERS: [(&str, &str, &str); 4] = [
    (
        "Top left",
        "hot-corners.top-left",
        "hot-corners.top-left-modifiers",
    ),
    (
        "Top right",
        "hot-corners.top-right",
        "hot-corners.top-right-modifiers",
    ),
    (
        "Bottom left",
        "hot-corners.bottom-left",
        "hot-corners.bottom-left-modifiers",
    ),
    (
        "Bottom right",
        "hot-corners.bottom-right",
        "hot-corners.bottom-right-modifiers",
    ),
];

/// What a corner can do, and the item each reads as.
const ACTIONS: [(&str, &str); 8] = [
    ("mission-control", "Mission Control"),
    ("app-windows", "Application Windows"),
    ("desktop", "Desktop"),
    ("notification-centre", "Notification Centre"),
    ("apps", "Apps"),
    ("lock-screen", "Lock Screen"),
    ("display-sleep", "Put Display to Sleep"),
    ("none", "—"),
];

#[derive(PartialEq)]
pub struct Desktop {
    pub at: Location,
}

impl Component for Desktop {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let at = self.at.clone();
        let services = use_follow(&[Follow::Desktops]);
        let here = services.desktops.read().state.iter().find(|d| d.focus).cloned();
        let staged = here.as_ref().is_some_and(|d| d.stage);
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(
                Some("Windows".into()),
                vec![
                    r.choice(
                        "titlebar-action",
                        "Double-click a window's title bar to",
                        "windows.titlebar-action",
                        &[
                            ("fill", "Fill"),
                            ("minimise", "Minimise"),
                            ("none", "No Action"),
                        ],
                    )
                    .into_element(),
                    r.switch(
                        "edge-tiling",
                        "Drag windows to screen edges to tile",
                        "windows.edge-tiling",
                    )
                    .into_element(),
                    r.choice(
                        "tile-margin",
                        "Margins around tiled windows",
                        "windows.tile-margin",
                        &[
                            ("none", "None"),
                            ("small", "Small"),
                            ("medium", "Medium"),
                            ("large", "Large"),
                        ],
                    )
                    .description("Space between tiled and filled windows, and around them.")
                    .into_element(),
                ],
            ))
            .child(form::group(
                Some("Mission Control".into()),
                vec![
                    r.switch(
                        "rearrange-desktops",
                        "Automatically rearrange desktops based on most recent use",
                        "mission-control.rearrange-desktops",
                    )
                    .into_element(),
                    r.switch(
                        "group-by-app",
                        "Group windows by application",
                        "mission-control.group-by-app",
                    )
                    .into_element(),
                ],
            ))
            .child(crate::ui::foot(vec![
                (
                    "Shortcuts…".into(),
                    EventHandler::new({
                        let at = at.clone();
                        move |_| super::open(nav, &at, "Shortcuts…")
                    }),
                ),
                (
                    "Hot Corners…".into(),
                    EventHandler::new({
                        let at = at.clone();
                        move |_| super::open(nav, &at, "Hot Corners…")
                    }),
                ),
            ]))
            .child(form::group(
                Some("Stage Manager".into()),
                vec![
                    r.switch_with("stage", "Stage Manager", staged, move |on| {
                        if let Some(d) = &here {
                            services.act(Act::SetStage {
                                output: d.output.clone(),
                                index: d.index,
                                on,
                            });
                        }
                    })
                    .description("On for this desktop. Each desktop keeps its own.")
                    .into_element(),
                    r.switch("recent-apps", "Show recent apps", "stage-manager.recent-apps")
                        .description("Off, the apps show when the pointer presses the screen's left edge.")
                        .into_element(),
                    r.choice(
                        "stage-windows",
                        "Show windows from an application",
                        "stage-manager.windows",
                        &[("all-at-once", "All at once"), ("one-at-a-time", "One at a time")],
                    )
                    .into_element(),
                ],
            ))
            .maybe(self.at.sub.as_deref() == Some("Hot Corners…"), |el| {
                el.child(HotCorners { at: at.clone() })
            })
            .maybe(self.at.sub.as_deref() == Some("Shortcuts…"), |el| {
                el.child(MissionShortcuts { at: at.clone() })
            })
    }
}

/// Modifier keys in Isle's words, Mac order: "Option+Shift".
pub fn modifier_words(m: Modifiers) -> String {
    let mut words = Vec::new();
    if m.contains(Modifiers::CONTROL) {
        words.push("Control");
    }
    if m.contains(Modifiers::ALT) {
        words.push("Option");
    }
    if m.contains(Modifiers::SHIFT) {
        words.push("Shift");
    }
    if m.contains(Modifiers::META) {
        words.push("Cmd");
    }
    words.join("+")
}

/// The symbols for modifier words: "Option+Shift" is "⌥⇧".
pub fn modifier_symbols(words: &str) -> String {
    let m = isle_keys::chord::Mods::parse(words).unwrap_or_default();
    [(m.ctrl, '⌃'), (m.option, '⌥'), (m.shift, '⇧'), (m.cmd, '⌘')]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, c)| *c)
        .collect()
}

#[derive(PartialEq)]
struct HotCorners {
    at: Location,
}

impl Component for HotCorners {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let mut held = use_state(Modifiers::empty);
        let corner = |(name, key, mods_key): (&'static str, &'static str, &'static str)| {
            let current = r.str(key).unwrap_or_else(|| "none".into());
            let mods = r.str(mods_key).unwrap_or_default();
            let selected = ACTIONS
                .iter()
                .position(|(v, _)| *v == current)
                .unwrap_or(ACTIONS.len() - 1);
            let labels: Vec<String> = ACTIONS
                .iter()
                .enumerate()
                .map(|(i, (_, label))| {
                    let symbols = modifier_symbols(&mods);
                    if i == selected && !symbols.is_empty() && current != "none" {
                        format!("{symbols} {label}")
                    } else {
                        (*label).to_owned()
                    }
                })
                .collect();
            let desktop = r.desktop;
            form::PopUp::new(labels, selected, name, move |i: usize| {
                desktop.set(key, Value::Str(ACTIONS[i].0.to_owned()));
                let words = if ACTIONS[i].0 == "none" {
                    String::new()
                } else {
                    modifier_words(*held.peek())
                };
                desktop.set(mods_key, Value::Str(words));
            })
            .into_element()
        };
        let [tl, tr, bl, br] = CORNERS.map(corner);
        let at = self.at.clone();
        let screen = rect()
            .width(Size::px(240.))
            .height(Size::px(150.))
            .corner_radius(t.radius_card)
            .background(colour(t.colour_fill_strong));
        let side = |top: Element, bottom: Element, align: Alignment| {
            rect()
                .height(Size::px(150.))
                .cross_align(align)
                .main_align(Alignment::SpaceBetween)
                .child(top)
                .child(bottom)
                .into_element()
        };
        crate::ui::sheet(
            664.,
            vec![
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .spacing(t.space_m)
                    .on_global_key_down(move |e: Event<KeyboardEventData>| held.set(e.modifiers))
                    .on_global_key_up(move |e: Event<KeyboardEventData>| held.set(e.modifiers))
                    .child(side(tl, bl, Alignment::End))
                    .child(screen)
                    .child(side(tr, br, Alignment::Start))
                    .into_element(),
            ],
            vec![(
                "Done".into(),
                EventHandler::new(move |_| super::close(nav, &at)),
            )],
        )
    }
}

/// The actions Mission Control's Shortcuts… sheet sets, by shortcut id.
const MISSION_SHORTCUTS: [(&str, &str); 3] = [
    ("mission-control", "Mission Control"),
    ("app-expose", "Application windows"),
    ("show-desktop", "Show Desktop"),
];

/// The mouse buttons a shortcut may take, in the pop-up's order after "—".
const BUTTONS: [(&str, &str); 5] = [
    ("Back", "Back"),
    ("Forward", "Forward"),
    ("Button6", "Button 6"),
    ("Button7", "Button 7"),
    ("Button8", "Button 8"),
];

/// Mission Control's shortcuts: each action's keys as Keyboard Shortcuts sets them, and a
/// mouse button with any modifiers held while it is chosen.
#[derive(PartialEq)]
struct MissionShortcuts {
    at: Location,
}

impl Component for MissionShortcuts {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let mut held = use_state(Modifiers::empty);
        let rows: Vec<Element> = MISSION_SHORTCUTS
            .iter()
            .map(|(id, label)| {
                let key = format!("shortcuts.{id}");
                let chords = r.list(&key);
                let (buttons, keys): (Vec<String>, Vec<String>) = chords
                    .iter()
                    .cloned()
                    .partition(|c| isle_keys::chord::ButtonChord::parse(c).is_some());
                let current = buttons.first().and_then(|b| isle_keys::chord::ButtonChord::parse(b));
                let selected = current
                    .and_then(|c| isle_keys::chord::BUTTONS.iter().position(|(_, code)| *code == c.button))
                    .map_or(0, |i| i + 1);
                let symbols = current.map(|c| {
                    modifier_symbols(&[
                        (c.mods.ctrl, "Ctrl"),
                        (c.mods.option, "Option"),
                        (c.mods.shift, "Shift"),
                        (c.mods.cmd, "Cmd"),
                    ]
                    .iter()
                    .filter(|(on, _)| *on)
                    .map(|(_, w)| *w)
                    .collect::<Vec<_>>()
                    .join("+"))
                });
                let labels: Vec<String> = std::iter::once("—".to_owned())
                    .chain(BUTTONS.iter().enumerate().map(|(i, (_, l))| match &symbols {
                        Some(s) if i + 1 == selected && !s.is_empty() => format!("{s} {l}"),
                        _ => (*l).to_owned(),
                    }))
                    .collect();
                let desktop = r.desktop;
                let keys_shown = if keys.is_empty() { "None".to_owned() } else { keys.iter().map(|k| isle_keys::chord::shortcut_words(k)).collect::<Vec<_>>().join(", ") };
                let set_key = key.clone();
                let popup = form::PopUp::new(labels, selected, *label, move |i: usize| {
                    let mut list = keys.clone();
                    if i > 0 {
                        let mods = modifier_words(*held.peek());
                        let name = BUTTONS[i - 1].0;
                        list.push(if mods.is_empty() { name.to_owned() } else { format!("{mods}+{name}") });
                    }
                    desktop.set(set_key.clone(), Value::List(list));
                });
                r.row(id, *label, Some(&key))
                    .control(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_m)
                            .child(isle_ui::text::line(
                                &theme,
                                isle_ui::text::Style::Body,
                                colour(t.colour_text_2),
                                keys_shown,
                            ))
                            .child(popup),
                    )
                    .into_element()
            })
            .collect();
        let at = self.at.clone();
        crate::ui::sheet(
            560.,
            vec![
                isle_ui::text::line(
                    &theme,
                    isle_ui::text::Style::Callout,
                    colour(t.colour_text_2),
                    "A modifier held while choosing a mouse button is needed with it.",
                )
                .into_element(),
                rect()
                    .width(Size::fill())
                    .on_global_key_down(move |e: Event<KeyboardEventData>| held.set(e.modifiers))
                    .on_global_key_up(move |e: Event<KeyboardEventData>| held.set(e.modifiers))
                    .child(form::group(Some("Mission Control shortcuts".into()), rows))
                    .into_element(),
            ],
            vec![(
                "Done".into(),
                EventHandler::new(move |_| super::close(nav, &at)),
            )],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_read_in_mac_order() {
        assert_eq!(
            modifier_words(Modifiers::META | Modifiers::SHIFT),
            "Shift+Cmd"
        );
        assert_eq!(modifier_symbols("Option+Shift"), "⌥⇧");
        assert_eq!(modifier_symbols(""), "");
    }
}
