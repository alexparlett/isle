//! The window's toolbar: back and forward with the title, the view switcher and the Group
//! and Sort menu in the centre, the preview toggle at the end (`p1/surfaces/files.md` §
//! The toolbar).

use std::rc::Rc;

use freya::prelude::*;
use isle_app::ToolbarExt;
use isle_fs::SortBy;
use isle_ui::{
    Glyph, Menu, MenuEntry, MenuEvent, MenuItem, Popover, Slider, colour, field::CheckRow, menu::Mark, text,
    use_theme,
};

use crate::{
    format,
    jobs::{JobState, JobView},
    view::{GroupBy, ICON_STEPS, Shown, View},
};

/// What the toolbar asks of its window.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Back,
    Forward,
    ShowSidebar,
    TogglePreview,
    /// The view changed: by the switcher, the Group and Sort menu or View Options.
    Shown(Shown),
    UseAsDefaults,
    StopJob(u64),
    EmptyTrash,
    /// Return in the search field: search everything under the folder.
    Search(String),
    /// Escape in the search field.
    EndSearch,
}

pub fn icon_button(
    theme: &isle_tokens::Theme,
    glyph: &'static str,
    alt: &'static str,
    enabled: bool,
    pressed: bool,
    on_press: impl Fn() + 'static,
) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(t.size_control_large))
        .height(Size::px(t.size_control_large))
        .center()
        .corner_radius(t.radius_control)
        .maybe(pressed, |el| el.background(colour(t.colour_control_pressed)))
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(alt)
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            if enabled {
                on_press();
            }
        })
        .child(Glyph::new(
            glyph,
            t.size_glyph_large,
            colour(if enabled { t.colour_text_2 } else { t.colour_text_disabled }),
        ))
        .into_element()
}

const SORTS: [(SortBy, &str); 5] = [
    (SortBy::Name, "Name"),
    (SortBy::Kind, "Kind"),
    (SortBy::Modified, "Date Modified"),
    (SortBy::Created, "Date Created"),
    (SortBy::Size, "Size"),
];

const GROUPS: [(GroupBy, &str); 3] = [(GroupBy::Kind, "Kind"), (GroupBy::Modified, "Date Modified"), (GroupBy::Size, "Size")];

/// The Group and Sort menu's entries, by id: 1 Use Groups, 10+ group by, 20+ sort by.
fn group_sort_entries(shown: &Shown) -> Rc<Vec<MenuEntry>> {
    let mark = |on: bool| if on { Mark::Check } else { Mark::None };
    let mut entries = vec![
        MenuEntry::Item(MenuItem::new(1, "Use Groups").mark(mark(shown.group.is_some()))),
        MenuEntry::Separator,
        MenuEntry::Section("Group By".into()),
    ];
    entries.extend(GROUPS.iter().enumerate().map(|(i, (by, name))| {
        MenuEntry::Item(MenuItem::new(10 + i as u64, *name).mark(mark(shown.group == Some(*by))))
    }));
    entries.push(MenuEntry::Separator);
    entries.push(MenuEntry::Section("Sort By".into()));
    entries.extend(SORTS.iter().enumerate().map(|(i, (by, name))| {
        MenuEntry::Item(MenuItem::new(20 + i as u64, *name).mark(mark(shown.sort == *by)))
    }));
    Rc::new(entries)
}

fn chosen(shown: &Shown, id: u64) -> Shown {
    let mut next = *shown;
    match id {
        1 => next.group = if shown.group.is_some() { None } else { Some(GroupBy::Kind) },
        10..=12 => next.group = Some(GROUPS[(id - 10) as usize].0),
        20..=24 => next.sort = SORTS[(id - 20) as usize].0,
        _ => {}
    }
    next
}

#[derive(PartialEq)]
pub struct Toolbar {
    pub title: String,
    pub count: usize,
    pub back: bool,
    pub forward: bool,
    pub sidebar: bool,
    pub preview: bool,
    pub shown: Shown,
    /// ⌘J asks for View Options from the menu.
    pub options: State<bool>,
    pub jobs: Vec<JobView>,
    /// In the Trash: its count, for Empty Trash….
    pub in_trash: Option<usize>,
    /// The search field's text and id; ⌘F focuses it.
    pub query: State<String>,
    pub find: AccessibilityId,
    /// A walk stands: Columns is off.
    pub searching: bool,
    /// The title's glyph: the magnifying glass for a search.
    pub glyph: Option<&'static str>,
    /// The folder shown, whose icon is the title's proxy when there is no glyph.
    pub proxy: Option<isle_fs::Entry>,
    pub on_act: EventHandler<Act>,
}

impl Component for Toolbar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut sorting = use_state(|| false);
        let mut jobs_open = use_state(|| false);
        let mut options = self.options;
        let act = self.on_act.clone();
        let send = move |a: Act| act.call(a);
        let shown = self.shown;

        let searching = self.searching;
        let segment = |glyph: &'static str, alt: &'static str, view: View| {
            let chosen = shown.view == view;
            let enabled = !(searching && view == View::Columns);
            let send = send.clone();
            rect()
                .width(Size::px(36.))
                .height(Size::fill())
                .center()
                .corner_radius(t.radius_control - t.space_xxs)
                .maybe(chosen, |el| {
                    isle_ui::shadows(&t.shadow_control)
                        .into_iter()
                        .fold(el.background(colour(t.colour_window_raised)), |el, s| el.shadow(s))
                })
                .a11y_role(AccessibilityRole::RadioButton)
                .a11y_alt(alt)
                .a11y_builder(move |node| node.set_selected(chosen))
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    if enabled {
                        send(Act::Shown(Shown { view, ..shown }));
                    }
                })
                .child(
                    Glyph::new(
                        glyph,
                        t.size_glyph_large,
                        colour(if !enabled {
                            t.colour_text_disabled
                        } else if chosen {
                            t.colour_text
                        } else {
                            t.colour_text_2
                        }),
                    )
                    .fill(chosen),
                )
                .into_element()
        };
        let switcher = rect()
            .height(Size::px(t.size_control_large))
            .padding(t.space_xxs)
            .horizontal()
            .corner_radius(t.radius_control)
            .background(colour(t.colour_fill))
            .a11y_role(AccessibilityRole::RadioGroup)
            .a11y_alt("View")
            .child(segment("squares-four", "as Icons", View::Icons))
            .child(segment("list-bullets", "as List", View::List))
            .child(segment("columns", "as Columns", View::Columns));

        let view_options = {
            let send_slider = send.clone();
            let send_info = send.clone();
            let send_group = send.clone();
            let send_sort = send.clone();
            let send_defaults = send.clone();
            let group_at = shown.group.map_or(0, |g| GROUPS.iter().position(|(b, _)| *b == g).unwrap_or(0) + 1);
            let sort_at = SORTS.iter().position(|(b, _)| *b == shown.sort).unwrap_or(0);
            let row = |label: &str, control: Element| {
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .child(rect().width(Size::flex(1.)).child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(t.colour_text),
                        label.to_owned(),
                    )))
                    .child(control)
                    .into_element()
            };
            Popover::new(switcher, *options.read(), move |_| options.set(false))
                .alt("View Options")
                .child(row(
                    "Group By",
                    isle_ui::form::PopUp::new(
                        std::iter::once("None".to_owned()).chain(GROUPS.iter().map(|(_, n)| n.to_string())).collect(),
                        group_at,
                        "Group By",
                        move |i: usize| {
                            let group = if i == 0 { None } else { Some(GROUPS[i - 1].0) };
                            send_group(Act::Shown(Shown { group, ..shown }));
                        },
                    )
                    .into_element(),
                ))
                .child(row(
                    "Sort By",
                    isle_ui::form::PopUp::new(
                        SORTS.iter().map(|(_, n)| n.to_string()).collect(),
                        sort_at,
                        "Sort By",
                        move |i: usize| send_sort(Act::Shown(Shown { sort: SORTS[i].0, ..shown })),
                    )
                    .into_element(),
                ))
                .maybe_child((shown.view == View::Icons).then(|| {
                    let at = ICON_STEPS.iter().position(|s| *s >= shown.icon_size).unwrap_or(3) as f32;
                    row(
                        "Icon size",
                        rect()
                            .width(Size::px(120.))
                            .child(Slider::new(at / (ICON_STEPS.len() - 1) as f32, "Icon size", move |v: f32| {
                                let step = (v * (ICON_STEPS.len() - 1) as f32).round() as usize;
                                send_slider(Act::Shown(Shown { icon_size: ICON_STEPS[step], ..shown }));
                            }))
                            .into_element(),
                    )
                }))
                .maybe_child((shown.view == View::Icons).then(|| {
                    CheckRow::new("Show item info", shown.item_info)
                        .on_toggle(move |on: bool| send_info(Act::Shown(Shown { item_info: on, ..shown })))
                        .into_element()
                }))
                .child(
                    rect().width(Size::fill()).main_align(Alignment::End).horizontal().child(
                        isle_ui::Button::new("Use as Defaults")
                            .on_press(move |_| send_defaults(Act::UseAsDefaults))
                            .into_element(),
                    ),
                )
                .into_element()
        };

        let group_sort = {
            let send = send.clone();
            Attached::new(
                rect()
                    .width(Size::px(48.))
                    .height(Size::px(t.size_control_large))
                    .horizontal()
                    .center()
                    .spacing(t.space_xxs)
                    .corner_radius(t.radius_control)
                    .maybe(*sorting.read(), |el| el.background(colour(t.colour_control_pressed)))
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt("Group and Sort")
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        sorting.toggle();
                    })
                    .child(Glyph::new("rows", t.size_glyph_large, colour(t.colour_text_2)))
                    .child(Glyph::new("caret-down", t.size_glyph_small, colour(t.colour_text_2))),
            )
            .bottom()
            .offset(t.space_xs)
            .maybe_child((*sorting.read()).then(|| {
                Menu::new(group_sort_entries(&shown), move |event| match event {
                    MenuEvent::Chosen(id) => {
                        sorting.set(false);
                        send(Act::Shown(chosen(&shown, id)));
                    }
                    MenuEvent::Back => sorting.set(false),
                    _ => {}
                })
                .material(isle_ui::menu::Material::Window)
            }))
        };

        let (back_send, forward_send, sidebar_send, preview_send) = (send.clone(), send.clone(), send.clone(), send.clone());
        let search_field = {
            let (submit, escape) = (send.clone(), send.clone());
            isle_ui::TextField::new(self.query, "Search")
                .a11y_id(self.find)
                .placeholder("Search")
                .width(Size::px(t.size_toolbar_search))
                .leading(Glyph::new("magnifying-glass", t.size_glyph_small, colour(t.colour_text_2)))
                .on_submit(move |text: String| submit(Act::Search(text)))
                .on_escape(move |_| escape(Act::EndSearch))
        };
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .background(colour(t.colour_window))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .toolbar()
            .maybe_child((!self.sidebar).then(|| {
                rect()
                    .padding((0., 0., 0., t.space_xxxl + t.space_xl))
                    .child(icon_button(&theme, "sidebar-simple", "Show Sidebar", true, false, move || {
                        sidebar_send(Act::ShowSidebar)
                    }))
            }))
            .child(icon_button(&theme, "caret-left", "Back", self.back, false, move || back_send(Act::Back)))
            .child(icon_button(&theme, "caret-right", "Forward", self.forward, false, move || {
                forward_send(Act::Forward)
            }))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .padding((0., 0., 0., t.space_s))
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_xs)
                            .maybe_child(match (self.glyph, &self.proxy) {
                                (Some(g), _) => Some(Glyph::new(g, t.size_glyph, colour(t.colour_text_2)).into_element()),
                                (None, Some(folder)) => Some(crate::list_view::icon(
                                    folder,
                                    theme.appearance.mode == isle_tokens::Mode::Dark,
                                    t.size_app_icon_small,
                                    t.colour_text_2,
                                )),
                                (None, None) => None,
                            })
                            .child(text::line(&theme, text::Style::Headline, colour(t.colour_text), self.title.clone())),
                    )
                    .child(text::line(
                        &theme,
                        text::Style::Subheadline,
                        colour(t.colour_text_2),
                        format::items(self.count),
                    )),
            )
            .child(view_options)
            .child(group_sort)
            .child(rect().width(Size::flex(1.)))
            .maybe_child((!self.jobs.is_empty()).then(|| {
                let open = *jobs_open.read();
                Popover::new(
                    jobs_button(&theme, &self.jobs, open, move || {
                        let mut jobs_open = jobs_open;
                        jobs_open.toggle()
                    }),
                    open,
                    move |_| jobs_open.set(false),
                )
                    .alt("File jobs")
                    .children(self.jobs.iter().map(|j| job_row(&theme, j, send.clone())))
                    .into_element()
            }))
            .maybe_child(self.in_trash.map(|n| {
                let send = send.clone();
                isle_ui::Button::new("Empty Trash…")
                    .enabled(n > 0)
                    .on_press(move |_| send(Act::EmptyTrash))
                    .into_element()
            }))
            .child(icon_button(&theme, "sidebar-simple", "Show Preview", true, self.preview, move || {
                preview_send(Act::TogglePreview)
            }))
            .child(search_field)
    }
}

/// The jobs button: a ring filling with the running total, a dot while one waits, a warning
/// once one failed (`surfaces/files.md` § The toolbar, Jobs button).
fn jobs_button(theme: &isle_tokens::Theme, jobs: &[JobView], open: bool, on_press: impl Fn() + 'static) -> Element {
    let t = &theme.tokens;
    let waiting = jobs.iter().any(|j| j.state == JobState::Waiting);
    let failed = jobs.iter().all(|j| matches!(j.state, JobState::Failed(_)));
    let running: Vec<_> = jobs.iter().filter(|j| matches!(j.state, JobState::Running | JobState::Waiting)).collect();
    let share = if running.is_empty() {
        1.
    } else {
        running
            .iter()
            .map(|j| {
                let p = j.progress;
                if p.total_bytes > 0 {
                    p.bytes as f32 / p.total_bytes as f32
                } else if p.total_items > 0 {
                    p.items as f32 / p.total_items as f32
                } else {
                    0.
                }
            })
            .sum::<f32>()
            / running.len() as f32
    };
    let alt = format!(
        "File jobs: {} running{}{}",
        running.len(),
        if waiting { ", waiting for you" } else { "" },
        if failed { ", failed" } else { "" }
    );
    let ring = t.size_progress_ring;
    let mark = if failed && running.is_empty() {
        isle_ui::Glyph::new("warning-circle", t.size_glyph_large, colour(t.colour_danger_text)).fill(true).into_element()
    } else {
        rect()
            .width(Size::px(ring))
            .height(Size::px(ring))
            .child(ring_svg(ring, share, colour(t.colour_fill_strong), colour(t.colour_accent)))
            .maybe_child(waiting.then(|| {
                rect()
                    .position(Position::new_absolute().right(-1.).top(-1.))
                    .width(Size::px(t.size_mark))
                    .height(Size::px(t.size_mark))
                    .corner_radius(t.size_mark / 2.)
                    .background(colour(t.colour_warn))
            }))
            .into_element()
    };
    rect()
        .width(Size::px(t.size_control_large))
        .height(Size::px(t.size_control_large))
        .center()
        .corner_radius(t.radius_control)
        .maybe(open, |el| el.background(colour(t.colour_control_pressed)))
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(alt)
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            on_press();
        })
        .child(mark)
        .into_element()
}

/// A job's row in the popover: what it does, its bar, how far, and its stop button.
fn job_row(theme: &isle_tokens::Theme, job: &JobView, send: impl Fn(Act) + 'static) -> Element {
    let t = &theme.tokens;
    let p = job.progress;
    let share = if p.total_bytes > 0 {
        p.bytes as f32 / p.total_bytes as f32
    } else if p.total_items > 0 {
        p.items as f32 / p.total_items as f32
    } else {
        0.
    };
    let line = match &job.state {
        JobState::Waiting => "Waiting".to_owned(),
        JobState::Done => "Done".to_owned(),
        JobState::Failed(why) => why.clone(),
        JobState::Running if p.total_bytes > 0 => {
            let elapsed = job.started.elapsed().as_secs_f64();
            let left = if share > 0.01 { elapsed / share as f64 - elapsed } else { 0. };
            let about = if left < 60. { "less than a minute".to_owned() } else { format!("{} minutes", (left / 60.).ceil()) };
            format!("{} of {}, about {about}", format::size(p.bytes), format::size(p.total_bytes))
        }
        JobState::Running => format!("{} of {}", p.items, p.total_items),
    };
    let id = job.id;
    let ink = match job.state {
        JobState::Waiting => t.colour_warn_text,
        JobState::Failed(_) => t.colour_danger_text,
        _ => t.colour_text_2,
    };
    rect()
        .width(Size::px(t.size_jobs - 2. * t.space_m))
        .padding((t.space_s, 0.))
        .spacing(t.space_xs)
        .child(text::line(theme, text::Style::Body, colour(t.colour_text), job.title.clone()))
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::px(6.))
                        .corner_radius(3.)
                        .background(colour(t.colour_fill_strong))
                        .child(
                            rect()
                                .width(Size::percent(share.clamp(0., 1.) * 100.))
                                .height(Size::fill())
                                .corner_radius(3.)
                                .background(colour(if job.state == JobState::Waiting { t.colour_text_3 } else { t.colour_accent })),
                        ),
                )
                .child(
                    rect()
                        .width(Size::px(20.))
                        .height(Size::px(20.))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt("Stop")
                        .on_press(move |_| send(Act::StopJob(id)))
                        .child(isle_ui::Glyph::new("x-circle", 16., colour(t.colour_text_2)).fill(true)),
                ),
        )
        .child(text::line(theme, text::Style::Subheadline, colour(ink), line))
        .into_element()
}

/// A progress ring: its track and the share done from twelve o'clock, 2 points wide.
fn ring_svg(size: f32, share: f32, track: Color, fill: Color) -> Element {
    let hex = |c: Color| format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
    let alpha = |c: Color| c.a() as f32 / 255.;
    let r = size / 2. - 1.;
    let c = size / 2.;
    let share = share.clamp(0., 0.9999);
    let angle = share * std::f32::consts::TAU;
    let (x, y) = (c + r * angle.sin(), c - r * angle.cos());
    let large = if share > 0.5 { 1 } else { 0 };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}"><circle cx="{c}" cy="{c}" r="{r}" fill="none" stroke="{}" stroke-opacity="{}" stroke-width="2"/><path d="M {c} {} A {r} {r} 0 {large} 1 {x} {y}" fill="none" stroke="{}" stroke-width="2" stroke-linecap="round"/></svg>"#,
        hex(track),
        alpha(track),
        c - r,
        hex(fill),
    );
    let key = format!("ring-{size}-{:.3}", share);
    SvgViewer::new((key, bytes::Bytes::from(svg)))
        .show_loader(false)
        .width(Size::px(size))
        .height(Size::px(size))
        .into_element()
}
