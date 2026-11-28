//! Mouse: each pointing device's speed, acceleration, scrolling and primary button, and what
//! a Keychron mouse is set to itself.

use freya::prelude::*;
use isle_keychron::{Peripheral, protocol::Kind};
use isle_settings::Value;
use isle_ui::{Glyph, colour, form, text};

use crate::{
    nav::Location,
    services::{Follow, use_follow},
    ui::{self, Rows},
};

#[derive(PartialEq)]
pub struct Mouse {
    pub at: Location,
}

impl Component for Mouse {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Pointers, Follow::Keychron]);
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let devices = services.pointers.read().state.clone();
        if devices.is_empty() {
            return ui::missing("No mouse is connected.");
        }
        let single = devices.len() == 1;
        let keychron = services.keychron.read().state.clone();
        let groups: Vec<Element> = devices
            .iter()
            .map(|name| {
                let made = keychron.iter().find(|p| p.kind == Kind::Mouse && p.name == *name);
                let field = |f: &str| format!("pointer[{name}].{f}");
                let accel_key = field("accel-profile");
                let accel = r.str(&accel_key).as_deref() == Some("adaptive");
                let left_key = field("left-handed");
                let left = r.bool_or(&left_key, false);
                let rows = vec![
                        r.slider_key(
                            "speed",
                            "Tracking speed",
                            field("speed"),
                            (-1., 1.),
                            ("Slow", "Fast"),
                            0.,
                        )
                        .into_element(),
                        r.switch_with("acceleration", "Pointer acceleration", accel, move |on| {
                            desktop.set(
                                accel_key.clone(),
                                Value::Str(if on { "adaptive" } else { "flat" }.into()),
                            )
                        })
                        .into_element(),
                        r.switch_key(
                            "natural-scroll",
                            "Natural scrolling",
                            field("natural-scroll"),
                            false,
                            true,
                        )
                        .description("Content moves with your fingers.")
                        .into_element(),
                        r.row("primary-button", "Primary mouse button", Some(&left_key))
                            .control(form::Segmented::new(
                                vec!["Left".into(), "Right".into()],
                                usize::from(left),
                                "Primary mouse button",
                                move |i: usize| desktop.set(left_key.clone(), Value::Bool(i == 1)),
                            ))
                            .into_element(),
                ];
                let settings = match made.and_then(battery_title) {
                    Some(trailing) => form::group_with(name.clone(), trailing, rows).into_element(),
                    None => form::group((!single).then(|| name.clone()), rows).into_element(),
                };
                match made.and_then(|m| sensor(&r, m)) {
                    Some(sensor) => rect()
                        .width(Size::fill())
                        .spacing(12.)
                        .child(settings)
                        .child(sensor)
                        .into_element(),
                    None => settings,
                }
            })
            .collect();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .children(groups)
            .into_element()
    }
}

/// The mouse's battery beside its name, or that it is not connected.
fn battery_title(mouse: &Peripheral) -> Option<Element> {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let (glyph, words) = match (mouse.connected, mouse.battery) {
        (true, Some(level)) => ui::battery(level, mouse.charging),
        (false, Some(_)) => ("battery-empty", "Not connected".to_owned()),
        _ => return None,
    };
    Some(
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .child(Glyph::new(glyph, 16., colour(t.colour_text_2)))
            .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), words))
            .into_element(),
    )
}

/// What the mouse is set to itself, read from its receiver: shown, not set here.
fn sensor(r: &Rows, mouse: &Peripheral) -> Option<Element> {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let stage = mouse.stage? as usize;
    if mouse.dpi.is_empty() || stage >= mouse.dpi.len() {
        return None;
    }
    let ink = colour(if mouse.connected { t.colour_text_2 } else { t.colour_text_disabled });
    let mut rows = vec![
        r.row("dpi", "DPI", None)
            .description("Change it with the DPI button on the mouse.")
            .control(
                form::Segmented::new(
                    mouse.dpi.iter().map(|d| d.to_string()).collect(),
                    stage,
                    "DPI",
                    |_: usize| {},
                )
                .enabled(mouse.connected)
                .read_only(),
            )
            .into_element(),
    ];
    if let Some(hz) = mouse.rate_hz {
        rows.push(
            r.row("report-rate", "Report rate", None)
                .description("How often the mouse reports its movement.")
                .control(text::line(&theme, text::Style::Body, ink, format!("{hz} Hz")))
                .into_element(),
        );
    }
    if !mouse.connected {
        let when = mouse.seen.map(|s| format!(", at {}", ui::clock(s))).unwrap_or_default();
        rows.push(
            rect()
                .width(Size::fill())
                .padding((t.space_s, t.space_m))
                .child(text::line(
                    &theme,
                    text::Style::Callout,
                    colour(t.colour_text_2),
                    format!("As the mouse last reported{when}. It is asleep or out of range."),
                ))
                .into_element(),
        );
    }
    Some(form::group(None, rows).into_element())
}
