//! The widgets dashboard: its model, its keys, its routes and its backdrop
//! (`p1/structure/widgets.md`; `p17-widgets.md` choices 5–8).

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    time::{Duration, Instant},
};

use isle_shell::{
    dashboard::{
        catalogue,
        grid::{Cells, Dir, Extent, History, Layout, Metrics, Sizes, default_layout, kind_of},
        surface::{
            BatteryRow, Body, ClockView, Control, DashboardAction, DashboardSurface, DesktopRow,
            Dial, FormRow, FormValue, FormView, Frame, Ghost, Lifted, MenuView, NoteRow,
            SeriesView, VolumeRow, WidgetView,
        },
    },
    prompts::session::Choice,
    transition::Motion,
};
use isle_ui::{MenuEntry, MenuEvent, MenuItem};
use smithay::{
    backend::renderer::element::{Kind, solid::SolidColorBuffer, solid::SolidColorRenderElement},
    output::Output,
    utils::{Logical, Point},
    reexports::calloop::{
        RegistrationToken,
        timer::{TimeoutAction, Timer},
    },
    utils::{Rectangle, Scale},
};
use tracing::info;

use crate::{
    focus::Focus,
    overlays::Overlay,
    render::IsleElement,
    state::{Isle, State},
};

pub struct DashboardState {
    pub surface: Rc<DashboardSurface>,
    pub open: Option<Open>,
    /// The output its backdrop is drawn on, until it has faded out.
    pub shown_on: Option<String>,
    pub backdrop: Motion,
    last_step: Instant,
    tick: Option<RegistrationToken>,
    /// Each mounted volume's use, read on the worker when the dashboard opens.
    pub storage: Option<Vec<VolumeRow>>,
    /// The last minute of samples, newest last, while a System widget shows.
    pub system: Vec<isle_sysinfo::Sample>,
    sampler: std::sync::Arc<std::sync::Mutex<Option<isle_sysinfo::Sampler>>>,
    sample_tick: Option<RegistrationToken>,
    pub timer_tick: Option<RegistrationToken>,
    /// Providers on disk, by id, read as the dashboard opens.
    pub providers: BTreeMap<String, crate::providers::Installed>,
    pub running: crate::providers::Runs,
    pub answers: crate::providers::Answers,
    /// Each source's last good index, or why there is none.
    pub browse: BTreeMap<String, Result<isle_providers::sources::Index, String>>,
    pub fetching: std::collections::HashSet<String>,
    /// The `widgets` domain: last data, timers, sources.
    pub memory: isle_state::domains::Widgets,
    saver: Option<crate::state_writer::Saver<isle_state::domains::Widgets>>,
    floors: RefCell<HashMap<String, (SolidColorBuffer, SolidColorBuffer)>>,
}

pub struct Open {
    pub output: String,
    pub identity: String,
    pub extent: Extent,
    pub layout: Layout,
    pub edit: bool,
    pub selected: Option<String>,
    /// The selection came from a key, so the ring shows.
    pub ring: bool,
    pub history: History,
    /// The hot corner that opened it, which closes it again.
    pub corner: bool,
    pub press: Option<Press>,
    pub menu: Option<Menu>,
    /// The widget whose settings popover is open.
    pub form: Option<String>,
    pub gallery: Option<Gallery>,
    pub consent: Option<crate::widget_sources::Consent>,
}

/// The gallery's state in edit mode.
#[derive(Clone, Debug, Default)]
pub struct Gallery {
    /// Installed, Yours, Browse.
    pub section: usize,
    pub query: String,
    /// It has the keyboard: typing searches, arrows choose.
    pub focused: bool,
    pub selected: usize,
    pub details: Option<String>,
    /// Sources in its place; true while an address is being added.
    pub sources: Option<bool>,
}

/// A press held on a widget in edit mode: a move or a resize once it travels.
pub struct Press {
    pub id: String,
    pub part: Part,
    /// Where it began, in points on the output.
    pub start: (f32, f32),
    /// From the widget's top left to the pointer.
    pub grab: (f32, f32),
    /// The layout before it began, which every step is worked out from.
    pub base: Layout,
    pub moved: bool,
    /// Where it would land now, and what that does to the rest; none where refused.
    pub plan: Option<Layout>,
    pub ghost: Cells,
    pub at: (f32, f32),
    /// The widget was selected when pressed.
    pub was_selected: bool,
    /// A press on a gallery item: the widget it adds and its size.
    pub adding: Option<(i32, i32)>,
    /// The pointer is over the gallery.
    pub over_gallery: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Body,
    Badge,
    /// An edge or the corner, by the sides it moves.
    Resize { left: bool, right: bool, top: bool, bottom: bool },
}

/// A menu open on the dashboard, and what it was opened on.
pub struct Menu {
    pub on: Option<String>,
    pub at: (f32, f32),
    pub by_key: bool,
}

const MENU_EDIT_ONE: u64 = 1;
const MENU_REMOVE: u64 = 2;
const MENU_EDIT: u64 = 3;
const MENU_CLOSE: u64 = 4;
const MENU_TRY_AGAIN: u64 = 5;
const MENU_ADD: u64 = 6;
const MENU_DETAILS: u64 = 7;
const MENU_SIZE: u64 = 100;

/// How far a press travels before it is a drag, in points.
const DRAG_START: f32 = 4.0;

impl DashboardState {
    pub fn new() -> Self {
        Self {
            surface: DashboardSurface::new(),
            open: None,
            shown_on: None,
            backdrop: Motion::at(0.0),
            last_step: Instant::now(),
            tick: None,
            storage: None,
            system: Vec::new(),
            sampler: Default::default(),
            sample_tick: None,
            timer_tick: None,
            providers: BTreeMap::new(),
            running: Default::default(),
            answers: Default::default(),
            browse: BTreeMap::new(),
            fetching: Default::default(),
            memory: Default::default(),
            saver: None,
            floors: RefCell::default(),
        }
    }
}

impl Isle {
    pub fn load_widgets(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = isle_state::StateDir::new(isle_state::StateDir::default_dir());
        let (memory, loaded) = dir.load::<isle_state::domains::Widgets>();
        if !matches!(loaded, isle_state::Loaded::Read | isle_state::Loaded::Missing) {
            tracing::warn!(?loaded, "the widgets' memory");
        }
        self.dashboard.memory = memory;
        self.dashboard.saver = Some(crate::state_writer::Saver::spawn(runtime, dir));
    }

    pub fn save_widgets(&self) {
        if let Some(saver) = &self.dashboard.saver {
            saver.save(self.dashboard.memory.clone());
        }
    }

    /// A Weather widget's last forecast.
    fn forecast(&self, id: &str) -> Option<crate::weather::Forecast> {
        let data = self.dashboard.memory.data.get(id)?;
        serde_json::from_value(data.value.clone()).ok()
    }

    fn weather_view(&self, id: &str) -> Option<isle_shell::dashboard::surface::WeatherView> {
        use isle_shell::dashboard::surface::WeatherView;
        let f = self.forecast(id)?;
        let degrees = |t: f64| format!("{:.0}°", t.round());
        let (glyph, words) = crate::weather::condition(f.code, f.day);
        let (week_low, week_high) = f
            .days
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d.2), hi.max(d.3)));
        let span = (week_high - week_low).max(1.0);
        Some(WeatherView {
            place: f.place.clone(),
            now: degrees(f.now),
            glyph: glyph.to_owned(),
            condition: words.to_owned(),
            high_low: format!("H:{} L:{}", degrees(f.high), degrees(f.low)),
            hours: f
                .hours
                .iter()
                .map(|(h, code, t)| (h.clone(), crate::weather::condition(*code, true).0.to_owned(), degrees(*t)))
                .collect(),
            days: f
                .days
                .iter()
                .map(|(d, code, lo, hi)| {
                    (
                        d.clone(),
                        crate::weather::condition(*code, true).0.to_owned(),
                        degrees(*lo),
                        degrees(*hi),
                        ((lo - week_low) / span) as f32,
                        ((hi - week_low) / span) as f32,
                    )
                })
                .collect(),
        })
    }

    /// The grid on `output`, from the tokens.
    pub fn dashboard_extent(&self, output: &Output) -> Extent {
        let size = self.space.output_geometry(output).unwrap_or_default().size;
        let t = &self.shell.theme().tokens;
        Extent::of(
            size.w as f32,
            size.h as f32,
            Metrics {
                cell: t.size_widget_cell,
                gap: t.space_l,
                side: t.space_xxxl,
                top: t.size_strip + t.space_l,
                bottom: t.space_xxxl + t.size_control_large + t.space_xxl,
            },
        )
    }

    /// The layout `settings.kdl` keeps for the screen of `identity`, or the default.
    pub fn dashboard_layout(&self, identity: &str, extent: &Extent) -> Layout {
        match self
            .settings
            .dashboard_output
            .get(identity)
            .and_then(|d| d.widgets.as_ref())
        {
            Some(lines) => {
                let (layout, skipped) = Layout::from_lines(lines, extent);
                if !skipped.is_empty() {
                    tracing::warn!(identity, ?skipped, "dashboard lines that do not read");
                }
                layout
            }
            None => default_layout(extent),
        }
    }

    /// The backdrop's opacity on `output`, while the dashboard is there or fading from it.
    pub fn dashboard_backdrop(&self, output: &Output) -> Option<f32> {
        (self.dashboard.shown_on.as_deref() == Some(&output.name()))
            .then(|| self.dashboard.backdrop.value().clamp(0.0, 1.0) as f32)
    }

    pub fn dashboard_open_on(&self, output: &Output) -> bool {
        self.dashboard
            .open
            .as_ref()
            .is_some_and(|o| o.output == output.name())
    }

    /// The wash, the softened wallpaper and, once it covers everything, a floor, front to
    /// back (as the lock's).
    pub fn dashboard_backdrop_elements(
        &self,
        output: &Output,
        alpha: f32,
        push: &mut dyn FnMut(IsleElement),
    ) {
        let Some(geometry) = self.space.output_geometry(output) else {
            return;
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let tokens = &self.shell.theme().tokens;
        let wash = if self.shell.theme().appearance.increase_contrast {
            tokens.material_lock_wash_contrast
        } else {
            tokens.material_lock_wash
        };
        let mut floors = self.dashboard.floors.borrow_mut();
        let (floor, wash_buffer) = floors.entry(output.name()).or_default();
        wash_buffer.update(geometry.size, crate::wallpaper::premultiplied(wash));
        floor.update(
            geometry.size,
            crate::wallpaper::premultiplied(tokens.colour_window_sunken),
        );
        push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
            wash_buffer,
            (0, 0),
            scale,
            alpha,
            Kind::Unspecified,
        )));
        self.softened_elements(output, alpha, &mut |e| push(IsleElement::Wallpaper(e)));
        if alpha >= 1.0 {
            push(IsleElement::Solid(SolidColorRenderElement::from_buffer(
                floor,
                (0, 0),
                scale,
                1.0,
                Kind::Unspecified,
            )));
        }
    }

    fn clock_view(&self, id: &str) -> ClockView {
        let analogue = self
            .settings
            .widget
            .get(id)
            .and_then(|w| w.get("face"))
            .is_some_and(|f| matches!(f, isle_settings::Value::Str(f) if f == "analogue"));
        let zone = self
            .settings
            .widget
            .get(id)
            .and_then(|w| match w.get("zone") {
                Some(isle_settings::Value::Str(z)) => jiff::tz::TimeZone::get(z).ok(),
                _ => None,
            })
            .unwrap_or_else(jiff::tz::TimeZone::system);
        let now = jiff::Zoned::now().with_time_zone(zone.clone());
        let hours = crate::island::clock_24_hour(&self.settings.island);
        let time = if hours {
            now.strftime("%H:%M").to_string()
        } else {
            now.strftime("%-I:%M").to_string()
        };
        let dial = analogue.then(|| {
            let (h, m, sec) = (now.hour() as f32, now.minute() as f32, now.second() as f32);
            Dial {
                hour: ((h % 12.0) + m / 60.0) / 12.0,
                minute: (m + sec / 60.0) / 60.0,
                second: self.settings.island.clock_seconds.then_some(sec / 60.0),
                night: !(6..18).contains(&now.hour()),
            }
        });
        ClockView {
            city: city_of(zone.iana_name().unwrap_or("UTC")),
            time,
            weekday: now.strftime("%A").to_string(),
            date: now.strftime("%-d %B").to_string(),
            dial,
        }
    }

    fn timer_view(&self, id: &str) -> isle_shell::dashboard::surface::TimerView {
        use isle_shell::dashboard::surface::{TimerState, TimerView};
        let length = self.timer_length(id);
        let (left, state) = match self.timer_left(id) {
            Some((left, true)) => (left, TimerState::Running),
            Some((left, false)) => (left, TimerState::Paused),
            None => (length, TimerState::Idle),
        };
        let total = self.dashboard.memory.timers.get(id).map_or(length, |t| t.duration_ms).max(1);
        TimerView {
            label: self.timer_label(id),
            time: crate::timer::clock(left),
            state,
            left: left as f32 / total as f32,
            presets: vec![1, 5, 10, 25],
        }
    }

    fn battery_rows(&self) -> Vec<BatteryRow> {
        let power = &self.services.power;
        if !power.available {
            return Vec::new();
        }
        power
            .state
            .batteries
            .iter()
            .map(|b| BatteryRow {
                name: b.name.clone(),
                glyph: b.kind.glyph().to_owned(),
                level: b.level,
                charging: b.charging,
                low: b.level.is_some_and(|l| l <= 20) && !b.charging,
            })
            .collect()
    }

    fn desktop_rows(&self, output: &str) -> Vec<DesktopRow> {
        use isle_shell::mission::model::DeskKind;
        let (screens, _) = self.mission_screens();
        let Some(screen) = screens.into_iter().find(|s| s.output == output) else {
            return Vec::new();
        };
        let app_of = |id: u64| screen.window(id).map(|w| w.app.clone()).unwrap_or_default();
        let mut normal = 0;
        screen
            .desks
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let name = match d.kind {
                    DeskKind::Normal => {
                        normal += 1;
                        format!("Desktop {normal}")
                    }
                    DeskKind::FullScreen(id) => self.app_name(&app_of(id)),
                    DeskKind::Split(a, b) => {
                        format!("{} and {}", self.app_name(&app_of(a)), self.app_name(&app_of(b)))
                    }
                };
                // One icon per app, front first.
                let mut apps: Vec<String> = Vec::new();
                for id in d.windows.iter().rev() {
                    let app = app_of(*id);
                    if !app.is_empty() && !apps.contains(&app) {
                        apps.push(app);
                    }
                }
                DesktopRow {
                    name,
                    icons: apps.iter().map(|a| self.app_icon(a, 32)).collect(),
                    more: 0,
                    current: i == screen.current,
                }
            })
            .collect()
    }

    fn note_rows(&self) -> Vec<NoteRow> {
        let mut rows: Vec<NoteRow> = Vec::new();
        // Newest first: each app's place is where its newest arrived.
        for entry in self.notifications.entries.iter().rev() {
            if !entry.live || entry.fate == crate::notifications::Fate::Transient {
                continue;
            }
            let app = self.notification_app(&entry.n);
            match rows.iter_mut().find(|r| r.app == app) {
                Some(row) => row.count += 1,
                None => {
                    let name = self.app_name(&app);
                    rows.push(NoteRow {
                        name: if name.is_empty() { entry.n.app_name.clone() } else { name },
                        icon: self.app_icon(&app, 32),
                        app,
                        count: 1,
                    });
                }
            }
        }
        rows
    }

    /// A provider's settings from its manifest, its title line's switch first.
    fn provider_form(&self, id: &str) -> Vec<FormRow> {
        use isle_providers::manifest::SettingType;
        use isle_settings::Value;
        let stored = self.settings.widget.get(id);
        let value = |key: &str| stored.and_then(|o| o.get(key)).cloned();
        let mut rows = vec![FormRow {
            key: "title".into(),
            label: "Show name".into(),
            control: Control::Toggle(value("title") != Some(Value::Bool(false))),
        }];
        let Some(p) = self.provider_of(id) else {
            return rows;
        };
        for s in &p.manifest.settings {
            let default = s.default.clone().unwrap_or(serde_json::Value::Null);
            let control = match s.kind {
                SettingType::Toggle => Control::Toggle(match value(&s.key) {
                    Some(Value::Bool(b)) => b,
                    _ => default.as_bool().unwrap_or(false),
                }),
                SettingType::Choice => {
                    let chosen = match value(&s.key) {
                        Some(Value::Str(v)) => v,
                        _ => default.as_str().unwrap_or("").to_owned(),
                    };
                    Control::Choice {
                        options: s.options.clone(),
                        selected: s.options.iter().position(|o| *o == chosen).unwrap_or(0),
                    }
                }
                SettingType::Number => {
                    let (min, max) = (s.min.unwrap_or(0), s.max.unwrap_or(1000));
                    Control::Number {
                        value: match value(&s.key) {
                            Some(Value::Int(i)) => i,
                            _ => default.as_i64().unwrap_or(min),
                        }
                        .clamp(min, max),
                        min,
                        max,
                    }
                }
                SettingType::Text => Control::Text(match value(&s.key) {
                    Some(Value::Str(v)) => v,
                    _ => default.as_str().unwrap_or("").to_owned(),
                }),
            };
            rows.push(FormRow {
                key: s.key.clone(),
                label: s.label.clone(),
                control,
            });
        }
        rows
    }

    /// What a widget shows now: its data, or the line for why there is none.
    pub fn widget_body(&self, id: &str, name: &str, output: &str) -> Body {
        let empty = |line: &str| Body::Empty(line.to_owned());
        match kind_of(id) {
            "clock" => Body::Clock(self.clock_view(id)),
            "session" => Body::Session,
            "media" => self.now_playing_card().map_or_else(|| empty("Not playing"), Body::Media),
            "devices" => match self.battery_rows() {
                rows if rows.is_empty() => empty("No devices"),
                rows => Body::Devices(rows),
            },
            "storage" => self.dashboard.storage.clone().map_or(Body::Title, Body::Storage),
            "desktops" => Body::Desktops(self.desktop_rows(output)),
            "notifications" => match self.note_rows() {
                rows if rows.is_empty() => empty("No notifications"),
                rows => Body::Notifications(rows),
            },
            "controls" => Body::Controls(self.control_modules(id)),
            "weather" => self.weather_view(id).map_or(Body::Title, Body::Weather),
            "calendar" => empty("No calendars"),
            "timer" => Body::Timer(self.timer_view(id)),
            "system" if self.dashboard.system.is_empty() => Body::Title,
            "system" => Body::System(self.series_views(id)),
            _ => self.provider_body(id, name),
        }
    }

    /// A provider widget's body: its answer, or the line for why there is none.
    fn provider_body(&self, id: &str, name: &str) -> Body {
        use crate::providers::Standing;
        use isle_providers::Data;
        use isle_shell::dashboard::surface::{ProviderData, ProviderView};
        let strip = |glyph: Option<&str>, line: String, button: Option<(&str, &str)>| Body::Strip {
            glyph: glyph.map(str::to_owned),
            line,
            button: button.map(|(l, r)| (l.to_owned(), r.to_owned())),
        };
        let installed = match self.standing(id) {
            Standing::NotInstalled => {
                return strip(None, format!("“{name}” isn't installed."), Some(("Remove", "remove")));
            }
            Standing::NotAllowed(_) => {
                return strip(None, format!("“{name}” isn't allowed to run."), Some(("Allow…", "allow")));
            }
            Standing::Runs(p) => p,
        };
        let error = self.dashboard.answers.get(id).and_then(|a| a.error.clone());
        let Some((data, at)) = self.provider_data(id) else {
            return match error {
                Some(reason) => strip(Some("warning-circle"), reason, None),
                None => Body::Title,
            };
        };
        let interval = installed.manifest.interval.unwrap_or(60);
        let age = now_secs().saturating_sub(at);
        let updated = (!installed.manifest.stream && age > 2 * interval).then(|| {
            let when = jiff::Timestamp::from_second(at as i64)
                .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%H:%M").to_string())
                .unwrap_or_default();
            format!("Updated {when}")
        });
        let titled = !self
            .settings
            .widget
            .get(id)
            .and_then(|w| w.get("title"))
            .is_some_and(|v| *v == isle_settings::Value::Bool(false));
        let data = match data {
            Data::Text { text } => ProviderData::Text(text),
            Data::Number { value, unit, label } => ProviderData::Number { value, unit, label },
            Data::Gauge { value, label, text } => ProviderData::Gauge { value, label, text },
            Data::Sparkline { values, value } => ProviderData::Sparkline { values, value },
            Data::Graph { series } => ProviderData::Graph(series.into_iter().map(|s| (s.label, s.values)).collect()),
            Data::List { items } => ProviderData::List(items.into_iter().map(|i| (i.label, i.value, i.glyph)).collect()),
            Data::Card { title, body, image } => ProviderData::Card {
                title,
                body,
                // Only a picture in its own folders is drawn.
                image: image
                    .map(|i| installed.folder.join(i))
                    .filter(|p| p.starts_with(&installed.folder) && p.is_file()),
            },
        };
        Body::Provider(ProviderView {
            title: titled.then(|| name.to_owned()),
            updated,
            data,
        })
    }

    /// A System widget's four series from the samples so far.
    fn series_views(&self, id: &str) -> Vec<SeriesView> {
        use isle_shell::dashboard::catalogue::{SERIES, SettingKind, builtin};
        let Some(now) = self.dashboard.system.last() else {
            return Vec::new();
        };
        let history = &self.dashboard.system;
        let net_peak = history.iter().map(|s| s.down.max(s.up)).max().unwrap_or(1).max(1) as f32;
        builtin("system")
            .map_or(&[][..], |b| b.settings)
            .iter()
            .map(|setting| {
                let default = match setting.kind {
                    SettingKind::Choice(_, d) => d,
                    _ => "cpu",
                };
                let kind = match self.settings.widget.get(id).and_then(|w| w.get(setting.key)) {
                    Some(isle_settings::Value::Str(v)) if SERIES.iter().any(|(k, _)| k == v) => v.as_str(),
                    _ => default,
                };
                // Short where the form's name would be cut.
                let label = match kind {
                    "cpu-temp" => "Temperature",
                    "gpu-temp" => "GPU temp",
                    "gpu-memory" => "GPU memory",
                    _ => SERIES.iter().find(|(k, _)| *k == kind).map_or(kind, |(_, l)| l),
                };
                let of = |s: &isle_sysinfo::Sample| -> Option<f32> {
                    match kind {
                        "cpu" => Some(s.cpu),
                        "memory" => Some(s.memory()),
                        "gpu" => s.gpu,
                        "gpu-memory" => s.gpu_memory.map(|(u, t)| u as f32 / t.max(1) as f32),
                        "cpu-temp" => s.cpu_temp.map(|c| c / 100.0),
                        "gpu-temp" => s.gpu_temp.map(|c| c / 100.0),
                        "down" => Some(s.down as f32 / net_peak),
                        "up" => Some(s.up as f32 / net_peak),
                        _ => None,
                    }
                };
                let percent = |v: Option<f32>| v.map_or("—".to_owned(), |v| format!("{:.0}%", v * 100.0));
                let degrees = |v: Option<f32>| v.map_or("—".to_owned(), |v| format!("{v:.0}°"));
                let value = match kind {
                    "cpu" => percent(Some(now.cpu)),
                    "memory" => percent(Some(now.memory())),
                    "gpu" => percent(now.gpu),
                    "gpu-memory" => percent(now.gpu_memory.map(|(u, t)| u as f32 / t.max(1) as f32)),
                    "cpu-temp" => degrees(now.cpu_temp),
                    "gpu-temp" => degrees(now.gpu_temp),
                    "down" => format!("{}/s", bytes(now.down)),
                    "up" => format!("{}/s", bytes(now.up)),
                    _ => {
                        let (d, h, m) = (now.uptime / 86400, now.uptime / 3600 % 24, now.uptime / 60 % 60);
                        if d > 0 { format!("{d} d {h} h") } else { format!("{h} h {m} m") }
                    }
                };
                SeriesView {
                    label: (*label).to_owned(),
                    value,
                    fraction: of(now),
                    history: history.iter().map(|s| of(s).unwrap_or(0.0)).collect(),
                }
            })
            .collect()
    }

    /// The modules a Controls widget shows: its own list, else Sound and Display.
    fn control_modules(&self, id: &str) -> Vec<isle_shell::island::panel::Module> {
        use isle_shell::island::grid::TileSize;
        let list: Vec<String> = match self.settings.widget.get(id).and_then(|w| w.get("modules")) {
            Some(isle_settings::Value::List(l)) => l.clone(),
            Some(isle_settings::Value::Str(one)) => vec![one.clone()],
            _ => vec!["sound:w".into(), "display:w".into()],
        };
        list.iter()
            .filter_map(|entry| {
                let (id, size) = entry.split_once(':').unwrap_or((entry, "m"));
                let size = match size {
                    "s" => TileSize::Small,
                    "l" => TileSize::Large,
                    "w" => TileSize::Wide,
                    _ => TileSize::Medium,
                };
                self.module_of(id, size)
            })
            .collect()
    }

    pub fn dashboard_frame(&self) -> Option<Frame> {
        let open = self.dashboard.open.as_ref()?;
        let widgets = open
            .layout
            .widgets
            .iter()
            .map(|p| {
                let name = self.widget_name(&p.id);
                let body = self.widget_body(&p.id, &name, &open.output);
                WidgetView {
                    id: p.id.clone(),
                    name,
                    at: p.at,
                    body,
                }
            })
            .collect();
        let dragging = open.press.as_ref().filter(|p| p.moved);
        let lifted = dragging.filter(|p| p.part == Part::Body).map(|p| Lifted {
            id: p.id.clone(),
            x: p.at.0 - p.grab.0,
            y: p.at.1 - p.grab.1,
        });
        let ghost = dragging.map(|p| Ghost {
            at: p.ghost,
            refused: p.plan.is_none(),
        });
        let menu = open.menu.as_ref().map(|m| MenuView {
            entries: std::rc::Rc::new(self.dashboard_menu_entries(open, m.on.as_deref())),
            x: m.at.0,
            y: m.at.1,
            by_key: m.by_key,
        });
        let form = open.form.as_ref().map(|id| self.dashboard_form(id));
        let gallery = open.gallery.as_ref().map(|g| self.gallery_view(open, g));
        let consent = open.consent.as_ref().map(|c| self.consent_view(c));
        let mut frame = Frame {
            extent: open.extent,
            widgets,
            edit: open.edit,
            selected: open.selected.clone(),
            ring: open.ring,
            solid: self.game_mode_active() || self.shell.theme().appearance.reduce_transparency,
            lifted,
            ghost,
            menu,
            form,
            gallery,
            consent,
        };
        // A widget dragged from the gallery is drawn under the pointer.
        if let Some(p) = dragging.filter(|p| p.adding.is_some())
            && let Some((w, h)) = p.adding
        {
            let name = self.widget_name(&p.id);
            frame.widgets.push(WidgetView {
                id: p.id.clone(),
                body: self.widget_body(&p.id, &name, &open.output),
                name,
                at: Cells::new(p.ghost.x, p.ghost.y, w, h),
            });
        }
        if dragging.is_some_and(|p| p.over_gallery) {
            frame.ghost = None;
        }
        if let Some(g) = frame.gallery.as_mut() {
            g.faded = dragging.is_some_and(|p| p.adding.is_some() && !p.over_gallery);
        }
        // While a drag or resize is planned, the rest are drawn where it would put them.
        if let Some(plan) = dragging.and_then(|p| p.plan.as_ref()) {
            for w in &mut frame.widgets {
                if let Some(p) = plan.get(&w.id) {
                    w.at = p.at;
                }
            }
        }
        Some(frame)
    }

    fn dashboard_menu_entries(&self, open: &Open, on: Option<&str>) -> Vec<MenuEntry> {
        let item = |id, label: &str| MenuEntry::Item(MenuItem::new(id, label));
        if on.is_some_and(|o| o.starts_with("gallery:")) {
            return vec![item(MENU_ADD, "Add"), item(MENU_DETAILS, "Details")];
        }
        let Some(placed) = on.and_then(|id| open.layout.get(id)) else {
            return vec![item(MENU_EDIT, "Edit widgets"), item(MENU_CLOSE, "Close widgets")];
        };
        let name = self.widget_name(&placed.id);
        let mut entries = vec![item(MENU_EDIT_ONE, &format!("Edit “{name}”…")), MenuEntry::Separator];
        if self.dashboard.answers.get(&placed.id).is_some_and(|a| a.error.is_some() || a.stopped) {
            entries.insert(0, item(MENU_TRY_AGAIN, "Try Again"));
        }
        for (i, (w, h)) in self.widget_sizes(&placed.id).all.iter().enumerate() {
            let mut size = MenuItem::new(MENU_SIZE + i as u64, format!("{w} × {h}"));
            if (placed.at.w, placed.at.h) == (*w, *h) {
                size.mark = isle_ui::menu::Mark::Check;
            }
            entries.push(MenuEntry::Item(size));
        }
        entries.push(MenuEntry::Separator);
        entries.push(item(MENU_REMOVE, "Remove widget"));
        if !open.edit {
            entries.push(item(MENU_EDIT, "Edit widgets"));
        }
        entries
    }

    /// A widget's settings as its form's rows, from its manifest and `settings.kdl`.
    fn dashboard_form(&self, id: &str) -> FormView {
        use isle_settings::Value;
        use isle_shell::dashboard::catalogue::SettingKind;
        let stored = self.settings.widget.get(id);
        if catalogue::builtin(kind_of(id)).is_none() {
            return FormView {
                id: id.to_owned(),
                rows: self.provider_form(id),
            };
        }
        let rows = catalogue::builtin(kind_of(id))
            .map_or(&[][..], |b| b.settings)
            .iter()
            .map(|s| {
                let value = stored.and_then(|o| o.get(s.key));
                let control = match s.kind {
                    SettingKind::Toggle(default) => Control::Toggle(match value {
                        Some(Value::Bool(b)) => *b,
                        _ => default,
                    }),
                    SettingKind::Choice(options, default) => Control::Choice {
                        options: options.iter().map(|(_, l)| (*l).to_owned()).collect(),
                        selected: {
                            let chosen = match value {
                                Some(Value::Str(v)) => v.as_str(),
                                _ => default,
                            };
                            options.iter().position(|(k, _)| *k == chosen).unwrap_or(0)
                        },
                    },
                    SettingKind::Number { min, max, default } => Control::Number {
                        value: match value {
                            Some(Value::Int(i)) => (*i).clamp(min, max),
                            _ => default,
                        },
                        min,
                        max,
                    },
                    SettingKind::Text => Control::Text(match value {
                        Some(Value::Str(v)) => v.clone(),
                        _ => String::new(),
                    }),
                };
                FormRow {
                    key: s.key.to_owned(),
                    label: s.label.to_owned(),
                    control,
                }
            })
            .collect();
        FormView {
            id: id.to_owned(),
            rows,
        }
    }
}

impl Isle {
    /// The dashboard's model for the Test interface.
    pub fn dashboard_status(&self) -> String {
        let open = self.dashboard.open.as_ref();
        serde_json::json!({
            "open": open.is_some(),
            "output": open.map(|o| o.output.clone()),
            "identity": open.map(|o| o.identity.clone()),
            "edit": open.is_some_and(|o| o.edit),
            "selected": open.and_then(|o| o.selected.clone()),
            "ring": open.is_some_and(|o| o.ring),
            "grid": open.map(|o| [o.extent.cols, o.extent.rows]),
            "layout": open.map(|o| o.layout.to_lines()),
            "bodies": self.dashboard_frame().map(|f| {
                f.widgets
                    .iter()
                    .map(|w| (w.id.clone(), body_summary(&w.body)))
                    .collect::<std::collections::BTreeMap<_, _>>()
            }),
            "menu": open.and_then(|o| o.menu.as_ref()).map(|m| m.on.clone()),
            "form": open.and_then(|o| o.form.clone()),
            "dragging": open.and_then(|o| o.press.as_ref()).is_some_and(|p| p.moved),
            "running": self.dashboard.running.len(),
            "updates": self.updates().iter().map(|u| u.0.clone()).collect::<Vec<_>>(),
            "consent": open.and_then(|o| o.consent.as_ref()).map(|c| {
                let v = self.consent_view(c);
                serde_json::json!({ "title": v.title, "armed": v.armed, "permissions": v.permissions.iter().map(|p| &p.1).collect::<Vec<_>>() })
            }),
            "browse": self.dashboard.browse.iter().map(|(u, r)| (u.clone(), r.as_ref().map(|i| i.widgets.iter().map(|l| l.id.clone()).collect::<Vec<_>>()).map_err(Clone::clone))).collect::<BTreeMap<_, _>>(),
            "providers": self.dashboard.providers.keys().cloned().collect::<Vec<_>>(),
            "gallery": open.and_then(|o| o.gallery.as_ref()).map(|g| {
                let view = self.gallery_view(open.unwrap(), g);
                serde_json::json!({
                    "section": g.section,
                    "query": g.query,
                    "focused": g.focused,
                    "details": g.details,
                    "items": view.groups.iter().flat_map(|gr| gr.items.iter().map(|i| (i.key.clone(), i.added, i.no_room))).collect::<Vec<_>>(),
                })
            }),
            "backdrop": self.dashboard.backdrop.value(),
            "shown_on": self.dashboard.shown_on.clone(),
        })
        .to_string()
    }
}

impl Isle {
    /// A placed widget's name: a built-in's, or its provider's.
    pub fn widget_name(&self, id: &str) -> String {
        let kind = kind_of(id);
        match (catalogue::builtin(kind), self.provider_of(id)) {
            (Some(b), _) => b.name.to_owned(),
            (None, Some(p)) => p.manifest.name.clone(),
            _ => kind.to_owned(),
        }
    }

    /// The sizes a placed widget comes in.
    pub fn widget_sizes(&self, id: &str) -> Sizes {
        if let Some(b) = catalogue::builtin(kind_of(id)) {
            return b.sizes();
        }
        match self.provider_of(id) {
            Some(p) => {
                let all = p.manifest.cells();
                let default = isle_providers::manifest::size(&p.manifest.default).unwrap_or(all[0]);
                Sizes::new(&all, default)
            }
            None => Sizes::new(&[(2, 1)], (2, 1)),
        }
    }
}

/// What a widget shows, for the Test interface.
fn body_summary(body: &Body) -> serde_json::Value {
    use serde_json::json;
    match body {
        Body::Title => json!("title"),
        Body::Empty(line) => json!({ "empty": line }),
        Body::Clock(c) => json!({ "clock": c.city, "time": c.time, "dial": c.dial.is_some() }),
        Body::Session => json!("session"),
        Body::Media(card) => json!({ "media": card.title }),
        Body::Devices(rows) => json!({ "devices": rows.iter().map(|r| &r.name).collect::<Vec<_>>() }),
        Body::Storage(rows) => json!({ "storage": rows.iter().map(|r| (&r.name, &r.free)).collect::<Vec<_>>() }),
        Body::Desktops(rows) => json!({ "desktops": rows.iter().map(|r| (&r.name, r.icons.len(), r.current)).collect::<Vec<_>>() }),
        Body::Notifications(rows) => json!({ "notifications": rows.iter().map(|r| (&r.name, r.count)).collect::<Vec<_>>() }),
        Body::Controls(modules) => json!({ "controls": modules.iter().map(|m| &m.id).collect::<Vec<_>>() }),
        Body::Weather(w) => json!({ "weather": w.place, "now": w.now }),
        Body::Timer(t) => json!({ "timer": t.time, "state": format!("{:?}", t.state) }),
        Body::Strip { line, .. } => json!({ "strip": line }),
        Body::Provider(p) => json!({ "provider": format!("{:?}", p.data), "updated": p.updated }),
        Body::System(series) => json!({ "system": series.iter().map(|s| (&s.label, &s.value)).collect::<Vec<_>>() }),
    }
}

/// A zone's city as people write it: `America/New_York` is New York.
fn city_of(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

impl State {
    /// The chord, the CLI and Spotlight: the dashboard on the screen with the focused
    /// window, else the pointer's, or closed where it is open. A hot corner names its screen.
    pub fn toggle_dashboard(&mut self, on: Option<Output>, edit: bool, corner: bool) {
        if let Some(open) = &self.isle.dashboard.open {
            let same = on.as_ref().is_none_or(|o| o.name() == open.output);
            if same && edit && !open.edit {
                return self.dashboard_edit(true);
            }
            if same && (!corner || open.corner) {
                return self.close_dashboard();
            }
        }
        let output = on
            .or_else(|| self.isle.output_with_focus())
            .or_else(|| self.isle.active_output())
            .or_else(|| self.isle.space.outputs().next().cloned());
        if let Some(output) = output {
            self.open_dashboard(&output, edit, corner);
        }
    }

    pub fn open_dashboard(&mut self, output: &Output, edit: bool, corner: bool) {
        if self.isle.lock.locked || self.isle.prompts_hold_keyboard() {
            return;
        }
        if self.isle.dashboard.open.is_some() {
            self.close_dashboard();
        }
        self.close_overlays(Overlay::Dashboard);
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        self.isle.dashboard.providers = crate::providers::scan();
        let identity = Isle::output_identity(output);
        let extent = self.isle.dashboard_extent(output);
        let layout = self.isle.dashboard_layout(&identity, &extent);
        info!(output = output.name(), edit, "widgets");
        self.isle.dashboard.open = Some(Open {
            output: output.name(),
            identity,
            extent,
            layout,
            edit,
            selected: None,
            ring: false,
            history: History::default(),
            corner,
            press: None,
            menu: None,
            form: None,
            gallery: edit.then(Gallery::default),
            consent: None,
        });
        let screen = self.isle.space.output_geometry(output).unwrap_or_default();
        self.isle.dashboard.surface.set_size(screen.size);
        self.put_dashboard_frame();
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle
            .shell
            .open_placed("dashboard", &place, Rectangle::from_size(screen.size));
        if let Some(focus) = self.isle.shell.keyboard_taker("dashboard") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle.shell_keyboard_return.push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        let theme = self.isle.shell.theme().clone();
        let still = !self.isle.shell.animated()
            || theme.appearance.reduce_motion
            || self.isle.game_mode_active();
        if self.isle.dashboard.shown_on.as_deref() != Some(&output.name()) {
            self.isle.dashboard.backdrop = Motion::at(0.0);
        }
        self.isle.dashboard.shown_on = Some(output.name());
        if still {
            self.isle.dashboard.backdrop = Motion::at(1.0);
        } else {
            self.isle
                .dashboard
                .backdrop
                .fade_to(1.0, Duration::from_millis(theme.tokens.motion_fade_ms as u64));
        }
        self.isle.dashboard.last_step = Instant::now();
        self.arm_dashboard_tick();
        self.dashboard_feeds();
        self.isle.refresh_island();
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    pub fn close_dashboard(&mut self) {
        let Some(_) = self.isle.dashboard.open.take() else {
            return;
        };
        if let Some(token) = self.isle.dashboard.tick.take() {
            self.isle.loop_handle.remove(token);
        }
        if let Some(token) = self.isle.dashboard.sample_tick.take() {
            self.isle.loop_handle.remove(token);
        }
        self.isle.dashboard.running.clear();
        let theme = self.isle.shell.theme().clone();
        let still = !self.isle.shell.animated()
            || theme.appearance.reduce_motion
            || self.isle.game_mode_active();
        if still {
            self.isle.dashboard.backdrop = Motion::at(0.0);
            self.isle.dashboard.shown_on = None;
        } else {
            self.isle.dashboard.backdrop.fade_to(
                0.0,
                Duration::from_millis(theme.tokens.motion_exit_fade_ms as u64),
            );
        }
        self.isle.dashboard.last_step = Instant::now();
        self.close_surface("dashboard");
        self.isle.refresh_island();
        self.isle.queue_redraw_all();
    }

    /// Moves the backdrop's fade on; its end on closing stops drawing it.
    pub fn step_dashboard(&mut self, now: Instant) {
        let d = &mut self.isle.dashboard;
        if d.shown_on.is_none() {
            return;
        }
        let dt = now.duration_since(d.last_step).as_secs_f64().min(0.1);
        d.last_step = now;
        d.backdrop.step(now, dt);
        if d.open.is_none() && d.backdrop.settled() {
            d.shown_on = None;
        }
        if !d.backdrop.settled() {
            self.isle.queue_redraw_all();
        }
    }

    pub fn put_dashboard_frame(&mut self) {
        if let Some(frame) = self.isle.dashboard_frame() {
            self.isle.dashboard.surface.set(frame);
            self.isle.shell.with_root("dashboard", || {});
            self.isle.queue_redraw_all();
        }
    }

    /// Wakes on the next minute while open, for the clocks.
    fn arm_dashboard_tick(&mut self) {
        if let Some(token) = self.isle.dashboard.tick.take() {
            self.isle.loop_handle.remove(token);
        }
        let wall = jiff::Timestamp::now();
        let into = wall.as_second().rem_euclid(60) as f64 + wall.subsec_nanosecond() as f64 / 1e9;
        let next = Duration::from_secs_f64(60.0 - into + 0.001);
        self.isle.dashboard.tick = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(next), |_, _, state| {
                state.isle.dashboard.tick = None;
                if state.isle.dashboard.open.is_some() {
                    state.put_dashboard_frame();
                    state.arm_dashboard_tick();
                }
                TimeoutAction::Drop
            })
            .ok();
    }

    pub fn dashboard_edit(&mut self, on: bool) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if open.edit == on {
            return;
        }
        open.edit = on;
        open.history.clear();
        open.press = None;
        open.menu = None;
        open.form = None;
        open.gallery = on.then(Gallery::default);
        if !on {
            self.forget_removed_widgets();
        }
        self.put_dashboard_frame();
    }

    pub fn dashboard_action(&mut self, action: DashboardAction) {
        match action {
            DashboardAction::Close => self.close_dashboard(),
            DashboardAction::Edit => self.dashboard_edit(true),
            DashboardAction::Done => self.dashboard_edit(false),
            DashboardAction::Gallery
            | DashboardAction::GallerySection(_)
            | DashboardAction::GalleryPress(_)
            | DashboardAction::GallerySecondary(_)
            | DashboardAction::GalleryDetails(_)
            | DashboardAction::GalleryButton { .. } => self.gallery_action(action),
            DashboardAction::ConsentConfirm | DashboardAction::ConsentCancel => {
                crate::widget_sources::consent_action(self, &action);
            }
            DashboardAction::Open(id) => self.dashboard_open_widget(&id),
            DashboardAction::Menu(event) => self.dashboard_menu_event(event),
            DashboardAction::Row { id, row } => self.dashboard_row(&id, &row),
            DashboardAction::Form { id, key, value } => self.dashboard_form_set(&id, &key, value),
            DashboardAction::FormClosed => {
                if let Some(open) = self.isle.dashboard.open.as_mut() {
                    open.form = None;
                }
                self.put_dashboard_frame();
            }
            DashboardAction::Session(choice) => {
                self.close_dashboard();
                match choice {
                    Choice::Lock => self.lock("widgets"),
                    Choice::Sleep => self.isle.suspend(),
                    other => self.open_session_confirm(other),
                }
            }
        }
    }

    /// What a widget's body opens, where it has one; until each widget has its place, none.
    fn dashboard_open_widget(&mut self, id: &str) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if open.edit {
            open.selected = Some(id.to_owned());
            open.ring = false;
            self.put_dashboard_frame();
        }
    }

    /// A key while the dashboard has the keyboard; true when it took it.
    pub fn dashboard_key(&mut self, code: u32) -> bool {
        use smithay::input::keyboard::keysyms;
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return false;
        };
        let focus = self.isle.shell.keyboard_taker("dashboard");
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        if focus.is_none() || keyboard.current_focus() != focus.map(Focus::Shell) {
            return false;
        }
        let mods = self.held_mods();
        let sym = self
            .isle
            .keyboards
            .with_engine(false, &[], |_, ctx| ctx.keys.sym(code))
            .map_or(0, |s| s.raw());
        if let Some(c) = open.consent.as_ref() {
            let armed = c.armed;
            match sym {
                keysyms::KEY_Escape => self.consent_answer(false),
                keysyms::KEY_Return | keysyms::KEY_KP_Enter if armed => self.consent_answer(true),
                _ => {}
            }
            return true;
        }
        // An open menu and the form take their own keys; Escape closes the form.
        if open.menu.is_some() {
            return false;
        }
        if open.form.is_some() {
            if sym == keysyms::KEY_Escape && !mods.ctrl && !mods.cmd && !mods.option {
                self.dashboard_action(DashboardAction::FormClosed);
                return true;
            }
            return false;
        }
        if mods.ctrl {
            return false;
        }
        if !mods.cmd && !mods.option && self.gallery_key(sym, code) {
            return true;
        }
        let open = self.isle.dashboard.open.as_ref().unwrap();
        let edit = open.edit;
        if edit && mods.cmd && !mods.option {
            let section = match sym {
                keysyms::KEY_1 => Some(0),
                keysyms::KEY_2 => Some(1),
                keysyms::KEY_3 => Some(2),
                _ => None,
            };
            if let Some(i) = section {
                self.gallery_action(DashboardAction::GallerySection(i));
                return true;
            }
            if sym == keysyms::KEY_f {
                if let Some(open) = self.isle.dashboard.open.as_mut() {
                    open.gallery.get_or_insert_with(Gallery::default).focused = true;
                }
                self.put_dashboard_frame();
                return true;
            }
        }
        if edit && !mods.cmd && !mods.option && sym == keysyms::KEY_Tab {
            if let Some(open) = self.isle.dashboard.open.as_mut() {
                open.gallery.get_or_insert_with(Gallery::default).focused = true;
            }
            self.put_dashboard_frame();
            return true;
        }
        let arrow = match sym {
            keysyms::KEY_Left => Some((-1, 0)),
            keysyms::KEY_Right => Some((1, 0)),
            keysyms::KEY_Up => Some((0, -1)),
            keysyms::KEY_Down => Some((0, 1)),
            _ => None,
        };
        if mods.cmd {
            match (arrow, sym) {
                (Some((dx, dy)), _) if edit && mods.option => self.dashboard_resize_by(dx, dy),
                (Some((dx, dy)), _) if edit => self.dashboard_nudge(dx, dy),
                (_, keysyms::KEY_z | keysyms::KEY_Z) if edit && mods.shift => self.dashboard_redo(),
                (_, keysyms::KEY_z | keysyms::KEY_Z) if edit => self.dashboard_undo(),
                _ => return false,
            }
            return true;
        }
        if mods.option {
            return false;
        }
        match sym {
            keysyms::KEY_Escape => {
                if edit {
                    self.dashboard_edit(false);
                } else {
                    self.close_dashboard();
                }
            }
            keysyms::KEY_e | keysyms::KEY_E if !edit => self.dashboard_edit(true),
            _ if arrow.is_some() => {
                let (dx, dy) = arrow.unwrap();
                self.dashboard_select(dx, dy);
            }
            keysyms::KEY_Home => self.dashboard_select_end(false),
            keysyms::KEY_End => self.dashboard_select_end(true),
            keysyms::KEY_Return | keysyms::KEY_KP_Enter => {
                if let Some(id) = open.selected.clone() {
                    if edit {
                        self.dashboard_open_form(&id);
                    } else {
                        self.dashboard_open_widget(&id);
                    }
                }
            }
            keysyms::KEY_Delete | keysyms::KEY_BackSpace if edit => {
                if let Some(id) = open.selected.clone() {
                    self.dashboard_remove(&id);
                }
            }
            keysyms::KEY_Menu => {
                let on = open.selected.clone();
                self.dashboard_open_menu(on, None, true);
            }
            keysyms::KEY_F10 if mods.shift => {
                let on = open.selected.clone();
                self.dashboard_open_menu(on, None, true);
            }
            _ => return false,
        }
        true
    }

    /// The selection moved to the nearest widget in a direction, by position; the first in
    /// reading order when there is none.
    fn dashboard_select(&mut self, dx: i32, dy: i32) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let reading = reading_order(&open.layout);
        let Some(from) = open.selected.as_ref().and_then(|id| open.layout.get(id)) else {
            open.selected = reading.first().cloned();
            return self.put_dashboard_frame();
        };
        let centre = |c: isle_shell::dashboard::grid::Cells| (2 * c.x + c.w, 2 * c.y + c.h);
        let (fx, fy) = centre(from.at);
        let best = open
            .layout
            .widgets
            .iter()
            .filter(|p| p.id != from.id)
            .filter_map(|p| {
                let (x, y) = centre(p.at);
                let (along, across) = if dx != 0 { ((x - fx) * dx, (y - fy).abs()) } else { ((y - fy) * dy, (x - fx).abs()) };
                (along > 0).then_some((along + 2 * across, p.id.clone()))
            })
            .min();
        if let Some((_, id)) = best {
            open.selected = Some(id);
        }
        self.put_dashboard_frame();
    }

    fn dashboard_select_end(&mut self, last: bool) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let reading = reading_order(&open.layout);
        open.selected = if last { reading.last() } else { reading.first() }.cloned();
        self.put_dashboard_frame();
    }
}

/// Widgets by row, then column, of their top left.
fn reading_order(layout: &Layout) -> Vec<String> {
    let mut all: Vec<_> = layout.widgets.iter().map(|p| (p.at.y, p.at.x, p.id.clone())).collect();
    all.sort();
    all.into_iter().map(|(_, _, id)| id).collect()
}

impl State {
    /// Writes the layout as it is now and refreshes the surface.
    fn dashboard_changed(&mut self, before: Layout) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if open.layout == before {
            return self.put_dashboard_frame();
        }
        open.history.did(before);
        self.dashboard_feeds();
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return;
        };
        let key = format!("dashboard-output[{}].widgets", open.identity);
        let lines = open.layout.to_lines();
        self.isle.write_setting(&key, isle_settings::Value::List(lines));
        self.put_dashboard_frame();
    }

    fn dashboard_undo(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if let Some(layout) = open.history.undo(&open.layout.clone()) {
            open.layout = layout;
            self.dashboard_write();
        }
    }

    fn dashboard_redo(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        if let Some(layout) = open.history.redo(&open.layout.clone()) {
            open.layout = layout;
            self.dashboard_write();
        }
    }

    fn dashboard_write(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return;
        };
        let key = format!("dashboard-output[{}].widgets", open.identity);
        self.isle
            .write_setting(&key, isle_settings::Value::List(open.layout.to_lines()));
        self.put_dashboard_frame();
    }

    /// Cmd+arrows: the selected widget a cell along, pushing as a drag does.
    fn dashboard_nudge(&mut self, dx: i32, dy: i32) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let Some(placed) = open.selected.as_ref().and_then(|id| open.layout.get(id)).cloned() else {
            return;
        };
        let to = Cells::new(placed.at.x + dx, placed.at.y + dy, placed.at.w, placed.at.h);
        let before = open.layout.clone();
        if let Some(after) = before.push(&placed.id, to, Dir::of(dx, dy), &open.extent) {
            open.layout = after;
        }
        self.dashboard_changed(before);
    }

    /// Option+Cmd+arrows: Right and Down grow it, Left and Up shrink it, through its sizes.
    fn dashboard_resize_by(&mut self, dx: i32, dy: i32) {
        let Some(placed) = self
            .isle
            .dashboard
            .open
            .as_ref()
            .and_then(|o| o.selected.as_ref().and_then(|id| o.layout.get(id)))
            .cloned()
        else {
            return;
        };
        let sizes = self.isle.widget_sizes(&placed.id);
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.ring = true;
        let Some(size) = sizes.step((placed.at.w, placed.at.h), dx, dy) else {
            return;
        };
        let before = open.layout.clone();
        if let Some(after) = before.resize(&placed.id, size, &open.extent) {
            open.layout = after;
        }
        self.dashboard_changed(before);
    }

    pub fn dashboard_remove(&mut self, id: &str) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        let before = open.layout.clone();
        open.layout.remove(id);
        if open.selected.as_deref() == Some(id) {
            open.selected = None;
        }
        if open.form.as_deref() == Some(id) {
            open.form = None;
        }
        self.dashboard_changed(before);
    }

    fn dashboard_resize_to(&mut self, id: &str, size: (i32, i32)) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        let before = open.layout.clone();
        if let Some(after) = before.resize(id, size, &open.extent) {
            open.layout = after;
        }
        self.dashboard_changed(before);
    }

    /// Settings of widgets no screen's layout holds any more, taken out on leaving edit mode.
    fn forget_removed_widgets(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return;
        };
        let placed = |id: &str| {
            open.layout.get(id).is_some()
                || self.isle.settings.dashboard_output.iter().any(|(identity, d)| {
                    identity != &open.identity
                        && d.widgets
                            .as_ref()
                            .is_some_and(|l| l.iter().any(|line| line.split_whitespace().next() == Some(id)))
                })
        };
        let gone: Vec<String> = self.isle.settings.widget.keys().filter(|id| !placed(id)).cloned().collect();
        for id in gone {
            self.isle.remove_setting(&format!("widget[{id}]"));
        }
    }

    pub fn dashboard_open_form(&mut self, id: &str) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        open.selected = Some(id.to_owned());
        open.menu = None;
        let has = catalogue::builtin(kind_of(id)).is_none_or(|b| !b.settings.is_empty());
        open.form = has.then(|| id.to_owned());
        self.put_dashboard_frame();
    }

    fn dashboard_form_set(&mut self, id: &str, key: &str, value: FormValue) {
        use isle_settings::Value;
        use isle_shell::dashboard::catalogue::SettingKind;
        if catalogue::builtin(kind_of(id)).is_none() {
            return self.provider_form_set(id, key, value);
        }
        let Some(setting) = catalogue::builtin(kind_of(id))
            .and_then(|b| b.settings.iter().find(|s| s.key == key))
        else {
            return;
        };
        let value = match (setting.kind, value) {
            (SettingKind::Toggle(_), FormValue::Bool(b)) => Value::Bool(b),
            (SettingKind::Choice(options, _), FormValue::Index(i)) => match options.get(i) {
                Some((k, _)) => Value::Str((*k).to_owned()),
                None => return,
            },
            (SettingKind::Number { min, max, .. }, FormValue::Int(n)) => Value::Int(n.clamp(min, max)),
            (SettingKind::Text, FormValue::Text(t)) => {
                // A zone that jiff does not know is left as it was.
                if key == "zone" && !t.is_empty() && jiff::tz::TimeZone::get(&t).is_err() {
                    return;
                }
                Value::Str(t)
            }
            _ => return,
        };
        self.isle.write_setting(&format!("widget[{id}].{key}"), value);
    }

    /// A widget's menu, or the gap's with none; at `at` in points on the output, or by the
    /// widget when opened by a key.
    pub fn dashboard_open_menu(&mut self, on: Option<String>, at: Option<(f32, f32)>, by_key: bool) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        let at = at.unwrap_or_else(|| {
            on.as_ref()
                .and_then(|id| open.layout.get(id))
                .map(|p| {
                    let (x, y, w, _) = open.extent.rect(p.at);
                    (x + w / 2.0, y + 24.0)
                })
                .unwrap_or((open.extent.x, open.extent.y))
        });
        if on.is_some() {
            open.selected = on.clone();
        }
        open.form = None;
        open.menu = Some(Menu { on, at, by_key });
        self.put_dashboard_frame();
    }

    fn dashboard_menu_event(&mut self, event: MenuEvent) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        match event {
            MenuEvent::Chosen(item) => {
                let on = open.menu.take().and_then(|m| m.on);
                let gallery_key = on.as_ref().and_then(|o| o.strip_prefix("gallery:")).map(str::to_owned);
                if let Some(key) = gallery_key {
                    match item {
                        MENU_ADD => self.gallery_add(&key),
                        MENU_DETAILS => self.gallery_action(DashboardAction::GalleryDetails(Some(key))),
                        _ => self.put_dashboard_frame(),
                    }
                    return;
                }
                match (item, on) {
                    (MENU_EDIT_ONE, Some(id)) => {
                        self.dashboard_edit(true);
                        self.dashboard_open_form(&id);
                    }
                    (MENU_REMOVE, Some(id)) => self.dashboard_remove(&id),
                    (MENU_TRY_AGAIN, Some(id)) => self.provider_try_again(&id),
                    (MENU_EDIT, _) => self.dashboard_edit(true),
                    (MENU_CLOSE, _) => self.close_dashboard(),
                    (n, Some(id)) if n >= MENU_SIZE => {
                        if let Some(size) = self.isle.widget_sizes(&id).all.get((n - MENU_SIZE) as usize).copied() {
                            self.dashboard_resize_to(&id, size);
                        }
                    }
                    _ => self.put_dashboard_frame(),
                }
            }
            MenuEvent::Back => {
                open.menu = None;
                self.put_dashboard_frame();
            }
            _ => {}
        }
    }

    /// The output under `at` when the dashboard is open there, with the point on it.
    fn dashboard_point(&self, at: Point<f64, Logical>) -> Option<(f32, f32)> {
        let open = self.isle.dashboard.open.as_ref()?;
        let output = self.isle.space.outputs().find(|o| o.name() == open.output)?;
        let g = self.isle.space.output_geometry(output)?;
        g.to_f64()
            .contains(at)
            .then(|| ((at.x - g.loc.x as f64) as f32, (at.y - g.loc.y as f64) as f32))
    }

    /// The widget and part under a point, in edit mode's terms.
    fn dashboard_hit(&self, (px, py): (f32, f32)) -> Option<(String, Part)> {
        let open = self.isle.dashboard.open.as_ref()?;
        let band = self.isle.shell.theme().tokens.size_resize_band;
        open.layout.widgets.iter().rev().find_map(|p| {
            let (x, y, w, h) = open.extent.rect(p.at);
            let inside = px >= x && px < x + w && py >= y && py < y + h;
            if !open.edit {
                return inside.then(|| (p.id.clone(), Part::Body));
            }
            // The badge's box reaches past the plate's corner.
            if (px - (x + 15.0)).abs() <= 12.0 && (py - (y + 15.0)).abs() <= 12.0 {
                return Some((p.id.clone(), Part::Badge));
            }
            if !inside {
                return None;
            }
            if px >= x + w - 24.0 && py >= y + h - 24.0 {
                return Some((p.id.clone(), Part::Resize { left: false, right: true, top: false, bottom: true }));
            }
            let (left, right) = (px < x + band, px >= x + w - band);
            let (top, bottom) = (py < y + band, py >= y + h - band);
            if left || right || top || bottom {
                return Some((p.id.clone(), Part::Resize { left, right, top, bottom }));
            }
            Some((p.id.clone(), Part::Body))
        })
    }

    /// A button over the open dashboard; true when the model took it rather than its surface.
    pub fn dashboard_button(&mut self, button: u32, pressed: bool) -> bool {
        const LEFT: u32 = 0x110;
        const RIGHT: u32 = 0x111;
        if self.isle.lock.locked || self.isle.prompts_hold_keyboard() {
            return false;
        }
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return false;
        };
        let location = self.isle.seat.get_pointer().unwrap().current_location();
        if !pressed {
            if button == LEFT && open.press.is_some() {
                self.dashboard_release();
                return true;
            }
            return false;
        }
        // The surface's own menu and form take presses over them.
        if open.menu.is_some() || open.form.is_some() {
            return false;
        }
        let Some(point) = self.dashboard_point(location) else {
            return false;
        };
        if let Some(g) = open.gallery.as_ref().map(|_| self.isle.gallery_rect(open))
            && point.0 >= g.0
            && point.0 < g.0 + g.2
            && point.1 >= g.1
            && point.1 < g.1 + g.3
        {
            return false;
        }
        let hit = self.dashboard_hit(point);
        let on_grid = {
            let e = &open.extent;
            point.1 < e.y + e.rows as f32 * e.pitch()
        };
        match (button, hit) {
            (RIGHT, Some((id, _))) => {
                self.dashboard_open_menu(Some(id), Some(point), false);
                true
            }
            (RIGHT, None) if on_grid => {
                self.dashboard_open_menu(None, Some(point), false);
                true
            }
            (LEFT, Some((id, Part::Badge))) if open.edit => {
                self.dashboard_remove(&id);
                true
            }
            (LEFT, Some((id, part))) if open.edit => {
                let placed = open.layout.get(&id).cloned().unwrap();
                let (x, y, _, _) = open.extent.rect(placed.at);
                let was_selected = open.selected.as_deref() == Some(id.as_str());
                let base = open.layout.clone();
                let open = self.isle.dashboard.open.as_mut().unwrap();
                open.ring = false;
                open.selected = Some(id.clone());
                open.press = Some(Press {
                    id,
                    part,
                    start: point,
                    grab: (point.0 - x, point.1 - y),
                    base,
                    moved: false,
                    plan: None,
                    ghost: placed.at,
                    at: point,
                    was_selected,
                    adding: None,
                    over_gallery: false,
                });
                self.put_dashboard_frame();
                true
            }
            (LEFT, None) if open.edit && on_grid => {
                let open = self.isle.dashboard.open.as_mut().unwrap();
                open.selected = None;
                self.put_dashboard_frame();
                false
            }
            _ => false,
        }
    }

    /// The pointer moved while a press is held on a widget; true when it took the motion.
    pub fn dashboard_pointer_to(&mut self, location: Point<f64, Logical>) -> bool {
        if self.isle.dashboard.open.as_ref().is_none_or(|o| o.press.is_none()) {
            return false;
        }
        let Some(point) = self.dashboard_point(location) else {
            return true;
        };
        let sizes = {
            let id = &self.isle.dashboard.open.as_ref().unwrap().press.as_ref().unwrap().id;
            self.isle.widget_sizes(id)
        };
        let gallery = {
            let open = self.isle.dashboard.open.as_ref().unwrap();
            open.gallery.as_ref().map(|_| self.isle.gallery_rect(open))
        };
        let open = self.isle.dashboard.open.as_mut().unwrap();
        let press = open.press.as_mut().unwrap();
        press.at = point;
        press.over_gallery = gallery.is_some_and(|g| {
            point.0 >= g.0 && point.0 < g.0 + g.2 && point.1 >= g.1 && point.1 < g.1 + g.3
        });
        if !press.moved {
            let (dx, dy) = (point.0 - press.start.0, point.1 - press.start.1);
            if dx.hypot(dy) < DRAG_START {
                return true;
            }
            press.moved = true;
        }
        let e = open.extent;
        let from = match press.adding {
            Some((w, h)) => Cells::new(press.ghost.x, press.ghost.y, w, h),
            None => press.base.get(&press.id).unwrap().at,
        };
        let pitch = e.pitch();
        let (to, dir) = match press.part {
            Part::Body | Part::Badge => {
                let left = point.0 - press.grab.0 - e.x;
                let top = point.1 - press.grab.1 - e.y;
                let x = (left / pitch).round() as i32;
                let y = (top / pitch).round() as i32;
                let x = x.clamp(0, (e.cols - from.w).max(0));
                let y = y.clamp(0, (e.rows - from.h).max(0));
                (Cells::new(x, y, from.w, from.h), Dir::of(x - from.x, y - from.y))
            }
            Part::Resize { left, right, top, bottom } => {
                let (cx, cy) = (((point.0 - e.x) / pitch).round() as i32, ((point.1 - e.y) / pitch).round() as i32);
                let (mut x0, mut y0, mut x1, mut y1) = (from.x, from.y, from.x + from.w, from.y + from.h);
                if left {
                    x0 = cx.min(x1 - 1);
                }
                if right {
                    x1 = cx.max(x0 + 1);
                }
                if top {
                    y0 = cy.min(y1 - 1);
                }
                if bottom {
                    y1 = cy.max(y0 + 1);
                }
                let (w, h) = sizes.nearest(x1 - x0, y1 - y0);
                let x = if left { from.x + from.w - w } else { from.x };
                let y = if top { from.y + from.h - h } else { from.y };
                let dir = if w > from.w {
                    Some(if left { Dir::Left } else { Dir::Right })
                } else if h > from.h {
                    Some(if top { Dir::Up } else { Dir::Down })
                } else {
                    None
                };
                (Cells::new(x, y, w, h), dir)
            }
        };
        if press.adding.is_some() {
            // A new widget pushes the way the pointer last moved, once it lands on others.
            let step = Dir::of(to.x - press.ghost.x, to.y - press.ghost.y).or(dir);
            press.ghost = to;
            press.plan = if press.over_gallery {
                None
            } else {
                press.base.push(&press.id, to, None, &e).or_else(|| press.base.push(&press.id, to, step, &e))
            };
            self.put_dashboard_frame();
            return true;
        }
        press.ghost = to;
        press.plan = if press.over_gallery { None } else { press.base.push(&press.id, to, dir, &e) };
        self.put_dashboard_frame();
        true
    }

    fn dashboard_release(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        let Some(press) = open.press.take() else {
            return;
        };
        if let Some(_size) = press.adding {
            if !press.moved {
                let kind = kind_of(&press.id).to_owned();
                return self.gallery_add(&kind);
            }
            if let Some(plan) = press.plan.filter(|_| !press.over_gallery) {
                open.layout = plan;
                open.selected = Some(press.id.clone());
                return self.dashboard_changed(press.base);
            }
            return self.put_dashboard_frame();
        }
        if press.moved && press.over_gallery && press.part == Part::Body {
            let id = press.id.clone();
            return self.dashboard_remove(&id);
        }
        if !press.moved {
            if press.part == Part::Body && press.was_selected {
                let id = press.id.clone();
                return self.dashboard_open_form(&id);
            }
            return self.put_dashboard_frame();
        }
        if let Some(plan) = press.plan {
            open.layout = plan;
        }
        self.dashboard_changed(press.base);
    }
}

/// The volumes' use: the system's own filesystems, then each mounted volume UDisks lists.
fn read_volumes(mounted: Vec<(String, String)>) -> Vec<VolumeRow> {
    let mut seen: Vec<(u64, u64, u64)> = Vec::new();
    let mut rows = Vec::new();
    let system = [("/", "System"), ("/home", "Home")];
    let all = system
        .iter()
        .map(|(p, n)| ((*p).to_owned(), (*n).to_owned()))
        .chain(mounted);
    for (path, name) in all {
        let Ok(stat) = rustix::fs::statvfs(path.as_str()) else {
            continue;
        };
        // The same filesystem mounted twice, or as btrfs subvolumes, is one volume.
        let id = (stat.f_blocks, stat.f_bfree, stat.f_frsize);
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let total = stat.f_blocks * stat.f_frsize;
        if total == 0 {
            continue;
        }
        let free = stat.f_bavail * stat.f_frsize;
        let used = 1.0 - free as f32 / total as f32;
        rows.push(VolumeRow {
            name,
            path,
            used,
            free: format!("{} free", bytes(free)),
            nearly_full: used >= 0.9,
        });
    }
    rows
}

/// A size in decimal units, as Files writes them.
fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1000.0 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    if unit == 0 || v >= 100.0 {
        format!("{v:.0} {}", UNITS[unit])
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

impl State {
    /// Reads every volume's use on the worker, for the Storage widget, while it shows.
    pub fn read_dashboard_storage(&mut self) {
        let showing = self
            .isle
            .dashboard
            .open
            .as_ref()
            .is_some_and(|o| o.layout.widgets.iter().any(|p| p.kind() == "storage"));
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        if !showing {
            return;
        }
        let mounted: Vec<(String, String)> = self
            .isle
            .services
            .disks
            .state
            .volumes
            .iter()
            .filter_map(|v| {
                let at = v.mount_point.clone()?;
                let name = if v.label.is_empty() {
                    std::path::Path::new(&at).file_name()?.to_string_lossy().into_owned()
                } else {
                    v.label.clone()
                };
                Some((at, name))
            })
            .collect();
        runtime.spawn_blocking(move || {
            let rows = read_volumes(mounted);
            to_loop.post(move |state| {
                state.isle.dashboard.storage = Some(rows);
                state.put_dashboard_frame();
            });
        });
    }

    /// A row inside a widget: where it goes, closing the dashboard first.
    fn dashboard_row(&mut self, id: &str, row: &str) {
        let output = self
            .isle
            .dashboard
            .open
            .as_ref()
            .and_then(|o| self.isle.space.outputs().find(|out| out.name() == o.output).cloned());
        match (kind_of(id), row) {
            ("media", "raise") => {
                self.close_dashboard();
                self.media_action(isle_shell::island::panel::PanelAction::Choose {
                    module: "now-playing".into(),
                    row: "raise".into(),
                });
            }
            ("media", key) => self.media_action(isle_shell::island::panel::PanelAction::Choose {
                module: "now-playing".into(),
                row: key.into(),
            }),
            ("storage", "more") => {
                self.close_dashboard();
                self.show_folder_in_files(std::path::Path::new("/"));
            }
            ("storage", path) => {
                self.close_dashboard();
                self.show_folder_in_files(std::path::Path::new(path));
            }
            ("desktops", index) => {
                if let (Ok(index), Some(output)) = (index.parse::<usize>(), output) {
                    self.close_dashboard();
                    self.switch_desktop(&output, index);
                }
            }
            (_, "remove") => self.dashboard_remove(id),
            (_, "allow") => self.ask_allow(id),
            (_, "try-again") => self.provider_try_again(id),
            ("timer", "start") => self.timer_start(id),
            ("timer", "pause") => self.timer_pause(id),
            ("timer", "reset") => self.timer_reset(id),
            ("timer", preset) => {
                if let Some(minutes) = preset.strip_prefix("preset:").and_then(|m| m.parse().ok()) {
                    self.timer_preset(id, minutes);
                }
            }
            ("notifications", app) => {
                self.close_dashboard();
                if let Some(output) = output.or_else(|| self.isle.main_output()) {
                    self.open_centre_on(&output);
                    if app != "more" {
                        self.isle.notifications.expanded.insert(app.to_owned());
                        self.isle.refresh_island();
                    }
                }
            }
            _ => {}
        }
    }

    /// A module's action from the Controls widget: a page closes the dashboard and opens
    /// Control Centre there.
    pub fn dashboard_panel_action(&mut self, action: isle_shell::island::panel::PanelAction) {
        use isle_shell::island::panel::PanelAction;
        match action {
            PanelAction::Page(Some(page)) => {
                let output = self
                    .isle
                    .dashboard
                    .open
                    .as_ref()
                    .and_then(|o| self.isle.space.outputs().find(|out| out.name() == o.output).cloned());
                self.close_dashboard();
                if let Some(output) = output {
                    self.open_control_centre(&output, true, Some(page));
                }
            }
            PanelAction::Page(None) => {}
            other => {
                self.panel_action(other);
                self.put_dashboard_frame();
            }
        }
    }
}

/// How often System samples while it shows.
const SAMPLE_EVERY: Duration = Duration::from_secs(2);

impl State {
    /// Samples every two seconds on the worker while a System widget shows; the first at once.
    fn arm_dashboard_sampling(&mut self) {
        let showing = self
            .isle
            .dashboard
            .open
            .as_ref()
            .is_some_and(|o| o.layout.widgets.iter().any(|p| p.kind() == "system"));
        if !showing || self.isle.dashboard.sample_tick.is_some() {
            return;
        }
        self.take_dashboard_sample();
        self.isle.dashboard.sample_tick = self
            .isle
            .loop_handle
            .insert_source(Timer::from_duration(SAMPLE_EVERY), |_, _, state| {
                if state.isle.dashboard.open.is_none() {
                    state.isle.dashboard.sample_tick = None;
                    return TimeoutAction::Drop;
                }
                state.take_dashboard_sample();
                TimeoutAction::ToDuration(SAMPLE_EVERY)
            })
            .ok();
    }

    fn take_dashboard_sample(&mut self) {
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        let sampler = self.isle.dashboard.sampler.clone();
        runtime.spawn_blocking(move || {
            let mut sampler = sampler.lock().unwrap();
            let sampler = sampler.get_or_insert_with(|| isle_sysinfo::Sampler::new(30));
            sampler.sample();
            let history: Vec<_> = sampler.history().iter().cloned().collect();
            to_loop.post(move |state| {
                state.isle.dashboard.system = history;
                state.put_dashboard_frame();
            });
        });
    }
}

/// How long a forecast stays fresh while shown.
const WEATHER_FRESH: u64 = 15 * 60;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl State {
    /// Fetches a forecast for each Weather widget shown whose last is stale or for another
    /// place.
    pub fn refresh_weather(&mut self) {
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return;
        };
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone()) else {
            return;
        };
        // The tests' compositors fetch nothing.
        if std::env::var_os("ISLE_NO_NETWORK").is_some() {
            return;
        }
        let ids: Vec<String> = open
            .layout
            .widgets
            .iter()
            .filter(|p| p.kind() == "weather")
            .map(|p| p.id.clone())
            .collect();
        let zone = jiff::tz::TimeZone::system();
        let zone = zone.iana_name().unwrap_or("UTC").to_owned();
        let hours_24 = crate::island::clock_24_hour(&self.isle.settings.island);
        let fahrenheit = crate::weather::fahrenheit();
        for id in ids {
            let asked = match self.isle.settings.widget.get(&id).and_then(|w| w.get("place")) {
                Some(isle_settings::Value::Str(p)) => p.trim().to_owned(),
                _ => String::new(),
            };
            let cached = self.isle.forecast(&id);
            if cached.as_ref().is_some_and(|f| f.asked == asked && now_secs().saturating_sub(f.at) < WEATHER_FRESH) {
                continue;
            }
            let (to_loop, zone) = (to_loop.clone(), zone.clone());
            runtime.spawn(async move {
                let place = if asked.is_empty() {
                    isle_sun::coordinates(&zone)
                        .map(|(lat, lon)| (city_of(&zone), lat, lon))
                        .ok_or_else(|| format!("No place for {zone}"))
                } else {
                    crate::weather::find(&asked).await
                };
                let forecast = match place {
                    Ok((name, lat, lon)) => crate::weather::fetch(lat, lon, fahrenheit).await.and_then(|json| {
                        crate::weather::parse(&json, &name, &asked, fahrenheit, now_secs(), hours_24)
                            .ok_or_else(|| "Open-Meteo's answer did not read".to_owned())
                    }),
                    Err(e) => Err(e),
                };
                to_loop.post(move |state| match forecast {
                    Ok(forecast) => {
                        let value = serde_json::to_value(&forecast).unwrap_or_default();
                        state.isle.dashboard.memory.data.insert(
                            id,
                            isle_state::domains::WidgetData { value, at: forecast.at },
                        );
                        state.isle.save_widgets();
                        state.put_dashboard_frame();
                    }
                    Err(err) => tracing::warn!(id, "weather: {err}"),
                });
            });
        }
    }
}

impl State {
    /// Starts what the widgets shown read: volumes, samples, forecasts.
    pub fn dashboard_feeds(&mut self) {
        self.run_providers();
        self.read_dashboard_storage();
        self.arm_dashboard_sampling();
        self.refresh_weather();
    }
}

impl State {
    /// A provider's setting written as its manifest types it; it runs again with it.
    fn provider_form_set(&mut self, id: &str, key: &str, value: FormValue) {
        use isle_providers::manifest::SettingType;
        use isle_settings::Value;
        let value = if key == "title" {
            match value {
                FormValue::Bool(b) => Value::Bool(b),
                _ => return,
            }
        } else {
            let Some(setting) = self
                .isle
                .provider_of(id)
                .and_then(|p| p.manifest.settings.iter().find(|s| s.key == key))
                .cloned()
            else {
                return;
            };
            match (setting.kind, value) {
                (SettingType::Toggle, FormValue::Bool(b)) => Value::Bool(b),
                (SettingType::Choice, FormValue::Index(i)) => match setting.options.get(i) {
                    Some(o) => Value::Str(o.clone()),
                    None => return,
                },
                (SettingType::Number, FormValue::Int(n)) => {
                    Value::Int(n.clamp(setting.min.unwrap_or(i64::MIN), setting.max.unwrap_or(i64::MAX)))
                }
                (SettingType::Text, FormValue::Text(t)) => Value::Str(t),
                _ => return,
            }
        };
        self.isle.write_setting(&format!("widget[{id}].{key}"), value);
        if key != "title" {
            self.isle.dashboard.running.remove(id);
        }
    }
}

const GROUPS: [&str; 6] = ["Time and calendar", "Weather", "Media", "System", "Devices", "Desktop"];

impl Isle {
    /// The items a gallery section lists, as (heading, keys).
    fn gallery_groups(&self, open: &Open, g: &Gallery) -> Vec<(String, Vec<String>)> {
        let query = g.query.to_lowercase();
        let matches = |name: &str| query.is_empty() || name.to_lowercase().contains(&query);
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        match g.section {
            0 => {
                for group in GROUPS {
                    let keys: Vec<String> = catalogue::BUILTINS
                        .iter()
                        .filter(|b| b.group == group && matches(b.name))
                        .map(|b| b.kind.to_owned())
                        .collect();
                    if !keys.is_empty() {
                        out.push((group.to_owned(), keys));
                    }
                }
                let mut by_source: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for (id, p) in &self.dashboard.providers {
                    if let Some(source) = &p.source
                        && matches(&p.manifest.name)
                    {
                        by_source.entry(source_name(source)).or_default().push(id.clone());
                    }
                }
                out.extend(by_source);
            }
            2 => {
                for url in &self.settings.widgets.sources {
                    let Some(Ok(index)) = self.dashboard.browse.get(url) else {
                        continue;
                    };
                    let keys: Vec<String> = index
                        .widgets
                        .iter()
                        .filter(|l| matches(&l.manifest.name))
                        .map(|l| format!("{}@{url}", l.id))
                        .collect();
                    if !keys.is_empty() {
                        out.push((index.name.clone(), keys));
                    }
                }
            }
            1 => {
                let keys: Vec<String> = self
                    .dashboard
                    .providers
                    .iter()
                    .filter(|(_, p)| p.source.is_none() && matches(&p.manifest.name))
                    .map(|(id, _)| id.clone())
                    .collect();
                if !keys.is_empty() {
                    out.push(("Your widgets".to_owned(), keys));
                }
            }
            _ => {}
        }
        let _ = open;
        out
    }

    fn gallery_view(&self, open: &Open, g: &Gallery) -> isle_shell::dashboard::surface::GalleryView {
        use isle_shell::dashboard::surface::{GalleryView, GroupView, ItemView};
        let groups = self.gallery_groups(open, g);
        let mut index = 0;
        let groups: Vec<GroupView> = groups
            .into_iter()
            .map(|(heading, keys)| GroupView {
                heading,
                items: keys
                    .into_iter()
                    .map(|key| {
                        let updates = self.updates();
                        let item = match key.split_once('@') {
                            Some((id, url)) => self.browse_item(&key, id, url, g.focused && index == g.selected),
                            None => {
                                let preview = format!("{key}#preview");
                                let sizes = self.widget_sizes(&preview);
                                let name = self.widget_name(&preview);
                                let several = catalogue::builtin(&key).map_or_else(
                                    || self.dashboard.providers.get(&key).is_some_and(|p| p.manifest.multiple),
                                    |b| b.several,
                                );
                                let added = !several && open.layout.widgets.iter().any(|p| p.kind() == key);
                                let no_room = !added && open.layout.place_for(sizes.default, &open.extent).is_none();
                                let line = if no_room {
                                    format!("No room for “{name}”. Remove or resize a widget to make space.")
                                } else {
                                    sizes.all.iter().map(|(w, h)| format!("{w} × {h}")).collect::<Vec<_>>().join(", ")
                                };
                                ItemView {
                                    picture: WidgetView {
                                        id: preview.clone(),
                                        body: self.widget_body(&preview, &name, &open.output),
                                        name: name.clone(),
                                        at: Cells::new(0, 0, sizes.default.0, sizes.default.1),
                                    },
                                    button: updates
                                        .iter()
                                        .any(|(i, _, _)| *i == key)
                                        .then(|| ("Update".to_owned(), "update".to_owned())),
                                    key,
                                    name,
                                    line,
                                    added,
                                    no_room,
                                    selected: g.focused && index == g.selected,
                                    permissions: Vec::new(),
                                }
                            }
                        };
                        index += 1;
                        item
                    })
                    .collect(),
            })
            .collect();
        let note = (groups.is_empty()).then(|| match (g.section, g.query.is_empty()) {
            (_, false) => format!("No widgets match “{}”", g.query),
            (1, true) => "Your own widgets go in ~/.config/isle/widgets.".to_owned(),
            _ => "Nothing here yet.".to_owned(),
        });
        GalleryView {
            section: g.section,
            query: g.query.clone(),
            focused: g.focused,
            details: g.details.as_ref().map(|key| self.details_view(key)),
            groups,
            updates: self.updates().len(),
            top: match g.section {
                1 => Some(("New widget…".to_owned(), "new".to_owned())),
                2 => Some(("Sources…".to_owned(), "sources".to_owned())),
                _ => None,
            },
            note,
            faded: false,
            sources: g.sources.map(|adding| self.sources_view(adding)),
        }
    }

    fn browse_item(&self, key: &str, id: &str, url: &str, selected: bool) -> isle_shell::dashboard::surface::ItemView {
        use isle_shell::dashboard::surface::ItemView;
        let listing = self.listing(url, id);
        let (name, line, size, permissions) = match &listing {
            Some(l) => {
                let m = &l.manifest;
                let size = isle_providers::manifest::size(&m.default).unwrap_or((2, 1));
                let mut permissions: Vec<(String, String)> = Vec::new();
                for g in &m.declared() {
                    if !permissions.iter().any(|(glyph, _)| glyph == g.glyph()) {
                        permissions.push((g.glyph().to_owned(), g.words()));
                    }
                }
                let byline = [m.author.name.clone(), m.version.clone()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
                (m.name.clone(), byline, size, permissions)
            }
            None => (id.to_owned(), String::new(), (2, 1), Vec::new()),
        };
        let installed = self.dashboard.providers.get(id).filter(|p| p.source.as_deref() == Some(url));
        let button = match (installed, &listing) {
            (None, Some(_)) => Some(("Install".to_owned(), "install".to_owned())),
            (Some(p), Some(l)) if p.manifest.older_than(&l.manifest) => Some(("Update".to_owned(), "update".to_owned())),
            _ => None,
        };
        ItemView {
            picture: WidgetView {
                id: format!("{key}#preview"),
                name: name.clone(),
                at: Cells::new(0, 0, size.0, size.1),
                body: Body::Title,
            },
            key: key.to_owned(),
            name,
            line,
            added: false,
            no_room: false,
            selected,
            button,
            permissions,
        }
    }

    fn details_view(&self, key: &str) -> isle_shell::dashboard::surface::DetailsView {
        use isle_shell::dashboard::surface::DetailsView;
        if let Some((id, url)) = key.split_once('@')
            && let Some(listing) = self.listing(url, id)
        {
            let m = &listing.manifest;
            let installed = self.dashboard.providers.get(id).filter(|p| p.source.as_deref() == Some(url));
            let granted = self.granted(id);
            let buttons = match installed {
                None => vec![("Install".to_owned(), "install".to_owned())],
                Some(p) if p.manifest.older_than(m) => vec![("Update".to_owned(), "update".to_owned()), ("Remove".to_owned(), "remove".to_owned())],
                Some(_) => vec![("Remove".to_owned(), "remove".to_owned())],
            };
            return DetailsView {
                key: key.to_owned(),
                name: m.name.clone(),
                description: m.description.clone(),
                byline: [m.author.name.clone(), m.version.clone(), m.license.clone()]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                source: Some(source_name(url)),
                sizes: m.sizes.iter().map(|s| s.replace('x', " × ")).collect::<Vec<_>>().join(", "),
                command: Some(m.command()),
                permissions: m.declared().iter().map(|g| (g.glyph().to_owned(), g.words(), granted.contains(g))).collect(),
                buttons,
                readme: None,
            };
        }
        let preview = format!("{key}#preview");
        let sizes = self.widget_sizes(&preview).all.iter().map(|(w, h)| format!("{w} × {h}")).collect::<Vec<_>>().join(", ");
        match self.dashboard.providers.get(key) {
            Some(p) => {
                let m = &p.manifest;
                let granted = self.granted(&m.id);
                let mut byline = vec![m.author.name.clone(), m.version.clone()];
                if !m.license.is_empty() {
                    byline.push(m.license.clone());
                }
                DetailsView {
                    key: key.to_owned(),
                    name: m.name.clone(),
                    description: m.description.clone(),
                    byline: byline.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "),
                    source: Some(p.source.clone().unwrap_or_else(|| "Your own".to_owned())),
                    sizes,
                    command: Some(m.command()),
                    permissions: m
                        .declared()
                        .iter()
                        .map(|g| (g.glyph().to_owned(), g.words(), granted.contains(g)))
                        .collect(),
                    buttons: match &p.source {
                        Some(_) if self.updates().iter().any(|(i, _, _)| i == key) => {
                            vec![("Update".to_owned(), "update".to_owned()), ("Remove".to_owned(), "remove".to_owned())]
                        }
                        Some(_) => vec![("Remove".to_owned(), "remove".to_owned())],
                        None if m.declared().iter().any(|g| !granted.contains(g)) => {
                            vec![("Allow…".to_owned(), "allow".to_owned())]
                        }
                        None => Vec::new(),
                    },
                    readme: std::fs::read_to_string(p.folder.join("README.md")).ok(),
                }
            }
            None => DetailsView {
                key: key.to_owned(),
                name: self.widget_name(&preview),
                description: "Built into Isle.".to_owned(),
                byline: String::new(),
                source: None,
                sizes,
                command: None,
                permissions: Vec::new(),
                buttons: Vec::new(),
                readme: None,
            },
        }
    }

    /// The gallery's box on the dashboard's screen, in points.
    fn gallery_rect(&self, open: &Open) -> (f32, f32, f32, f32) {
        let t = &self.shell.theme().tokens;
        let height = self
            .space
            .outputs()
            .find(|o| o.name() == open.output)
            .and_then(|o| self.space.output_geometry(o))
            .map_or(0.0, |g| g.size.h as f32);
        let top = t.size_strip + t.space_l;
        (t.space_l, top, t.size_panel, height - top - t.space_l)
    }
}

/// A source's name as the gallery heads it: its repository's.
pub fn source_name(url: &str) -> String {
    url.trim_end_matches('/').trim_end_matches(".git").rsplit('/').next().unwrap_or(url).to_owned()
}

impl State {
    fn gallery_keys(&self) -> Vec<String> {
        let Some(open) = self.isle.dashboard.open.as_ref() else {
            return Vec::new();
        };
        let Some(g) = open.gallery.as_ref() else {
            return Vec::new();
        };
        self.isle.gallery_groups(open, g).into_iter().flat_map(|(_, keys)| keys).collect()
    }

    /// Adds `kind` at its default size where a click puts it: the free place nearest the top
    /// centre, selected.
    pub fn gallery_add(&mut self, kind: &str) {
        let preview = format!("{kind}#preview");
        let size = self.isle.widget_sizes(&preview).default;
        let taken: Vec<String> = self.isle.settings.widget.keys().cloned().collect();
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        let Some(at) = open.layout.place_for(size, &open.extent) else {
            return;
        };
        let id = open.layout.next_id(kind, |id| taken.iter().any(|t| t == id));
        let before = open.layout.clone();
        if let Some(after) = before.push(&id, at, None, &open.extent) {
            open.layout = after;
            open.selected = Some(id);
            open.ring = true;
        }
        self.dashboard_changed(before);
    }

    pub fn gallery_action(&mut self, action: DashboardAction) {
        let Some(open) = self.isle.dashboard.open.as_mut() else {
            return;
        };
        match action {
            DashboardAction::Gallery => {
                open.gallery = match open.gallery.take() {
                    Some(_) => None,
                    None => Some(Gallery::default()),
                };
            }
            DashboardAction::GallerySection(i) => {
                if let Some(g) = open.gallery.as_mut() {
                    g.section = i.min(2);
                    g.selected = 0;
                    g.details = None;
                    g.sources = None;
                }
                // Browse is fetched each time it opens (`structure/widgets.md` The gallery).
                if i == 2 {
                    self.refresh_sources(true);
                }
            }
            DashboardAction::GalleryDetails(key) => {
                if let Some(g) = open.gallery.as_mut() {
                    g.details = key;
                }
            }
            DashboardAction::GallerySecondary(key) => {
                let at = self.isle.seat.get_pointer().unwrap().current_location();
                let point = self.dashboard_point(at);
                if let Some(open) = self.isle.dashboard.open.as_mut() {
                    open.menu = Some(Menu {
                        on: Some(format!("gallery:{key}")),
                        at: point.unwrap_or((0.0, 0.0)),
                        by_key: false,
                    });
                }
            }
            DashboardAction::GalleryPress(key) if key.contains('@') => {
                if let Some(g) = open.gallery.as_mut() {
                    g.details = Some(key);
                }
            }
            DashboardAction::GalleryPress(key) => {
                let preview = format!("{key}#preview");
                let size = self.isle.widget_sizes(&preview).default;
                let taken: Vec<String> = self.isle.settings.widget.keys().cloned().collect();
                let location = self.isle.seat.get_pointer().unwrap().current_location();
                let point = self.dashboard_point(location).unwrap_or((0.0, 0.0));
                let Some(open) = self.isle.dashboard.open.as_mut() else {
                    return;
                };
                let id = open.layout.next_id(&key, |id| taken.iter().any(|t| t == id));
                let pitch = open.extent.pitch();
                open.press = Some(Press {
                    id,
                    part: Part::Body,
                    start: point,
                    grab: ((size.0 as f32 * pitch - open.extent.gap) / 2.0, (size.1 as f32 * pitch - open.extent.gap) / 2.0),
                    base: open.layout.clone(),
                    moved: false,
                    plan: None,
                    ghost: Cells::new(0, 0, size.0, size.1),
                    at: point,
                    was_selected: false,
                    adding: Some(size),
                    over_gallery: true,
                });
            }
            DashboardAction::GalleryButton { key, action } => {
                return self.gallery_button(&key, &action);
            }
            _ => {}
        }
        self.put_dashboard_frame();
    }

    /// Keys while the gallery has the keyboard; true when taken.
    fn gallery_key(&mut self, sym: u32, code: u32) -> bool {
        use smithay::input::keyboard::keysyms;
        let count = self.gallery_keys().len();
        let Some(g) = self.isle.dashboard.open.as_mut().and_then(|o| o.gallery.as_mut()) else {
            return false;
        };
        if !g.focused {
            return false;
        }
        match sym {
            keysyms::KEY_Escape if g.details.is_some() => g.details = None,
            keysyms::KEY_Escape => return false,
            keysyms::KEY_Tab => g.focused = false,
            keysyms::KEY_BackSpace => {
                g.query.pop();
                g.selected = 0;
            }
            keysyms::KEY_Left | keysyms::KEY_Up => g.selected = g.selected.saturating_sub(1),
            keysyms::KEY_Right | keysyms::KEY_Down => g.selected = (g.selected + 1).min(count.saturating_sub(1)),
            keysyms::KEY_Return | keysyms::KEY_KP_Enter => {
                let selected = g.selected;
                if let Some(key) = self.gallery_keys().get(selected).cloned() {
                    self.gallery_add(&key);
                }
                return true;
            }
            keysyms::KEY_space if g.query.is_empty() => {
                let selected = g.selected;
                let key = self.gallery_keys().get(selected).cloned();
                if let Some(g) = self.isle.dashboard.open.as_mut().and_then(|o| o.gallery.as_mut()) {
                    g.details = key;
                }
            }
            _ => match self.typed_text(code).filter(|t| !t.chars().any(char::is_control)) {
                Some(text) => {
                    if let Some(g) = self.isle.dashboard.open.as_mut().and_then(|o| o.gallery.as_mut()) {
                        g.query.push_str(&text);
                        g.selected = 0;
                    }
                }
                None => return false,
            },
        }
        self.put_dashboard_frame();
        true
    }
}
