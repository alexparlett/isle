//! The footer under the table (`surfaces/monitor.md` § The footer): figures, one graph and
//! the resource's sensors; for GPU, a block per GPU.

use std::rc::Rc;

use freya::prelude::*;
use isle_sysinfo::{gpus::Backend, monitor::Full, sensors::Sensors};
use isle_tokens::Colour;
use isle_ui::{
    Glyph, colour,
    graph::{Graph, GraphData, Series, Shape, Unit},
    text,
};

use crate::{history::History, model::{Resource, bytes}};

/// A figure row: its label, its value, and the series it keys where it names one.
pub struct Figure {
    pub label: String,
    pub value: String,
    pub key: Option<Colour>,
}

fn figure(label: impl Into<String>, value: impl Into<String>) -> Figure {
    Figure { label: label.into(), value: value.into(), key: None }
}

fn keyed(label: impl Into<String>, value: impl Into<String>, key: Colour) -> Figure {
    Figure { label: label.into(), value: value.into(), key: Some(key) }
}

fn rate(v: Option<f32>, unit: Unit) -> String {
    v.map(|v| unit.format(v)).unwrap_or_default()
}

fn percent(v: f32) -> String {
    format!("{:.1}%", v * 100.0)
}

/// What the footer shows for a resource, for any width.
pub struct Shown {
    pub notes: Vec<String>,
    pub leading: Vec<Figure>,
    /// The leading group is a toggle choosing the graph (Disk and Network): selected or not.
    pub leading_choice: Option<bool>,
    pub trailing: Vec<Figure>,
    pub trailing_choice: Option<bool>,
    pub sensors: Vec<Figure>,
    pub title: String,
    pub data: GraphData,
    pub summary: String,
}

/// A summary of a series for its graph's accessible name: "CPU load, 12 to 46%, now 31%".
fn summary(title: &str, values: &[Option<f32>], unit: Unit) -> String {
    let seen: Vec<f32> = values.iter().flatten().copied().collect();
    match (seen.iter().copied().reduce(f32::min), seen.iter().copied().reduce(f32::max), seen.last()) {
        (Some(lo), Some(hi), Some(now)) => format!("{title}, {} to {}, now {}", unit.format(lo), unit.format(hi), unit.format(*now)),
        _ => title.to_owned(),
    }
}

pub fn content(resource: Resource, full: &Full, history: &History, theme: &isle_tokens::Theme, disk_data: bool, network_data: bool) -> Shown {
    let t = &theme.tokens;
    let (a, b, c) = (t.series_a, t.series_b, t.series_c);
    let latest = history.latest().cloned().unwrap_or_default();
    let mut notes = Vec::new();
    for gpu in full.gpus.iter().filter(|g| g.backend == Backend::None) {
        notes.push(format!("GPU figures aren't available for {}.", gpu.name));
    }
    let times: Vec<String> = history.times.iter().cloned().collect();
    let data = |series: Vec<Series>, shape, max, unit| GraphData { series, shape, max, unit, times: times.clone() };
    match resource {
        Resource::Cpu => {
            let user = history.series(|p| Some(p.cpu_user));
            let system = history.series(|p| Some(p.cpu_system));
            let threads: u32 = full.processes.iter().map(|p| p.threads).sum();
            let s = &full.sensors;
            let mut sensors_out = Vec::new();
            if let Some(temp) = Sensors::cpu_temperature(s) {
                sensors_out.push(figure("Package", format!("{temp:.0} °C")));
            }
            if let Some(fan) = full.cpu_fan {
                sensors_out.push(figure("Fan", format!("{fan:.0} rpm")));
            }
            let load: Vec<Option<f32>> = user.iter().zip(&system).map(|(u, s)| u.zip(*s).map(|(u, s)| u + s)).collect();
            Shown {
                notes,
                leading: vec![
                    keyed("System", percent(full.cpu.system), b),
                    keyed("User", percent(full.cpu.user), a),
                    figure("Idle", percent(full.cpu.idle)),
                ],
                leading_choice: None,
                trailing: vec![figure("Threads", threads.to_string()), figure("Processes", full.processes.len().to_string())],
                trailing_choice: None,
                sensors: sensors_out,
                title: "CPU load".into(),
                summary: summary("CPU load", &load, Unit::Share),
                data: data(vec![Series { values: user, colour: a }, Series { values: system, colour: b }], Shape::Stacked, Some(1.0), Unit::Share),
            }
        }
        Resource::Memory => {
            let m = &full.memory;
            let pressure = history.series(|p| Some(p.pressure));
            Shown {
                notes,
                leading: vec![
                    figure("Physical memory", bytes(m.physical)),
                    figure("Memory used", bytes(m.used)),
                    figure("Cached files", bytes(m.cached)),
                ],
                leading_choice: None,
                trailing: vec![figure("Swap used", bytes(m.swap_used)), figure("Compressed", bytes(m.compressed))],
                trailing_choice: None,
                sensors: Vec::new(),
                title: "Memory pressure".into(),
                summary: summary("Memory pressure", &pressure, Unit::Share),
                data: data(vec![Series { values: pressure, colour: a }], Shape::Lines, Some(1.0), Unit::Share),
            }
        }
        Resource::Energy => {
            let colours = [b, c, t.series_d];
            let mut series = vec![Series { values: history.series(|p| p.cpu_power), colour: a }];
            let mut trailing = Vec::new();
            if let Some(w) = latest.cpu_power {
                trailing.push(keyed("CPU package", Unit::Watts.format(w), a));
            }
            for (i, gpu) in full.gpus.iter().enumerate().filter(|(_, g)| g.power.is_some()) {
                let colour = colours[i % colours.len()];
                series.push(Series { values: history.series(|p| p.gpu_power.get(i).copied().flatten()), colour });
                trailing.push(keyed(gpu.name.clone(), Unit::Watts.format(gpu.power.unwrap_or(0.0)), colour));
            }
            let total = latest.cpu_power.unwrap_or(0.0) + latest.gpu_power.iter().flatten().sum::<f32>();
            let totals: Vec<Option<f32>> = (0..history.points.len())
                .map(|i| history.points[i].as_ref().map(|p| p.cpu_power.unwrap_or(0.0) + p.gpu_power.iter().flatten().sum::<f32>()))
                .collect();
            Shown {
                notes,
                leading: vec![figure("Power", Unit::Watts.format(total))],
                leading_choice: None,
                trailing,
                trailing_choice: None,
                sensors: Vec::new(),
                title: "Power".into(),
                summary: summary("Power", &totals, Unit::Watts),
                data: data(series, Shape::Stacked, None, Unit::Watts),
            }
        }
        Resource::Disk => {
            let ops = !disk_data;
            let (up, down, unit) = if ops {
                (history.series(|p| Some(p.reads)), history.series(|p| Some(p.writes)), Unit::PerSecond)
            } else {
                (history.series(|p| Some(p.read_bytes)), history.series(|p| Some(p.written_bytes)), Unit::BytesPerSecond)
            };
            let sensors_out = full
                .disks
                .iter()
                .filter_map(|d| Some(figure(if d.model.is_empty() { d.name.clone() } else { d.model.clone() }, format!("{:.0} °C", d.temperature?))))
                .collect();
            Shown {
                notes,
                leading: vec![
                    keyed("Reads in/s", format!("{:.0}", latest.reads), a),
                    keyed("Writes out/s", format!("{:.0}", latest.writes), b),
                ],
                leading_choice: Some(ops),
                trailing: vec![
                    keyed("Data read/s", Unit::BytesPerSecond.format(latest.read_bytes), a),
                    keyed("Data written/s", Unit::BytesPerSecond.format(latest.written_bytes), b),
                ],
                trailing_choice: Some(!ops),
                sensors: sensors_out,
                title: String::new(),
                summary: summary("Disk", &up, unit),
                data: data(vec![Series { values: up, colour: a }, Series { values: down, colour: b }], Shape::Mirrored, None, unit),
            }
        }
        Resource::Network => {
            notes.push("UDP traffic isn't counted per process.".into());
            let packets = !network_data;
            let (up, down, unit) = if packets {
                (history.series(|p| p.packets_in), history.series(|p| p.packets_out), Unit::PerSecond)
            } else {
                (history.series(|p| p.received), history.series(|p| p.sent), Unit::BytesPerSecond)
            };
            Shown {
                notes,
                leading: vec![
                    keyed("Packets in/s", rate(latest.packets_in, Unit::PerSecond).replace("/s", ""), a),
                    keyed("Packets out/s", rate(latest.packets_out, Unit::PerSecond).replace("/s", ""), b),
                ],
                leading_choice: Some(packets),
                trailing: vec![
                    keyed("Data received/s", rate(latest.received, Unit::BytesPerSecond), a),
                    keyed("Data sent/s", rate(latest.sent, Unit::BytesPerSecond), b),
                ],
                trailing_choice: Some(!packets),
                sensors: Vec::new(),
                title: String::new(),
                summary: summary("Network", &up, unit),
                data: data(vec![Series { values: up, colour: a }, Series { values: down, colour: b }], Shape::Mirrored, None, unit),
            }
        }
        Resource::Gpu => Shown {
            notes,
            leading: Vec::new(),
            leading_choice: None,
            trailing: Vec::new(),
            trailing_choice: None,
            sensors: Vec::new(),
            title: String::new(),
            summary: String::new(),
            data: data(Vec::new(), Shape::Lines, Some(1.0), Unit::Share),
        },
    }
}

/// A GPU's block: its figures and its utilisation graph.
pub fn gpu_figures(gpu: &isle_sysinfo::gpus::Gpu) -> Vec<Figure> {
    let mut out = Vec::new();
    if let Some((used, total)) = gpu.memory {
        out.push(figure("Memory", format!("{} of {}", bytes(used), bytes(total))));
    }
    if let Some(t) = gpu.temperature {
        out.push(figure("Temperature", format!("{t:.0} °C")));
    }
    if let Some(w) = gpu.power {
        out.push(figure("Power", Unit::Watts.format(w)));
    }
    if let Some(f) = gpu.fan {
        out.push(figure("Fan", format!("{f} rpm")));
    }
    if let Some(c) = gpu.clock {
        out.push(figure("Clock", format!("{c} MHz")));
    }
    out
}

/// A column of figure rows, `size-monitor-figures` wide; pressed, a choosing group toggles.
pub fn figures(theme: &isle_tokens::Theme, rows: &[Figure], choice: Option<bool>, on_choose: Option<EventHandler<()>>) -> Element {
    let t = &theme.tokens;
    let chosen = choice == Some(true);
    let label_ink = if chosen { t.colour_text } else { t.colour_text_2 };
    rect()
        .width(Size::px(t.size_monitor_figures))
        .spacing(0.)
        .maybe(choice.is_some(), |el| {
            el.a11y_role(AccessibilityRole::Button)
                .a11y_builder(move |node| node.set_toggled(accesskit::Toggled::from(chosen)))
        })
        .map(on_choose, |el, on| el.on_press(move |_| on.call(())))
        .children(rows.iter().map(|f| {
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_control_small))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .maybe_child(f.key.map(|k| rect().width(Size::px(t.size_mark)).height(Size::px(t.size_mark)).corner_radius(t.size_mark / 2.).background(colour(k)).into_element()))
                .child(
                    rect().width(Size::flex(1.)).child(
                        text::line(theme, text::Style::Callout, colour(label_ink), f.label.clone()).font_weight(if chosen { 600 } else { 400 }),
                    ),
                )
                .child(label().font_family("JetBrains Mono").font_size(t.type_style_body_size).color(colour(t.colour_text)).max_lines(1).text(f.value.clone()))
                .into_element()
        }))
        .into_element()
}

/// The footer's notes: why a figure is missing, each on a line of its own.
pub fn notes(theme: &isle_tokens::Theme, notes: &[String]) -> Option<Element> {
    let t = &theme.tokens;
    (!notes.is_empty()).then(|| {
        rect()
            .width(Size::fill())
            .children(notes.iter().map(|n| {
                rect()
                    .height(Size::px(t.size_control_small))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_xs)
                    .child(Glyph::new("info", t.size_glyph_small, colour(t.colour_text_2)))
                    .child(text::line(theme, text::Style::Callout, colour(t.colour_text_2), n.clone()))
                    .into_element()
            }))
            .into_element()
    })
}

/// A graph with its title over it.
pub fn titled(theme: &isle_tokens::Theme, title: &str, data: GraphData, width: f32, summary: String) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(width))
        .spacing(t.space_xs)
        .cross_align(Alignment::Center)
        .maybe_child((!title.is_empty()).then(|| text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), title.to_owned()).into_element()))
        .child(Graph { data: Rc::new(data), width, name: title.to_owned(), summary })
        .into_element()
}
