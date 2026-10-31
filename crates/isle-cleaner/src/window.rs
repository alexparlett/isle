//! Cleaner's window (`p1/structure/cleaner.md`, `p1/surfaces/cleaner.md`): the toolbar with the
//! scope and the scan's progress, the sidebar of views with the disks at its foot, the view,
//! and the basket as the inspector.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, mpsc},
    time::Duration,
};

use freya::prelude::*;
use isle_app::{Chord, ChordKey, Command, Desktop, Menu, ToolbarExt};
use isle_disks::Usage;
use isle_scan::{Find, duplicates::Group, system::Share};
use isle_ui::{Button, Glyph, SearchField, colour, form::PopUp, scroll_area, text, use_theme};

use crate::{
    basket::{Basket, Item},
    cleanup,
    model::{Scope, Snapshot, View, disk_name, size},
    overview,
    scanner::{self, Ask},
    state::{self, Remembered},
    ui::{kind_tile, mono},
};

/// The compositor's lights over the toolbar's leading end (`isle_shell::chrome::LIGHTS_END`).
const LIGHTS: f32 = 68.;

/// The window, on All disks or on a folder named at launch.
#[derive(Clone)]
pub struct Cleaner {
    pub folder: Option<PathBuf>,
}

fn commands(
    r: &Remembered,
    basket: &Basket,
    scanning: bool,
    can_back: bool,
    can_forward: bool,
) -> Vec<Command> {
    let mut out = Vec::new();
    for (i, v) in View::ALL.iter().enumerate() {
        out.push(
            Command::new(format!("view:{i}"), v.title(), Menu::View, 1)
                .chord(Chord::cmd(char::from(b'1' + i as u8)))
                .checked(r.view == *v),
        );
    }
    out.push(
        Command::new("basket", "Show Basket", Menu::View, 2)
            .chord(Chord::cmd('i').option())
            .enabled(!basket.is_empty()),
    );
    out.push(
        Command::new("back", "Back", Menu::Go, 1)
            .chord(Chord::cmd('['))
            .enabled(can_back),
    );
    out.push(
        Command::new("forward", "Forward", Menu::Go, 1)
            .chord(Chord::cmd(']'))
            .enabled(can_forward),
    );
    out.push(
        Command::new("up", "Enclosing Folder", Menu::Go, 1)
            .chord(Chord::cmd_key(ChordKey::Up))
            .enabled(r.view == View::Space),
    );
    out.push(match scanning {
        true => Command::new("stop", "Stop", Menu::File, 1).chord(Chord::cmd('.')),
        false => Command::new("rescan", "Rescan", Menu::File, 1).chord(Chord::cmd('r')),
    });
    out.push(Command::new("scan-folder", "Scan Folder…", Menu::File, 1).chord(Chord::cmd('o')));
    out.push(
        Command::new("move-to-trash", "Move to Trash", Menu::File, 2)
            .chord(Chord::cmd_key(ChordKey::Backspace))
            .enabled(!basket.is_empty()),
    );
    out.push(
        Command::new("empty-trash", "Empty Trash…", Menu::File, 2)
            .chord(Chord::cmd_key(ChordKey::Backspace).shift()),
    );
    out.push(Command::new("search", "Search", Menu::Edit, 9).chord(Chord::cmd('f')));
    out
}

/// The disks of the scope, by their own figures.
fn disks_of(scope: &Scope) -> Vec<Usage> {
    match scope {
        Scope::Folder(p) => {
            let mounts = isle_fs::mounts::read();
            let point = isle_fs::mounts::of(&mounts, p)
                .map_or_else(|| PathBuf::from("/"), |m| m.point.clone());
            isle_disks::usage([(point.display().to_string(), disk_name(&point))])
        }
        _ => isle_disks::usage(
            scope
                .roots()
                .into_iter()
                .map(|r| (r.display().to_string(), disk_name(&r))),
        ),
    }
}

/// A find as a basket item, with the device and inode the scan saw.
pub fn item_of(snapshot: &Snapshot, find: &Find) -> Item {
    let (dev, ino) = match find.node {
        Some(id) => snapshot.scan.read(|t| (t.node(id).dev, t.node(id).ino)),
        None => std::fs::symlink_metadata(&find.path).map_or((0, 0), |m| {
            use std::os::unix::fs::MetadataExt;
            (m.dev(), m.ino())
        }),
    };
    Item {
        path: find.path.clone(),
        name: find.name.clone(),
        bytes: find.bytes,
        kind: find.kind,
        dev,
        ino,
    }
}

/// The scope pop-up's choices: All disks, each disk, recent folders, Choose folder….
fn scope_choices(r: &Remembered, current: &Scope) -> Vec<(String, Option<Scope>)> {
    let mut out = vec![("All disks".to_owned(), Some(Scope::All))];
    for d in isle_scan::all_disks(&isle_fs::mounts::read()) {
        out.push((disk_name(&d.point), Some(Scope::Disk(d.point))));
    }
    for p in &r.recent {
        out.push((
            Scope::Folder(p.clone()).title(),
            Some(Scope::Folder(p.clone())),
        ));
    }
    if let Scope::Folder(p) = current
        && !r.recent.contains(p)
    {
        out.push((current.title(), Some(current.clone())));
    }
    out.push(("Choose folder…".to_owned(), None));
    out
}

impl App for Cleaner {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        let mut remembered = use_state(state::load);
        use_side_effect(move || state::save(&remembered.read()));
        let shown = isle_app::use_shown();
        let start = self.folder.clone();
        let mut scope = use_state(move || start.map_or(Scope::All, Scope::Folder));
        let mut snapshot = use_state(|| None::<Arc<Snapshot>>);
        let mut groups = use_state(|| None::<(u64, Arc<Vec<Group>>)>);
        let mut system = use_state(Vec::<Share>::new);
        let mut basket = use_state(Basket::default);
        let mut basket_hidden = use_state(|| false);
        let mut filled = use_state(|| 0u64);
        let mut wanted = use_state(|| 1u64);
        let mut started = use_state(|| 0u64);
        let mut cards_open = use_state(|| (false, false));
        let mut history = use_state(|| (vec![(View::Overview, None::<PathBuf>)], 0usize));
        let mut folder = use_state(|| None::<PathBuf>);
        let mut clearing = use_state(|| None::<isle_scan::system::Item>);
        let query = use_state(String::new);
        let mut space_selected = use_state(|| None::<PathBuf>);
        let mut zoom = use_state(|| None::<(crate::treemap::Rect, bool)>);
        let mut item_menu = use_state(|| None::<(crate::space::Entry, (f32, f32))>);
        let mut kind_filter = use_state(|| None::<isle_scan::Kind>);
        let mut more_menu = use_state(|| None::<(f32, f32)>);
        let app = use_consume::<isle_app::AppHandle>();

        let (ask, incoming) = use_hook(|| {
            let (ask, rx) = scanner::start(Duration::from_millis(t.timing_scan_layout_ms as u64));
            (Rc::new(ask), Rc::new(RefCell::new(Some(rx))))
        });
        use_hook(move || {
            let Some(mut rx) = incoming.borrow_mut().take() else {
                return;
            };
            spawn(async move {
                use futures_util::StreamExt;
                while let Some(event) = rx.next().await {
                    match event {
                        scanner::Event::Snapshot(snap) => {
                            // Safe to clear goes into the basket when a scan finishes (`apps.md`
                            // Cleaner 7).
                            if snap.finished() && *filled.peek() != snap.generation {
                                filled.set(snap.generation);
                                let mut b = basket.write();
                                for f in snap.finds.iter().filter(|f| f.safe) {
                                    b.add(item_of(&snap, f));
                                }
                            }
                            snapshot.set(Some(snap));
                        }
                        scanner::Event::Duplicates(g, found) => groups.set(Some((g, found))),
                        scanner::Event::System(shares) => system.set(shares),
                    }
                }
            });
        });

        // A scan starts only while the window is shown; one running carries on hidden.
        use_side_effect({
            let ask = ask.clone();
            move || {
                if *shown.read() && *wanted.read() > *started.peek() {
                    started.set(*wanted.peek());
                    let _ = ask.send(Ask::Scan(scope.peek().roots()));
                }
            }
        });

        let r = remembered.read().clone();
        let snap = snapshot.read().clone();
        let scanning = snap.as_ref().is_none_or(|s| !s.finished());
        let (stack, at) = history.read().clone();
        let can_back = at > 0;
        let can_forward = at + 1 < stack.len();
        isle_app::use_commands(commands(
            &r,
            &basket.read(),
            scanning,
            can_back,
            can_forward,
        ));

        let mut go = move |view: View, to: Option<PathBuf>| {
            let (mut stack, at) = history.peek().clone();
            if stack.get(at) == Some(&(view, to.clone())) {
                return;
            }
            stack.truncate(at + 1);
            stack.push((view, to.clone()));
            let at = stack.len() - 1;
            history.set((stack, at));
            remembered.write().view = view;
            folder.set(to);
        };
        let mut step = move |by: isize| {
            let (stack, at) = history.peek().clone();
            let next = at as isize + by;
            if next < 0 || next as usize >= stack.len() {
                return;
            }
            let (view, to) = stack[next as usize].clone();
            history.set((stack, next as usize));
            remembered.write().view = view;
            folder.set(to);
        };

        use_side_effect({
            let ask = ask.clone();
            move || {
                if desktop.commands.read().is_empty() {
                    return;
                }
                let mut queue = desktop.commands;
                for id in std::mem::take(&mut *queue.write()) {
                    if let Some(i) = id
                        .strip_prefix("view:")
                        .and_then(|i| i.parse::<usize>().ok())
                    {
                        go(View::ALL[i.min(5)], None);
                        continue;
                    }
                    match id.as_str() {
                        "back" => step(-1),
                        "forward" => step(1),
                        "rescan" => *wanted.write() += 1,
                        "stop" => {
                            let _ = ask.send(Ask::Stop);
                        }
                        "basket" => {
                            let hidden = *basket_hidden.peek();
                            basket_hidden.set(!hidden);
                        }
                        "up" => {
                            let at = folder.peek().clone();
                            let roots = snapshot
                                .peek()
                                .as_ref()
                                .map(|s| s.roots.clone())
                                .unwrap_or_default();
                            if let Some(at) = at {
                                let up = at
                                    .parent()
                                    .map(Path::to_path_buf)
                                    .filter(|p| roots.iter().any(|r| p.starts_with(r)));
                                zoom.set(None);
                                space_selected.set(Some(at));
                                go(View::Space, up);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        let disks = use_memo(move || {
            let _ = snapshot.read();
            disks_of(&scope.read())
        });
        let disks = disks.read().clone();

        // The toolbar.
        let subtitle = match snap.as_ref() {
            None => "Scanning…".to_owned(),
            Some(s) if s.progress.partial => "Partial scan".into(),
            Some(s) if !s.finished() => match s.progress.share() {
                Some(share) => format!("Scanning… {}%", (share * 100.).floor() as u32),
                None => "Scanning…".into(),
            },
            Some(s) => format!("{} · {} scanned", scope.read().title(), size(s.bytes)),
        };
        let share = snap
            .as_ref()
            .and_then(|s| (!s.finished()).then(|| s.progress.share()).flatten());
        let title_block = rect()
            .main_align(Alignment::Center)
            .spacing(t.space_xxs)
            .child(text::line(
                &theme,
                text::Style::Headline,
                colour(t.colour_text),
                r.view.title(),
            ))
            .child(text::line(
                &theme,
                text::Style::Subheadline,
                colour(t.colour_text_2),
                subtitle,
            ))
            .maybe_child(scanning.then(|| {
                crate::ui::progress_bar(
                    &theme,
                    share.unwrap_or(0.) as f32,
                    Size::fill(),
                    t.colour_accent,
                )
            }));
        let choices = scope_choices(&r, &scope.read());
        let current = choices
            .iter()
            .position(|(_, s)| s.as_ref() == Some(&*scope.read()))
            .unwrap_or(0);
        let scope_popup = {
            let choices = choices.clone();
            PopUp::new(
                choices.iter().map(|(n, _)| n.clone()).collect(),
                current,
                "Scope",
                move |i: usize| {
                    if let Some(Some(next)) = choices.get(i).map(|(_, s)| s.clone()) {
                        if next != *scope.peek() {
                            scope.set(next);
                            snapshot.set(None);
                            groups.set(None);
                            *wanted.write() += 1;
                        }
                    }
                },
            )
        };
        let ask_stop = ask.clone();
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .padding((0., t.space_l, 0., LIGHTS + t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .toolbar()
            .child(
                Button::icon("caret-left", "Back")
                    .enabled(can_back)
                    .on_press(move |_| step(-1)),
            )
            .child(
                Button::icon("caret-right", "Forward")
                    .enabled(can_forward)
                    .on_press(move |_| step(1)),
            )
            .child(
                rect()
                    .min_width(Size::px(160.))
                    .max_width(Size::px(320.))
                    .child(title_block),
            )
            .child(rect().width(Size::flex(1.)))
            .child(rect().min_width(Size::px(140.)).child(scope_popup))
            .maybe_child(
                matches!(
                    r.view,
                    View::Duplicates | View::LargeFiles | View::Untouched
                )
                .then(|| {
                    let mut options = vec!["All kinds".to_owned()];
                    options.extend(isle_scan::Kind::ALL.iter().map(|k| k.name().to_owned()));
                    let at = kind_filter.read().map_or(0, |k| k.index() + 1);
                    PopUp::new(options, at, "Kind", move |i: usize| {
                        kind_filter.set(i.checked_sub(1).map(|i| isle_scan::Kind::ALL[i.min(9)]))
                    })
                    .into_element()
                }),
            )
            .maybe_child((r.view == View::Untouched).then(|| {
                let options = crate::state::Period::ALL
                    .iter()
                    .map(|p| p.title().to_owned())
                    .collect();
                let at = crate::state::Period::ALL
                    .iter()
                    .position(|p| *p == r.period)
                    .unwrap_or(0);
                PopUp::new(options, at, "Period", move |i: usize| {
                    remembered.write().period = crate::state::Period::ALL[i.min(2)]
                })
                .into_element()
            }))
            .maybe_child((r.view == View::Space).then(|| {
                isle_ui::form::Segmented::new(
                    vec!["Map".into(), "List".into()],
                    usize::from(r.list),
                    "Space",
                    move |i: usize| remembered.write().list = i == 1,
                )
                .into_element()
            }))
            .child(rect().width(Size::flex(1.)))
            .child(match scanning {
                true => Button::new("Stop").on_press(move |_| {
                    let _ = ask_stop.send(Ask::Stop);
                }),
                false => Button::new("Rescan").on_press(move |_| *wanted.write() += 1),
            })
            .child(
                rect()
                    .on_mouse_down(move |e: Event<MouseEventData>| {
                        e.stop_propagation();
                        more_menu.set(Some((
                            e.global_location.x as f32,
                            e.global_location.y as f32 + 12.,
                        )));
                    })
                    .child(Button::icon("dots-three-circle", "More")),
            )
            .child(
                Button::icon(
                    if *basket_hidden.read() || basket.read().is_empty() {
                        "sidebar-simple"
                    } else {
                        "sidebar-simple-fill"
                    },
                    "Basket",
                )
                .enabled(!basket.read().is_empty())
                .on_press(move |_| {
                    let hidden = *basket_hidden.peek();
                    basket_hidden.set(!hidden);
                }),
            )
            .child(SearchField {
                value: query.into_writable(),
                placeholder: "Search".into(),
                width: 140.,
            });

        // The sidebar.
        let view_rows: Vec<Element> = View::ALL
            .iter()
            .map(|v| {
                let (v, selected) = (*v, r.view == *v);
                let ink = if selected {
                    t.colour_on_accent
                } else {
                    t.colour_text
                };
                let glyph_ink = if selected {
                    t.colour_on_accent
                } else {
                    t.colour_accent_text
                };
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.size_row))
                    .padding((0., t.space_s))
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(t.space_s)
                    .corner_radius(t.radius_row)
                    .maybe(selected, |el| el.background(colour(t.colour_accent)))
                    .a11y_role(AccessibilityRole::ListItem)
                    .a11y_alt(v.title())
                    .a11y_builder(move |node| node.set_selected(selected))
                    .on_press(move |_| go(v, None))
                    .child(Glyph::new(v.glyph(), t.size_glyph, colour(glyph_ink)))
                    .child(text::line(
                        &theme,
                        text::Style::Body,
                        colour(ink),
                        v.title(),
                    ))
                    .into_element()
            })
            .collect();
        let trash_bytes = snap.as_ref().map_or(0, |s| {
            let trashes = isle_fs::trash::all_trashes();
            s.scan.read(|tree| {
                trashes
                    .iter()
                    .filter_map(|d| tree.find(&d.root.join("files")))
                    .map(|id| tree.node(id).bytes)
                    .sum::<u64>()
            })
        });
        let disk_rows: Vec<Element> = disks
            .iter()
            .map(|d| {
                let full = d.nearly_full();
                rect()
                    .width(Size::fill())
                    .spacing(t.space_xs)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .content(Content::flex())
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(Glyph::new(
                                if full { "warning" } else { "hard-drive" },
                                16.,
                                colour(if full { t.colour_warn } else { t.colour_text_2 }),
                            ))
                            .child(rect().width(Size::flex(1.)).child(text::line(
                                &theme,
                                text::Style::Body,
                                colour(t.colour_text),
                                d.name.clone(),
                            )))
                            .child(text::line(
                                &theme,
                                text::Style::Subheadline,
                                colour(if full { t.colour_text } else { t.colour_text_2 }),
                                format!("{} free", size(d.free)),
                            )),
                    )
                    .child(crate::ui::progress_bar(
                        &theme,
                        d.used(),
                        Size::fill(),
                        if full { t.colour_warn } else { t.colour_text_2 },
                    ))
                    .into_element()
            })
            .collect();
        let sidebar = rect()
            .width(Size::px(220.))
            .height(Size::fill())
            .content(Content::flex())
            .background(colour(t.colour_sidebar))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .padding(t.space_s)
                    .spacing(t.space_xxs)
                    .children(view_rows),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(t.stroke_hairline))
                    .background(colour(t.colour_hairline)),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .padding(t.space_m)
                    .spacing(t.space_s)
                    .children(disk_rows)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .content(Content::flex())
                            .cross_align(Alignment::Center)
                            .spacing(t.space_s)
                            .child(Glyph::new("trash", 16., colour(t.colour_text_2)))
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Body,
                                        colour(t.colour_text),
                                        "Trash",
                                    ))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Subheadline,
                                        colour(t.colour_text_2),
                                        size(trash_bytes),
                                    )),
                            )
                            .child(Button::new("Empty Trash…").small()),
                    ),
            );

        // The view.
        let search = query.read().trim().to_lowercase();
        let finds: Vec<Find> = snap
            .as_ref()
            .map(|s| {
                s.finds
                    .iter()
                    .filter(|f| search.is_empty() || f.name.to_lowercase().contains(&search))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        // A tick or a menu from Space or a table.
        let on_pick = EventHandler::new(move |a: crate::space::Act| match a {
            crate::space::Act::Basket(e, on) => {
                if on {
                    let (dev, ino) = if e.dev == 0 && e.ino == 0 {
                        std::fs::symlink_metadata(&e.path).map_or((0, 0), |m| {
                            use std::os::unix::fs::MetadataExt;
                            (m.dev(), m.ino())
                        })
                    } else {
                        (e.dev, e.ino)
                    };
                    basket.write().add(Item {
                        path: e.path,
                        name: e.name,
                        bytes: e.bytes,
                        kind: e.kind,
                        dev,
                        ino,
                    });
                } else {
                    basket.write().remove(&e.path);
                }
            }
            crate::space::Act::Menu(e, at) => item_menu.set(Some((e, at))),
            _ => {}
        });
        let content = match r.view {
            View::Overview => {
                let safe = snap.as_ref().map_or(0, |s| {
                    s.finds.iter().filter(|f| f.safe).map(|f| f.bytes).sum()
                });
                let column = overview::view(
                    &theme,
                    overview::Overview {
                        title: scope.read().title(),
                        snapshot: snap.as_deref(),
                        disks: &disks,
                        safe,
                        on_review: EventHandler::new(move |_| go(View::Cleanup, None)),
                        on_folder: EventHandler::new(move |p: PathBuf| go(View::Space, Some(p))),
                        on_file: EventHandler::new(move |_| go(View::LargeFiles, None)),
                    },
                );
                column_view(&theme, column)
            }
            View::Cleanup => {
                let tick_snap = snap.clone();
                let all_snap = snap.clone();
                let column = cleanup::view(
                    &theme,
                    cleanup::Cleanup {
                        finds: &finds,
                        system: &system.read(),
                        basket: &basket.read(),
                        open: *cards_open.read(),
                        on_tick: EventHandler::new(move |(f, on): (Find, bool)| {
                            let Some(s) = tick_snap.as_ref() else { return };
                            if on {
                                basket.write().add(item_of(s, &f));
                            } else {
                                basket.write().remove(&f.path);
                            }
                        }),
                        on_tick_all: EventHandler::new(move |on: bool| {
                            let Some(s) = all_snap.as_ref() else { return };
                            let mut b = basket.write();
                            for f in s.finds.iter().filter(|f| f.safe) {
                                if on {
                                    b.add(item_of(s, f))
                                } else {
                                    b.remove(&f.path)
                                }
                            }
                        }),
                        on_more: EventHandler::new(move |open| cards_open.set(open)),
                        on_clear: EventHandler::new(move |item| clearing.set(Some(item))),
                    },
                );
                column_view(&theme, column)
            }
            View::LargeFiles | View::Untouched => match snap.as_ref() {
                Some(s) => {
                    let untouched = r.view == View::Untouched;
                    let kind = *kind_filter.read();
                    let cutoff = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs() as i64)
                        - r.period.secs();
                    let records = crate::tables::records_access(&s.roots);
                    let files: Vec<crate::model::FileRow> =
                        (if untouched { &s.untouched } else { &s.largest })
                            .iter()
                            .filter(|f| !untouched || f.used < cutoff)
                            .filter(|f| kind.is_none_or(|k| k == f.kind))
                            .filter(|f| {
                                search.is_empty()
                                    || f.path.file_name().is_some_and(|n| {
                                        n.to_string_lossy().to_lowercase().contains(&search)
                                    })
                            })
                            .cloned()
                            .collect();
                    let empty = match (untouched, search.is_empty()) {
                        (_, false) => format!("No results for “{}”", query.read().trim()),
                        (true, true) => format!("No files untouched in {}", r.period.title()),
                        (false, true) => "No large files".into(),
                    };
                    crate::tables::Files {
                        files: Rc::new(files),
                        basket: basket.read().clone(),
                        used_title: if records {
                            "Last opened"
                        } else {
                            "Last changed"
                        },
                        empty,
                        on_act: on_pick.clone(),
                    }
                    .into_element()
                }
                None => rect().into_element(),
            },
            View::Duplicates => crate::tables::duplicates(
                &theme,
                groups
                    .read()
                    .as_ref()
                    .filter(|(g, _)| snap.as_ref().is_some_and(|s| s.generation == *g))
                    .map(|(_, found)| found.as_slice()),
                *kind_filter.read(),
                &search,
                &basket.read(),
                on_pick.clone(),
            ),
            View::Space => match snap.clone() {
                Some(s) => {
                    let space_snap = s.clone();
                    let on_pick = on_pick.clone();
                    crate::space::Space {
                        snapshot: s.clone(),
                        folder: folder.read().clone(),
                        scope_title: scope.read().title(),
                        basket: basket.read().clone(),
                        selected: space_selected.read().clone(),
                        list: r.list,
                        zoom: *zoom.read(),
                        on_act: EventHandler::new(move |act: crate::space::Act| match act {
                            crate::space::Act::Open(to, from) => {
                                zoom.set(from);
                                space_selected.set(None);
                                go(View::Space, to);
                            }
                            crate::space::Act::Select(p) => space_selected.set(p),
                            other => {
                                let _ = &space_snap;
                                on_pick.call(other);
                            }
                        }),
                    }
                    .into_element()
                }
                None => rect().into_element(),
            },
            other => rect()
                .width(Size::fill())
                .height(Size::fill())
                .center()
                .child(text::line(
                    &theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    other.title(),
                ))
                .into_element(),
        };

        let basket_view = (!basket.read().is_empty() && !*basket_hidden.read())
            .then(|| basket_panel(&theme, &basket.read(), basket));

        let menu_app = app.clone();
        let overlay_menu = item_menu.read().clone().map(|(e, at)| {
            let covered = basket.read().covers(&e.path);
            let entries = Rc::new(vec![
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(
                    1,
                    if covered {
                        "Remove from basket"
                    } else {
                        "Add to basket"
                    },
                )),
                isle_ui::MenuEntry::Separator,
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(2, "Show in Files")),
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(3, "Quick Look")),
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(4, "Copy path")),
            ]);
            let app = menu_app.clone();
            isle_ui::ContextMenu::new(entries, at, move |event: isle_ui::MenuEvent| match event {
                isle_ui::MenuEvent::Chosen(id) => {
                    item_menu.set(None);
                    match id {
                        1 if covered => basket.write().remove(&e.path),
                        1 => basket.write().add(Item {
                            path: e.path.clone(),
                            name: e.name.clone(),
                            bytes: e.bytes,
                            kind: e.kind,
                            dev: e.dev,
                            ino: e.ino,
                        }),
                        2 => crate::ties::show_in_files(vec![e.path.clone()]),
                        3 => crate::ties::quick_look(&e.path),
                        4 => crate::ties::copy_path(&app, &e.path),
                        _ => {}
                    }
                }
                isle_ui::MenuEvent::Back => item_menu.set(None),
                _ => {}
            })
            .into_element()
        });
        let more_groups = groups.read().clone();
        let more_snap = snap.clone();
        let overlay_more = (*more_menu.read()).map(|at| {
            let is_dups = r.view == View::Duplicates;
            let entries = Rc::new(vec![
                isle_ui::MenuEntry::Item(
                    isle_ui::MenuItem::new(1, "Select all but the newest").enabled(is_dups),
                ),
                isle_ui::MenuEntry::Separator,
                isle_ui::MenuEntry::Item(isle_ui::MenuItem::new(2, "Scan Folder…")),
            ]);
            let (found, s) = (more_groups.clone(), more_snap.clone());
            isle_ui::ContextMenu::new(entries, at, move |event: isle_ui::MenuEvent| match event {
                isle_ui::MenuEvent::Chosen(id) => {
                    more_menu.set(None);
                    if id == 1
                        && let (Some((g, found)), Some(s)) = (&found, &s)
                        && *g == s.generation
                    {
                        // Copies are newest first; every copy after the first goes in.
                        let mut b = basket.write();
                        for group in found.iter() {
                            for c in group.copies.iter().skip(1) {
                                let (dev, ino) =
                                    std::fs::symlink_metadata(&c.path).map_or((0, 0), |m| {
                                        use std::os::unix::fs::MetadataExt;
                                        (m.dev(), m.ino())
                                    });
                                let name = c
                                    .path
                                    .file_name()
                                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                                b.add(Item {
                                    path: c.path.clone(),
                                    name,
                                    bytes: c.bytes,
                                    kind: isle_scan::kinds::of_path(&c.path),
                                    dev,
                                    ino,
                                });
                            }
                        }
                    }
                }
                isle_ui::MenuEvent::Back => more_menu.set(None),
                _ => {}
            })
            .into_element()
        });
        let overlay = (*clearing.read()).map(|item| {
            let ask = ask.clone();
            isle_ui::alert(
                &format!(
                    "{} {}?",
                    if item.action() == "Remove…" {
                        "Remove"
                    } else {
                        "Clear"
                    },
                    item.name().to_lowercase()
                ),
                "This can't be put back.",
                Vec::new(),
                vec![
                    isle_ui::AlertButton::from((
                        "Cancel".to_owned(),
                        EventHandler::new(move |_| clearing.set(None)),
                    )),
                    isle_ui::AlertButton::destructive(
                        item.action().trim_end_matches('…'),
                        EventHandler::new(move |_| {
                            clearing.set(None);
                            let ask: mpsc::Sender<Ask> = (*ask).clone();
                            let mut command = item.command();
                            // pkexec asks through the polkit agent; refused, nothing changes, and the
                            // sizes are read again either way.
                            isle_app::worker().spawn(async move {
                                let _ = tokio::task::spawn_blocking(move || command.status()).await;
                                let _ = ask.send(Ask::System);
                            });
                        }),
                    ),
                ],
            )
        });

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .horizontal()
                    .content(Content::flex())
                    .child(sidebar)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .background(colour(t.colour_window))
                            .child(content),
                    )
                    .maybe_child(basket_view),
            )
            .maybe_child(overlay_menu)
            .maybe_child(overlay_more)
            .maybe_child(overlay)
    }
}

/// Overview's and Cleanup's column: `size-cleaner-column` wide, centred, scrolling.
fn column_view(theme: &isle_tokens::Theme, column: Element) -> Element {
    let t = &theme.tokens;
    scroll_area(theme)
        .width(Size::fill())
        .height(Size::fill())
        .child(
            rect()
                .width(Size::fill())
                .padding((t.space_xl, t.space_xxl))
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .width(Size::fill())
                        .max_width(Size::px(t.size_cleaner_column))
                        .child(column),
                ),
        )
        .into_element()
}

/// The basket as the inspector (`p1/surfaces/cleaner.md` The basket).
fn basket_panel(theme: &isle_tokens::Theme, b: &Basket, mut basket: State<Basket>) -> Element {
    let t = &theme.tokens;
    let rows: Vec<Element> = b
        .items()
        .iter()
        .map(|i| {
            let path = i.path.clone();
            rect()
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(kind_tile(theme, i.kind))
                .child(rect().width(Size::flex(1.)).child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text),
                    text::cut_middle(&i.name, 24),
                )))
                .child(mono(theme, t.colour_text_2, size(i.bytes)))
                .child(
                    rect()
                        .width(Size::px(20.))
                        .height(Size::px(20.))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt(format!("Remove {} from basket", i.name))
                        .on_press(move |_| basket.write().remove(&path))
                        .child(Glyph::new("x-circle-fill", 16., colour(t.colour_text_2))),
                )
                .into_element()
        })
        .collect();
    let by_disk = frees(b.items());
    rect()
        .width(Size::px(260.))
        .height(Size::fill())
        .content(Content::flex())
        .background(colour(t.colour_window))
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: 0.,
                    left: t.stroke_hairline,
                })
                .alignment(BorderAlignment::Inner),
        )
        .child(
            rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_xs)
                .child(text::line(
                    theme,
                    text::Style::Headline,
                    colour(t.colour_text),
                    "Basket",
                ))
                .child(text::line(
                    theme,
                    text::Style::Title2,
                    colour(t.colour_text),
                    size(b.total()),
                ))
                .child(text::line(
                    theme,
                    text::Style::Subheadline,
                    colour(t.colour_text_2),
                    format!("{} items", b.items().len()),
                )),
        )
        .child(
            scroll_area(theme)
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(
                    rect()
                        .width(Size::fill())
                        .padding((0., t.space_s))
                        .children(rows),
                ),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(t.stroke_hairline))
                .background(colour(t.colour_hairline)),
        )
        .child(
            rect()
                .width(Size::fill())
                .padding(t.space_l)
                .spacing(t.space_m)
                .children(
                    by_disk
                        .into_iter()
                        .map(|(disk, bytes)| {
                            text::line(
                                theme,
                                text::Style::Callout,
                                colour(t.colour_text_2),
                                format!(
                                    "Frees {} on {disk} once the Trash is emptied",
                                    size(bytes)
                                ),
                            )
                            .max_lines(3)
                            .width(Size::fill())
                            .into_element()
                        })
                        .collect::<Vec<_>>(),
                )
                .child(Button::new("Move to Trash").wide()),
        )
        .into_element()
}

/// What the basket frees on each disk.
fn frees(items: &[Item]) -> Vec<(String, u64)> {
    let mounts = isle_fs::mounts::read();
    let mut out: Vec<(String, u64)> = Vec::new();
    for i in items {
        let disk = isle_fs::mounts::of(&mounts, &i.path)
            .map_or_else(|| "this disk".to_owned(), |m| disk_name(&m.point));
        match out.iter_mut().find(|(d, _)| *d == disk) {
            Some((_, b)) => *b += i.bytes,
            None => out.push((disk, i.bytes)),
        }
    }
    out
}
