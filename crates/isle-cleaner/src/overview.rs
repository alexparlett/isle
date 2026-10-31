//! Overview (`p1/surfaces/cleaner.md` Overview): the scope's figures, its bar and legend by
//! kind, what is safe to clear, the disks, and the largest folders and files.

use std::path::PathBuf;

use freya::prelude::*;
use isle_disks::Usage;
use isle_scan::Kind;
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, colour, form, text};

use crate::{
    model::{Snapshot, size},
    ui::{self, kind_colour, kind_tile, mono},
};

pub struct Overview<'a> {
    pub title: String,
    pub snapshot: Option<&'a Snapshot>,
    pub disks: &'a [Usage],
    pub safe: u64,
    pub on_review: EventHandler<()>,
    pub on_folder: EventHandler<PathBuf>,
    pub on_file: EventHandler<PathBuf>,
}

/// One segment of the bar or row of the legend.
struct Part {
    kind: Option<Kind>,
    name: String,
    bytes: u64,
}

fn figures(theme: &Theme, pairs: Vec<(&str, String)>) -> Element {
    let t = &theme.tokens;
    rect()
        .horizontal()
        .spacing(t.space_l)
        .children(
            pairs
                .into_iter()
                .map(|(label, value)| {
                    text::line(
                        theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        format!("{label} {value}"),
                    )
                    .into_element()
                })
                .collect::<Vec<_>>(),
        )
        .into_element()
}

fn bar(theme: &Theme, parts: &[Part], total: u64) -> Element {
    let t = &theme.tokens;
    let segments: Vec<Element> = parts
        .iter()
        .filter(|p| p.bytes > 0)
        .map(|p| {
            let fill = match p.kind {
                Some(k) => kind_colour(theme, k),
                None if p.name == "Free" => t.colour_fill,
                None => t.colour_fill_strong,
            };
            rect()
                .width(Size::flex(p.bytes as f32 / total.max(1) as f32))
                .min_width(Size::px(2.))
                .height(Size::fill())
                .background(colour(fill))
                .maybe(theme.appearance.increase_contrast, |el| {
                    el.border(
                        Border::new()
                            .fill(colour(t.colour_hairline_strong))
                            .width(t.stroke_hairline)
                            .alignment(BorderAlignment::Inner),
                    )
                })
                .a11y_alt(format!("{}, {}", p.name, size(p.bytes)))
                .into_element()
        })
        .collect();
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_kind_bar))
        .corner_radius(t.radius_small)
        .overflow(Overflow::Clip)
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_xxs)
        .children(segments)
        .into_element()
}

fn legend(theme: &Theme, parts: &[Part]) -> Element {
    let t = &theme.tokens;
    let rows: Vec<Element> = parts
        .iter()
        .filter(|p| p.bytes > 0)
        .map(|p| {
            let tile = match p.kind {
                Some(k) => kind_tile(theme, k),
                None => rect()
                    .width(Size::px(t.size_pane_tile))
                    .height(Size::px(t.size_pane_tile))
                    .corner_radius(t.radius_row)
                    .background(colour(if p.name == "Free" {
                        t.colour_fill
                    } else {
                        t.colour_fill_strong
                    }))
                    .border(
                        Border::new()
                            .fill(colour(t.colour_hairline))
                            .width(t.stroke_hairline)
                            .alignment(BorderAlignment::Inner),
                    )
                    .into_element(),
            };
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(tile)
                .child(rect().width(Size::flex(1.)).child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    p.name.clone(),
                )))
                .child(mono(theme, t.colour_text_2, size(p.bytes)))
                .into_element()
        })
        .collect();
    let one_column = theme.appearance.text_scale > 1.3;
    let per = if one_column {
        rows.len()
    } else {
        rows.len().div_ceil(2)
    };
    let mut columns: Vec<Element> = Vec::new();
    let mut rows = rows.into_iter();
    while columns.len() < 2 {
        let column: Vec<Element> = rows.by_ref().take(per).collect();
        if column.is_empty() {
            break;
        }
        columns.push(rect().width(Size::flex(1.)).children(column).into_element());
    }
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .spacing(t.space_xxl)
        .children(columns)
        .into_element()
}

fn nav_row(
    theme: &Theme,
    kind: Kind,
    name: String,
    path: &std::path::Path,
    bytes: u64,
    on_open: EventHandler<()>,
) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., t.space_m))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .a11y_role(AccessibilityRole::Link)
        .a11y_alt(name.clone())
        .on_press(move |_| on_open.call(()))
        .child(kind_tile(theme, kind))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    name,
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    ui::cut_path(path, 64),
                )),
        )
        .child(mono(theme, t.colour_text, size(bytes)))
        .child(Glyph::new("caret-right", 12., colour(t.colour_text_2)))
        .into_element()
}

/// A disk's row: its name over used of total, and its bar.
pub fn disk_row(theme: &Theme, disk: &Usage) -> Element {
    let t = &theme.tokens;
    let full = disk.nearly_full();
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., t.space_m))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .child(Glyph::new(
            if full { "warning" } else { "hard-drive" },
            20.,
            colour(if full { t.colour_warn } else { t.colour_text_2 }),
        ))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    disk.name.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    format!(
                        "{} of {} used",
                        size(disk.total - disk.free),
                        size(disk.total)
                    ),
                )),
        )
        .child(ui::progress_bar(
            theme,
            disk.used(),
            Size::px(t.size_row_slider),
            if full { t.colour_warn } else { t.colour_text_2 },
        ))
        .into_element()
}

pub fn view(theme: &Theme, o: Overview) -> Element {
    let t = &theme.tokens;
    let used: u64 = o.disks.iter().map(|d| d.total - d.free).sum();
    let free: u64 = o.disks.iter().map(|d| d.free).sum();
    let scanned = o.snapshot.map_or(0, |s| s.bytes);
    let whole = o.snapshot.is_some_and(|s| s.progress.used.is_some());
    let mut parts: Vec<Part> = Kind::ALL
        .iter()
        .map(|k| Part {
            kind: Some(*k),
            name: k.name().into(),
            bytes: o.snapshot.map_or(0, |s| s.kinds[k.index()]),
        })
        .collect();
    if whole {
        parts.push(Part {
            kind: None,
            name: "Not scanned".into(),
            bytes: used.saturating_sub(scanned),
        });
        parts.push(Part {
            kind: None,
            name: "Free".into(),
            bytes: free,
        });
    }
    let total: u64 = parts.iter().map(|p| p.bytes).sum();

    let mut sections: Vec<Element> = Vec::new();
    let mut head = vec![("Scanned", size(scanned))];
    if whole {
        head.insert(0, ("Free", size(free)));
        head.insert(0, ("Used", size(used)));
    }
    sections.push(
        rect()
            .width(Size::fill())
            .spacing(t.space_xxs)
            .child(text::line(
                theme,
                text::Style::Title2,
                colour(t.colour_text),
                o.title.clone(),
            ))
            .child(figures(theme, head))
            .into_element(),
    );
    sections.push(bar(theme, &parts, total));
    sections.push(legend(theme, &parts));
    if o.safe > 0 {
        let review = o.on_review.clone();
        sections.push(
            form::group(
                None,
                vec![
                    rect()
                        .width(Size::fill())
                        .height(Size::px(t.size_row_large))
                        .padding((0., t.space_m))
                        .horizontal()
                        .content(Content::flex())
                        .cross_align(Alignment::Center)
                        .spacing(t.space_s)
                        .child(ui::plain_tile(
                            theme,
                            "broom-fill",
                            t.pane_green,
                            t.colour_on_accent,
                        ))
                        .child(rect().width(Size::flex(1.)).child(text::line(
                            theme,
                            text::Style::Headline,
                            colour(t.colour_text),
                            format!("{} safe to clear", size(o.safe)),
                        )))
                        .child(Button::new("Review").on_press(move |_| review.call(())))
                        .into_element(),
                ],
            )
            .into_element(),
        );
    }
    let mut disk_rows: Vec<Element> = o.disks.iter().map(|d| disk_row(theme, d)).collect();
    if let Some(s) = o.snapshot
        && !s.unreadable.is_empty()
    {
        disk_rows.push(
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row_large))
                .padding((0., t.space_m))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(Glyph::new("info", 16., colour(t.colour_text_2)))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    format!("{} folders couldn't be read", s.unreadable.len()),
                ))
                .into_element(),
        );
    }
    if !disk_rows.is_empty() {
        sections.push(form::group(None, disk_rows).into_element());
    }
    let limit = t.limit_overview_rows as usize;
    if let Some(s) = o.snapshot {
        let folders: Vec<Element> = s
            .folders
            .iter()
            .take(limit)
            .map(|f| {
                let (path, on) = (f.path.clone(), o.on_folder.clone());
                let name = f.path.file_name().map_or_else(
                    || f.path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                nav_row(
                    theme,
                    f.kind,
                    name,
                    &f.path,
                    f.bytes,
                    EventHandler::new(move |_| on.call(path.clone())),
                )
            })
            .collect();
        if !folders.is_empty() {
            sections.push(form::group(Some("Largest folders".into()), folders).into_element());
        }
        let files: Vec<Element> = s
            .largest
            .iter()
            .take(limit)
            .map(|f| {
                let (path, on) = (f.path.clone(), o.on_file.clone());
                let name = f
                    .path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let folder = f.path.parent().unwrap_or(&f.path);
                nav_row(
                    theme,
                    f.kind,
                    name,
                    folder,
                    f.bytes,
                    EventHandler::new(move |_| on.call(path.clone())),
                )
            })
            .collect();
        if !files.is_empty() {
            sections.push(form::group(Some("Largest files".into()), files).into_element());
        }
    }
    rect()
        .width(Size::fill())
        .spacing(t.space_xl)
        .children(sections)
        .into_element()
}
