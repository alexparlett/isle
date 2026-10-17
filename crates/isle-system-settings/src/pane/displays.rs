//! Displays: the arrangement, each display's rows, keep-or-revert, Night Light and Advanced.

use std::collections::BTreeMap;

use freya::prelude::*;
use isle_ipc::DisplayInfo;
use isle_settings::Value;
use isle_ui::{colour, form, text, use_theme};

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, Rows},
};

/// A display's key in the settings: make, model and serial.
pub fn identity(d: &DisplayInfo) -> String {
    format!("{} {} {}", d.make, d.model, d.serial)
}

/// A display's name for people: its model, else its connector.
pub fn label(d: &DisplayInfo) -> String {
    if d.model.is_empty() {
        d.name.clone()
    } else {
        d.model.clone()
    }
}

/// Its size in the layout, in points: the mode over the scale, turned with the rotation.
pub fn logical_size(d: &DisplayInfo) -> (f64, f64) {
    let (w, h) = (
        f64::from(d.width) / d.scale.max(0.1),
        f64::from(d.height) / d.scale.max(0.1),
    );
    if matches!(d.transform.as_str(), "90" | "270") {
        (h, w)
    } else {
        (w, h)
    }
}

/// The sizes a display offers, largest first, once each.
pub fn sizes(d: &DisplayInfo) -> Vec<(i32, i32)> {
    let mut sizes: Vec<(i32, i32)> = d.modes.iter().map(|m| (m.width, m.height)).collect();
    sizes.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)).then(b.0.cmp(&a.0)));
    sizes.dedup();
    sizes
}

/// The rates at a size, fastest first, in whole hertz with the millihertz behind them.
pub fn rates(d: &DisplayInfo, size: (i32, i32)) -> Vec<(i32, i32)> {
    let mut rates: Vec<(i32, i32)> = d
        .modes
        .iter()
        .filter(|m| (m.width, m.height) == size)
        .map(|m| ((m.refresh + 500) / 1000, m.refresh))
        .collect();
    rates.sort_by(|a, b| b.1.cmp(&a.1));
    rates.dedup_by_key(|r| r.0);
    rates
}

/// Where a display dropped at `at` lands: against the nearest edge of another, its top or
/// left lined up when close.
pub fn snap(at: (f64, f64), size: (f64, f64), others: &[((f64, f64), (f64, f64))]) -> (f64, f64) {
    let centre = (at.0 + size.0 / 2., at.1 + size.1 / 2.);
    let mut best: Option<((f64, f64), f64)> = None;
    for &((ox, oy), (ow, oh)) in others {
        let oc = (ox + ow / 2., oy + oh / 2.);
        let (dx, dy) = (centre.0 - oc.0, centre.1 - oc.1);
        let candidate = if dx.abs() / ow.max(1.) >= dy.abs() / oh.max(1.) {
            let x = if dx > 0. { ox + ow } else { ox - size.0 };
            let y = if (at.1 - oy).abs() < 40. {
                oy
            } else {
                at.1.clamp(oy - size.1 + 1., oy + oh - 1.)
            };
            (x, y)
        } else {
            let y = if dy > 0. { oy + oh } else { oy - size.1 };
            let x = if (at.0 - ox).abs() < 40. {
                ox
            } else {
                at.0.clamp(ox - size.0 + 1., ox + ow - 1.)
            };
            (x, y)
        };
        let distance = (candidate.0 - at.0).hypot(candidate.1 - at.1);
        if best.is_none_or(|(_, d)| distance < d) {
            best = Some((candidate, distance));
        }
    }
    best.map_or(at, |(p, _)| p)
}

fn configure(services: crate::services::Services, id: &str, edit: serde_json::Value) {
    let mut changes = BTreeMap::new();
    changes.insert(id.to_owned(), edit);
    services.act(Act::Configure(
        serde_json::to_string(&changes).unwrap_or_default(),
    ));
}

#[derive(PartialEq)]
pub struct Displays {
    pub at: Location,
}

impl Component for Displays {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Displays, Follow::NightLight]);
        let theme = use_theme();
        let t = &theme.tokens;
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let mut selected = use_state(|| None::<String>);
        let mut drag = use_state(|| None::<(String, (f64, f64), (f64, f64))>);
        let mut scale_other = use_state(|| false);
        let all = services.displays.read().state.clone();
        let lit: Vec<DisplayInfo> = all.iter().filter(|d| d.enabled).cloned().collect();
        let off: Vec<DisplayInfo> = all.iter().filter(|d| !d.enabled).cloned().collect();
        if all.is_empty() {
            return ui::missing("No displays.");
        }
        let chosen = selected
            .read()
            .clone()
            .and_then(|n| lit.iter().find(|d| d.name == n).cloned())
            .or_else(|| lit.first().cloned());

        // The arrangement, to scale in a box the form's width.
        let boxes: Vec<(DisplayInfo, (f64, f64), (f64, f64))> = lit
            .iter()
            .map(|d| (d.clone(), (f64::from(d.x), f64::from(d.y)), logical_size(d)))
            .collect();
        let min_x = boxes.iter().map(|b| b.1.0).fold(f64::MAX, f64::min);
        let min_y = boxes.iter().map(|b| b.1.1).fold(f64::MAX, f64::min);
        let max_x = boxes.iter().map(|b| b.1.0 + b.2.0).fold(f64::MIN, f64::max);
        let max_y = boxes.iter().map(|b| b.1.1 + b.2.1).fold(f64::MIN, f64::max);
        let (area_w, area_h) = (f64::from(t.size_form) - 48., 200.);
        let ratio = (area_w / (max_x - min_x).max(1.)).min(area_h / (max_y - min_y).max(1.)) * 0.8;
        let offset = (
            (area_w - (max_x - min_x) * ratio) / 2. - min_x * ratio,
            (area_h - (max_y - min_y) * ratio) / 2. - min_y * ratio,
        );
        let main = r.str("shell.main-screen");
        let island_on = |d: &DisplayInfo| match &main {
            Some(id) => *id == identity(d),
            None => lit.first().is_some_and(|f| f.name == d.name),
        };
        let dragging = drag.read().clone();
        let map = rect()
            .width(Size::fill())
            .height(Size::px(area_h as f32))
            .corner_radius(t.radius_card)
            .background(colour(t.colour_fill))
            .on_capture_global_pointer_move(move |e: Event<PointerEventData>| {
                let mut drag = drag;
                let held = drag.peek().clone();
                if let Some((name, start, _)) = held {
                    let p = e.global_location();
                    drag.set(Some((name, start, (p.x, p.y))));
                }
            })
            .on_global_pointer_up({
                let boxes = boxes.clone();
                move |_| {
                    let mut drag = drag;
                    let held = drag.peek().clone();
                    let Some((name, start, now)) = held else {
                        return;
                    };
                    drag.set(None);
                    let Some((d, at, size)) = boxes.iter().find(|b| b.0.name == name).cloned()
                    else {
                        return;
                    };
                    let moved = ((now.0 - start.0) / ratio, (now.1 - start.1) / ratio);
                    if moved.0.abs() < 2. && moved.1.abs() < 2. {
                        return;
                    }
                    let others: Vec<_> = boxes
                        .iter()
                        .filter(|b| b.0.name != name)
                        .map(|b| (b.1, b.2))
                        .collect();
                    let (x, y) = snap((at.0 + moved.0, at.1 + moved.1), size, &others);
                    let id = identity(&d);
                    desktop.set(format!("output[{id}].x"), Value::Int(x.round() as i64));
                    desktop.set(format!("output[{id}].y"), Value::Int(y.round() as i64));
                }
            })
            .children(boxes.iter().map(|(d, at, size)| {
                let shift = match &dragging {
                    Some((name, start, now)) if *name == d.name => {
                        (now.0 - start.0, now.1 - start.1)
                    }
                    _ => (0., 0.),
                };
                let left = at.0 * ratio + offset.0 + shift.0;
                let top = at.1 * ratio + offset.1 + shift.1;
                let on = chosen.as_ref().is_some_and(|c| c.name == d.name);
                let name = d.name.clone();
                rect()
                    .key(d.name.clone())
                    .position(Position::new_absolute().left(left as f32).top(top as f32))
                    .width(Size::px((size.0 * ratio) as f32))
                    .height(Size::px((size.1 * ratio) as f32))
                    .corner_radius(t.radius_control)
                    .background(colour(t.colour_window_raised))
                    .border(
                        Border::new()
                            .fill(colour(if on {
                                t.colour_accent
                            } else {
                                t.colour_hairline_strong
                            }))
                            .width(if on {
                                t.stroke_found
                            } else {
                                t.stroke_hairline
                            })
                            .alignment(BorderAlignment::Inner),
                    )
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(label(d))
                    .on_pointer_down(move |e: Event<PointerEventData>| {
                        let p = e.global_location();
                        selected.set(Some(name.clone()));
                        drag.set(Some((name.clone(), (p.x, p.y), (p.x, p.y))));
                    })
                    .cross_align(Alignment::Center)
                    .maybe(island_on(d), |el| {
                        el.child(
                            rect()
                                .margin((4., 0., 0., 0.))
                                .width(Size::px(28.))
                                .height(Size::px(6.))
                                .corner_radius(3.)
                                .background(colour(t.colour_text)),
                        )
                    })
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::fill())
                            .center()
                            .child(text::line(
                                &theme,
                                text::Style::Callout,
                                colour(t.colour_text),
                                label(d),
                            )),
                    )
                    .into_element()
            }));

        let mut main_options: Vec<String> = lit.iter().map(label).collect();
        main_options.push("Follow focus".into());
        let ids: Vec<String> = lit.iter().map(identity).collect();
        let main_index = main
            .as_ref()
            .and_then(|m| ids.iter().position(|i| i == m))
            .unwrap_or(ids.len());
        let main_row = r
            .row("main-display", "Main display", Some("shell.main-screen"))
            .control(form::PopUp::new(
                main_options,
                main_index,
                "Main display",
                move |i: usize| match ids.get(i) {
                    Some(id) => desktop.set("shell.main-screen", Value::Str(id.clone())),
                    None => desktop.reset("shell.main-screen"),
                },
            ));

        let mut body = rect()
            .width(Size::fill())
            .spacing(20.)
            .child(map)
            .child(form::group(None, vec![main_row.into_element()]));

        if let Some(d) = chosen.clone() {
            let id = identity(&d);
            let size = (d.width, d.height);
            let all_sizes = sizes(&d);
            let preferred = d.modes.first().map(|m| (m.width, m.height));
            let size_labels: Vec<String> = all_sizes
                .iter()
                .map(|s| {
                    let base = format!("{} × {}", s.0, s.1);
                    if Some(*s) == preferred {
                        format!("{base} (default)")
                    } else {
                        base
                    }
                })
                .collect();
            let size_index = all_sizes.iter().position(|s| *s == size).unwrap_or(0);
            let at_size = rates(&d, size);
            let rate_index = at_size
                .iter()
                .position(|r| (r.1 - d.refresh).abs() < 1000)
                .unwrap_or(0);
            let mode_of =
                |s: (i32, i32), mhz: i32| format!("{}x{}@{}", s.0, s.1, f64::from(mhz) / 1000.);
            let mut rows = Vec::new();
            {
                let id = id.clone();
                let last = lit.len() == 1;
                rows.push(
                    r.row("use", "Use this display", None)
                        .enabled(!last)
                        .control(
                            isle_ui::Switch::new(true, "Use this display", move |on: bool| {
                                configure(services, &id, serde_json::json!({ "enabled": on }))
                            })
                            .enabled(!last),
                        )
                        .into_element(),
                );
            }
            {
                let (id, all_sizes, d2) = (id.clone(), all_sizes.clone(), d.clone());
                rows.push(
                    r.row("resolution", "Resolution", None)
                        .control(form::PopUp::new(
                            size_labels,
                            size_index,
                            "Resolution",
                            move |i: usize| {
                                let s = all_sizes[i];
                                let fastest = rates(&d2, s).first().map_or(60000, |r| r.1);
                                configure(
                                    services,
                                    &id,
                                    serde_json::json!({ "mode": mode_of(s, fastest) }),
                                )
                            },
                        ))
                        .into_element(),
                );
            }
            {
                let (id, at_size2) = (id.clone(), at_size.clone());
                rows.push(
                    r.row("refresh-rate", "Refresh rate", None)
                        .control(form::PopUp::new(
                            at_size.iter().map(|r| format!("{} Hz", r.0)).collect(),
                            rate_index,
                            "Refresh rate",
                            move |i: usize| {
                                configure(
                                    services,
                                    &id,
                                    serde_json::json!({ "mode": mode_of(size, at_size2[i].1) }),
                                )
                            },
                        ))
                        .into_element(),
                );
            }
            {
                let scale_key = format!("output[{id}].scale");
                let steps: Vec<f64> = (0..5).map(|i| 1. + f64::from(i) * 0.25).collect();
                let mut labels: Vec<String> = steps
                    .iter()
                    .map(|s| {
                        let l = format!("{}%", (s * 100.).round());
                        if (*s - 1.).abs() < 0.01 {
                            format!("{l} (default)")
                        } else {
                            l
                        }
                    })
                    .collect();
                let mut index = steps.iter().position(|s| (s - d.scale).abs() < 0.01);
                if index.is_none() {
                    labels.push(format!("{}%", (d.scale * 100.).round()));
                    index = Some(labels.len() - 1);
                }
                labels.push("Other…".into());
                let count = steps.len();
                rows.push(
                    r.row("scale", "Scale", Some(&scale_key))
                        .control(form::PopUp::new(
                            labels,
                            index.unwrap_or(0),
                            "Scale",
                            move |i: usize| match steps.get(i) {
                                Some(s) => desktop.set(scale_key.clone(), Value::Float(*s)),
                                None if i >= count => scale_other.set(true),
                                None => {}
                            },
                        ))
                        .into_element(),
                );
            }
            {
                const ROTATIONS: [(&str, &str); 4] = [
                    ("normal", "Standard"),
                    ("90", "90°"),
                    ("180", "180°"),
                    ("270", "270°"),
                ];
                let id = id.clone();
                rows.push(
                    r.row("rotation", "Rotation", None)
                        .control(form::PopUp::new(
                            ROTATIONS.iter().map(|(_, l)| (*l).to_owned()).collect(),
                            ROTATIONS
                                .iter()
                                .position(|(v, _)| *v == d.transform)
                                .unwrap_or(0),
                            "Rotation",
                            move |i: usize| {
                                configure(
                                    services,
                                    &id,
                                    serde_json::json!({ "transform": ROTATIONS[i].0 }),
                                )
                            },
                        ))
                        .into_element(),
                );
            }
            {
                let name = d.name.clone();
                let settable = d.brightness >= 0.;
                rows.push(
                    r.row("brightness", "Brightness", None)
                        .refused(
                            (!settable)
                                .then(|| "Brightness can't be set on this display.".to_owned()),
                        )
                        .control(
                            rect().width(Size::px(200.)).child(
                                isle_ui::Slider::new(
                                    d.brightness.max(0.) as f32,
                                    "Brightness",
                                    move |v: f32| {
                                        services.act(Act::Brightness {
                                            display: name.clone(),
                                            level: f64::from(v),
                                        })
                                    },
                                )
                                .enabled(settable),
                            ),
                        )
                        .into_element(),
                );
            }
            if d.vrr_capability != "none" {
                let vrr_key = format!("output[{id}].vrr");
                const VRR: [(&str, &str); 3] = [
                    ("off", "Off"),
                    ("fullscreen", "Full screen"),
                    ("always", "Always"),
                ];
                let mut row = r
                    .row("vrr", "Variable refresh rate", Some(&vrr_key))
                    .control(form::Segmented::new(
                        VRR.iter().map(|(_, l)| (*l).to_owned()).collect(),
                        VRR.iter()
                            .position(|(v, _)| *v == d.vrr_setting)
                            .unwrap_or(0),
                        "Variable refresh rate",
                        move |i: usize| desktop.set(vrr_key.clone(), Value::Str(VRR[i].0.into())),
                    ));
                if d.min_hz > 0 && d.max_hz > 0 {
                    row = row.description(format!(
                        "This display supports {}–{} Hz.",
                        d.min_hz, d.max_hz
                    ));
                }
                rows.push(row.into_element());
            }
            rows.push(
                r.switch_key(
                    "tearing",
                    "Allow tearing in games",
                    format!("output[{id}].tearing"),
                    false,
                    true,
                )
                .description("A game that asks is shown without waiting for the display.")
                .into_element(),
            );
            body = body.child(form::group(Some(label(&d)), rows));
        }

        if !off.is_empty() {
            let rows: Vec<Element> = off
                .iter()
                .map(|d| {
                    let id = identity(d);
                    r.switch_with(&d.name, &label(d), false, move |on| {
                        configure(services, &id, serde_json::json!({ "enabled": on }))
                    })
                    .description("Off")
                    .into_element()
                })
                .collect();
            body = body.child(form::group(Some("Off".into()), rows));
        }

        let opener = |sub: &'static str| {
            let at = self.at.clone();
            EventHandler::new(move |_| super::open(nav, &at, sub))
        };
        body = body.child(ui::foot(vec![
            (
                "Identify".into(),
                EventHandler::new(move |_| services.act(Act::Identify)),
            ),
            (
                "Detect Displays".into(),
                EventHandler::new(move |_| services.act(Act::Redetect)),
            ),
            ("Advanced…".into(), opener("Advanced…")),
            ("Night Light…".into(), opener("Night Light…")),
        ]));

        let sheet = self.at.sub.clone();
        let change = *services.display_change.read();
        let at = self.at.clone();
        body.maybe(sheet.as_deref() == Some("Night Light…"), |el| {
            el.child(NightLight { at: at.clone() })
        })
        .maybe(sheet.as_deref() == Some("Advanced…"), |el| {
            el.child(Advanced { at: at.clone() })
        })
        .maybe(*scale_other.read(), |el| {
            let id = chosen.as_ref().map(identity).unwrap_or_default();
            el.child(ui::Ask {
                title: "Scale".into(),
                placeholder: "50% to 400%".into(),
                initial: String::new(),
                on_done: EventHandler::new(move |text: String| {
                    if let Ok(percent) = text.trim().trim_end_matches('%').parse::<f64>()
                        && (50. ..=400.).contains(&percent)
                    {
                        desktop.set(format!("output[{id}].scale"), Value::Float(percent / 100.));
                        scale_other.set(false);
                    }
                }),
                on_cancel: EventHandler::new(move |_| scale_other.set(false)),
            })
        })
        .map(change, |el, id| el.child(KeepOrRevert { id }))
        .into_element()
    }
}

/// "Keep these display settings?", counting down to the compositor's own revert.
#[derive(PartialEq)]
struct KeepOrRevert {
    id: u64,
}

impl Component for KeepOrRevert {
    fn render(&self) -> impl IntoElement {
        let services = crate::services::Services::get();
        let theme = use_theme();
        let total = i64::from(theme.tokens.timing_display_revert_ms.div_ceil(1000));
        let mut left = use_state(move || total);
        let id = self.id;
        let worker = isle_app::worker();
        use_hook(move || {
            spawn(async move {
                loop {
                    let _ = worker
                        .spawn(async {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await
                        })
                        .await;
                    let now = *left.peek() - 1;
                    left.set(now);
                    if now <= 0 {
                        let mut change = services.display_change;
                        if *change.peek() == Some(id) {
                            change.set(None);
                        }
                        return;
                    }
                }
            })
        });
        let done = move |keep: bool| {
            EventHandler::new(move |_| {
                services.act(if keep { Act::Keep(id) } else { Act::Revert(id) });
                let mut change = services.display_change;
                change.set(None);
            })
        };
        ui::sheet(
            320.,
            vec![ui::alert_text(
                "Keep these display settings?",
                &format!("Reverting in {} seconds.", (*left.read()).max(0)),
            )],
            vec![("Revert".into(), done(false)), ("Keep".into(), done(true))],
        )
    }
}

#[derive(PartialEq)]
struct NightLight {
    at: Location,
}

impl Component for NightLight {
    fn render(&self) -> impl IntoElement {
        let services = crate::services::Services::get();
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let status = services.night_light.read().state.clone();
        let schedule = r
            .str("night-light.schedule")
            .unwrap_or_else(|| "sunset-to-sunrise".into());
        let on = status["on"].as_bool().unwrap_or(false);
        let times = [use_state(String::new), use_state(String::new)];
        let until = match schedule.as_str() {
            "custom" => format!(
                "Turn on until {}",
                r.str("night-light.to").unwrap_or_default()
            ),
            "off" => "Turn on".to_owned(),
            _ => "Turn on until sunrise".to_owned(),
        };
        let mut rows = vec![
            r.choice(
                "schedule",
                "Schedule",
                "night-light.schedule",
                &[
                    ("off", "Off"),
                    ("sunset-to-sunrise", "Sunset to Sunrise"),
                    ("custom", "Custom"),
                ],
            )
            .into_element(),
        ];
        if schedule == "custom" {
            for ((key, name), value) in [("night-light.from", "From"), ("night-light.to", "To")]
                .into_iter()
                .zip(times)
            {
                let desktop = r.desktop;
                let current = r.str(key).unwrap_or_default();
                rows.push(
                    r.row(key, name, Some(key))
                        .control(
                            isle_ui::TextField::new(value, name)
                                .placeholder(current)
                                .width(Size::px(80.))
                                .on_submit(move |text: String| {
                                    desktop.set(key, Value::Str(text.trim().to_owned()))
                                }),
                        )
                        .into_element(),
                );
            }
        }
        rows.push(
            r.switch_with("turn-on", &until, on, move |_| {
                services.act(Act::ToggleNightLight);
                let mut light = services.night_light;
                let mut next = light.peek().clone();
                next.state["on"] = serde_json::Value::Bool(!on);
                light.set(next);
            })
            .into_element(),
        );
        // Warmth runs from less warm to more: 6000 K down to 2700 K.
        let warmth = r.int("night-light.warmth").unwrap_or(3400);
        let desktop = r.desktop;
        rows.push(
            r.row("warmth", "Warmth", Some("night-light.warmth"))
                .control(ui::Ranged {
                    value: ((6000 - warmth) as f32 / 3300.).clamp(0., 1.),
                    alt: "Warmth".into(),
                    words: ("Less warm".into(), "More warm".into()),
                    ticks: 0,
                    on_change: EventHandler::new(move |v: f32| {
                        let kelvin = 6000 - (v * 3300.).round() as i64;
                        desktop.set("night-light.warmth", Value::Int((kelvin / 100) * 100))
                    }),
                })
                .into_element(),
        );
        let at = self.at.clone();
        ui::sheet(
            520.,
            vec![form::group(None, rows).into_element()],
            vec![(
                "Done".into(),
                EventHandler::new(move |_| super::close(nav, &at)),
            )],
        )
    }
}

/// Each render node and the graphics card behind it.
fn render_nodes() -> Vec<(String, String)> {
    let ids = std::fs::read_to_string("/usr/share/hwdata/pci.ids").unwrap_or_default();
    let mut nodes: Vec<(String, String)> = std::fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("renderD"))
        .map(|e| {
            let node = format!("/dev/dri/{}", e.file_name().to_string_lossy());
            let uevent =
                std::fs::read_to_string(e.path().join("device/uevent")).unwrap_or_default();
            let name = uevent
                .lines()
                .find_map(|l| l.strip_prefix("PCI_ID="))
                .map(str::to_lowercase)
                .and_then(|id| {
                    let (v, d) = id.split_once(':')?;
                    super::general::pci_name(&ids, v, d)
                })
                .unwrap_or_else(|| node.clone());
            (node, name)
        })
        .collect();
    nodes.sort();
    nodes
}

#[derive(PartialEq)]
struct Advanced {
    at: Location,
}

impl Component for Advanced {
    fn render(&self) -> impl IntoElement {
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let desktop = r.desktop;
        let nodes = use_memo(render_nodes);
        let current = r.str("render-device");
        let list = nodes.read().clone();
        let mut labels = vec!["Automatic".to_owned()];
        labels.extend(list.iter().map(|(_, n)| n.clone()));
        let index = current
            .as_ref()
            .and_then(|c| list.iter().position(|(p, _)| p == c))
            .map_or(0, |i| i + 1);
        let at = self.at.clone();
        ui::sheet(
            480.,
            vec![
                form::group(
                    None,
                    vec![
                        r.row("graphics-card", "Graphics card", Some("render-device"))
                            .description("Takes effect the next time you log in.")
                            .control(form::PopUp::new(
                                labels,
                                index,
                                "Graphics card",
                                move |i: usize| match i {
                                    0 => desktop.reset("render-device"),
                                    i => desktop
                                        .set("render-device", Value::Str(list[i - 1].0.clone())),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn display(name: &str, modes: &[(i32, i32, i32)]) -> DisplayInfo {
        DisplayInfo {
            name: name.into(),
            make: "Dell".into(),
            model: "U2723".into(),
            serial: "1".into(),
            width: 2560,
            height: 1440,
            refresh: 60000,
            scale: 1.25,
            x: 0,
            y: 0,
            vrr_capability: "none".into(),
            vrr_setting: "off".into(),
            vrr_on: false,
            enabled: true,
            transform: "90".into(),
            modes: modes
                .iter()
                .map(|(w, h, r)| isle_ipc::DisplayMode {
                    width: *w,
                    height: *h,
                    refresh: *r,
                })
                .collect(),
            width_mm: 597,
            height_mm: 336,
            min_hz: 0,
            max_hz: 0,
            brightness: -1.,
        }
    }

    #[test]
    fn modes_group_by_size_and_rate() {
        let d = display(
            "DP-1",
            &[
                (2560, 1440, 59951),
                (2560, 1440, 143912),
                (1920, 1080, 60000),
                (2560, 1440, 59940),
            ],
        );
        assert_eq!(sizes(&d), [(2560, 1440), (1920, 1080)]);
        assert_eq!(rates(&d, (2560, 1440)), [(144, 143912), (60, 59951)]);
        assert_eq!(logical_size(&d), (1152., 2048.));
    }

    #[test]
    fn a_dropped_display_meets_its_neighbour() {
        let left = ((0., 0.), (1920., 1080.));
        assert_eq!(snap((2000., 30.), (1920., 1080.), &[left]), (1920., 0.));
        assert_eq!(
            snap((-1500., 500.), (1920., 1080.), &[left]),
            (-1920., 500.)
        );
        assert_eq!(snap((100., 1300.), (1920., 1080.), &[left]), (100., 1080.));
    }
}
