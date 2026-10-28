//! Go to Folder: a path field completing from the filesystem, Recent Folders under it
//! (`p1/surfaces/files.md` § Sheets, Go to Folder).

use std::path::{Path, PathBuf};

use freya::prelude::*;
use isle_ui::{colour, text, use_theme};

use crate::window::off_thread;

/// `typed` as a path: `~` is home, and a relative path is under `from`.
pub fn resolve(typed: &str, from: Option<&Path>) -> Option<PathBuf> {
    let typed = typed.trim();
    if typed.is_empty() {
        return None;
    }
    let home = crate::places::home();
    if typed == "~" {
        return home;
    }
    if let Some(rest) = typed.strip_prefix("~/") {
        return home.map(|h| h.join(rest));
    }
    let path = PathBuf::from(typed);
    if path.is_absolute() { Some(path) } else { from.map(|f| f.join(path)) }
}

/// The folders `typed` could go on to: those in its folder whose names start with what
/// follows its last `/`, case aside, hidden ones only when that starts with a dot.
pub fn completions(typed: &str, from: Option<&Path>) -> Vec<PathBuf> {
    let (dir, prefix) = match typed.rfind('/') {
        Some(at) => (&typed[..=at], &typed[at + 1..]),
        None if typed.starts_with('~') => return Vec::new(),
        None => ("", typed),
    };
    let Some(dir) = resolve(if dir.is_empty() { "." } else { dir }, from) else { return Vec::new() };
    let lower = prefix.to_lowercase();
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            name.starts_with(&lower) && (prefix.starts_with('.') || !name.starts_with('.'))
        })
        .map(|e| e.path())
        .collect();
    found.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    found.truncate(8);
    found
}

/// `path` as the field shows it, home as `~`.
fn shown(path: &Path) -> String {
    match crate::places::home().and_then(|h| path.strip_prefix(&h).ok().map(PathBuf::from)) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[derive(PartialEq)]
pub struct GoToFolder {
    /// What the field starts with: empty, or the `/` or `~` typed in a view.
    pub start: String,
    /// The folder shown, which a relative path is under.
    pub from: Option<PathBuf>,
    pub recent: Vec<PathBuf>,
    /// The path to go to, or none for Cancel.
    pub on_done: EventHandler<Option<PathBuf>>,
}

impl Component for GoToFolder {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let start = self.start.clone();
        let typed = use_state(move || start);
        let mut found = use_state(Vec::<PathBuf>::new);
        let error = use_state(|| None::<String>);
        let from = self.from.clone();
        let value = typed.read().clone();
        use_side_effect_with_deps(&value, {
            let from = from.clone();
            move |value: &String| {
                let (value, from) = (value.clone(), from.clone());
                spawn(async move {
                    if let Some(list) = off_thread(move || completions(&value, from.as_deref())).await {
                        found.set(list);
                    }
                });
            }
        });

        let go = {
            let (on_done, from) = (self.on_done.clone(), from.clone());
            move |text: String| {
                let mut error = error;
                match resolve(&text, from.as_deref()) {
                    Some(path) if path.exists() => on_done.call(Some(path)),
                    _ => error.set(Some(format!("The folder “{}” can't be found.", text.trim()))),
                }
            }
        };
        let pick = move |path: &Path| {
            let (mut typed, mut error) = (typed, error);
            typed.set(format!("{}/", shown(path)));
            error.set(None);
        };
        let row = |label: Element, on_press: Box<dyn Fn()>| {
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .main_align(Alignment::Center)
                .corner_radius(t.radius_row)
                .a11y_role(AccessibilityRole::Button)
                .on_press(move |_| on_press())
                .child(label)
                .into_element()
        };
        let typed_name = value.rsplit('/').next().unwrap_or_default().chars().count();
        let completions: Vec<Element> = found.read().iter().map(|path| {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let cut = name.char_indices().nth(typed_name).map_or(name.len(), |(i, _)| i);
            let (head, tail) = name.split_at(cut);
            let label = paragraph()
                .span(Span::new(head.to_owned()).color(colour(t.colour_text)))
                .span(Span::new(tail.to_owned()).color(colour(t.colour_text_2)))
                .font_size(text::metrics(&theme, text::Style::Body).0)
                .into_element();
            let path = path.clone();
            row(label, Box::new(move || pick(&path)))
        }).collect();
        let recent: Vec<Element> = if completions.is_empty() && !value.contains('/') {
            self.recent
                .iter()
                .take(6)
                .map(|path| {
                    let label = text::line(&theme, text::Style::Body, colour(t.colour_text), shown(path)).into_element();
                    let (go, path) = (go.clone(), path.clone());
                    row(label, Box::new(move || go(path.display().to_string())))
                })
                .collect()
        } else {
            Vec::new()
        };
        let complete_first = {
            let first = found.read().first().cloned();
            move |_: ()| {
                if let Some(first) = first.clone() {
                    pick(&first);
                }
            }
        };
        let cancel = self.on_done.clone();
        let mut body = vec![
            isle_ui::alert_text("Go to Folder", ""),
            rect()
                .width(Size::fill())
                .child(
                    isle_ui::TextField::new(typed, "Folder")
                        .placeholder("Folder")
                        .editing_part(value.chars().count()..value.chars().count())
                        .on_tab(complete_first)
                        .on_submit({
                            let go = go.clone();
                            move |text: String| go(text)
                        }),
                )
                .into_element(),
        ];
        if let Some(error) = error.read().clone() {
            body.push(text::line(&theme, text::Style::Callout, colour(t.colour_danger_text), error).into_element());
        }
        if !completions.is_empty() {
            body.push(rect().width(Size::fill()).children(completions).into_element());
        }
        if !recent.is_empty() {
            body.push(
                text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), "Recent Folders")
                    .font_weight(600)
                    .into_element(),
            );
            body.push(rect().width(Size::fill()).children(recent).into_element());
        }
        isle_ui::sheet(
            t.size_form_narrow,
            body,
            vec![
                ("Cancel".into(), EventHandler::new(move |_| cancel.call(None))),
                ("Go".into(), EventHandler::new(move |_| go(typed.peek().clone()))),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_path_resolves_and_completes_its_last_part() {
        let base = tempfile::tempdir().unwrap();
        for dir in ["Documents", "Downloads", "desk", ".hidden", "Music"] {
            std::fs::create_dir(base.path().join(dir)).unwrap();
        }
        std::fs::write(base.path().join("Doc.txt"), "").unwrap();
        let at = |p: &str| format!("{}/{p}", base.path().display());
        let names = |typed: &str| -> Vec<String> {
            completions(typed, None).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
        };
        assert_eq!(names(&at("do")), ["Documents", "Downloads"]);
        assert_eq!(names(&at("D")), ["desk", "Documents", "Downloads"]);
        assert_eq!(names(&at(".")), [".hidden"]);
        assert_eq!(completions("Mus", Some(base.path())), [base.path().join("Music")]);
        assert_eq!(resolve("Music", Some(base.path())), Some(base.path().join("Music")));
        assert_eq!(resolve("  ", None), None);
    }
}
