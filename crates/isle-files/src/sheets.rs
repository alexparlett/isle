//! The alerts a window shows over itself: a job's question, its failures, and what asks
//! before it erases (`p1/structure/files.md` § File jobs, Trash; `surfaces/files.md` §
//! Questions and alerts).

use std::path::{Path, PathBuf};

use freya::prelude::*;
use isle_fs::jobs::{Answer, Choice, Work};
use isle_ui::{colour, field::CheckRow, text, use_theme};

use crate::{format, jobs::Jobs, jobs::Question};

/// What a button of an alert does beyond closing it.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    Job(Work),
    /// Ejects the volume; `true` even while it is in use.
    Eject(isle_disks::Volume, bool),
}

/// An alert waiting on the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub title: String,
    pub message: String,
    pub items: Vec<String>,
    /// Buttons, the last the default.
    pub buttons: Vec<(String, Option<Then>)>,
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
            buttons: vec![("Delete".into(), Some(Then::Job(Work::Delete { items }))), ("Cancel".into(), None)],
        }
    }

    /// Asks before erasing the Trash's `items`, read off the window's thread.
    pub fn empty_trash(items: Vec<isle_fs::trash::Trashed>) -> Self {
        Self {
            title: format!("Permanently erase {} in the Trash?", format::items(items.len())),
            message: "You can't undo this.".into(),
            items: Vec::new(),
            buttons: vec![("Empty Trash".into(), Some(Then::Job(Work::EmptyTrash { items }))), ("Cancel".into(), None)],
        }
    }

    /// A volume in use: Force Eject never the default (`files.md` § Volumes).
    pub fn in_use(volume: isle_disks::Volume) -> Self {
        Self {
            title: format!("“{}” is in use and can't be ejected.", volume.label),
            message: "Quit the apps using it and try again.".into(),
            items: Vec::new(),
            buttons: vec![
                ("Force Eject".into(), Some(Then::Eject(volume.clone(), true))),
                ("Cancel".into(), None),
                ("Try Again".into(), Some(Then::Eject(volume, false))),
            ],
        }
    }

    pub fn message(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { title: title.into(), message: message.into(), items: Vec::new(), buttons: vec![("OK".into(), None)] }
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
pub fn alert_sheet(alert: Alert, on_done: impl Fn(Option<Then>) + Clone + 'static) -> Element {
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

/// Rename Many: one name where `#` is each item's number, a start number, and the first and
/// last names it makes (`files.md` § Acting on files; v1's D71).
#[derive(PartialEq)]
pub struct RenameMany {
    pub items: Vec<PathBuf>,
    pub on_done: EventHandler<Option<Work>>,
}

/// The name an item takes: the pattern with its number, the item's extension kept.
pub fn many_name(pattern: &str, n: u64, item: &Path) -> String {
    let pattern = if pattern.contains('#') { pattern.to_owned() } else { format!("{pattern} #") };
    let name = item.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = if item.is_dir() { "" } else { isle_fs::jobs::split_ext(&name).1 };
    format!("{}{ext}", pattern.replace('#', &n.to_string()))
}

impl Component for RenameMany {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let pattern = use_state(|| "Untitled #".to_owned());
        let start = use_state(|| "1".to_owned());
        let from: u64 = start.read().trim().parse().unwrap_or(1);
        let (one, last) = (self.items.first(), self.items.last());
        let example = match (one, last) {
            (Some(a), Some(b)) => format!(
                "{} … {}",
                many_name(&pattern.read(), from, a),
                many_name(&pattern.read(), from + self.items.len() as u64 - 1, b)
            ),
            _ => String::new(),
        };
        let (items, done, cancel) = (self.items.clone(), self.on_done.clone(), self.on_done.clone());
        let body = vec![
            isle_ui::alert_text(&format!("Rename {}", format::items(self.items.len())), ""),
            isle_ui::TextField::new(pattern, "Name").editing().into_element(),
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(text::line(&theme, text::Style::Body, colour(t.colour_text), "Start numbers at:"))
                .child(isle_ui::TextField::new(start, "Start number").width(Size::px(80.)))
                .into_element(),
            text::line(&theme, text::Style::Callout, colour(t.colour_text_2), example).into_element(),
        ];
        isle_ui::sheet(
            t.size_form_narrow,
            body,
            vec![
                ("Cancel".into(), EventHandler::new(move |_| cancel.call(None))),
                (
                    "Rename".into(),
                    EventHandler::new(move |_| {
                        done.call(Some(Work::RenameMany {
                            items: items.clone(),
                            pattern: pattern.peek().clone(),
                            start: from,
                        }))
                    }),
                ),
            ],
        )
    }
}

/// Open With: the apps for the file's type, the default first, or every app; Always Open
/// With makes the choice the type's default (`files.md` § Acting on files, Open With).
#[derive(PartialEq)]
pub struct OpenWith {
    pub offer: crate::open::Offer,
    /// The app chosen, whether it is now the default, the type and the file; `None` cancels.
    pub on_done: EventHandler<Option<(String, bool, String, PathBuf)>>,
}

impl Component for OpenWith {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut all = use_state(|| false);
        let mut always = use_state(|| false);
        let mut chosen = use_state(|| self.offer.apps.first().map(|a| a.id.clone()));
        let showing = if *all.read() || self.offer.apps.is_empty() { &self.offer.all } else { &self.offer.apps };
        let rows: Vec<Element> = showing
            .iter()
            .map(|app| {
                let selected = chosen.read().as_deref() == Some(app.id.as_str());
                let id = app.id.clone();
                rect()
                    .key(app.id.clone())
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .padding((0., t.space_s))
                    .main_align(Alignment::Center)
                    .corner_radius(t.radius_row)
                    .maybe(selected, |el| el.background(colour(t.colour_accent)))
                    .a11y_role(AccessibilityRole::ListItem)
                    .a11y_alt(app.name.clone())
                    .on_press(move |_| chosen.set(Some(id.clone())))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(if selected { t.colour_on_accent } else { t.colour_text }),
                        app.name.clone(),
                    ))
                    .into_element()
            })
            .collect();
        let name = self.offer.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let body = vec![
            isle_ui::alert_text(&format!("Choose an app to open “{name}”."), ""),
            isle_ui::scroll_area(&theme)
                .width(Size::fill())
                .height(Size::px(8. * t.size_row))
                .child(rect().width(Size::fill()).children(rows))
                .into_element(),
            CheckRow::new("Show all apps", *all.read()).on_toggle(move |on: bool| all.set(on)).into_element(),
            CheckRow::new("Always open with", *always.read()).on_toggle(move |on: bool| always.set(on)).into_element(),
        ];
        let (done, cancel) = (self.on_done.clone(), self.on_done.clone());
        let (kind, path) = (self.offer.kind.clone(), self.offer.path.clone());
        isle_ui::sheet(
            t.size_form_narrow,
            body,
            vec![
                ("Cancel".into(), EventHandler::new(move |_| cancel.call(None))),
                (
                    "Open".into(),
                    EventHandler::new(move |_| {
                        if let Some(app) = chosen.peek().clone() {
                            done.call(Some((app, *always.peek(), kind.clone(), path.clone())));
                        }
                    }),
                ),
            ],
        )
    }
}

/// An encrypted volume's passphrase, with Remember in Keychain (`surfaces/files.md` §
/// Questions and alerts, An encrypted volume).
#[derive(PartialEq)]
pub struct Unlock {
    pub volume: isle_disks::Volume,
    /// Why the last passphrase was refused.
    pub refused: Option<String>,
    /// The passphrase and whether to keep it; `None` cancels.
    pub on_done: EventHandler<Option<(isle_ui::field::Secret, bool)>>,
}

impl Component for Unlock {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let text: isle_ui::FieldText = use_hook(Default::default);
        let mut remember = use_state(|| true);
        let submit = {
            let (text, done) = (text.clone(), self.on_done.clone());
            move || {
                let secret = isle_ui::field::Secret::new(text.borrow().to_string());
                done.call(Some((secret, *remember.peek())));
            }
        };
        let on_submit = submit.clone();
        let mut body = vec![
            isle_ui::alert_text(&format!("“{}” is encrypted.", self.volume.label), "Enter its passphrase to unlock it."),
            isle_ui::Field::new(text.clone(), "Passphrase")
                .secret(true)
                .error(self.refused.is_some())
                .auto_focus(true)
                .on_submit(move |_| on_submit())
                .into_element(),
        ];
        if let Some(why) = &self.refused {
            body.push(text::line(&theme, text::Style::Callout, colour(t.colour_danger_text), why.clone()).into_element());
        }
        body.push(CheckRow::new("Remember in Keychain", *remember.read()).on_toggle(move |on: bool| remember.set(on)).into_element());
        let cancel = self.on_done.clone();
        isle_ui::sheet(
            t.size_form_narrow,
            body,
            vec![
                ("Cancel".into(), EventHandler::new(move |_| cancel.call(None))),
                ("Unlock".into(), EventHandler::new(move |_| submit())),
            ],
        )
    }
}
