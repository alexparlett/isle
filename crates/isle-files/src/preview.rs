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
    /// The pane's leading edge dragged to a width, to keep.
    pub on_resize: Option<EventHandler<f32>>,
}

/// The facts of an item's kind: an image's dimensions, a recording's duration, a PDF's
/// pages; reads the file, so call it on a worker.
pub fn kind_facts(path: &std::path::Path, mime: &str) -> Vec<(&'static str, String)> {
    let mut facts = Vec::new();
    if mime.starts_with("image/")
        && let Ok(size) = imagesize::size(path)
    {
        facts.push(("Dimensions", format!("{} × {}", size.width, size.height)));
    }
    if mime.starts_with("video/") || mime.starts_with("audio/") {
        let probed = std::process::Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
            .arg(path)
            .output();
        if let Some(seconds) = probed.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok()) {
            let s = seconds.round() as u64;
            facts.push(("Duration", if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }));
        }
    }
    if mime == "application/pdf"
        && let Ok(bytes) = std::fs::read(path)
    {
        let pages = bytes.windows(10).filter(|w| *w == b"/Type/Page" || w.starts_with(b"/Type /Pag") && !w.ends_with(b"s")).count();
        if pages > 0 {
            facts.push(("Pages", pages.to_string()));
        }
    }
    facts
}

/// How many facts show before Show More.
const FACTS_SHOWN: usize = 3;

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
        let mut facts = use_state(|| None::<(PathBuf, Vec<(&'static str, String)>)>);
        let mut more = use_state(|| false);
        // The pane's edge being dragged: where it started and the width then, and the width now.
        let mut resizing = use_state(|| None::<(f64, f32)>);
        let mut live = use_state(|| None::<f32>);
        let width = live.read().unwrap_or(self.width);
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
        let reading = shown.as_ref().filter(|e| !e.is_dir()).map(|e| (e.path.clone(), e.mime.as_deref().unwrap_or("").to_owned()));
        use_side_effect_with_deps(&reading, move |reading| {
            let Some((path, mime)) = reading.clone() else { return };
            if facts.peek().as_ref().is_some_and(|(p, _)| *p == path) {
                return;
            }
            spawn(async move {
                let read = path.clone();
                if let Some(found) = off_thread(move || kind_facts(&read, &mime)).await {
                    facts.set(Some((path, found)));
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
                    .children({
                        // Created, Modified, Last opened, then the kind's facts; Show More past three.
                        let mut rows: Vec<(&str, String)> = Vec::new();
                        if let Some(c) = entry.created {
                            rows.push(("Created", format::date(c, &now)));
                        }
                        rows.push(("Modified", format::date(entry.modified, &now)));
                        rows.push(("Last opened", format::date(entry.accessed, &now)));
                        if entry.is_dir() {
                            rows.push(("Size", size.clone()));
                        }
                        if let Some((path, found)) = facts.read().as_ref()
                            && *path == entry.path
                        {
                            rows.extend(found.iter().map(|(l, v)| (*l, v.clone())));
                        }
                        let open = *more.read();
                        let hidden = rows.len().saturating_sub(FACTS_SHOWN);
                        let mut out: Vec<Element> = rows
                            .into_iter()
                            .take(if open { usize::MAX } else { FACTS_SHOWN })
                            .map(|(label, value)| fact(&theme, label, value))
                            .collect();
                        if hidden > 0 {
                            out.push(
                                rect()
                                    .width(Size::fill())
                                    .padding((t.space_xs, 0., 0., 0.))
                                    .horizontal()
                                    .main_align(Alignment::End)
                                    .child(
                                        rect()
                                            .a11y_role(AccessibilityRole::Button)
                                            .on_press(move |_| more.toggle())
                                            .child(text::line(
                                                &theme,
                                                text::Style::Callout,
                                                colour(t.colour_accent_text),
                                                if open { "Show Less" } else { "Show More" },
                                            )),
                                    )
                                    .into_element(),
                            );
                        }
                        out
                    })
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
        let on_resize = self.on_resize.clone();
        let edge = on_resize.is_some().then(|| {
            rect()
                .position(Position::new_absolute().left(-t.size_resize_band / 2.).top(0.))
                .layer(Layer::Overlay)
                .width(Size::px(t.size_resize_band))
                .height(Size::fill())
                .a11y_role(AccessibilityRole::Splitter)
                .a11y_alt("Preview width")
                .on_mouse_down(move |e: Event<MouseEventData>| {
                    if e.button == Some(MouseButton::Left) {
                        e.stop_propagation();
                        resizing.set(Some((e.global_location.x, width)));
                    }
                })
        });
        let (min, max) = (t.size_inspector_min, t.size_inspector_max);
        rect()
            .width(Size::px(width))
            .height(Size::fill())
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                let Some((x0, start)) = *resizing.peek() else { return };
                // The leading edge moved left widens the pane.
                live.set(Some((start - (e.global_location().x - x0) as f32).clamp(min, max)));
            })
            .on_global_pointer_up(move |_: Event<PointerEventData>| {
                if resizing.peek().is_some() {
                    resizing.set(None);
                    if let (Some(width), Some(on_resize)) = (*live.peek(), on_resize.as_ref()) {
                        on_resize.call(width);
                    }
                }
            })
            .maybe_child(edge)
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
