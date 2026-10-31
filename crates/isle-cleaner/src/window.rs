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
};

/// A job on the basket or the Trash, and where its progress comes back.
enum Job {
    Move {
        rx: futures_channel::mpsc::UnboundedReceiver<crate::mover::Moving>,
        items: Vec<Item>,
        refused: Vec<(PathBuf, String)>,
    },
    Undo {
        rx: futures_channel::mpsc::UnboundedReceiver<crate::mover::Moving>,
        items: Vec<Item>,
    },
    Empty {
        rx: futures_channel::mpsc::UnboundedReceiver<crate::mover::Moving>,
    },
}

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
    can_undo: bool,
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
    out.push(
        Command::new("undo", "Undo Move to Trash", Menu::Edit, 1)
            .chord(Chord::cmd('z'))
            .enabled(can_undo),
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
        let mut sidebar_hidden = use_state(|| false);
        let mut width = use_state(|| 1200f32);
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
        let mut moving = use_state(|| None::<f64>);
        let mut moved = use_state(|| None::<crate::basket_view::Moved>);
        let mut skip_fill = use_state(|| false);
        let mut empty_alert = use_state(|| None::<Vec<isle_fs::trash::Trashed>>);
        let hold = use_hook(|| Rc::new(RefCell::new(None::<isle_app::Hold>)));
        let mut asked = use_state(Vec::<&'static str>::new);
        let jobs = use_hook({
            let (app, hold) = (app.clone(), hold.clone());
            move || {
                let (tx, mut rx) = futures_channel::mpsc::unbounded::<Job>();
                spawn(async move {
                    use futures_util::StreamExt;
                    while let Some(job) = rx.next().await {
                        let (mut progress, kind) = match job {
                            Job::Move { rx, items, refused } => (rx, Some((items, refused))),
                            Job::Undo { rx, items } => {
                                let mut rx = rx;
                                while let Some(m) = rx.next().await {
                                    if let crate::mover::Moving::Done(_) = m {
                                        let mut b = basket.write();
                                        for i in &items {
                                            b.add(i.clone());
                                        }
                                    }
                                }
                                skip_fill.set(true);
                                *wanted.write() += 1;
                                continue;
                            }
                            Job::Empty { rx } => (rx, None),
                        };
                        while let Some(m) = progress.next().await {
                            match m {
                                crate::mover::Moving::Progress(share) if kind.is_some() => {
                                    moving.set(Some(share));
                                    app.progress(Some(share));
                                }
                                crate::mover::Moving::Progress(_) => {}
                                crate::mover::Moving::Done(report) => {
                                    let Some((items, refused)) = &kind else {
                                        moved.set(None);
                                        continue;
                                    };
                                    moving.set(None);
                                    app.progress(None);
                                    hold.borrow_mut().take();
                                    let reason =
                                        |p: &PathBuf| items.iter().find(|i| &i.path == p).cloned();
                                    let mut failed: Vec<(Item, String)> = refused
                                        .iter()
                                        .filter_map(|(p, why)| Some((reason(p)?, why.clone())))
                                        .collect();
                                    failed.extend(
                                        report
                                            .failed
                                            .iter()
                                            .filter_map(|(p, why)| Some((reason(p)?, why.clone()))),
                                    );
                                    let gone: Vec<Item> = items
                                        .iter()
                                        .filter(|i| !failed.iter().any(|(f, _)| f.path == i.path))
                                        .cloned()
                                        .collect();
                                    let undo = match report.undo {
                                        Some(isle_fs::jobs::Undo::Steps(steps)) => Some(steps),
                                        _ => None,
                                    };
                                    basket.write().clear();
                                    moved.set(Some(crate::basket_view::Moved {
                                        count: gone.len(),
                                        failed,
                                        undo,
                                        items: gone,
                                    }));
                                }
                            }
                        }
                        skip_fill.set(true);
                        *wanted.write() += 1;
                    }
                });
                tx
            }
        });

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
                                // A rescan after a move leaves the basket as the move left it.
                                if *skip_fill.peek() {
                                    skip_fill.set(false);
                                } else {
                                    let mut b = basket.write();
                                    for f in snap.finds.iter().filter(|f| f.safe) {
                                        b.add(item_of(&snap, f));
                                    }
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
        let can_undo = moved.read().as_ref().is_some_and(|m| m.undo.is_some());
        isle_app::use_commands(commands(
            &r,
            &basket.read(),
            scanning,
            can_back,
            can_forward,
            can_undo,
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
                        "move-to-trash" => asked.write().push("move"),
                        "scan-folder" => asked.write().push("choose"),
                        "empty-trash" => asked.write().push("empty"),
                        "undo" => asked.write().push("undo"),
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
                move |i: usize| match choices.get(i).map(|(_, s)| s.clone()) {
                    Some(Some(next)) if next != *scope.peek() => {
                        scope.set(next);
                        snapshot.set(None);
                        groups.set(None);
                        *wanted.write() += 1;
                    }
                    Some(None) => asked.write().push("choose"),
                    _ => {}
                },
            )
        };
        let ask_stop = ask.clone();
        // Narrow, the basket folds into its toggle first, then the sidebar goes (`surfaces/cleaner.md`
        // States); either toggle brings it back.
        let narrow = *width.read();
        let basket_shown = (!basket.read().is_empty() || moved.read().is_some())
            && ((narrow >= 1000.) != *basket_hidden.read());
        let sidebar_shown = (narrow >= 820.) != *sidebar_hidden.read();
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
                Button::icon("sidebar-simple", "Sidebar").on_press(move |_| {
                    let hidden = *sidebar_hidden.peek();
                    sidebar_hidden.set(!hidden);
                }),
            )
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
                    .width(Size::flex(1.))
                    .min_width(Size::px(80.))
                    .max_width(Size::px(320.))
                    .child(title_block),
            )
            .child(
                rect()
                    .min_width(Size::px(if narrow < 760. { 100. } else { 120. }))
                    .child(scope_popup),
            )
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
            // Narrow, the primary action keeps its symbol alone.
            .child(
                match (scanning, narrow < 760.) {
                    (true, false) => Button::new("Stop"),
                    (true, true) => Button::icon("stop-circle", "Stop"),
                    (false, false) => Button::new("Rescan"),
                    (false, true) => Button::icon("arrow-clockwise", "Rescan"),
                }
                .on_press(move |_| {
                    if scanning {
                        let _ = ask_stop.send(Ask::Stop);
                    } else {
                        *wanted.write() += 1;
                    }
                }),
            )
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
                rect()
                    .width(Size::px(t.size_control))
                    .height(Size::px(t.size_control))
                    .child(
                        Button::icon(
                            if basket_shown {
                                "sidebar-simple-fill"
                            } else {
                                "sidebar-simple"
                            },
                            "Basket",
                        )
                        .enabled(!basket.read().is_empty())
                        .on_press(move |_| {
                            let hidden = *basket_hidden.peek();
                            basket_hidden.set(!hidden);
                        }),
                    )
                    // Folded, the toggle carries the basket's count.
                    .maybe_child((!basket_shown && !basket.read().is_empty()).then(|| {
                        rect()
                            .position(Position::new_absolute().right(-4.).top(-4.))
                            .min_width(Size::px(16.))
                            .height(Size::px(16.))
                            .padding((0., 4.))
                            .corner_radius(8.)
                            .background(colour(t.colour_fill_strong))
                            .center()
                            .child(text::line(
                                &theme,
                                text::Style::Footnote,
                                colour(t.colour_text),
                                basket.read().items().len().to_string(),
                            ))
                            .into_element()
                    })),
            )
            .child(SearchField {
                value: query.into_writable(),
                placeholder: "Search".into(),
                width: if narrow < 760. {
                    64.
                } else if narrow < 820. {
                    96.
                } else {
                    140.
                },
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
                            .child(
                                Button::new("Empty Trash…")
                                    .small()
                                    .on_press(move |_| asked.write().push("empty")),
                            ),
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
                        query: query.read().trim().to_owned(),
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
        };

        // Moving: one job for the whole basket, held open until it ends.
        let start_move = {
            let (app, hold, jobs) = (app.clone(), hold.clone(), jobs.clone());
            move || {
                if moving.peek().is_some() || basket.peek().is_empty() {
                    return;
                }
                let items: Vec<Item> = basket.peek().items().to_vec();
                let (ok, refused) = crate::mover::check(&items);
                moving.set(Some(0.));
                *hold.borrow_mut() = Some(app.hold());
                app.progress(Some(0.));
                let rx = crate::mover::run(isle_fs::jobs::Work::Trash { items: ok });
                let _ = jobs.unbounded_send(Job::Move { rx, items, refused });
            }
        };
        let undo_move = {
            let jobs = jobs.clone();
            move || {
                let Some(m) = moved.peek().clone() else {
                    return;
                };
                let Some(steps) = m.undo.clone() else { return };
                moved.set(None);
                let rx = crate::mover::run(isle_fs::jobs::Work::Undo { steps });
                let _ = jobs.unbounded_send(Job::Undo { rx, items: m.items });
            }
        };
        let ask_empty = {
            let roots = snap.as_ref().map(|s| s.roots.clone()).unwrap_or_default();
            move || {
                let roots = roots.clone();
                let items: Vec<isle_fs::trash::Trashed> = isle_fs::trash::all_trashes()
                    .iter()
                    .filter(|d| roots.iter().any(|r| d.root.starts_with(r)))
                    .flat_map(isle_fs::trash::list)
                    .collect();
                if !items.is_empty() {
                    empty_alert.set(Some(items));
                }
            }
        };
        // A folder chosen or dropped becomes the scope, and a recent folder.
        let mut scan_folder = move |path: PathBuf| {
            remembered.write().scanned_folder(path.clone());
            scope.set(Scope::Folder(path));
            snapshot.set(None);
            groups.set(None);
            *wanted.write() += 1;
        };
        let drag = isle_app::use_drag();
        use_side_effect(move || {
            let dropped = drag.dropped.read().clone();
            if dropped.is_empty() {
                return;
            }
            let mut queue = drag.dropped;
            queue.write().clear();
            let folder = dropped
                .iter()
                .filter(|d| !d.own)
                .flat_map(|d| {
                    String::from_utf8_lossy(&d.data)
                        .lines()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter_map(|uri| isle_fs::recents::path_of(uri.trim()))
                .find(|p| p.is_dir());
            if let Some(folder) = folder {
                scan_folder(folder);
            }
        });
        let chooser_app = app.clone();
        use_side_effect({
            let (mut go_move, mut go_undo, mut go_empty) =
                (start_move.clone(), undo_move.clone(), ask_empty.clone());
            move || {
                if asked.read().is_empty() {
                    return;
                }
                for a in std::mem::take(&mut *asked.write()) {
                    match a {
                        "move" => go_move(),
                        "undo" => go_undo(),
                        "choose" => {
                            let Some(bus) = chooser_app.bus() else {
                                continue;
                            };
                            let parent = chooser_app
                                .export(isle_app::this_window())
                                .unwrap_or_default();
                            spawn(async move {
                                let wanted_folder = isle_app::portal::Wanted {
                                    title: "Choose a folder to scan".into(),
                                    directory: true,
                                    ..Default::default()
                                };
                                match isle_app::portal::choose_file(&bus, &parent, wanted_folder)
                                    .await
                                {
                                    Ok(Some(path)) => scan_folder(path),
                                    Ok(None) => {}
                                    Err(err) => tracing::warn!("no file chooser: {err}"),
                                }
                            });
                        }
                        _ => go_empty(),
                    }
                }
            }
        });
        let frees_of = use_memo(move || crate::basket_view::frees(basket.read().items()));
        let basket_view = basket_shown.then(|| {
            let (mut go_move, mut go_undo, mut go_empty) =
                (start_move.clone(), undo_move.clone(), ask_empty.clone());
            crate::basket_view::view(
                &theme,
                crate::basket_view::Panel {
                    basket: &basket.read(),
                    frees: &frees_of.read(),
                    moving: *moving.read(),
                    moved: moved.read().as_ref(),
                    on_remove: EventHandler::new(move |p: PathBuf| basket.write().remove(&p)),
                    on_move: EventHandler::new(move |_| go_move()),
                    on_undo: EventHandler::new(move |_| go_undo()),
                    on_empty: EventHandler::new(move |_| go_empty()),
                },
            )
        });

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
                    if id == 2 {
                        asked.write().push("choose");
                    }
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
        let overlay_empty = empty_alert.read().clone().map(|items| {
            let empty_jobs = jobs.clone();
            let count = items.len();
            isle_ui::alert(
                &format!("Permanently erase {count} items in the Trash?"),
                "You can't undo this.",
                Vec::new(),
                vec![
                    isle_ui::AlertButton::destructive(
                        "Empty Trash",
                        EventHandler::new(move |_| {
                            empty_alert.set(None);
                            let rx = crate::mover::run(isle_fs::jobs::Work::EmptyTrash {
                                items: items.clone(),
                            });
                            let _ = empty_jobs.unbounded_send(Job::Empty { rx });
                        }),
                    ),
                    isle_ui::AlertButton::from((
                        "Cancel".to_owned(),
                        EventHandler::new(move |_| empty_alert.set(None)),
                    )),
                ],
            )
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
            .on_sized(move |e: Event<SizedEventData>| width.set_if_modified(e.area.width()))
            .child(toolbar)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .horizontal()
                    .content(Content::flex())
                    .maybe_child(sidebar_shown.then_some(sidebar))
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .background(colour(t.colour_window))
                            .child(content)
                            .maybe_child(drag.over.read().is_some().then(|| {
                                let inset = t.space_xs;
                                rect()
                                    .position(
                                        Position::new_absolute().left(t.space_xs).top(t.space_xs),
                                    )
                                    .width(Size::func(move |c| Some(c.parent - 2. * inset)))
                                    .height(Size::func(move |c| Some(c.parent - 2. * inset)))
                                    .corner_radius(t.radius_card)
                                    .border(
                                        Border::new()
                                            .fill(colour(t.colour_accent))
                                            .width(t.stroke_found)
                                            .alignment(BorderAlignment::Inner),
                                    )
                                    .into_element()
                            })),
                    )
                    .maybe_child(basket_view),
            )
            .maybe_child(overlay_menu)
            .maybe_child(overlay_more)
            .maybe_child(overlay_empty)
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
