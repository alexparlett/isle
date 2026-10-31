//! Large files, Untouched and Duplicates (`p1/surfaces/cleaner.md`): the files the scan kept and
//! the duplicate groups, each ticked into the basket.

use std::{path::Path, rc::Rc};

use freya::prelude::*;
use isle_scan::{Kind, duplicates::Group, kinds};
use isle_tokens::Theme;
use isle_ui::{
    Glyph, colour, scroll_area,
    table::{Cell, Column, Mark, Row, Sort, Table, TableEvent},
    text, use_theme,
};

use crate::{
    basket::Basket,
    cleanup::{Tick, checkbox},
    model::{FileRow, size},
    space::{Act, Entry},
    ui::{kind_colour, kind_glyph, kind_tile, mono},
};

/// Whether every root records when a file is opened: with `noatime` the access time stays at
/// the file's making, and Untouched says "changed" (`p19b-cleaner.md` choice 13).
pub fn records_access(roots: &[std::path::PathBuf]) -> bool {
    let mounts = isle_fs::mounts::read();
    roots.iter().all(|r| {
        isle_fs::mounts::of(&mounts, r)
            .is_some_and(|m| !m.options.split(',').any(|o| o == "noatime"))
    })
}

/// "2 Oct 2026".
pub fn date(seconds: i64) -> String {
    jiff::Timestamp::from_second(seconds)
        .map(|t| {
            t.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%-d %b %Y")
                .to_string()
        })
        .unwrap_or_else(|_| "--".into())
}

pub fn entry_of(f: &FileRow) -> Entry {
    Entry {
        path: f.path.clone(),
        name: f
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        bytes: f.bytes,
        kind: f.kind,
        node: None,
        dev: f.dev,
        ino: f.ino,
    }
}

#[derive(Clone)]
pub struct Files {
    pub files: Rc<Vec<FileRow>>,
    pub basket: Basket,
    /// The last column's title: "Last opened" or "Last changed".
    pub used_title: &'static str,
    pub empty: String,
    /// The window has focus, for the selection's colour.
    pub active: bool,
    pub on_act: EventHandler<Act>,
}

impl PartialEq for Files {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Component for Files {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut sort = use_state(|| Sort {
            column: "size",
            descending: true,
        });
        let mut selected = use_state(Vec::<u64>::new);
        let controller = use_scroll_controller(ScrollConfig::default);
        let ties = crate::ties::Ties::get();
        if self.files.is_empty() {
            return rect()
                .width(Size::fill())
                .height(Size::fill())
                .center()
                .background(colour(t.colour_view))
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    self.empty.clone(),
                ))
                .into_element();
        }
        let s = *sort.read();
        let mut order: Vec<usize> = (0..self.files.len()).collect();
        let files = self.files.clone();
        order.sort_by(|a, b| {
            let (a, b) = (&files[*a], &files[*b]);
            let o = match s.column {
                "name" => a.path.file_name().cmp(&b.path.file_name()),
                "folder" => a.path.parent().cmp(&b.path.parent()),
                "kind" => a.kind.cmp(&b.kind),
                "used" => a.used.cmp(&b.used),
                _ => a.bytes.cmp(&b.bytes),
            };
            if s.descending { o.reverse() } else { o }
        });
        let rows: Vec<Row> = order
            .iter()
            .map(|i| {
                let f = &self.files[*i];
                Row {
                    key: *i as u64,
                    name: f
                        .path
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                    mark: Some(Mark {
                        fill: kind_colour(&theme, f.kind),
                        glyph: format!("{}-fill", kind_glyph(f.kind)),
                        alt: f.kind.name().into(),
                    }),
                    check: Some(self.basket.covers(&f.path)),
                    cells: vec![
                        Cell::from(crate::ui::cut_path(
                            f.path.parent().unwrap_or(Path::new("/")),
                            48,
                        )),
                        Cell::from(f.kind.name().to_owned()),
                        Cell::from(date(f.used)),
                        Cell::from(size(f.bytes)),
                    ],
                    ..Default::default()
                }
            })
            .collect();
        let (on, files) = (self.on_act.clone(), self.files.clone());
        Table {
            columns: Rc::new(vec![
                Column::text("name", "Name", 140.),
                Column::text("folder", "Folder", 150.),
                Column::text("kind", "Kind", 110.),
                Column::text("used", self.used_title, 100.),
                Column::figures("size", "Size", 72.),
            ]),
            rows: Rc::new(rows),
            sort: s,
            selected: Rc::new(selected.read().clone()),
            focused: self.active,
            hold: false,
            on_event: EventHandler::new(move |ev: TableEvent| {
                let at = |k: u64| files.get(k as usize).map(entry_of);
                match ev {
                    TableEvent::Selected(keys) => selected.set(keys),
                    TableEvent::Sort(next) => sort.set(next),
                    TableEvent::Check(k, on_) => {
                        if let Some(e) = at(k) {
                            on.call(Act::Basket(e, on_));
                        }
                    }
                    TableEvent::Open(k) => {
                        if let Some(e) = at(k) {
                            crate::ties::show_in_files(&ties, vec![e.path]);
                        }
                    }
                    TableEvent::Menu(Some(k), point) => {
                        if let Some(e) = at(k) {
                            on.call(Act::Menu(e, point));
                        }
                    }
                    _ => {}
                }
            }),
            controller,
        }
        .into_element()
    }
}

/// Duplicates: each group a head and its copies under it.
pub fn duplicates(
    theme: &Theme,
    groups: Option<&[Group]>,
    kind: Option<Kind>,
    query: &str,
    basket: &Basket,
    on_act: EventHandler<Act>,
) -> Element {
    let t = &theme.tokens;
    let centred = |line: &str| {
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .center()
            .background(colour(t.colour_view))
            .child(text::line(
                theme,
                text::Style::Body,
                colour(t.colour_text_2),
                line.to_owned(),
            ))
            .into_element()
    };
    let Some(groups) = groups else {
        return centred("Comparing files…");
    };
    let shown: Vec<&Group> = groups
        .iter()
        .filter(|g| {
            let first = &g.copies[0].path;
            let k = kinds::of_path(first);
            kind.is_none_or(|want| want == k)
                && (query.is_empty()
                    || first
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().to_lowercase().contains(query)))
        })
        .collect();
    if shown.is_empty() {
        return centred(if query.is_empty() {
            "No duplicates found"
        } else {
            "No results"
        });
    }
    let mut children: Vec<Element> = Vec::new();
    for (gi, g) in shown.iter().enumerate() {
        if gi > 0 {
            children.push(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.stroke_hairline))
                    .background(colour(t.colour_hairline))
                    .into_element(),
            );
        }
        let first = &g.copies[0].path;
        let k = kinds::of_path(first);
        let name = first
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        children.push(
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row_large))
                .padding((0., t.space_m))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(kind_tile(theme, k))
                .child(rect().width(Size::flex(1.)).child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    name.clone(),
                )))
                .child(mono(
                    theme,
                    t.colour_text_2,
                    format!("{} × {}", size(g.size), g.copies.len()),
                ))
                .into_element(),
        );
        for (ci, c) in g.copies.iter().enumerate() {
            let ticked = basket.covers(&c.path);
            let entry = Entry {
                path: c.path.clone(),
                name: name.clone(),
                bytes: c.bytes,
                kind: k,
                node: None,
                dev: 0,
                ino: 0,
            };
            let (tick, menu) = (on_act.clone(), on_act.clone());
            let (for_tick, for_menu) = (entry.clone(), entry.clone());
            children.push(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .padding((0., t.space_m, 0., t.space_m + 36.))
                    .horizontal()
                    .content(Content::flex())
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .maybe(ci % 2 == 1, |el| el.background(colour(t.colour_fill)))
                    .on_mouse_down(move |e: Event<MouseEventData>| {
                        if e.button == Some(MouseButton::Right) {
                            menu.call(Act::Menu(
                                for_menu.clone(),
                                (e.global_location.x as f32, e.global_location.y as f32),
                            ));
                        }
                    })
                    .child(checkbox(
                        theme,
                        if ticked { Tick::On } else { Tick::Off },
                        c.path.display().to_string(),
                        EventHandler::new(move |_| {
                            tick.call(Act::Basket(for_tick.clone(), !ticked))
                        }),
                    ))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(text::line(
                                theme,
                                text::Style::Body,
                                colour(t.colour_text),
                                crate::ui::cut_path(c.path.parent().unwrap_or(Path::new("/")), 64),
                            ))
                            .maybe_child(c.shares_storage.then(|| {
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(t.space_xxs)
                                    .child(Glyph::new("link", 12., colour(t.colour_text_2)))
                                    .child(text::line(
                                        theme,
                                        text::Style::Callout,
                                        colour(t.colour_text_2),
                                        "Shares storage",
                                    ))
                                    .into_element()
                            })),
                    )
                    .child(text::line(
                        theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        date(c.modified),
                    ))
                    .into_element(),
            );
        }
    }
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(colour(t.colour_view))
        .child(
            scroll_area(theme)
                .width(Size::fill())
                .height(Size::fill())
                .child(rect().width(Size::fill()).children(children)),
        )
        .into_element()
}
