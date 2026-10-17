//! Sound: alert sounds, devices, volumes, balance and the input level.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::{Glyph, LevelBar, Slider, Switch, colour, form, text};

use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, Ranged, Rows},
};

/// The freedesktop theme's alert sounds and their names.
const ALERTS: [(&str, &str); 8] = [
    ("message-new-instant", "Message"),
    ("message", "Note"),
    ("bell", "Bell"),
    ("complete", "Complete"),
    ("dialog-information", "Information"),
    ("dialog-warning", "Warning"),
    ("window-attention", "Attention"),
    ("alarm-clock-elapsed", "Alarm"),
];

#[derive(PartialEq)]
pub struct Sound {
    pub at: Location,
}

/// A device list: each device by its label and kind, the default selected.
fn devices(list: &[isle_audio::Device], on_choose: impl Fn(String) + Clone + 'static) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let rows: Vec<Element> = list
        .iter()
        .map(|d| {
            let name = d.name.clone();
            let choose = on_choose.clone();
            let kind = if d.bluetooth { "Bluetooth" } else { "" };
            rect()
                .key(d.name.clone())
                .width(Size::fill())
                .min_height(Size::px(t.size_row))
                .padding((t.space_xs, t.space_m))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .maybe(d.default, |el| el.background(colour(t.colour_accent)))
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(d.label.clone())
                .on_press(move |_| choose(name.clone()))
                .child(rect().width(Size::flex(1.)).child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(if d.default {
                        t.colour_on_accent
                    } else {
                        t.colour_text
                    }),
                    d.label.clone(),
                )))
                .child(text::line(
                    &theme,
                    text::Style::Callout,
                    colour(if d.default {
                        t.colour_on_accent
                    } else {
                        t.colour_text_2
                    }),
                    kind,
                ))
                .into_element()
        })
        .collect();
    form::group(None, rows).into_element()
}

/// A volume slider between its quiet and loud glyphs.
fn volume(
    level: f32,
    glyphs: (&str, &str),
    alt: &str,
    on_change: impl Fn(f32) + 'static,
) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .child(Glyph::new(glyphs.0, 14., colour(t.colour_text_2)))
        .child(
            rect()
                .width(Size::px(200.))
                .child(Slider::new(level, alt, move |v: f32| on_change(v))),
        )
        .child(Glyph::new(glyphs.1, 14., colour(t.colour_text_2)))
        .into_element()
}

impl Component for Sound {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Audio]);
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let worker = isle_app::worker();
        let mut level = use_state(|| 0f32);
        let mut metering = use_state(|| None::<String>);
        let snapshot = services.audio.read().clone();
        let audio = snapshot.state;
        let input = audio.default_input().cloned();

        // The meter follows the default input while the pane is shown.
        let wanted = input.as_ref().map(|d| d.name.clone());
        if *metering.peek() != wanted {
            metering.set(wanted.clone());
            if let Some(source) = wanted {
                let (tx, mut rx) = tokio::sync::watch::channel(0f32);
                let mine = source.clone();
                let job = worker.spawn(async move {
                    let Ok(mut meter) = isle_audio::meter(&source).await else {
                        return;
                    };
                    while let Some(peak) = meter.peaks.recv().await {
                        if tx.send(peak).is_err() {
                            return;
                        }
                    }
                });
                spawn(async move {
                    let _job = AbortOnDrop(job);
                    while rx.changed().await.is_ok() {
                        if metering.peek().as_deref() != Some(mine.as_str()) {
                            return;
                        }
                        let peak = *rx.borrow_and_update();
                        level.set(peak);
                    }
                });
            }
        }

        let alert = r.str("notifications.alert-sound").unwrap_or_default();
        let mut labels: Vec<String> = ALERTS.iter().map(|(_, l)| (*l).to_owned()).collect();
        let mut names: Vec<String> = ALERTS.iter().map(|(n, _)| (*n).to_owned()).collect();
        let selected = match names.iter().position(|n| *n == alert) {
            Some(i) => i,
            None => {
                names.push(alert.clone());
                labels.push(alert.clone());
                names.len() - 1
            }
        };
        let effects = form::group(
            Some("Sound effects".into()),
            vec![
                r.row(
                    "alert-sound",
                    "Alert sound",
                    Some("notifications.alert-sound"),
                )
                .control(form::PopUp::new(
                    labels,
                    selected,
                    "Alert sound",
                    move |i: usize| {
                        desktop.set("notifications.alert-sound", Value::Str(names[i].clone()));
                        services.act(Act::PlaySound(names[i].clone()));
                    },
                ))
                .into_element(),
                r.switch(
                    "sounds",
                    "Play sound for notifications",
                    "notifications.sounds",
                )
                .into_element(),
                r.switch(
                    "capture-sound",
                    "Play sound when taking a screenshot",
                    "capture.sound",
                )
                .into_element(),
            ],
        );
        if !snapshot.available {
            return rect()
                .width(Size::fill())
                .spacing(20.)
                .child(effects)
                .child(ui::missing("The sound server isn't running."))
                .into_element();
        }

        let output = audio.default_output().cloned();
        let mut output_rows = Vec::new();
        if let Some(out) = output.clone() {
            let (o1, o2, o3) = (out.clone(), out.clone(), out.clone());
            output_rows.push(
                r.row("output-volume", "Output volume", None)
                    .control(volume(
                        out.volume,
                        ("speaker-none", "speaker-high"),
                        "Output volume",
                        move |v| {
                            services.act(Act::Volume {
                                output: true,
                                device: o1.clone(),
                                value: v,
                            })
                        },
                    ))
                    .into_element(),
            );
            output_rows.push(
                r.row("mute", "Mute", None)
                    .control(Switch::new(out.muted, "Mute", move |on: bool| {
                        services.act(Act::Mute {
                            output: true,
                            device: o2.name.clone(),
                            on,
                        })
                    }))
                    .into_element(),
            );
            output_rows.push(
                r.row("balance", "Balance", None)
                    .control(Ranged {
                        value: (o3.balance + 1.) / 2.,
                        alt: "Balance".into(),
                        words: ("Left".into(), "Right".into()),
                        ticks: 0,
                        on_change: EventHandler::new(move |v: f32| {
                            let v = v * 2. - 1.;
                            let v = if v.abs() < 0.06 { 0. } else { v };
                            services.act(Act::Balance {
                                device: o3.clone(),
                                value: v,
                            })
                        }),
                    })
                    .into_element(),
            );
        }
        let mut input_rows = Vec::new();
        if let Some(inp) = input.clone() {
            input_rows.push(
                r.row("input-volume", "Input volume", None)
                    .control(volume(
                        inp.volume,
                        ("microphone-slash", "microphone"),
                        "Input volume",
                        move |v| {
                            services.act(Act::Volume {
                                output: false,
                                device: inp.clone(),
                                value: v,
                            })
                        },
                    ))
                    .into_element(),
            );
            let theme = isle_ui::theme();
            input_rows.push(
                r.row("input-level", "Input level", None)
                    .control(rect().width(Size::px(200.)).child(LevelBar {
                        level: *level.read(),
                        track: colour(theme.tokens.colour_fill_strong),
                        fill: colour(theme.tokens.colour_accent),
                    }))
                    .into_element(),
            );
        }
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(effects)
            .child(ui::heading("Output"))
            .child(devices(&audio.outputs, move |name| {
                services.act(Act::SetDefaultOutput(name))
            }))
            .maybe(!output_rows.is_empty(), |el| {
                el.child(form::group(None, output_rows))
            })
            .child(ui::heading("Input"))
            .child(devices(&audio.inputs, move |name| {
                services.act(Act::SetDefaultInput(name))
            }))
            .maybe(!input_rows.is_empty(), |el| {
                el.child(form::group(None, input_rows))
            })
            .into_element()
    }
}

/// Stops the worker's job when the UI's side is dropped.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
