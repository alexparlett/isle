//! The Copy window: Files' jobs with no Files window open, each with its progress and stop
//! button, a name clash asked in its row and a failure told there (Finder's Copy window).

use std::rc::Rc;

use freya::prelude::*;
use isle_fs::jobs::{Answer, Choice};
use isle_ui::{Glyph, colour, field::CheckRow, text, use_theme};

use crate::{
    Shared, format,
    jobs::{JobState, JobView, Question},
    sheets::conflict_words,
};

pub const WIDTH: f64 = 420.;
/// A job's row, and the question asked under one.
const ROW: f64 = 68.;
const ASK: f64 = 120.;
const MAX: f64 = 480.;

#[derive(Clone)]
pub struct CopyWindow {
    pub shared: Rc<Shared>,
}

impl App for CopyWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let key = isle_app::this_window();
        let jobs = shared.jobs;
        let list = jobs.list.read().clone();
        let questions = jobs.questions.read().clone();
        // With nothing left to show, it goes.
        use_side_effect({
            let shared = shared.clone();
            move || {
                if jobs.list.read().is_empty() {
                    shared.app().close(key);
                }
            }
        });
        // Return and Escape answer the first question waiting.
        let first = questions.first().map(|q| q.job);
        let on_key = move |e: Event<KeyboardEventData>| {
            let Some(job) = first else { return };
            let choice = match e.key {
                Key::Named(NamedKey::Enter) => Choice::KeepBoth,
                Key::Named(NamedKey::Escape) => Choice::Stop,
                _ => return,
            };
            jobs.answer(job, Answer { choice, all: false });
        };
        rect()
            .expanded()
            .background(colour(t.colour_window))
            .on_global_key_down(on_key)
            .child(isle_ui::scroll_area(&theme).width(Size::fill()).height(Size::fill()).child(rect().width(Size::fill()).children(
                list.into_iter().enumerate().map(|(n, job)| {
                    let question = questions.iter().find(|q| q.job == job.id).cloned();
                    job_row(&theme, &shared, job, question, n > 0)
                }),
            )))
    }
}

/// The window's height for its rows: a job's, and a question's under one.
pub fn height(list: &[JobView], questions: &[Question]) -> f64 {
    let asked = list.iter().filter(|j| questions.iter().any(|q| q.job == j.id)).count() as f64;
    (list.len() as f64 * ROW + asked * ASK).min(MAX)
}

fn job_row(theme: &isle_tokens::Theme, shared: &Rc<Shared>, job: JobView, question: Option<Question>, rule: bool) -> Element {
    let t = &theme.tokens;
    let p = job.progress;
    let share = if p.total_bytes > 0 {
        p.bytes as f32 / p.total_bytes as f32
    } else if p.total_items > 0 {
        p.items as f32 / p.total_items as f32
    } else {
        0.
    };
    let failed = matches!(job.state, JobState::Failed(_));
    let line = match &job.state {
        JobState::Waiting => "Waiting…".to_owned(),
        JobState::Failed(why) => format!("The operation can't be completed because {why}."),
        JobState::Done => "Done".to_owned(),
        JobState::Running => progress_line(&job, share),
    };
    let (id, jobs, stop) = (job.id, shared.jobs, job.stop.clone());
    let mut list = jobs.list;
    // Stop on a running job, clear on a failed one.
    let on_stop = move |_| {
        if failed {
            list.write().retain(|j| j.id != id);
        } else {
            stop.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    };
    let ink = if failed { t.colour_danger_text } else { t.colour_text_2 };
    rect()
        .width(Size::fill())
        .padding((t.space_m, t.space_l))
        .maybe(rule, |el| {
            el.border(Border::new().fill(colour(t.colour_hairline)).width(BorderWidth { top: 1., right: 0., bottom: 0., left: 0. }).alignment(BorderAlignment::Inner))
        })
        .spacing(t.space_s)
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_m)
                .child(Glyph::new("folder-simple", t.size_glyph_large, colour(t.colour_accent)).fill(true))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(t.space_xs)
                        .child(text::line(theme, text::Style::Body, colour(t.colour_text), job.title.clone()))
                        .maybe_child((job.state == JobState::Running || job.state == JobState::Waiting).then(|| {
                            rect()
                                .width(Size::fill())
                                .height(Size::px(4.))
                                .corner_radius(2.)
                                .background(colour(t.colour_fill_strong))
                                .child(rect().width(Size::percent(share.clamp(0., 1.) * 100.)).height(Size::fill()).corner_radius(2.).background(colour(t.colour_accent)))
                        }))
                        .child(
                            label()
                                .width(Size::fill())
                                .font_size(text::metrics(theme, text::Style::Subheadline).0)
                                .color(colour(ink))
                                .text(line),
                        ),
                )
                .child(
                    rect()
                        .width(Size::px(t.size_control))
                        .height(Size::px(t.size_control))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt(if job.state == JobState::Running || job.state == JobState::Waiting { "Stop" } else { "Clear" })
                        .on_press(on_stop)
                        .child(Glyph::new("x-circle", t.size_glyph, colour(t.colour_text_2)).fill(true)),
                ),
        )
        .maybe_child(question.map(|q| AskRow { question: q, jobs }.into_element()))
        .into_element()
}

/// "1.7 GB of 4.5 GB — About 2 minutes", or "10 of 12" for a job counted in items.
fn progress_line(job: &JobView, share: f32) -> String {
    let p = job.progress;
    if p.total_bytes == 0 {
        return format!("{} of {}", p.items, p.total_items);
    }
    let elapsed = job.started.elapsed().as_secs_f64();
    let left = if share > 0.01 { elapsed / share as f64 - elapsed } else { 0. };
    let about = match left {
        l if l < 5. => "Almost done".to_owned(),
        l if l < 60. => "About a minute".to_owned(),
        l => format!("About {} minutes", (l / 60.).round().max(2.)),
    };
    format!("{} of {} — {about}", format::size(p.bytes), format::size(p.total_bytes))
}

/// A name clash asked inside its job's row.
#[derive(PartialEq)]
struct AskRow {
    question: Question,
    jobs: crate::jobs::Jobs,
}

impl Component for AskRow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut all = use_state(|| false);
        let c = &self.question.conflict;
        let (title, message) = conflict_words(c);
        let (job, jobs) = (self.question.job, self.jobs);
        let answer = move |choice: Choice| {
            let all = *all.peek();
            EventHandler::new(move |_| jobs.answer(job, Answer { choice, all }))
        };
        let button = |label: &str, choice: Choice, default: bool| {
            isle_ui::Button::new(label)
                .kind(if default { isle_ui::button::Kind::Default } else { isle_ui::button::Kind::Regular })
                .on_press(answer(choice))
                .into_element()
        };
        rect()
            .width(Size::fill())
            .padding(t.space_m)
            .spacing(t.space_s)
            .corner_radius(t.radius_card)
            .background(colour(t.colour_window_sunken))
            .child(
                label()
                    .width(Size::fill())
                    .font_size(text::metrics(&theme, text::Style::Headline).0)
                    .font_weight(text::metrics(&theme, text::Style::Headline).2 as i32)
                    .color(colour(t.colour_text))
                    .text(title),
            )
            .child(label().width(Size::fill()).font_size(text::metrics(&theme, text::Style::Callout).0).color(colour(t.colour_text_2)).text(message))
            .maybe_child(c.more.then(|| CheckRow::new("Apply to all", *all.read()).on_toggle(move |on: bool| all.set(on))))
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .main_align(Alignment::End)
                    .spacing(t.space_s)
                    .child(button("Stop", Choice::Stop, false))
                    .child(button("Replace", Choice::Replace, false))
                    .child(button("Keep Both", Choice::KeepBoth, true)),
            )
    }
}
