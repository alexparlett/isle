//! Accessibility: contrast, motion, transparency, text size, Sticky and Slow Keys.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

use crate::{nav::Location, ui::Rows};

/// Text size's steps, 100% to 150%.
const TEXT_SCALES: &[i64] = &[100, 110, 120, 130, 140, 150];

#[derive(PartialEq)]
pub struct Accessibility {
    pub at: Location,
}

impl Component for Accessibility {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let slow = r.int("keyboard.slow-keys").unwrap_or(0);
        let desktop = r.desktop;
        let scale = (r.float("appearance.text-scale").unwrap_or(1.) * 100.).round() as i64;
        let text_size = r
            .row("text-size", "Text size", Some("appearance.text-scale"))
            .control(crate::ui::Ranged {
                value: crate::ui::step_position(TEXT_SCALES, scale),
                alt: "Text size".into(),
                words: ("Default".into(), "Largest".into()),
                ticks: TEXT_SCALES.len(),
                on_change: EventHandler::new(move |v: f32| {
                    let step = TEXT_SCALES[crate::ui::step_index(TEXT_SCALES.len(), v)];
                    desktop.set("appearance.text-scale", Value::Float(step as f64 / 100.))
                }),
            });
        let mut keyboard = vec![
            r.switch("sticky-keys", "Sticky Keys", "keyboard.sticky-keys")
                .description("Modifier keys stay down until the next key.")
                .into_element(),
            r.switch_with("slow-keys", "Slow Keys", slow > 0, move |on| {
                desktop.set("keyboard.slow-keys", Value::Int(if on { 500 } else { 0 }))
            })
            .description("A key counts only once held.")
            .into_element(),
        ];
        if slow > 0 {
            keyboard.push(
                r.choice_int(
                    "acceptance-delay",
                    "Acceptance delay",
                    "keyboard.slow-keys",
                    &[
                        ("250", "0.25 s"),
                        ("500", "0.5 s"),
                        ("750", "0.75 s"),
                        ("1000", "1 s"),
                        ("1500", "1.5 s"),
                        ("2000", "2 s"),
                    ],
                )
                .into_element(),
            );
        }
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(
                Some("Display".into()),
                vec![
                    r.switch(
                        "increase-contrast",
                        "Increase contrast",
                        "appearance.increase-contrast",
                    )
                    .into_element(),
                    r.switch("reduce-motion", "Reduce motion", "appearance.reduce-motion")
                        .into_element(),
                    r.switch(
                        "reduce-transparency",
                        "Reduce transparency",
                        "appearance.reduce-transparency",
                    )
                    .into_element(),
                    text_size.into_element(),
                ],
            ))
            .child(form::group(Some("Keyboard".into()), keyboard))
    }
}
