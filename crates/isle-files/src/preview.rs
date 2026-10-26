//! The preview: the inspector at the window's trailing edge in Icons and List, the last
//! column in Columns (`p1/structure/files.md` § The preview; `surfaces/files.md` § The
//! preview). Quick Look's renderer replaces the icon in 15.13.

use std::path::PathBuf;

use freya::prelude::*;
use isle_fs::Entry;
use isle_ui::{colour, text, use_theme};

use crate::{format, list_view, window::off_thread};

#[derive(Clone, PartialEq)]
pub struct Preview {
    /// The items selected; none previews the folder shown.
    pub items: Vec<Entry>,
    /// The folder shown, previewed when nothing is selected.
    pub folder: Option<Entry>,
    pub width: f32,
    /// Drawn as the window's pane, edged and filled; bare in Quick Look.
    pub pane: bool,
    /// The button under one item's facts: Markup for an image, Put Back in the Trash.
    pub action: Option<(&'static str, EventHandler<()>)>,
}

/// A folder's size and count, counted on a thread of its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counted {
    pub path: PathBuf,
    pub items: usize,
    pub bytes: u64,
}

pub fn count(path: PathBuf) -> Counted {
    let (mut items, mut bytes) = (0usize, 0u64);
    if let Ok(read) = std::fs::read_dir(&path) {
        items = read.count();
    }
    let mut stack = vec![path.clone()];
    // A folder's size is its files', followed as far as a minute of counting allows.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while let Some(dir) = stack.pop() {
        if std::time::Instant::now() > deadline {
            break;
        }
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for entry in read.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                bytes += meta.len();
            }
        }
    }
    Counted { path, items, bytes }
}

fn fact(theme: &isle_tokens::Theme, label: &str, value: String) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_control_small))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .child(rect().width(Size::flex(1.)).child(text::line(
            theme,
            text::Style::Callout,
            colour(t.colour_text_2),
            label.to_owned(),
        )))
        .child(text::line(theme, text::Style::Callout, colour(t.colour_text), value))
        .into_element()
}

impl Component for Preview {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dark = theme.appearance.mode == isle_tokens::Mode::Dark;
        let mut counted = use_state(|| None::<Counted>);
        let shown = match self.items.as_slice() {
            [] => self.folder.clone(),
            [one] => Some(one.clone()),
            _ => None,
        };
        let counting = shown.as_ref().filter(|e| e.is_dir()).map(|e| e.path.clone());
        use_side_effect_with_deps(&counting, move |counting| {
            let Some(path) = counting.clone() else { return };
            if counted.peek().as_ref().is_some_and(|c| c.path == path) {
                return;
            }
            counted.set(None);
            spawn(async move {
                if let Some(found) = off_thread(move || count(path)).await {
                    counted.set(Some(found));
                }
            });
        });
        let body = rect()
            .width(Size::fill())
            .padding(t.space_l)
            .cross_align(Alignment::Center);
        let body = match (&shown, self.items.len()) {
            (Some(entry), _) => {
                let now = format::now();
                let kind = list_view::kind(entry);
                let counted = counted.read().clone().filter(|c| c.path == entry.path);
                let size = if entry.is_dir() {
                    match &counted {
                        Some(c) => format!("{}, {}", format::items(c.items), format::size(c.bytes)),
                        None => "Calculating…".into(),
                    }
                } else {
                    format::size(entry.size)
                };
                body.child(crate::thumbnail::Thumbnail {
                    entry: entry.clone(),
                    size: t.size_inspector - 2. * t.space_l,
                    dark,
                    remote: false,
                })
                    .child(rect().height(Size::px(t.space_m)))
                    .child(
                        label()
                            .width(Size::fill())
                            .font_size(text::metrics(&theme, text::Style::Title3).0)
                            .font_weight(text::metrics(&theme, text::Style::Title3).2 as i32)
                            .color(colour(t.colour_text))
                            .text_align(TextAlign::Center)
                            .max_lines(3)
                            .text_overflow(TextOverflow::Ellipsis)
                            .text(entry.name.clone()),
                    )
                    .child(text::line(
                        &theme,
                        text::Style::Callout,
                        colour(t.colour_text_2),
                        if entry.is_dir() { kind } else { format!("{kind}, {size}") },
                    ))
                    .child(rect().height(Size::px(t.space_l)))
                    .child(
                        rect()
                            .width(Size::fill())
                            .border(
                                Border::new()
                                    .fill(colour(t.colour_hairline))
                                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                                    .alignment(BorderAlignment::Inner),
                            )
                            .child(
                                text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), "Information")
                                    .font_weight(600),
                            ),
                    )
                    .maybe_child(entry.created.map(|c| fact(&theme, "Created", format::date(c, &now))))
                    .child(fact(&theme, "Modified", format::date(entry.modified, &now)))
                    .child(fact(&theme, "Last opened", format::date(entry.accessed, &now)))
                    .maybe_child(entry.is_dir().then(|| fact(&theme, "Size", size)))
                    .maybe_child(self.action.clone().map(|(label, on_press)| {
                        rect()
                            .width(Size::fill())
                            .padding((t.space_l, 0., 0., 0.))
                            .center()
                            .child(isle_ui::Button::new(label).on_press(on_press))
                    }))
            }
            (None, n) => {
                let total: u64 = self.items.iter().filter(|e| !e.is_dir()).map(|e| e.size).sum();
                let stack = self.items.iter().take(3).enumerate().rev().map(|(i, e)| {
                    rect()
                        .position(Position::new_absolute().left(i as f32 * t.space_s).top(i as f32 * t.space_s))
                        .opacity([1., 0.8, 0.6][i])
                        .child(list_view::icon(e, dark, t.size_app_icon_hero - 2. * t.space_s, t.colour_text_2))
                        .into_element()
                });
                body.child(
                    rect()
                        .width(Size::px(t.size_app_icon_hero))
                        .height(Size::px(t.size_app_icon_hero))
                        .children(stack),
                )
                .child(rect().height(Size::px(t.space_m)))
                .child(text::line(&theme, text::Style::Title3, colour(t.colour_text), format::items(n)))
                .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), format::size(total)))
            }
        };
        rect()
            .width(Size::px(self.width))
            .height(Size::fill())
            .maybe(self.pane, |pane| {
                pane.background(colour(t.colour_window)).border(
                    Border::new()
                        .fill(colour(t.colour_hairline))
                        .width(BorderWidth { top: 0., right: 0., bottom: 0., left: 1. })
                        .alignment(BorderAlignment::Inner),
                )
            })
            .a11y_role(AccessibilityRole::Complementary)
            .a11y_alt("Preview")
            .child(isle_ui::scroll_area(&theme).width(Size::fill()).height(Size::fill()).child(body))
    }
}
