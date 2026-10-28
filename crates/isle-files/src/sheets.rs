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

/// An alert waiting on the window, in Finder's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub title: String,
    pub message: String,
    pub items: Vec<String>,
    /// Buttons, the last the default.
    pub buttons: Vec<(String, Option<Then>)>,
}

impl Alert {
    /// A job's failures; `verb` is what the job did to its items: "copied", "moved".
    /// As `failures`, with Try Again running `retry` over the items that failed.
    pub fn failures_again(failed: &[(PathBuf, String)], verb: &str, retry: Option<Work>) -> Self {
        let mut alert = Self::failures(failed, verb);
        if let Some(work) = retry {
            alert.buttons = vec![("Try Again".into(), Some(Then::Job(work))), ("Done".into(), None)];
        }
        alert
    }

    pub fn failures(failed: &[(PathBuf, String)], verb: &str) -> Self {
        let n = failed.len();
        Self {
            title: if n == 1 {
                format!("The operation can't be completed because {}.", failed[0].1)
            } else {
                format!("The operation can't be completed because {n} items couldn't be {verb}.")
            },
            message: String::new(),
            items: if n == 1 { Vec::new() } else { failed.iter().take(20).map(|(p, _)| name(p)).collect() },
            buttons: vec![("OK".into(), None)],
        }
    }

    pub fn delete(items: Vec<PathBuf>) -> Self {
        let (title, message) = match items.as_slice() {
            [one] => (
                format!("Are you sure you want to delete “{}”?", name(one)),
                "This item will be deleted immediately. You can't undo this action.",
            ),
            many => (
                format!("Are you sure you want to delete these {} items?", many.len()),
                "These items will be deleted immediately. You can't undo this action.",
            ),
        };
        Self {
            title,
            message: message.into(),
            items: Vec::new(),
            buttons: vec![("Delete".into(), Some(Then::Job(Work::Delete { items }))), ("Cancel".into(), None)],
        }
    }

    /// Asks before erasing the Trash's `items`, read off the window's thread.
    pub fn empty_trash(items: Vec<isle_fs::trash::Trashed>) -> Self {
        Self {
            title: "Are you sure you want to permanently erase the items in the Trash?".into(),
            message: "You can't undo this action.".into(),
            items: Vec::new(),
            buttons: vec![("Empty Trash".into(), Some(Then::Job(Work::EmptyTrash { items }))), ("Cancel".into(), None)],
        }
    }

    /// A volume in use: Try Again the default, Force Eject never (`files.md` § Volumes).
    pub fn in_use(volume: isle_disks::Volume) -> Self {
        Self {
            title: format!("The disk “{}” wasn't ejected because one or more programs may be using it.", volume.label),
            message: "To eject the disk immediately, click the Force Eject button.".into(),
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

    /// A job that will not fit: `what` names its item, or "The items" for several.
    pub fn no_space(what: &str, verb: &str, short: u64) -> Self {
        Self {
            title: format!("{what} can't be {verb} because there isn't enough free space."),
            message: format!("You need an additional {} of free space.", format::size(short)),
            items: Vec::new(),
            buttons: vec![("OK".into(), None)],
        }
    }
}

pub fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// An alert as a sheet; `on_done` gets the work of the button pressed.
pub fn alert_sheet(alert: Alert, on_done: impl Fn(Option<Then>) + Clone + 'static) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let items = (!alert.items.is_empty()).then(|| {
        rect()
            .width(Size::fill())
            .max_height(Size::px(5. * t.size_row))
            .padding(t.space_s)
            .corner_radius(t.radius_control)
            .background(colour(t.colour_window_sunken))
            .children(alert.items.iter().map(|i| text::line(&theme, text::Style::Callout, colour(t.colour_text), i.clone()).into_element()))
            .into_element()
    });
    let buttons: Vec<isle_ui::AlertButton> = alert
        .buttons
        .into_iter()
        .map(|(label, work)| {
            // What erases or forces is never the default and reads as dangerous.
            let destructive = matches!(work, Some(Then::Job(Work::Delete { .. } | Work::EmptyTrash { .. })) | Some(Then::Eject(_, true)));
            let done = on_done.clone();
            isle_ui::AlertButton { label, on_press: EventHandler::new(move |_| done(work.clone())), destructive }
        })
        .collect();
    isle_ui::alert(&alert.title, &alert.message, items.into_iter().collect(), buttons)
}

/// The name-taken question in Finder's words (`files.md` § File jobs, A question): Keep
/// Both the default, which ⌥ turns into Merge when both are folders.
#[derive(PartialEq)]
pub struct ConflictSheet {
    pub question: Question,
    pub jobs: Jobs,
}

impl Component for ConflictSheet {
    fn render(&self) -> impl IntoElement {
        let mut all = use_state(|| false);
        let mut option = use_state(|| false);
        let c = &self.question.conflict;
        let (job, jobs) = (self.question.job, self.jobs);
        let answer = move |choice: Choice| {
            let all = *all.peek();
            EventHandler::new(move |_| jobs.answer(job, Answer { choice, all }))
        };
        let (title, message) = conflict_words(c);
        let extra = c.more.then(|| CheckRow::new("Apply to all", *all.read()).on_toggle(move |on: bool| all.set(on)).into_element());
        let last: (String, EventHandler<()>) = if c.folders && *option.read() {
            ("Merge".to_owned(), answer(Choice::Merge))
        } else {
            ("Keep Both".to_owned(), answer(Choice::KeepBoth))
        };
        let buttons = vec![
            ("Stop".to_owned(), answer(Choice::Stop)).into(),
            isle_ui::AlertButton::destructive("Replace", answer(Choice::Replace)),
            last.into(),
        ];
        let is_option = |e: &Event<KeyboardEventData>| matches!(e.key, Key::Named(NamedKey::Alt));
        rect()
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                if is_option(&e) {
                    option.set_if_modified(true);
                }
            })
            .on_global_key_up(move |e: Event<KeyboardEventData>| {
                if is_option(&e) {
                    option.set_if_modified(false);
                }
            })
            .child(isle_ui::alert(&title, &message, extra.into_iter().collect(), buttons))
    }
}

/// A name clash's title and question, as Finder words them.
pub fn conflict_words(c: &isle_fs::jobs::Conflict) -> (String, String) {
    let (item, other) = match c.existing_newer {
        Some(true) => ("A newer item", "the older one"),
        Some(false) => ("An older item", "the newer one"),
        None => ("An item", "the one"),
    };
    let doing = if c.moving { "moving" } else { "copying" };
    (
        format!("{item} named “{}” already exists in this location.", c.name),
        format!("Do you want to replace it with {other} you're {doing}?"),
    )
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
            isle_ui::alert_text(&format!("Rename {} Items", self.items.len()), ""),
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
            isle_ui::alert_text(&format!("Choose an application to open the document “{name}”."), ""),
            isle_ui::scroll_area(&theme)
                .width(Size::fill())
                .height(Size::px(8. * t.size_row))
                .child(rect().width(Size::fill()).children(rows))
                .into_element(),
            CheckRow::new("Show all applications", *all.read()).on_toggle(move |on: bool| all.set(on)).into_element(),
            CheckRow::new("Always Open With", *always.read()).on_toggle(move |on: bool| always.set(on)).into_element(),
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
            isle_ui::alert_text(&format!("Enter a password to unlock the disk “{}”.", self.volume.label), ""),
            isle_ui::Field::new(text.clone(), "Password")
                .secret(true)
                .error(self.refused.is_some())
                .auto_focus(true)
                .on_submit(move |_| on_submit())
                .into_element(),
        ];
        if let Some(why) = &self.refused {
            body.push(text::line(&theme, text::Style::Callout, colour(t.colour_danger_text), why.clone()).into_element());
        }
        body.push(CheckRow::new("Remember this password in my keychain", *remember.read()).on_toggle(move |on: bool| remember.set(on)).into_element());
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

/// Saving a search: its name; `None` cancels (`surfaces/files.md` § Sheets).
#[derive(PartialEq)]
pub struct SaveSearch {
    pub suggested: String,
    pub on_done: EventHandler<Option<String>>,
}

impl Component for SaveSearch {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let name = use_state({
            let suggested = self.suggested.clone();
            move || suggested
        });
        let (done, submit, cancel) = (self.on_done.clone(), self.on_done.clone(), self.on_done.clone());
        let save = move |typed: String| {
            let typed = typed.trim().to_owned();
            if !typed.is_empty() {
                done.call(Some(typed));
            }
        };
        let save_button = save.clone();
        isle_ui::sheet(
            t.size_form_narrow,
            vec![
                isle_ui::alert_text("Save Search", ""),
                isle_ui::TextField::new(name, "Name")
                    .editing()
                    .on_submit(move |typed: String| save(typed))
                    .on_escape(move |_| submit.call(None))
                    .into_element(),
            ],
            vec![
                ("Cancel".into(), EventHandler::new(move |_| cancel.call(None))),
                ("Save".into(), EventHandler::new(move |_| save_button(name.peek().clone()))),
            ],
        )
    }
}
