//! Monitor's main window (`p1/structure/monitor.md`, `p1/surfaces/monitor.md`): the toolbar
//! with the filter's name, the resource switch, Quit Process, More and search; the table;
//! the footer.

use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::Rc,
    time::Duration,
};

use freya::prelude::*;
use isle_app::{Chord, Command, Desktop, Menu, Request, ToolbarExt};
use isle_ui::{
    Button, SearchField, colour,
    form::Segmented,
    table::{Sort, Table, TableEvent},
    text, use_theme,
};

use crate::{
    footer,
    history::History,
    model::{self, Filter, Resource},
    sampler::{self, Control, Sample},
    state::{self, Remembered},
};

/// The compositor's lights over the toolbar's leading end (`isle_shell::chrome::LIGHTS_END`).
const LIGHTS: f32 = 68.;

#[derive(Clone)]
pub struct Monitor;

/// The resource's columns: the user's where they changed them, else the sheet's.
fn columns_of(remembered: &Remembered, resource: Resource, gpus: usize) -> Vec<&'static str> {
    let defaults = resource.columns(gpus);
    match remembered.columns.get(&resource) {
        Some(ids) => ALL_COLUMNS.iter().copied().filter(|c| ids.iter().any(|i| i == c)).collect(),
        None => defaults,
    }
}

/// Every column View › Columns offers, in the order they stand.
const ALL_COLUMNS: [&str; 15] = [
    "cpu", "cpu_time", "memory", "energy", "gpu", "gpu_memory", "gpu_name", "written", "read", "received", "sent", "sleep",
    "threads", "pid", "user",
];

fn sort_of(remembered: &Remembered, resource: Resource, columns: &[&'static str]) -> Sort {
    let fallback = Sort { column: columns.first().copied().unwrap_or("name"), descending: true };
    remembered
        .sorts
        .get(&resource)
        .and_then(|(id, descending)| {
            let column = std::iter::once("name").chain(columns.iter().copied()).find(|c| c == id)?;
            Some(Sort { column, descending: *descending })
        })
        .unwrap_or(fallback)
}

fn commands(remembered: &Remembered, gpus: usize) -> Vec<Command> {
    let mut out = Vec::new();
    for (i, r) in Resource::ALL.iter().enumerate() {
        let digit = char::from(b'1' + i as u8);
        out.push(
            Command::new(format!("resource:{}", r.index()), r.title(), Menu::View, 1)
                .chord(Chord::cmd(digit))
                .checked(remembered.resource == *r),
        );
    }
    for f in Filter::ALL {
        out.push(Command::new(format!("filter:{f:?}"), f.title(), Menu::View, 2).checked(remembered.filter == f));
    }
    let shown = columns_of(remembered, remembered.resource, gpus);
    for id in ALL_COLUMNS {
        let mut c = Command::new(format!("column:{id}"), model::column(id).title, Menu::View, 3).checked(shown.contains(&id));
        c.submenu = Some("Columns".into());
        out.push(c);
    }
    for (secs, label) in [(1, "Very Often (1 s)"), (2, "Often (2 s)"), (5, "Normally (5 s)")] {
        let mut c = Command::new(format!("every:{secs}"), label, Menu::View, 4).checked(remembered.every == secs);
        c.submenu = Some("Update Frequency".into());
        out.push(c);
    }
    out.push(Command::new("search", "Search", Menu::Edit, 9).chord(Chord::cmd('f')));
    out
}

impl App for Monitor {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        let mut remembered = use_state(state::load);
        use_side_effect(move || state::save(&remembered.read()));
        let shown = isle_app::use_shown();
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        let (control, incoming) = use_hook(move || {
            let (control, rx) = sampler::start(&worker, bus);
            (Rc::new(control), Rc::new(RefCell::new(Some(rx))))
        });
        use_side_effect({
            let control = control.clone();
            move || {
                let every = Duration::from_secs(remembered.read().every.max(1));
                let _ = control.send(Control { shown: *shown.read(), every });
            }
        });
        let mut latest = use_state(|| None::<Sample>);
        let mut history = use_state(History::default);
        use_hook(move || {
            let Some(mut rx) = incoming.borrow_mut().take() else { return };
            spawn(async move {
                use futures_util::StreamExt;
                while let Some(sample) = rx.next().await {
                    let every = Duration::from_secs(remembered.peek().every.max(1));
                    history.write().push(&sample, every);
                    latest.set(Some(sample));
                }
            });
        });

        // A launch on a resource: the System widget's click asks for CPU or GPU.
        use_side_effect(move || {
            if desktop.requests.read().is_empty() {
                return;
            }
            let mut queue = desktop.requests;
            for request in std::mem::take(&mut *queue.write()) {
                if let Request::Action { name, .. } = request {
                    let to = match name.as_str() {
                        "cpu" => Some(Resource::Cpu),
                        "gpu" => Some(Resource::Gpu),
                        _ => None,
                    };
                    if let Some(r) = to {
                        remembered.write().resource = r;
                    }
                }
            }
        });

        let gpus = latest.read().as_ref().map_or(0, |s| s.full.gpus.len());
        isle_app::use_commands(commands(&remembered.read(), gpus));
        let query = use_state(String::new);
        let mut selected = use_state(Vec::<u64>::new);
        let mut open = use_state(BTreeSet::<u64>::new);
        let mut search_focus = use_state(|| 0u64);
        use_side_effect(move || {
            if desktop.commands.read().is_empty() {
                return;
            }
            let mut queue = desktop.commands;
            for id in std::mem::take(&mut *queue.write()) {
                let mut r = remembered.write();
                if let Some(i) = id.strip_prefix("resource:").and_then(|i| i.parse::<usize>().ok()) {
                    r.resource = Resource::ALL[i.min(5)];
                } else if let Some(f) = id.strip_prefix("filter:") {
                    if let Some(f) = Filter::ALL.into_iter().find(|x| format!("{x:?}") == f) {
                        r.filter = f;
                    }
                } else if let Some(c) = id.strip_prefix("column:") {
                    let resource = r.resource;
                    let mut now: Vec<String> = columns_of(&r, resource, gpus).iter().map(|s| s.to_string()).collect();
                    match now.iter().position(|x| x == c) {
                        Some(at) => {
                            now.remove(at);
                        }
                        None => now.push(c.to_owned()),
                    }
                    r.columns.insert(resource, now);
                } else if let Some(secs) = id.strip_prefix("every:").and_then(|s| s.parse().ok()) {
                    r.every = secs;
                } else if id == "search" {
                    drop(r);
                    *search_focus.write() += 1;
                }
            }
        });

        let r = remembered.read().clone();
        let resource = r.resource;
        let columns = columns_of(&r, resource, gpus);
        let sort = sort_of(&r, resource, &columns);
        let (rows, uid) = match latest.read().as_ref() {
            Some(sample) => {
                let procs = model::processes(&sample.full, &sample.extras);
                let rows = model::rows(&procs, &columns, r.filter, sample.full.uid, &query.read(), sort, &open.read());
                (rows, sample.full.uid)
            }
            None => (Vec::new(), 0),
        };
        let _ = uid;
        let mut table_columns = vec![model::column("name")];
        table_columns.extend(columns.iter().map(|id| {
            let mut c = model::column(id);
            if let Some(w) = r.widths.get(*id) {
                c.width = *w;
            }
            if r.filter == Filter::Hierarchy && model::summed(id) {
                c.width += 56.;
            }
            c
        }));
        let empty = rows.is_empty() && latest.read().is_some() && !query.read().trim().is_empty();
        let controller = use_scroll_controller(ScrollConfig::default);
        let table = Table {
            columns: Rc::new(table_columns),
            rows: Rc::new(rows),
            sort,
            selected: Rc::new(selected.read().clone()),
            focused: true,
            hold: false,
            on_event: EventHandler::new(move |e: TableEvent| match e {
                TableEvent::Selected(keys) => selected.set(keys),
                TableEvent::Sort(s) => {
                    remembered.write().sorts.insert(resource, (s.column.to_owned(), s.descending));
                }
                TableEvent::Resize(id, width) => {
                    remembered.write().widths.insert(id.to_owned(), width);
                }
                TableEvent::Disclose(key, is_open) => {
                    let mut o = open.write();
                    if is_open {
                        o.insert(key);
                    } else {
                        o.remove(&key);
                    }
                }
                _ => {}
            }),
            controller,
        };

        let segments: Vec<String> = Resource::ALL.iter().map(|r| r.title().to_owned()).collect();
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS + t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .toolbar()
            .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Headline, colour(t.colour_text), r.filter.title())))
            .child(Segmented::new(segments, resource.index(), "Resource", move |i: usize| remembered.write().resource = Resource::ALL[i]))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .horizontal()
                    .main_align(Alignment::End)
                    .cross_align(Alignment::Center)
                    .spacing(t.space_l)
                    .child(Button::new("Quit Process").enabled(false))
                    .child(SearchField { value: query.into_writable(), placeholder: "Search".into(), width: 160. }),
            );
        let _ = search_focus;

        let mut footer_width = use_state(|| 0f32);
        let (footer, notes) = match latest.read().as_ref() {
            Some(sample) => footer_view(&theme, resource, sample, &history.read(), &r, *footer_width.read(), remembered),
            None => (rect().into_element(), 0),
        };

        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(
                rect().width(Size::fill()).height(Size::flex(1.)).child(if empty {
                    rect()
                        .expanded()
                        .center()
                        .background(colour(t.colour_view))
                        .child(text::line(&theme, text::Style::Body, colour(t.colour_text_2), format!("No results for “{}”", query.read().trim())))
                        .into_element()
                } else {
                    table.into_element()
                }),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_monitor_footer + notes as f32 * t.size_control_small))
                    .padding(t.space_l)
                    .background(colour(t.colour_window))
                    .border(
                        Border::new()
                            .fill(colour(t.colour_hairline))
                            .width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. })
                            .alignment(BorderAlignment::Inner),
                    )
                    .on_sized(move |e: Event<SizedEventData>| footer_width.set_if_modified(e.area.width()))
                    .child(footer),
            )
    }
}

/// The footer for `resource`: figures, graph, figures and sensors; for GPU a block a GPU.
/// With the number of note lines, each of which grows the footer by a row.
fn footer_view(theme: &isle_tokens::Theme, resource: Resource, sample: &Sample, history: &History, r: &Remembered, width: f32, mut remembered: State<Remembered>) -> (Element, usize) {
    let t = &theme.tokens;
    let inner = (width - 2. * t.space_l).max(0.);
    let c = footer::content(resource, &sample.full, history, theme, r.disk_data, r.network_data);
    let count = c.notes.len();
    let notes = footer::notes(theme, &c.notes);
    if resource == Resource::Gpu {
        let gpus: Vec<_> = sample.full.gpus.iter().enumerate().filter(|(_, g)| g.backend != isle_sysinfo::gpus::Backend::None).collect();
        let each = if gpus.is_empty() { inner } else { (inner - t.space_xxl * (gpus.len() as f32 - 1.)) / gpus.len() as f32 };
        let graph_w = (each - t.size_monitor_figures - t.space_xxl).clamp(160., 520.);
        return (rect()
            .width(Size::fill())
            .spacing(t.space_s)
            .maybe_child(notes)
            .child(
                rect().horizontal().spacing(t.space_xxl).children(gpus.into_iter().map(|(i, g)| {
                    let values = history.series(|p| p.gpu_busy.get(i).copied().flatten());
                    let data = isle_ui::graph::GraphData {
                        series: vec![isle_ui::graph::Series { values: values.clone(), colour: t.series_a }],
                        shape: isle_ui::graph::Shape::Lines,
                        max: Some(1.0),
                        unit: isle_ui::graph::Unit::Share,
                        times: history.times.iter().cloned().collect(),
                    };
                    rect()
                        .horizontal()
                        .spacing(t.space_xxl)
                        .child(footer::figures(theme, &footer::gpu_figures(g), None, None))
                        .child(footer::titled(theme, &g.name, data, graph_w, format!("{} utilisation", g.name)))
                        .into_element()
                })),
            )
            .into_element(), count);
    }
    let wide = inner >= 3. * t.size_monitor_figures + 2. * t.space_xxl + 240. + t.space_xxl;
    let graph_w = (inner - 2. * t.size_monitor_figures - 2. * t.space_xxl - if wide && !c.sensors.is_empty() { t.size_monitor_figures + t.space_xxl } else { 0. }).clamp(160., 520.);
    let toggle = |data: bool| -> EventHandler<()> {
        EventHandler::new(move |_| {
            let mut w = remembered.write();
            match resource {
                Resource::Disk => w.disk_data = data,
                _ => w.network_data = data,
            }
        })
    };
    (rect()
        .width(Size::fill())
        .spacing(t.space_s)
        .maybe_child(notes)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .spacing(t.space_xxl)
                .child(footer::figures(theme, &c.leading, c.leading_choice, c.leading_choice.map(|_| toggle(false))))
                .child(rect().width(Size::flex(1.)).cross_align(Alignment::Center).child(footer::titled(theme, &c.title, c.data, graph_w, c.summary)))
                .child(footer::figures(theme, &c.trailing, c.trailing_choice, c.trailing_choice.map(|_| toggle(true))))
                .maybe_child((wide && !c.sensors.is_empty()).then(|| footer::figures(theme, &c.sensors, None, None))),
        )
        .into_element(), count)
}
