//! Desktop: how windows tile and fill, and the Hot Corners… sheet.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{colour, form, use_theme};

use super::Nav;
use crate::{nav::Location, ui::Rows};

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
const ACTIONS: [(&str, &str); 5] = [
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
                    r.switch(
                        "tile-margins",
                        "Tiled windows have margins",
                        "windows.tile-margins",
                    )
                    .into_element(),
                ],
            ))
            .child(crate::ui::foot(vec![(
                "Hot Corners…".into(),
                EventHandler::new({
                    let at = at.clone();
                    move |_| super::open(nav, &at, "Hot Corners…")
                }),
            )]))
            .maybe(self.at.sub.as_deref() == Some("Hot Corners…"), |el| {
                el.child(HotCorners { at: at.clone() })
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
