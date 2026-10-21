//! The alerts a window shows over itself: a job's question, its failures, and what asks
//! before it erases (`p1/structure/files.md` § File jobs, Trash; `surfaces/files.md` §
//! Questions and alerts).

use std::path::{Path, PathBuf};

use freya::prelude::*;
use isle_fs::jobs::{Answer, Choice, Work};
use isle_ui::{colour, field::CheckRow, text, use_theme};

use crate::{format, jobs::Jobs, jobs::Question};

/// An alert waiting on the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub title: String,
    pub message: String,
    pub items: Vec<String>,
    /// Buttons, the last the default; one with work starts it.
    pub buttons: Vec<(String, Option<Work>)>,
}

impl Alert {
    pub fn failures(failed: &[(PathBuf, String)]) -> Self {
        let shared_reason = failed.iter().all(|(_, why)| *why == failed[0].1);
        let n = failed.len();
        Self {
            title: if n == 1 {
                format!("“{}” couldn't be copied.", name(&failed[0].0))
            } else {
                format!("{n} items couldn't be copied.")
            },
            message: if shared_reason { failed[0].1.clone() } else { String::new() },
            items: failed.iter().take(20).map(|(p, _)| name(p)).collect(),
            buttons: vec![("Done".into(), None)],
        }
    }

    pub fn delete(items: Vec<PathBuf>) -> Self {
        Self {
            title: format!("Delete {}?", if items.len() == 1 { format!("“{}”", name(&items[0])) } else { format::items(items.len()) }),
            message: "You can't undo this.".into(),
            items: Vec::new(),
            buttons: vec![("Delete".into(), Some(Work::Delete { items })), ("Cancel".into(), None)],
        }
    }

    /// Asks before erasing the Trash's `items`, read off the window's thread.
    pub fn empty_trash(items: Vec<isle_fs::trash::Trashed>) -> Self {
        Self {
            title: format!("Permanently erase {} in the Trash?", format::items(items.len())),
            message: "You can't undo this.".into(),
            items: Vec::new(),
            buttons: vec![("Empty Trash".into(), Some(Work::EmptyTrash { items })), ("Cancel".into(), None)],
        }
    }

    pub fn no_space(into: &Path, needed: u64, free: u64) -> Self {
        Self {
            title: format!("“{}” doesn't have enough space.", crate::places::folder_name(into)),
            message: format!(
                "{} is needed and {} is available. Free up space or choose another location.",
                format::size(needed),
                format::size(free)
            ),
            items: Vec::new(),
            buttons: vec![("OK".into(), None)],
        }
    }
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// An alert sheet; `on_done` gets the work of the button pressed.
pub fn alert_sheet(alert: Alert, on_done: impl Fn(Option<Work>) + Clone + 'static) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let mut body = vec![isle_ui::alert_text(&alert.title, &alert.message)];
    if !alert.items.is_empty() {
        body.push(
            rect()
                .width(Size::fill())
                .max_height(Size::px(5. * t.size_row))
                .padding(t.space_s)
                .corner_radius(t.radius_control)
                .background(colour(t.colour_window_sunken))
                .children(alert.items.iter().map(|i| text::line(&theme, text::Style::Body, colour(t.colour_text), i.clone()).into_element()))
                .into_element(),
        );
    }
    let buttons = alert
        .buttons
        .into_iter()
        .rev()
        .map(|(label, work)| {
            let done = on_done.clone();
            (label, EventHandler::new(move |_| done(work.clone())))
        })
        .collect();
    isle_ui::sheet(t.size_alert, body, buttons)
}

/// The name-taken question (`files.md` § File jobs, A question).
#[derive(PartialEq)]
pub struct ConflictSheet {
    pub question: Question,
    pub jobs: Jobs,
}

impl Component for ConflictSheet {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut all = use_state(|| false);
        let c = &self.question.conflict;
        let newer = match c.existing_newer {
            Some(true) => "The one already there is newer.",
            Some(false) => "The one you're copying is newer.",
            None => "",
        };
        let (job, jobs) = (self.question.job, self.jobs);
        let answer = move |choice: Choice| {
            let all = *all.peek();
            EventHandler::new(move |_| jobs.answer(job, Answer { choice, all }))
        };
        let mut body = vec![isle_ui::alert_text(
            &format!("An item named “{}” already exists in “{}”.", c.name, crate::places::folder_name(&c.folder)),
            newer,
        )];
        if c.more {
            body.push(
                CheckRow::new("Apply to all", *all.read())
                    .on_toggle(move |on: bool| all.set(on))
                    .into_element(),
            );
        }
        let mut buttons = vec![("Replace".to_owned(), answer(Choice::Replace)), ("Stop".to_owned(), answer(Choice::Stop))];
        if c.folders {
            buttons.push(("Merge".to_owned(), answer(Choice::Merge)));
        }
        buttons.push(("Keep Both".to_owned(), answer(Choice::KeepBoth)));
        isle_ui::sheet(t.size_alert, body, buttons)
    }
}
