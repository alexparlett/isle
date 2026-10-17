//! Mouse: each pointing device's speed, acceleration, scrolling and primary button.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

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
        let services = use_follow(&[Follow::Pointers]);
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let devices = services.pointers.read().state.clone();
        if devices.is_empty() {
            return ui::missing("No mouse is connected.");
        }
        let single = devices.len() == 1;
        let groups: Vec<Element> = devices
            .iter()
            .map(|name| {
                let field = |f: &str| format!("pointer[{name}].{f}");
                let accel_key = field("accel-profile");
                let accel = r.str(&accel_key).as_deref() == Some("adaptive");
                let left_key = field("left-handed");
                let left = r.bool_or(&left_key, false);
                form::group(
                    (!single).then(|| name.clone()),
                    vec![
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
                    ],
                )
                .into_element()
            })
            .collect();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .children(groups)
            .into_element()
    }
}
