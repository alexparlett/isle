//! The Inspect window, one per process (`structure/monitor.md` § Inspect,
//! `surfaces/monitor.md` § The Inspect window): General and Open Files and Ports, sampled
//! while shown, and the ended form once the process has gone.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use freya::prelude::*;
use isle_app::{AppHandle, ToolbarExt, WindowKey, WindowSpec};
use isle_sysinfo::{
    inspect::{self, Detail, OpenFile, Port},
    procs::Key,
};
use isle_ui::{
    Button, Glyph, SearchField, colour,
    form::Segmented,
    table::{Cell, Column, Row, Sort, Table, TableEvent},
    text, use_theme,
};

use crate::{
    act::{self, Act},
    model::{bytes, write},
};

/// The latest main sample's GPU use by pid, for Inspect's GPU memory.
pub static GPU_MEMORY: Mutex<Option<HashMap<i32, u64>>> = Mutex::new(None);

thread_local! {
    /// Open Inspect windows by process, so inspecting again raises the one open.
    static OPEN: RefCell<HashMap<Key, WindowKey>> = RefCell::new(HashMap::new());
}

const LIGHTS: f32 = 68.;

/// Opens the process's Inspect window, or raises the one open.
pub fn open(app: &AppHandle, key: Key, name: &str) {
    let title = format!("{name} ({})", key.pid);
    let open = OPEN.with(|o| o.borrow().get(&key).copied()).filter(|w| app.windows().contains(w));
    if open.is_some() {
        let bus = app.bus();
        isle_app::worker().spawn(raise(bus, title));
        return;
    }
    let spec = WindowSpec {
        title: title.clone(),
        size: (520.0, 600.0),
        min_size: (420.0, 360.0),
        max_size: None,
        lights: Some((0.0, 10.0)),
        backdrop: None,
        token: None,
        parent: None,
        float: None,
    };
    let window = app.open(spec, Inspect { key, name: name.to_owned() });
    OPEN.with(|o| o.borrow_mut().insert(key, window));
}

/// Focuses Monitor's own window titled `title` through the compositor's window list.
async fn raise(bus: Option<zbus::Connection>, title: String) {
    let Some(bus) = bus else { return };
    let Ok(windows) = isle_ipc::windows::WindowsProxy::new(&bus).await else { return };
    let me = std::process::id() as i32;
    if let Ok(list) = windows.list().await
        && let Some(w) = list.into_iter().find(|w| w.pid == me && w.title == title)
    {
        let _ = windows.focus(w.id).await;
    }
}

#[derive(Clone)]
pub struct Inspect {
    pub key: Key,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Figures {
    detail: Option<Detail>,
    /// Cores, from the last two samples.
    cpu: Option<f32>,
    memory: Option<u64>,
    gpu_memory: Option<u64>,
    files: Vec<OpenFile>,
    ports: Vec<Port>,
    /// Another user's process: its descriptors cannot be read.
    hidden: bool,
}

/// One reading of the process; none once it has ended.
fn read(key: Key, last: &mut Option<(u64, Instant)>) -> Option<Figures> {
    let detail = inspect::detail(key)?;
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", key.pid)).ok()?;
    let f: Vec<u64> = stat.rsplit_once(')')?.1.split_whitespace().filter_map(|v| v.parse().ok()).collect();
    let ticks = f.get(11).copied().unwrap_or(0) + f.get(12).copied().unwrap_or(0);
    let now = Instant::now();
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    let cpu = last.map(|(t0, at)| (ticks.saturating_sub(t0) as f64 / hz / now.duration_since(at).as_secs_f64().max(1e-3)) as f32);
    *last = Some((ticks, now));
    let status = std::fs::read_to_string(format!("/proc/{}/status", key.pid)).unwrap_or_default();
    let kb = |name: &str| status.lines().find_map(|l| l.strip_prefix(name)).and_then(|v| v.split_whitespace().next()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0) * 1024;
    let owners = owners();
    let open = inspect::open_files(key.pid, &owners);
    Some(Figures {
        cpu,
        memory: Some(kb("RssAnon:") + kb("RssShmem:") + kb("VmSwap:")),
        gpu_memory: GPU_MEMORY.lock().ok().and_then(|m| m.as_ref()?.get(&key.pid).copied()),
        hidden: open.is_none(),
        files: open.as_ref().map(|o| o.0.clone()).unwrap_or_default(),
        ports: open.map(|o| o.1).unwrap_or_default(),
        detail: Some(detail),
    })
}

/// The user's processes' sockets by inode, with each one's pid and name, for a Unix peer.
fn owners() -> HashMap<u32, (i32, String)> {
    // SAFETY: getuid cannot fail.
    let uid = unsafe { libc::getuid() };
    let mut out = HashMap::new();
    let Ok(dir) = std::fs::read_dir("/proc") else { return out };
    for entry in dir.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<i32>().ok()) else { continue };
        use std::os::unix::fs::MetadataExt;
        if entry.metadata().ok().map(|m| m.uid()) != Some(uid) {
            continue;
        }
        let name = std::fs::read_to_string(entry.path().join("comm")).map(|c| c.trim().to_owned()).unwrap_or_default();
        let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else { continue };
        for fd in fds.flatten() {
            if let Ok(t) = std::fs::read_link(fd.path())
                && let Some(inode) = t.to_string_lossy().strip_prefix("socket:[").and_then(|t| t.strip_suffix(']')).and_then(|t| t.parse().ok())
            {
                out.insert(inode, (pid, name.clone()));
            }
        }
    }
    out
}

impl App for Inspect {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let key = self.key;
        let name = self.name.clone();
        let shown = isle_app::use_shown();
        let mut figures = use_state(Figures::default);
        let mut ended = use_state(|| false);
        let mut tab = use_state(|| 0usize);
        let query = use_state(String::new);
        let mut alert = use_state(|| false);
        let worker = isle_app::worker();
        let app = use_consume::<AppHandle>();
        let controller = use_scroll_controller(ScrollConfig::default);
        // Sampled every two seconds while shown, until the process ends.
        use_hook({
            let worker = worker.clone();
            move || {
                let last = Arc::new(Mutex::new(None));
                spawn(async move {
                    loop {
                        if *ended.peek() {
                            return;
                        }
                        if *shown.peek() {
                            let last = last.clone();
                            let read = worker.spawn(async move { tokio::task::spawn_blocking(move || read(key, &mut last.lock().unwrap())).await.ok().flatten() });
                            match read.await.ok().flatten() {
                                Some(f) => figures.set(f),
                                None => ended.set(true),
                            }
                        }
                        crate::sleep(Duration::from_secs(2)).await;
                    }
                });
            }
        });
        use_drop(move || OPEN.with(|o| o.borrow_mut().remove(&key)).map(|_| ()).unwrap_or(()));

        let gone = *ended.read();
        let f = figures.read().clone();
        let ink = if gone { t.colour_text_2 } else { t.colour_text };
        let quit_ok = !gone && f.detail.as_ref().is_some_and(|d| !act::protected(&name, d.ppid));
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS + t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_l)
            .toolbar()
            .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Headline, colour(t.colour_text), format!("{name} ({})", key.pid))))
            .child(Segmented::new(vec!["General".into(), "Open Files and Ports".into()], *tab.read(), "View", move |i: usize| tab.set(i)))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(Button::new("Quit Process").enabled(quit_ok).on_press(move |_| alert.set(true)))
                    .maybe_child((*tab.read() == 1).then(|| SearchField { value: query.into_writable(), placeholder: "Search".into(), width: 140. }.into_element())),
            );
        let body = if *tab.read() == 0 {
            general(&theme, &f, ink, gone, &app)
        } else {
            files(&theme, &f, &query.read(), controller)
        };
        let quit_alert = (*alert.read()).then(|| {
            let (q, k) = (worker.clone(), worker.clone());
            isle_ui::alert(
                &format!("Quit “{name}”?"),
                "",
                Vec::new(),
                vec![
                    isle_ui::AlertButton::destructive("Force Quit", EventHandler::new(move |_| {
                        alert.set(false);
                        k.spawn(act::run(None, key, String::new(), Act::Signal(libc::SIGKILL)));
                    })),
                    isle_ui::AlertButton::from(("Cancel".to_owned(), EventHandler::new(move |_| alert.set(false)))),
                    isle_ui::AlertButton::from(("Quit".to_owned(), EventHandler::new(move |_| {
                        alert.set(false);
                        q.spawn(act::run(None, key, String::new(), Act::Signal(libc::SIGTERM)));
                    }))),
                ],
            )
        });
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(rect().width(Size::fill()).height(Size::flex(1.)).child(body))
            .maybe_child(quit_alert)
    }
}

/// A group of label and value rows, as Settings' groups.
fn group(theme: &isle_tokens::Theme, rows: Vec<(&str, Element)>) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .corner_radius(t.radius_card)
        .background(colour(t.colour_window_raised))
        .border(Border::new().fill(colour(t.colour_hairline)).width(1.).alignment(BorderAlignment::Inner))
        .padding((t.space_xs, t.space_m))
        .children(rows.into_iter().map(|(label, value)| {
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_m)
                .child(
                    rect()
                        .width(Size::px(t.size_detail_label))
                        .main_align(Alignment::End)
                        .horizontal()
                        .child(text::line(theme, text::Style::Callout, colour(t.colour_text_2), label.to_owned())),
                )
                .child(rect().width(Size::flex(1.)).child(value))
                .into_element()
        }))
        .into_element()
}

fn value(theme: &isle_tokens::Theme, ink: isle_tokens::Colour, s: impl Into<String>) -> Element {
    text::line(theme, text::Style::Body, colour(ink), s.into()).into_element()
}

fn figure(theme: &isle_tokens::Theme, ink: isle_tokens::Colour, s: impl Into<String>) -> Element {
    label().font_family("JetBrains Mono").font_size(theme.tokens.type_style_body_size).color(colour(ink)).max_lines(1).text(s.into()).into_element()
}

fn general(theme: &isle_tokens::Theme, f: &Figures, ink: isle_tokens::Colour, gone: bool, app: &AppHandle) -> Element {
    let t = &theme.tokens;
    let Some(d) = f.detail.clone() else { return rect().into_element() };
    let user = crate::model::users(std::path::Path::new("/")).get(&d.uid).cloned().unwrap_or_else(|| d.uid.to_string());
    let started = d
        .started
        .and_then(|s| jiff::Timestamp::from_second(s).ok())
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::system()).strftime("%-d %b %Y, %H:%M:%S").to_string())
        .unwrap_or_default();
    let parent = {
        let (app, ppid, pname) = (app.clone(), d.ppid, d.parent.clone());
        let label = format!("{} ({})", d.parent, d.ppid);
        rect()
            .on_press(move |_| {
                let stat = std::fs::read_to_string(format!("/proc/{ppid}/stat")).unwrap_or_default();
                if let Some(start) = stat.rsplit_once(')').and_then(|(_, r)| r.split_whitespace().nth(19)?.parse().ok()) {
                    open(&app, Key { pid: ppid, start }, &pname);
                }
            })
            .child(text::line(theme, text::Style::Body, colour(t.colour_accent_text), label))
            .into_element()
    };
    let cmdline = d.cmdline.join(" ");
    let copy = {
        let (app, text) = (app.clone(), cmdline.clone());
        Button::new("Copy").small().on_press(move |_| {
            if let Some(data) = app.data() {
                let b = text.clone().into_bytes();
                data.set_selection(vec![("text/plain;charset=utf-8".into(), b.clone()), ("text/plain".into(), b)]);
            }
        })
    };
    rect()
        .expanded()
        .padding(t.space_xl)
        .spacing(t.space_l)
        .maybe_child(gone.then(|| {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .child(Glyph::new("info", 16., colour(t.colour_text_2)))
                .child(value(theme, t.colour_text_2, "This process has ended."))
                .into_element()
        }))
        .child(group(
            theme,
            vec![
                ("Parent process", parent),
                ("User", value(theme, ink, user)),
                ("Started", value(theme, ink, started)),
                ("State", value(theme, ink, d.state.clone())),
                ("Priority", value(theme, ink, act::Priority::of(d.nice).label())),
            ],
        ))
        .child(group(
            theme,
            vec![
                ("% CPU", figure(theme, ink, f.cpu.map(|c| write("cpu", c as f64)).unwrap_or_default())),
                ("Memory", figure(theme, ink, f.memory.map(bytes).unwrap_or_default())),
                ("GPU memory", figure(theme, ink, f.gpu_memory.map(bytes).unwrap_or_default())),
                ("Threads", figure(theme, ink, d.threads.to_string())),
            ],
        ))
        .child(
            rect()
                .width(Size::fill())
                .spacing(t.space_s)
                .cross_align(Alignment::End)
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(t.space_s)
                        .corner_radius(t.radius_control)
                        .background(colour(t.colour_window_sunken))
                        .child(label().font_family("JetBrains Mono").font_size(t.type_style_body_size).color(colour(ink)).max_lines(6).text(cmdline)),
                )
                .child(copy),
        )
        .child(group(
            theme,
            vec![
                ("Executable", value(theme, ink, d.exe.clone().unwrap_or_default())),
                ("Working folder", value(theme, ink, d.cwd.clone().unwrap_or_default())),
                ("Unit", value(theme, ink, d.unit.clone())),
            ],
        ))
        .into_element()
}

fn files(theme: &isle_tokens::Theme, f: &Figures, query: &str, controller: ScrollController) -> Element {
    let t = &theme.tokens;
    if f.hidden {
        return rect()
            .expanded()
            .center()
            .background(colour(t.colour_view))
            .child(value(theme, t.colour_text_2, "Open files are shown only for your own processes."))
            .into_element();
    }
    let q = query.trim().to_lowercase();
    let matches = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);
    let mut rows: Vec<Row> = f
        .files
        .iter()
        .filter(|o| matches(&o.path))
        .map(|o| Row { key: o.fd as u64, name: o.path.clone(), cells: vec![Cell::from("File".to_owned()), Cell::from(String::new())], ..Default::default() })
        .collect();
    rows.extend(f.ports.iter().filter(|p| matches(&p.local) || matches(&p.remote)).map(|p| Row {
        key: (1 << 32) | p.fd as u64,
        name: if p.local.is_empty() { "—".to_owned() } else { p.local.clone() },
        cells: vec![Cell::from(p.protocol.to_owned()), Cell::from(p.remote.clone())],
        ..Default::default()
    }));
    Table {
        columns: Rc::new(vec![Column::text("name", "Name", 240.), Column::text("kind", "Kind", 64.), Column::text("remote", "Connected to", 160.)]),
        rows: Rc::new(rows),
        sort: Sort { column: "none", descending: false },
        selected: Rc::new(Vec::new()),
        focused: false,
        hold: true,
        on_event: EventHandler::new(|_: TableEvent| {}),
        controller,
    }
    .into_element()
}
