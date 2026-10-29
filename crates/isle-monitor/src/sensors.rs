//! The Sensors window (`structure/monitor.md` § Resources, `surfaces/monitor.md` § The Sensors
//! window): every hwmon reading by device, with its low and high since the window opened, and
//! Open CoolerControl when it is installed.

use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use freya::prelude::*;
use isle_app::{AppHandle, ToolbarExt, WindowSpec};
use isle_sysinfo::sensors::{Kind, Sensor, Sensors};
use isle_ui::{
    Button, colour,
    table::{Cell, Column, Row, Sort, Table, TableEvent},
    text, use_theme,
};

const LIGHTS: f32 = 68.;
pub const COOLERCONTROL: &str = "org.coolercontrol.CoolerControl";
pub const LACT: &str = "io.github.ilya_zlobintsev.LACT";

thread_local! {
    static OPEN: std::cell::Cell<Option<isle_app::WindowKey>> = const { std::cell::Cell::new(None) };
}

/// Whether an app's desktop entry is installed.
pub fn installed(id: &str) -> bool {
    isle_apps::Index::data_dirs().iter().any(|d| d.join(format!("{id}.desktop")).exists())
}

/// Launches an app through the compositor, as Spotlight does.
pub fn launch(bus: Option<zbus::Connection>, id: &'static str) {
    isle_app::worker().spawn(async move {
        let Some(bus) = bus else { return };
        if let Ok(windows) = isle_ipc::windows::WindowsProxy::new(&bus).await {
            let _ = windows.launch(id, "", &[]).await;
        }
    });
}

/// Opens the Sensors window, or leaves the open one be.
pub fn open(app: &AppHandle) {
    if OPEN.with(|o| o.get()).is_some_and(|w| app.windows().contains(&w)) {
        return;
    }
    let spec = WindowSpec {
        title: "Sensors".into(),
        size: (560.0, 640.0),
        min_size: (420.0, 320.0),
        max_size: None,
        lights: Some((0.0, 10.0)),
        backdrop: None,
        token: None,
        parent: None,
        float: None,
    };
    let window = app.open(spec, SensorsWindow);
    OPEN.with(|o| o.set(Some(window)));
}

fn written(kind: Kind, v: f32) -> String {
    match kind {
        Kind::Temperature => format!("{v:.1} °C"),
        Kind::Fan => format!("{v:.0} rpm"),
        Kind::Voltage => format!("{v:.3} V"),
        Kind::Current => format!("{v:.2} A"),
        Kind::Power => format!("{v:.1} W"),
        Kind::Clock => format!("{v:.0} MHz"),
    }
}

/// Rows by device: a heading, then each reading with its low and high.
fn rows(readings: &[Sensor], range: &HashMap<String, (f32, f32)>) -> Vec<Row> {
    let mut out = Vec::new();
    let mut last_device = String::new();
    for (i, s) in readings.iter().enumerate() {
        let device = format!("{} · {}", s.group, s.chip);
        if device != last_device {
            out.push(Row { key: (1 << 40) | i as u64, name: device.clone(), heading: true, ..Default::default() });
            last_device = device;
        }
        let (lo, hi) = range.get(&s.id).copied().unwrap_or((s.value, s.value));
        out.push(Row {
            key: i as u64,
            name: s.label.clone(),
            cells: vec![Cell::from(written(s.kind, s.value)), Cell::from(written(s.kind, lo)), Cell::from(written(s.kind, hi))],
            ..Default::default()
        });
    }
    out
}

#[derive(Clone)]
pub struct SensorsWindow;

impl App for SensorsWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shown = isle_app::use_shown();
        let mut readings = use_state(Vec::<Sensor>::new);
        let mut range = use_state(HashMap::<String, (f32, f32)>::new);
        let worker = isle_app::worker();
        let bus = isle_app::session_bus();
        use_hook(move || {
            let sensors = Arc::new(Mutex::new(Sensors::new(std::path::Path::new("/"))));
            spawn(async move {
                loop {
                    if *shown.peek() {
                        let sensors = sensors.clone();
                        let read = worker.spawn(async move { tokio::task::spawn_blocking(move || sensors.lock().unwrap().read()).await.unwrap_or_default() });
                        if let Ok(now) = read.await {
                            let mut r = range.write();
                            for s in &now {
                                let e = r.entry(s.id.clone()).or_insert((s.value, s.value));
                                *e = (e.0.min(s.value), e.1.max(s.value));
                            }
                            drop(r);
                            readings.set(now);
                        }
                    }
                    crate::sleep(Duration::from_secs(2)).await;
                }
            });
        });
        let cooler = use_hook(|| installed(COOLERCONTROL));
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS + t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .toolbar()
            .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Headline, colour(t.colour_text), "Sensors")))
            .maybe_child(cooler.then(|| Button::new("Open CoolerControl").on_press(move |_| launch(bus.clone(), COOLERCONTROL)).into_element()));
        let controller = use_scroll_controller(ScrollConfig::default);
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(
                rect().width(Size::fill()).height(Size::flex(1.)).child(Table {
                    columns: Rc::new(vec![
                        Column::text("name", "Sensor", 200.),
                        Column::figures("value", "Value", 96.),
                        Column::figures("low", "Low", 96.),
                        Column::figures("high", "High", 96.),
                    ]),
                    rows: Rc::new(rows(&readings.read(), &range.read())),
                    sort: Sort { column: "none", descending: false },
                    selected: Rc::new(Vec::new()),
                    focused: false,
                    hold: true,
                    on_event: EventHandler::new(|_: TableEvent| {}),
                    controller,
                }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readings_group_under_their_device_with_their_range() {
        let s = |id: &str, chip: &str, label: &str, v: f32| Sensor { id: id.into(), chip: chip.into(), group: "CPU".into(), label: label.into(), kind: Kind::Temperature, value: v };
        let readings = [s("hwmon6/temp1", "k10temp", "CPU package", 61.0), s("hwmon6/temp3", "k10temp", "CPU die 1", 55.0)];
        let range = HashMap::from([("hwmon6/temp1".to_owned(), (40.0, 80.0))]);
        let r = rows(&readings, &range);
        assert_eq!(r.len(), 3);
        assert!(r[0].heading && r[0].name == "CPU · k10temp");
        assert_eq!(r[1].cells[1].text, "40.0 °C");
        assert_eq!(r[1].cells[2].text, "80.0 °C");
        assert_eq!(r[2].cells[1].text, "55.0 °C", "no range yet: the value itself");
    }
}
