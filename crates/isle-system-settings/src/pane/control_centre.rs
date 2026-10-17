//! Control Centre: the island's clock and glyphs, the modules and the tray.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

use super::Nav;
use crate::{
    nav::Location,
    services::{Follow, use_follow},
    ui::{self, Rows},
};

const TIERS: &[(&str, &str)] = &[
    ("always", "Always show"),
    ("when-active", "Show when active"),
    ("never", "Never show"),
];

/// The island's glyphs: their key and name.
const GLYPHS: [(&str, &str); 6] = [
    ("island-glyphs.wifi", "Wi-Fi"),
    ("island-glyphs.bluetooth", "Bluetooth"),
    ("island-glyphs.sound", "Sound"),
    ("island-glyphs.vpn", "VPN"),
    ("island-glyphs.focus", "Focus"),
    ("island-glyphs.updates", "Updates"),
];

fn size_name(letter: &str) -> &'static str {
    match letter {
        "s" => "Small",
        "m" => "Medium",
        "l" => "Large",
        _ => "Wide",
    }
}

/// `modules` with `id` shown at `size`, or left out for none; a newly shown one goes last.
pub fn with_module(modules: &[String], id: &str, size: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut placed = false;
    for entry in modules {
        if entry.split(':').next() == Some(id) {
            if let Some(size) = size {
                out.push(format!("{id}:{size}"));
            }
            placed = true;
        } else {
            out.push(entry.clone());
        }
    }
    if !placed && let Some(size) = size {
        out.push(format!("{id}:{size}"));
    }
    out
}

#[derive(PartialEq)]
pub struct ControlCentre {
    pub at: Location,
}

impl Component for ControlCentre {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Modules, Follow::Tray, Follow::Power]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let at = self.at.clone();

        let mut island = vec![
            r.row("clock", "Clock", None)
                .control(isle_ui::Button::new("Clock Options…").on_press({
                    let at = at.clone();
                    move |_| super::open(nav, &at, "Clock Options…")
                }))
                .into_element(),
        ];
        for (key, name) in GLYPHS {
            island.push(r.choice(key, name, key, TIERS).into_element());
        }
        if services.power.read().state.system().is_some() {
            island.push(
                r.choice(
                    "island-glyphs.battery",
                    "Battery",
                    "island-glyphs.battery",
                    TIERS,
                )
                .into_element(),
            );
        }

        let shown = r.list("control-centre.modules");
        let modules: Vec<Element> = services
            .modules
            .read()
            .state
            .iter()
            .map(|m| {
                let current = shown
                    .iter()
                    .find(|e| e.split(':').next() == Some(m.id.as_str()))
                    .and_then(|e| e.split(':').nth(1))
                    .map(str::to_owned);
                let mut labels = vec!["Don't show".to_owned()];
                labels.extend(m.sizes.iter().map(|s| size_name(s).to_owned()));
                let selected = current
                    .as_deref()
                    .and_then(|c| m.sizes.iter().position(|s| s == c))
                    .map_or(0, |i| i + 1);
                let (id, sizes, shown) = (m.id.clone(), m.sizes.clone(), shown.clone());
                r.row(&m.id, m.name.clone(), Some("control-centre.modules"))
                    .leading(isle_ui::Glyph::new(
                        m.glyph.clone(),
                        18.,
                        isle_ui::colour(isle_ui::theme().tokens.colour_text_2),
                    ))
                    .control(form::PopUp::new(
                        labels,
                        selected,
                        m.name.clone(),
                        move |i: usize| {
                            let size = i.checked_sub(1).map(|i| sizes[i].as_str());
                            desktop.set(
                                "control-centre.modules",
                                Value::List(with_module(&shown, &id, size)),
                            );
                        },
                    ))
                    .into_element()
            })
            .collect();

        let hidden = r.list("control-centre.tray-hidden");
        let mut tray: Vec<(String, String)> = services.tray.read().state.clone();
        for id in &hidden {
            if !tray.iter().any(|(t, _)| t == id) {
                tray.push((id.clone(), id.clone()));
            }
        }
        let tray_rows: Vec<Element> = tray
            .into_iter()
            .map(|(id, title)| {
                let on = !hidden.contains(&id);
                let hidden = hidden.clone();
                let name = if title.is_empty() { id.clone() } else { title };
                r.switch_with(&format!("tray:{id}"), &name, on, move |on| {
                    let mut next: Vec<String> =
                        hidden.iter().filter(|h| **h != id).cloned().collect();
                    if !on {
                        next.push(id.clone());
                    }
                    desktop.set("control-centre.tray-hidden", Value::List(next));
                })
                .into_element()
            })
            .collect();

        let sheet = self.at.sub.clone();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(Some("Island".into()), island))
            .child(form::group(Some("Control Centre".into()), modules))
            .child(ui::foot(vec![(
                "Edit controls…".into(),
                EventHandler::new(move |_| services.act(crate::services::Act::EditControls)),
            )]))
            .maybe(!tray_rows.is_empty(), |el| {
                el.child(form::group(Some("Allow in the tray".into()), tray_rows))
            })
            .child(ui::foot(vec![(
                "Reset Control Centre…".into(),
                EventHandler::new({
                    let at = at.clone();
                    move |_| super::open(nav, &at, "Reset Control Centre…")
                }),
            )]))
            .maybe(sheet.as_deref() == Some("Clock Options…"), |el| {
                el.child(ClockOptions { at: at.clone() })
            })
            .maybe(sheet.as_deref() == Some("Reset Control Centre…"), |el| {
                el.child(ui::sheet(
                    360.,
                    vec![ui::alert_text(
                        "Reset Control Centre?",
                        "The island and Control Centre go back to how they first were.",
                    )],
                    vec![
                        (
                            "Cancel".into(),
                            EventHandler::new({
                                let at = at.clone();
                                move |_| super::close(nav, &at)
                            }),
                        ),
                        (
                            "Reset".into(),
                            EventHandler::new({
                                let at = at.clone();
                                move |_| {
                                    for section in ["island", "island-glyphs", "control-centre"] {
                                        desktop.reset(section);
                                    }
                                    super::close(nav, &at)
                                }
                            }),
                        ),
                    ],
                ))
            })
    }
}

#[derive(PartialEq)]
struct ClockOptions {
    at: Location,
}

impl Component for ClockOptions {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let at = self.at.clone();
        ui::sheet(
            440.,
            vec![
                form::group(
                    None,
                    vec![
                        r.segmented(
                            "clock-24-hour",
                            "Time format",
                            "island.clock-24-hour",
                            &[("auto", "Automatic"), ("on", "24-hour"), ("off", "12-hour")],
                        )
                        .into_element(),
                        r.switch(
                            "clock-weekday",
                            "Show the day of the week",
                            "island.clock-weekday",
                        )
                        .into_element(),
                        r.switch("clock-date", "Show date", "island.clock-date")
                            .into_element(),
                        r.switch(
                            "clock-seconds",
                            "Display the time with seconds",
                            "island.clock-seconds",
                        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_is_resized_in_place_left_out_or_added_last() {
        let list: Vec<String> = ["wifi:m", "sound:w"].map(String::from).to_vec();
        assert_eq!(with_module(&list, "wifi", Some("l")), ["wifi:l", "sound:w"]);
        assert_eq!(with_module(&list, "wifi", None), ["sound:w"]);
        assert_eq!(
            with_module(&list, "dark-mode", Some("s")),
            ["wifi:m", "sound:w", "dark-mode:s"]
        );
    }
}
