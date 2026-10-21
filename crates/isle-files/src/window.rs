//! A Files window: the sidebar of places, the toolbar, the tab bar and the folder shown
//! (`p1/structure/files.md` § Content; `p1/surfaces/files.md` § Anatomy).

use std::{path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_app::{Chord, ChordKey, Command, Desktop, Menu, ToolbarExt, use_commands};
use isle_fs::{
    SortBy,
    jobs::{Step, Undo, Work},
};
use isle_ui::{Glyph, colour, text, use_theme};

use crate::{
    Shared, WindowLink,
    bars::{PathBar, StatusBar},
    columns_view::ColumnsView,
    icons_view::IconsView,
    interact::{Interact, Rename, TRASH},
    list_view::{Held, ListView, Pick},
    places::{self, Place, Places, Row},
    preview::Preview,
    tab::{self, Live, Status, Tab},
    jobs::Finished,
    sheets::{Alert, ConflictSheet, Then, alert_sheet},
    toolbar::{Act, Toolbar},
    view::{GroupBy, Shown, View},
};

/// What another part of the app asks a window to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    /// A tab at the place, the item revealed in it.
    Tab(Place, Option<PathBuf>),
    /// Open With for a file nothing opens by itself.
    OpenWith(PathBuf),
}

/// One tab as the window keeps it.
#[derive(Clone)]
pub struct TabHandle {
    pub tab: State<Tab>,
    pub live: Live,
    pub scroll: ScrollController,
}

impl PartialEq for TabHandle {
    fn eq(&self, other: &Self) -> bool {
        self.tab == other.tab
    }
}

#[derive(Clone)]
pub struct FilesWindow {
    pub shared: Rc<Shared>,
    pub tabs: Vec<(Place, Option<PathBuf>)>,
    pub current: usize,
}

/// Runs `f` on a thread of its own and hands its answer back.
pub async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.ok()
}

fn new_tab(shared: &Shared, place: Place, reveal: Option<PathBuf>) -> TabHandle {
    let mut t = Tab::new(shared.next_tab(), place);
    t.reveal = reveal;
    t.folder.set_show_hidden(*shared.hidden.peek());
    let memory = crate::SHARED.with(|s| s.borrow().clone());
    TabHandle {
        tab: State::create(t),
        live: match memory {
            Some(shared) => Live::new(Rc::new(move |place: &Place| shared.shown_for(place))),
            None => Live::default(),
        },
        scroll: ScrollController::new(0, 0),
    }
}

impl App for FilesWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let key = isle_app::this_window();
        let desktop = Desktop::get();
        let mut tabs = use_state({
            let (shared, start) = (shared.clone(), self.tabs.clone());
            move || {
                start
                    .into_iter()
                    .map(|(place, reveal)| new_tab(&shared, place, reveal))
                    .collect::<Vec<_>>()
            }
        });
        let mut current = use_state(|| self.current.min(self.tabs.len().saturating_sub(1)));
        let mut places = use_state(Places::default);
        let mut held = use_state(Held::default);
        let inbox = use_state(Vec::<Open>::new);
        let mut sidebar = use_state(|| true);
        let mut preview = use_state(|| false);
        let mut path_bar = use_state(|| false);
        let mut status_bar = use_state(|| false);
        let mut options = use_state(|| false);
        let mut undo_stack = use_state(Vec::<(String, Vec<Step>)>::new);
        let mut alert = use_state(|| None::<Alert>);
        let jobs = shared.jobs;
        let mut renaming = use_state(|| None::<PathBuf>);
        let rename_text = use_state(String::new);
        let mut rename_error = use_state(|| None::<String>);
        let mut rename_many = use_state(|| None::<Vec<PathBuf>>);
        let mut drop_target = use_state(|| None::<PathBuf>);
        let mut press = use_state(|| None::<(PathBuf, f64, f64)>);
        let mut drag_started = use_state(|| false);
        // The Return that commits a name is the field's alone.
        let mut committed = use_state(|| false);
        let open_with = use_state(|| None::<crate::open::Offer>);
        let unlocking = use_state(|| None::<(isle_disks::Volume, Option<String>)>);
        let drag = isle_app::use_drag();
        let surface = isle_app::this_surface();
        let data = shared.app().data();

        use_hook({
            let shared = shared.clone();
            move || {
                for handle in tabs.peek().iter() {
                    tab::load(handle.tab, &handle.live, false);
                }
                shared.register(key, WindowLink { inbox });
                spawn(async move {
                    if let Some(read) = off_thread(places::read).await {
                        places.set(read);
                    }
                });
            }
        });
        use_drop({
            let shared = shared.clone();
            move || shared.unregister(key)
        });

        let at = (*current.read()).min(tabs.read().len().saturating_sub(1));
        let Some(handle) = tabs.read().get(at).cloned() else {
            return rect().into_element();
        };
        let mut tab = handle.tab;

        // Hidden files follow the app's toggle in every tab.
        use_side_effect({
            let hidden = shared.hidden;
            move || {
                let show = *hidden.read();
                for handle in tabs.peek().iter() {
                    let mut tab = handle.tab;
                    if tab.peek().folder.shows_hidden() != show {
                        let mut t = tab.write();
                        t.folder.set_show_hidden(show);
                        t.rev += 1;
                    }
                }
            }
        });

        // A tab on a volume that has gone moves to the new-window folder rather than closing.
        let known_points = use_state(Vec::<PathBuf>::new);
        use_side_effect({
            let shared = shared.clone();
            move || {
                let points: Vec<PathBuf> = shared.locations.volumes.read().iter().filter_map(|v| v.mount_point.as_ref().map(PathBuf::from)).collect();
                let shares = shared.locations.shares.read().clone();
                let mut known = known_points;
                let gone: Vec<PathBuf> = known.peek().iter().filter(|p| !points.contains(p) && !shares.contains(p)).cloned().collect();
                known.set(points.iter().chain(shares.iter()).cloned().collect());
                if gone.is_empty() {
                    return;
                }
                for handle in tabs.peek().iter() {
                    let on_gone = handle.tab.peek().place.folder().is_some_and(|f| gone.iter().any(|g| f.starts_with(g)));
                    if on_gone {
                        tab::go(handle.tab, &handle.live, shared.new_window_place(), None);
                    }
                }
            }
        });

        // A job's end: its undo joins this window's stack, its failures one alert.
        use_side_effect(move || {
            let mine: Vec<Finished> = jobs.finished.read().iter().filter(|f| f.window == key).cloned().collect();
            if mine.is_empty() {
                return;
            }
            let mut finished = jobs.finished;
            finished.write().retain(|f| f.window != key);
            for f in mine {
                match &f.report.undo {
                    Some(Undo::Steps(steps)) if !steps.is_empty() => {
                        undo_stack.write().push((f.undo_label.clone(), steps.clone()))
                    }
                    // A step that replaced something ends the stack there.
                    Some(Undo::CantUndo) => undo_stack.write().clear(),
                    _ => {}
                }
                if !f.report.failed.is_empty() {
                    alert.set(Some(Alert::failures(&f.report.failed)));
                }
            }
        });
        // The Trash and Recents are gatherings no watch follows; each job's end reads them again.
        use_side_effect(move || {
            let _ = *jobs.ended.read();
            for handle in tabs.peek().iter() {
                if matches!(handle.tab.peek().place, Place::Trash | Place::Recents) {
                    tab::load(handle.tab, &handle.live, true);
                }
            }
        });

        // A drag from this window ended, dropped or not.
        use_side_effect(move || {
            let _ = *drag.ended.read();
            drag_started.set(false);
            press.set(None);
        });
        // Drops on the window: into the folder under the pointer, or the one shown.
        use_side_effect({
            let shared = shared.clone();
            move || {
                if drag.dropped.read().is_empty() {
                    return;
                }
                let mut dropped = drag.dropped;
                let drops = std::mem::take(&mut *dropped.write());
                let target = drop_target.peek().clone();
                drop_target.set(None);
                let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
                let here = handle.tab.peek().place.folder().map(PathBuf::from);
                for drop in drops {
                    let (paths, _) = crate::transfer::read(crate::transfer::URIS, &drop.data);
                    let Some(into) = target.clone().or_else(|| here.clone()) else { continue };
                    let work = if into.as_os_str() == TRASH {
                        Work::Trash { items: paths }
                    } else {
                        let items: Vec<PathBuf> = paths
                            .into_iter()
                            .filter(|p| p.parent() != Some(into.as_path()) && !into.starts_with(p))
                            .collect();
                        if items.is_empty() {
                            continue;
                        }
                        // Within one volume an item moves, as Finder's do; across volumes it copies.
                        let same = drop.own && same_volume(&items[0], &into);
                        if same { Work::Move { items, into } } else { Work::Copy { items, into } }
                    };
                    start_job(&shared, key, work, &mut alert);
                }
            }
        });

        // Requests from the app: a tab for a place, the item revealed.
        use_side_effect({
            let shared = shared.clone();
            move || {
                if inbox.read().is_empty() {
                    return;
                }
                let mut queue = inbox;
                for open in std::mem::take(&mut *queue.write()) {
                    match open {
                        Open::OpenWith(path) => ask_open_with(path, open_with),
                        Open::Tab(place, reveal) => {
                            let handle = new_tab(&shared, place, reveal);
                            tab::load(handle.tab, &handle.live, false);
                            let n = {
                                let mut list = tabs.write();
                                list.push(handle);
                                list.len() - 1
                            };
                            current.set(n);
                        }
                    }
                }
                shared.remember(key, &tabs.peek(), *current.peek());
            }
        });

        let (back, forward, place, shown) = {
            let t = tab.read();
            (!t.back.is_empty(), !t.forward.is_empty(), t.place.clone(), t.shown)
        };
        let has_selection = !tab.read().selection.is_empty();
        let enclosing = place.folder().and_then(|p| p.parent()).map(PathBuf::from);
        let fav = |label: &str| {
            places
                .read()
                .favourites
                .iter()
                .find(|r| r.label == label)
                .map(|r| r.place.clone())
        };
        let in_trash = place == Place::Trash;
        let editing = renaming.read().is_some();
        let selected_image = {
            let t = tab.read();
            t.selection.len() == 1
                && t.folder.get(&t.selection[0]).and_then(|e| e.mime.as_deref()).is_some_and(|m| m.starts_with("image/"))
        };
        let (selected_count, selection_name) = {
            let t = tab.read();
            let name = (t.selection.len() == 1)
                .then(|| t.selection[0].file_name().map(|n| n.to_string_lossy().into_owned()))
                .flatten();
            (t.selection.len(), name)
        };
        let undo_label = undo_stack.read().last().map(|(l, _)| format!("Undo {l}"));
        let commands = vec![
            Command::new("undo", undo_label.clone().unwrap_or_else(|| "Undo".into()), Menu::Edit, 0)
                .chord(Chord::cmd('z'))
                .enabled(undo_label.is_some()),
            Command::new("new-folder", "New Folder", Menu::File, 0)
                .chord(Chord::cmd('n').shift())
                .enabled(place.folder().is_some()),
            Command::new("new-folder-with-items", "New Folder with Items", Menu::File, 0)
                .chord(Chord::cmd('n').ctrl())
                .enabled(has_selection && place.folder().is_some()),
            Command::new("duplicate", "Duplicate", Menu::File, 3)
                .chord(Chord::cmd('d'))
                .enabled(has_selection && !in_trash),
            Command::new("make-alias", "Make Alias", Menu::File, 3)
                .chord(Chord::cmd('a').ctrl())
                .enabled(has_selection && !in_trash),
            Command::new("compress", "Compress", Menu::File, 3).enabled(has_selection && !in_trash),
            Command::new("expand", "Expand", Menu::File, 3).enabled(has_selection && !in_trash),
            Command::new(
                if in_trash { "put-back" } else { "move-to-trash" },
                if in_trash { "Put Back" } else { "Move to Trash" },
                Menu::File,
                4,
            )
            .chord(Chord::cmd_key(ChordKey::Backspace))
            .enabled(has_selection),
            Command::new("delete-now", "Delete Immediately…", Menu::File, 4)
                .chord(Chord::cmd_key(ChordKey::Backspace).option())
                .enabled(has_selection),
            Command::new("empty-trash", "Empty Trash…", Menu::File, 4).chord(Chord::cmd_key(ChordKey::Backspace).shift()),
            Command::new("empty-trash-now", "Empty Trash", Menu::File, 4)
                .chord(Chord::cmd_key(ChordKey::Backspace).shift().option()),
            field_when(Command::new("copy", "Copy", Menu::Edit, 1).chord(Chord::cmd('c')).enabled(has_selection), editing),
            field_when(Command::new("cut", "Cut", Menu::Edit, 1).chord(Chord::cmd('x')).enabled(has_selection), editing),
            field_when(Command::new("paste", "Paste", Menu::Edit, 1).chord(Chord::cmd('v')), editing),
            field_when(Command::new("select-all", "Select All", Menu::Edit, 1).chord(Chord::cmd('a')), editing),
            Command::new("paste-move", "Move Item Here", Menu::Edit, 1).chord(Chord::cmd('v').option()),
            Command::new(
                "copy-path",
                match selection_name.as_deref() {
                    Some(name) => format!("Copy “{name}” as Pathname"),
                    None => "Copy as Pathname".into(),
                },
                Menu::Edit,
                1,
            )
            .chord(Chord::cmd('c').option())
            .enabled(has_selection),
            Command::new("open-with", "Open With…", Menu::File, 1).enabled(has_selection && !in_trash),
            Command::new("open-terminal", "Open in terminal", Menu::File, 1).enabled(place.folder().is_some()),
            Command::new("set-wallpaper", "Set as wallpaper", Menu::File, 3).enabled(selected_image),
            Command::new("rename", if selected_count > 1 { format!("Rename {selected_count} Items…") } else { "Rename".into() }, Menu::File, 3)
                .enabled(has_selection && !in_trash),
            Command::new("new-window", "New Window", Menu::File, 0).chord(Chord::cmd('n')),
            Command::new("new-tab", "New Tab", Menu::File, 0).chord(Chord::cmd('t')),
            Command::new("open", "Open", Menu::File, 1)
                .chord(Chord::cmd('o'))
                .enabled(has_selection),
            Command::new("close-tab", "Close Tab", Menu::File, 2).enabled(tabs.read().len() > 1),
            Command::new("deselect", "Deselect All", Menu::Edit, 2)
                .chord(Chord::cmd('a').option())
                .enabled(has_selection),
            Command::new("as-icons", "as Icons", Menu::View, 0)
                .chord(Chord::cmd('1'))
                .checked(shown.view == View::Icons),
            Command::new("as-list", "as List", Menu::View, 0)
                .chord(Chord::cmd('2'))
                .checked(shown.view == View::List),
            Command::new("as-columns", "as Columns", Menu::View, 0)
                .chord(Chord::cmd('3'))
                .checked(shown.view == View::Columns),
            Command::new("use-groups", "Use Groups", Menu::View, 1)
                .chord(Chord::cmd('0').ctrl())
                .checked(shown.group.is_some()),
            Command::new("group-kind", "Group By Kind", Menu::View, 1).checked(shown.group == Some(GroupBy::Kind)),
            Command::new("group-modified", "Group By Date Modified", Menu::View, 1)
                .checked(shown.group == Some(GroupBy::Modified)),
            Command::new("group-size", "Group By Size", Menu::View, 1).checked(shown.group == Some(GroupBy::Size)),
            Command::new("sort-name", "Sort By Name", Menu::View, 2).checked(shown.sort == SortBy::Name),
            Command::new("sort-kind", "Sort By Kind", Menu::View, 2).checked(shown.sort == SortBy::Kind),
            Command::new("sort-modified", "Sort By Date Modified", Menu::View, 2)
                .checked(shown.sort == SortBy::Modified),
            Command::new("sort-size", "Sort By Size", Menu::View, 2).checked(shown.sort == SortBy::Size),
            Command::new("view-options", "Show View Options", Menu::View, 5).chord(Chord::cmd('j')),
            Command::new("bigger", "Bigger Icons", Menu::View, 5)
                .chord(Chord::cmd('='))
                .enabled(shown.view == View::Icons),
            Command::new("smaller", "Smaller Icons", Menu::View, 5)
                .chord(Chord::cmd('-'))
                .enabled(shown.view == View::Icons),
            Command::new("toggle-preview", if *preview.read() { "Hide Preview" } else { "Show Preview" }, Menu::View, 3)
                .chord(Chord::cmd('p').shift()),
            Command::new("toggle-path-bar", if *path_bar.read() { "Hide Path Bar" } else { "Show Path Bar" }, Menu::View, 3)
                .chord(Chord::cmd('p').option()),
            Command::new(
                "toggle-status-bar",
                if *status_bar.read() { "Hide Status Bar" } else { "Show Status Bar" },
                Menu::View,
                3,
            )
            .chord(Chord::cmd('/')),
            Command::new(
                "toggle-hidden",
                if *shared.hidden.read() { "Hide Hidden Files" } else { "Show Hidden Files" },
                Menu::View,
                4,
            )
            .chord(Chord::cmd('.').shift()),
            Command::new("toggle-sidebar", if *sidebar.read() { "Hide Sidebar" } else { "Show Sidebar" }, Menu::View, 3)
                .chord(Chord::cmd('s').ctrl()),
            Command::new("back", "Back", Menu::Go, 0).chord(Chord::cmd('[')).enabled(back),
            Command::new("forward", "Forward", Menu::Go, 0).chord(Chord::cmd(']')).enabled(forward),
            Command::new("enclosing", "Enclosing Folder", Menu::Go, 0)
                .chord(Chord::cmd_key(ChordKey::Up))
                .enabled(enclosing.is_some()),
            Command::new("open-down", "Open Selection", Menu::Go, 0)
                .chord(Chord::cmd_key(ChordKey::Down))
                .enabled(has_selection),
            Command::new("go-recents", "Recents", Menu::Go, 1).chord(Chord::cmd('f').shift()),
            Command::new("go-documents", "Documents", Menu::Go, 1)
                .chord(Chord::cmd('o').shift())
                .enabled(fav("Documents").is_some()),
            Command::new("go-desktop", "Desktop", Menu::Go, 1)
                .chord(Chord::cmd('d').shift())
                .enabled(fav("Desktop").is_some()),
            Command::new("go-downloads", "Downloads", Menu::Go, 1)
                .chord(Chord::cmd('l').option())
                .enabled(fav("Downloads").is_some()),
            Command::new("go-home", "Home", Menu::Go, 1).chord(Chord::cmd('h').shift()),
            Command::new("go-computer", "Computer", Menu::Go, 1).chord(Chord::cmd('c').shift()),
            Command::new("go-applications", "Applications", Menu::Go, 1)
                .chord(Chord::cmd('a').shift())
                .enabled(fav("Applications").is_some()),
        ];
        use_commands(commands);

        let live = handle.live.clone();
        let set_shown_here = {
            let shared = shared.clone();
            move |next: Shown| set_shown(&shared, tab, next)
        };
        let go = {
            let live = live.clone();
            move |place: Place| tab::go(tab, &live, place, None)
        };
        let open_new_tab_here = {
            let shared = shared.clone();
            move |place: Place| open_new_tab(&shared, tabs, current, key, place)
        };
        // The commands act on the tab current when each is taken, not when the effect began.
        use_side_effect({
            let shared = shared.clone();
            move || {
                let chosen: Vec<String> = desktop.commands.read().clone();
                if chosen.is_empty() {
                    return;
                }
                let mut queue = desktop.commands;
                queue.write().clear();
                for id in chosen {
                    let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
                    let (mut tab, live) = (handle.tab, handle.live.clone());
                    let go = |place: Place| tab::go(tab, &live, place, None);
                    let fav = |label: &str| {
                        places
                            .peek()
                            .favourites
                            .iter()
                            .find(|r| r.label == label)
                            .map(|r| r.place.clone())
                    };
                    match id.as_str() {
                        "new-window" => shared.open_window(vec![(shared.new_window_place(), None)], 0, None),
                        "new-tab" => open_new_tab(&shared, tabs, current, key, shared.new_window_place()),
                        "close-tab" => close_tab(tabs, current, *current.peek()),
                        "open" | "open-down" => {
                            let selection = tab.peek().selection.clone();
                            open_items(&shared, tabs, current, key, &handle, selection, false, open_with);
                        }
                        "deselect" => tab.write().deselect(),
                        "select-all" => {
                            let mut t = tab.write();
                            t.selection = t.folder.entries().iter().map(|e| e.path.clone()).collect();
                        }
                        "copy" | "cut" => {
                            let selection = tab.peek().selection.clone();
                            if let Some(data) = shared.app().data() {
                                data.set_selection(crate::transfer::contents(&selection, id == "cut"));
                            }
                            let mut cut = shared.cut;
                            cut.set(if id == "cut" { selection } else { Vec::new() });
                        }
                        "copy-path" => {
                            let text = tab.peek().selection.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n");
                            if let Some(data) = shared.app().data() {
                                let bytes = text.into_bytes();
                                data.set_selection(vec![
                                    ("text/plain;charset=utf-8".into(), bytes.clone()),
                                    ("text/plain".into(), bytes),
                                ]);
                            }
                        }
                        "paste" | "paste-move" => {
                            let (Some(data), Some(into)) = (shared.app().data(), tab.peek().place.folder().map(PathBuf::from)) else {
                                continue;
                            };
                            let moving = id == "paste-move";
                            let shared = shared.clone();
                            spawn(async move {
                                let read = off_thread(move || data.read_selection(&crate::transfer::READ)).await.flatten();
                                let Some((kind, bytes)) = read else { return };
                                let (items, cut) = crate::transfer::read(&kind, &bytes);
                                if items.is_empty() {
                                    return;
                                }
                                let mut cut_now = shared.cut;
                                let work = if moving || cut {
                                    cut_now.set(Vec::new());
                                    Work::Move { items, into }
                                } else {
                                    Work::Copy { items, into }
                                };
                                let mut alert = alert;
                                start_job(&shared, key, work, &mut alert);
                            });
                        }
                        "open-with" => {
                            if let Some(first) = tab.peek().selection.first().cloned() {
                                ask_open_with(first, open_with);
                            }
                        }
                        "open-terminal" => {
                            let t = tab.peek();
                            // The selected folder, or the folder shown.
                            let folder = t
                                .selection
                                .first()
                                .filter(|p| t.folder.get(p).is_some_and(|e| e.is_dir()))
                                .cloned()
                                .or_else(|| t.place.folder().map(PathBuf::from));
                            drop(t);
                            let command = match desktop.value("shortcuts.terminal-command") {
                                Some(isle_settings::Value::Str(c)) if !c.is_empty() => c,
                                _ => "kitty".to_owned(),
                            };
                            if let Some(folder) = folder {
                                crate::open::terminal(&folder, &command);
                            }
                        }
                        "set-wallpaper" => {
                            if let Some(image) = tab.peek().selection.first() {
                                desktop.set("wallpaper.source", isle_settings::Value::Str(image.to_string_lossy().into_owned()));
                            }
                        }
                        "rename" => {
                            let selection = tab.peek().selection.clone();
                            start_rename(selection, renaming, rename_text, rename_error, rename_many);
                        }
                        "toggle-sidebar" => sidebar.toggle(),
                        "toggle-preview" => preview.toggle(),
                        "toggle-path-bar" => path_bar.toggle(),
                        "toggle-status-bar" => status_bar.toggle(),
                        "view-options" => options.toggle(),
                        "as-icons" | "as-list" | "as-columns" | "use-groups" | "group-kind" | "group-modified"
                        | "group-size" | "sort-name" | "sort-kind" | "sort-modified" | "sort-size" | "bigger"
                        | "smaller" => {
                            let mut next = tab.peek().shown;
                            match id.as_str() {
                                "as-icons" => next.view = View::Icons,
                                "as-list" => next.view = View::List,
                                "as-columns" => next.view = View::Columns,
                                "use-groups" => {
                                    next.group = if next.group.is_some() { None } else { Some(GroupBy::Kind) }
                                }
                                "group-kind" => next.group = Some(GroupBy::Kind),
                                "group-modified" => next.group = Some(GroupBy::Modified),
                                "group-size" => next.group = Some(GroupBy::Size),
                                "sort-name" => next.sort = SortBy::Name,
                                "sort-kind" => next.sort = SortBy::Kind,
                                "sort-modified" => next.sort = SortBy::Modified,
                                "sort-size" => next.sort = SortBy::Size,
                                "bigger" => next.step_icons(true),
                                _ => next.step_icons(false),
                            }
                            set_shown(&shared, tab, next);
                        }
                        "undo" => {
                            if let Some((label, steps)) = undo_stack.write().pop() {
                                jobs.start(&shared.app(), key, Work::Undo { steps }, format!("Undoing {label}"), String::new());
                            }
                        }
                        "new-folder" | "new-folder-with-items" | "duplicate" | "make-alias" | "compress" | "expand"
                        | "move-to-trash" | "put-back" | "delete-now" | "empty-trash" | "empty-trash-now" => {
                            let t = tab.peek();
                            let selection: Vec<PathBuf> = t.selection.clone();
                            let here = t.place.folder().map(PathBuf::from);
                            let trashed: Vec<_> = selection
                                .iter()
                                .filter_map(|p| t.folder.get(p)?.trashed.as_deref().cloned())
                                .collect();
                            drop(t);
                            let work = match id.as_str() {
                                "new-folder" => here.map(|into| Work::NewFolder { into, name: "untitled folder".into(), with: Vec::new() }),
                                "new-folder-with-items" => here.map(|into| Work::NewFolder {
                                    into,
                                    name: "New Folder With Items".into(),
                                    with: selection.clone(),
                                }),
                                "duplicate" => Some(Work::Duplicate { items: selection.clone() }),
                                "compress" => Some(Work::Compress { items: selection.clone() }),
                                "expand" => selection.first().cloned().map(|archive| Work::Expand { archive }),
                                "move-to-trash" => Some(Work::Trash { items: selection.clone() }),
                                "put-back" => Some(Work::PutBack { items: trashed.clone() }),
                                "make-alias" => {
                                    for item in &selection {
                                        make_alias(item);
                                    }
                                    None
                                }
                                "delete-now" => {
                                    alert.set(Some(Alert::delete(selection.clone())));
                                    None
                                }
                                "empty-trash" => {
                                    ask_empty_trash(alert);
                                    None
                                }
                                _ => {
                                    let shared = shared.clone();
                                    spawn(async move {
                                        if let Some(items) = off_thread(trash_items).await {
                                            let mut alert = alert;
                                            start_job(&shared, key, Work::EmptyTrash { items }, &mut alert);
                                        }
                                    });
                                    None
                                }
                            };
                            if let Some(work) = work {
                                start_job(&shared, key, work, &mut alert);
                            }
                        }
                        "toggle-hidden" => {
                            let mut hidden = shared.hidden;
                            let now = !*hidden.peek();
                            hidden.set(now);
                        }
                        "back" => tab::back(tab, &live),
                        "forward" => tab::forward(tab, &live),
                        "enclosing" => {
                            let from = tab.peek().place.folder().map(PathBuf::from);
                            if let Some(from) = from
                                && let Some(up) = from.parent()
                            {
                                tab::go(tab, &live, Place::Folder(up.to_owned()), Some(from));
                            }
                        }
                        "go-recents" => go(Place::Recents),
                        "go-home" => {
                            if let Some(home) = places::home() {
                                go(Place::Folder(home));
                            }
                        }
                        "go-computer" => go(Place::Computer),
                        other => {
                            let label = match other {
                                "go-documents" => "Documents",
                                "go-desktop" => "Desktop",
                                "go-downloads" => "Downloads",
                                "go-applications" => "Applications",
                                _ => continue,
                            };
                            if let Some(place) = fav(label) {
                                go(place);
                            }
                        }
                    }
                }
                shared.remember(key, &tabs.peek(), *current.peek());
            }
        });

        let focused = *Platform::get().is_app_focused.read();
        let rev = tab.read().rev;
        // Made once, as the rename's handlers are, and acting on the tab current when called.
        let on_pick = use_hook({
            let shared = shared.clone();
            move || {
                EventHandler::new(move |pick: Pick| {
                    let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
                    let mut tab = handle.tab;
                    match pick {
                        Pick::Nothing => tab.write().deselect(),
                        Pick::Click(path, held) => {
                            let mut t = tab.write();
                            if held.cmd {
                                t.toggle(path);
                            } else if held.shift {
                                t.extend_to(&path);
                            } else {
                                t.select_only(path);
                            }
                        }
                        Pick::Open(path, held) => {
                            tab.write().select_only(path.clone());
                            open_items(&shared, tabs, current, key, &handle, vec![path], held.cmd, open_with);
                        }
                    }
                })
            }
        });
        let on_key = move |e: Event<KeyboardEventData>| {
            let m = e.modifiers;
            held.set_if_modified(Held {
                cmd: m.contains(Modifiers::META),
                shift: m.contains(Modifiers::SHIFT),
                option: m.contains(Modifiers::ALT),
            });
            if m.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::ALT) || renaming.peek().is_some() {
                return;
            }
            if e.key == Key::Named(NamedKey::Enter) && *committed.peek() {
                committed.set(false);
                return;
            }
            if e.key == Key::Named(NamedKey::Enter) && rename_many.peek().is_none() && alert.peek().is_none() {
                let selection = tab.peek().selection.clone();
                start_rename(selection, renaming, rename_text, rename_error, rename_many);
                return;
            }
            let step: isize = match e.key {
                Key::Named(NamedKey::ArrowUp) => -1,
                Key::Named(NamedKey::ArrowDown) => 1,
                _ => return,
            };
            let mut t = tab.write();
            let len = t.folder.len();
            if len == 0 {
                return;
            }
            let from = t.cursor.as_deref().and_then(|c| t.folder.position(c));
            let to = match from {
                Some(i) => (i as isize + step).clamp(0, len as isize - 1) as usize,
                None if step > 0 => 0,
                None => len - 1,
            };
            let path = t.folder.entries()[to].path.clone();
            if m.contains(Modifiers::SHIFT) {
                if !t.is_selected(&path) {
                    t.selection.push(path.clone());
                }
                t.cursor = Some(path);
            } else {
                t.select_only(path);
            }
        };
        let on_key_up = move |e: Event<KeyboardEventData>| {
            let m = e.modifiers;
            held.set_if_modified(Held {
                cmd: m.contains(Modifiers::META),
                shift: m.contains(Modifiers::SHIFT),
                option: m.contains(Modifiers::ALT),
            });
        };

        // Made once, so the rows they reach stay equal while a name is typed: a new handler
        // each render would rebuild the row and the field with it.
        let (on_commit, on_cancel) = use_hook({
            let shared = shared.clone();
            move || {
                let commit = EventHandler::new(move |text: String| {
                    let Some(path) = renaming.peek().clone() else { return };
                    let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
                    let check = check_rename(&handle.tab.peek(), &path, &text);
                    match check {
                        RenameCheck::Cancel => {}
                        RenameCheck::Refused(why) => {
                            rename_error.set(Some(why));
                            return;
                        }
                        RenameCheck::Ask(question) => alert.set(Some(question)),
                        RenameCheck::Go(work) => start_job(&shared, key, work, &mut alert),
                    }
                    renaming.set(None);
                    rename_error.set(None);
                    committed.set(true);
                });
                let cancel = EventHandler::new(move |_: ()| {
                    renaming.set(None);
                    rename_error.set(None);
                });
                (commit, cancel)
            }
        });
        let interact = Interact {
            rename: renaming.read().clone().map(|path| Rename {
                path,
                text: rename_text,
                on_commit: on_commit.clone(),
                on_cancel: on_cancel.clone(),
                error: rename_error.read().clone(),
            }),
            cut: shared.cut.read().clone(),
            dragging: drag.over.read().is_some(),
            drop_target,
            press,
        };
        let body = content(&theme, &handle, rev, focused, held, on_pick, interact.clone());
        // A question goes to the window that started its job, or to the front one when that
        // has closed.
        let question = {
            let open = shared.window_keys();
            jobs.questions
                .read()
                .iter()
                .find(|q| q.window == key || (!open.contains(&q.window) && shared.front_window() == Some(key)))
                .cloned()
        };
        let count = tab.read().folder.len();
        let title = place.title();
        let (selected_items, selected_size) = {
            let t = tab.read();
            let items: Vec<_> = t.selection.iter().filter_map(|p| t.folder.get(p).cloned()).collect();
            let size = items.iter().filter(|e| !e.is_dir()).map(|e| e.size).sum::<u64>();
            (items, size)
        };
        let pane = (*preview.read() && shown.view != View::Columns).then(|| Preview {
            items: selected_items.clone(),
            folder: tab.read().here.clone(),
            width: t.size_inspector,
        });
        let on_act = {
            let (live, set_shown, shared) = (live.clone(), set_shown_here.clone(), shared.clone());
            move |act: Act| match act {
                Act::Back => tab::back(tab, &live),
                Act::Forward => tab::forward(tab, &live),
                Act::ShowSidebar => {
                    let mut sidebar = sidebar;
                    sidebar.set(true);
                }
                Act::TogglePreview => {
                    let mut preview = preview;
                    preview.toggle();
                }
                Act::Shown(next) => set_shown(next),
                Act::UseAsDefaults => shared.set_default_shown(tab.peek().shown),
                Act::StopJob(id) => {
                    if let Some(job) = shared.jobs.list.peek().iter().find(|j| j.id == id) {
                        job.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                Act::EmptyTrash => ask_empty_trash(alert),
            }
        };

        rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .on_global_key_down(on_key)
            .on_global_key_up(on_key_up)
            .on_global_pointer_move({
                let (data, tab) = (data.clone(), tab);
                move |e: Event<PointerEventData>| {
                    let Some((path, x0, y0)) = press.peek().clone() else { return };
                    let at = e.global_location();
                    if *drag_started.peek() || (at.x - x0).hypot(at.y - y0) < 4. {
                        return;
                    }
                    let (Some(data), Some(surface)) = (data.clone(), surface) else { return };
                    // A drag carries the selection it starts in, or the item alone.
                    let items = {
                        let t = tab.peek();
                        if t.is_selected(&path) { t.selection.clone() } else { vec![path.clone()] }
                    };
                    data.start_drag(surface, crate::transfer::contents(&items, false));
                    drag_started.set(true);
                }
            })
            .on_global_pointer_up(move |_: Event<PointerEventData>| {
                if !*drag_started.peek() {
                    press.set(None);
                }
            })
            .on_mouse_down({
                let live = live.clone();
                move |e: Event<MouseEventData>| match e.button {
                    Some(MouseButton::Back) => tab::back(tab, &live),
                    Some(MouseButton::Forward) => tab::forward(tab, &live),
                    _ => {}
                }
            })
            .maybe_child((*sidebar.read()).then(|| {
                Sidebar {
                    places: places.read().clone(),
                    shown: place.clone(),
                    volumes: shared.locations.volumes.read().clone(),
                    shares: shared.locations.shares.read().clone(),
                    space: shared.locations.space.read().clone(),
                    dragging: drag.over.read().is_some(),
                    drop_target,
                    on_go: EventHandler::new({
                        let go = go.clone();
                        move |place: Place| go(place)
                    }),
                    on_volume: EventHandler::new({
                        let shared = shared.clone();
                        move |volume: isle_disks::Volume| open_volume(&shared, volume, unlocking, alert, tab_go(tabs, current))
                    }),
                    on_eject: EventHandler::new({
                        let shared = shared.clone();
                        move |volume: isle_disks::Volume| eject(&shared, volume, false, alert)
                    }),
                }
            }))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .content(Content::flex())
                    .background(colour(t.colour_view))
                    .child(Toolbar {
                        title,
                        count,
                        back,
                        forward,
                        sidebar: *sidebar.read(),
                        preview: *preview.read(),
                        shown,
                        options,
                        jobs: jobs.list.read().clone(),
                        in_trash: (place == Place::Trash).then_some(count),
                        on_act: EventHandler::new(on_act),
                    })
                    .maybe_child((tabs.read().len() > 1).then(|| {
                        TabBar {
                            titles: tabs.read().iter().map(|h| h.tab.read().place.title()).collect(),
                            current: at,
                            on_select: EventHandler::new({
                                let shared = shared.clone();
                                move |i: usize| {
                                    current.set(i);
                                    shared.remember(key, &tabs.peek(), i);
                                }
                            }),
                            on_close: EventHandler::new(move |i: usize| close_tab(tabs, current, i)),
                            on_new: EventHandler::new({
                                let (open_new_tab, shared) = (open_new_tab_here.clone(), shared.clone());
                                move |_| open_new_tab(shared.new_window_place())
                            }),
                        }
                    }))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.))
                            .horizontal()
                            .content(Content::flex())
                            .child(rect().width(Size::flex(1.)).height(Size::fill()).child(body))
                            .maybe_child(pane),
                    )
                    .maybe_child((*path_bar.read()).then(|| PathBar {
                        folder: place.folder().map(PathBuf::from),
                        on_go: EventHandler::new({
                            let go = go.clone();
                            move |path: PathBuf| go(Place::Folder(path))
                        }),
                    }))
                    .maybe_child((*status_bar.read()).then(|| StatusBar {
                        folder: place.folder().map(PathBuf::from),
                        count,
                        selected: selected_items.len(),
                        selected_size,
                    })),
            )
            .maybe_child(question.map(|q| ConflictSheet { question: q, jobs }.into_element()))
            .maybe_child(unlocking.read().clone().map(|(volume, refused)| {
                let shared = shared.clone();
                crate::sheets::Unlock {
                    volume: volume.clone(),
                    refused,
                    on_done: EventHandler::new(move |given: Option<(isle_ui::field::Secret, bool)>| {
                        let mut unlocking = unlocking;
                        let Some((secret, remember)) = given else {
                            unlocking.set(None);
                            return;
                        };
                        unlock(&shared, volume.clone(), secret, remember, unlocking, tab_go(tabs, current));
                    }),
                }
                .into_element()
            }))
            .maybe_child(open_with.read().clone().map(|offer| {
                let shared = shared.clone();
                crate::sheets::OpenWith {
                    offer,
                    on_done: EventHandler::new(move |chosen: Option<(String, bool, String, PathBuf)>| {
                        let mut open_with = open_with;
                        open_with.set(None);
                        let Some((app, always, kind, path)) = chosen else { return };
                        if always
                            && let Err(err) = isle_apps::mimeapps::set_default(&[&kind], &app, false)
                        {
                            tracing::warn!("the default is not kept: {err}");
                        }
                        if let Some(bus) = shared.app().bus() {
                            spawn(crate::open::launch(bus, app, vec![isle_thumbnail::uri(&path)]));
                        }
                    }),
                }
                .into_element()
            }))
            .maybe_child(rename_many.read().clone().map(|items| {
                let shared = shared.clone();
                crate::sheets::RenameMany {
                    items,
                    on_done: EventHandler::new(move |work: Option<Work>| {
                        rename_many.set(None);
                        if let Some(work) = work {
                            start_job(&shared, key, work, &mut alert);
                        }
                    }),
                }
                .into_element()
            }))
            .maybe_child(alert.read().clone().map(|a| {
                let shared = shared.clone();
                alert_sheet(a, move |then: Option<Then>| {
                    let mut alert = alert;
                    alert.set(None);
                    match then {
                        Some(Then::Job(work)) => start_job(&shared, key, work, &mut alert),
                        Some(Then::Eject(volume, force)) => eject(&shared, volume, force, alert),
                        None => {}
                    }
                })
            }))
            .into_element()
    }
}

/// Goes to a folder in the current tab, from any thread's callback on the window.
fn tab_go(tabs: State<Vec<TabHandle>>, current: State<usize>) -> impl Fn(PathBuf) + Clone + 'static {
    move |folder: PathBuf| {
        if let Some(handle) = tabs.peek().get(*current.peek()).cloned() {
            tab::go(handle.tab, &handle.live, Place::Folder(folder), None);
        }
    }
}

/// A volume's row clicked: shown when mounted, mounted first otherwise, unlocked first when
/// encrypted, with the keyring's passphrase when it has one.
fn open_volume(
    shared: &Shared,
    volume: isle_disks::Volume,
    unlocking: State<Option<(isle_disks::Volume, Option<String>)>>,
    alert: State<Option<Alert>>,
    go: impl Fn(PathBuf) + 'static,
) {
    if let Some(point) = &volume.mount_point {
        go(PathBuf::from(point));
        return;
    }
    let worker = shared.app().worker();
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move {
                if !volume.locked {
                    return Some(isle_disks::mount(&volume.path).await.map_err(|e| e.to_string()));
                }
                let kept = isle_disks::keyring::lookup(&volume.uuid).await.ok().flatten()?;
                let cleartext = isle_disks::unlock(&volume.path, &kept).await.ok()?;
                Some(isle_disks::mount(&cleartext).await.map_err(|e| e.to_string()))
            }
        });
        let (mut unlocking, mut alert) = (unlocking, alert);
        match job.await.ok().flatten() {
            Some(Ok(point)) => go(PathBuf::from(point)),
            Some(Err(why)) => alert.set(Some(Alert::message(format!("“{}” couldn't be mounted.", volume.label), why))),
            None => unlocking.set(Some((volume, None))),
        }
    });
}

/// Unlocks with the passphrase given, keeps it if asked, mounts and shows the volume.
fn unlock(
    shared: &Shared,
    volume: isle_disks::Volume,
    secret: isle_ui::field::Secret,
    remember: bool,
    unlocking: State<Option<(isle_disks::Volume, Option<String>)>>,
    go: impl Fn(PathBuf) + 'static,
) {
    let worker = shared.app().worker();
    let passphrase = secret.expose().to_owned();
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move {
                let cleartext = isle_disks::unlock(&volume.path, &passphrase).await.map_err(|_| "That passphrase is wrong.".to_owned())?;
                if remember && let Err(err) = isle_disks::keyring::save(&volume.uuid, &volume.label, &passphrase).await {
                    tracing::warn!("the passphrase is not kept: {err}");
                }
                isle_disks::mount(&cleartext).await.map_err(|e| e.to_string())
            }
        });
        let mut unlocking = unlocking;
        match job.await {
            Ok(Ok(point)) => {
                unlocking.set(None);
                go(PathBuf::from(point));
            }
            Ok(Err(why)) => unlocking.set(Some((volume, Some(why)))),
            Err(_) => unlocking.set(None),
        }
    });
}

/// Ejects a volume; one in use asks, with Force Eject.
fn eject(shared: &Shared, volume: isle_disks::Volume, force: bool, alert: State<Option<Alert>>) {
    let worker = shared.app().worker();
    spawn(async move {
        let job = worker.spawn({
            let volume = volume.clone();
            async move { isle_disks::eject_with(&volume, force).await.map_err(|e| (isle_disks::is_busy(&e), e.to_string())) }
        });
        let mut alert = alert;
        match job.await {
            Ok(Ok(())) => {}
            Ok(Err((true, _))) => alert.set(Some(Alert::in_use(volume))),
            Ok(Err((false, why))) => alert.set(Some(Alert::message(format!("“{}” couldn't be ejected.", volume.label), why))),
            Err(_) => {}
        }
    });
}

/// A command a field takes as its key while a name is being edited.
fn field_when(command: Command, editing: bool) -> Command {
    if editing { command.field() } else { command }
}

fn same_volume(a: &std::path::Path, b: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let dev = |p: &std::path::Path| p.ancestors().find_map(|a| std::fs::symlink_metadata(a).ok()).map(|m| m.dev());
    dev(a).is_some() && dev(a) == dev(b)
}

/// Return on a selection: one item's name opens for editing, several open Rename Many.
fn start_rename(
    selection: Vec<PathBuf>,
    mut renaming: State<Option<PathBuf>>,
    mut text: State<String>,
    mut error: State<Option<String>>,
    mut many: State<Option<Vec<PathBuf>>>,
) {
    match selection.as_slice() {
        [] => {}
        [one] => {
            text.set(one.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            error.set(None);
            renaming.set(Some(one.clone()));
        }
        _ => many.set(Some(selection)),
    }
}

enum RenameCheck {
    Cancel,
    Refused(String),
    Ask(Alert),
    Go(Work),
}

/// What a typed name does (`files.md` § Acting on files, Rename).
fn check_rename(tab: &Tab, path: &std::path::Path, typed: &str) -> RenameCheck {
    let name = typed.trim();
    let old = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if name.is_empty() || name == old {
        return RenameCheck::Cancel;
    }
    if name.contains('/') {
        return RenameCheck::Refused("A name can't contain “/”.".into());
    }
    if tab.folder.get(&path.with_file_name(name)).is_some() {
        return RenameCheck::Refused(format!("“{name}” is already taken. Choose a different name."));
    }
    let to = |n: &str| Work::Rename { item: path.to_owned(), to: n.to_owned() };
    if name.starts_with('.') && !old.starts_with('.') {
        return RenameCheck::Ask(Alert {
            title: "A name that begins with a dot makes the item hidden.".into(),
            message: String::new(),
            items: Vec::new(),
            buttons: vec![(format!("Use “{name}”"), Some(Then::Job(to(name)))), ("Cancel".into(), None)],
        });
    }
    let is_dir = tab.folder.get(path).is_some_and(|e| e.is_dir());
    let (_, old_ext) = isle_fs::jobs::split_ext(&old);
    let (_, new_ext) = isle_fs::jobs::split_ext(name);
    if !is_dir && !old_ext.is_empty() && new_ext != old_ext {
        let kept = format!("{}{old_ext}", isle_fs::jobs::split_ext(name).0);
        return RenameCheck::Ask(Alert {
            title: format!("Change the extension from “{old_ext}” to “{new_ext}”?"),
            message: String::new(),
            items: Vec::new(),
            buttons: vec![(format!("Use {new_ext}"), Some(Then::Job(to(name)))), (format!("Keep {old_ext}"), Some(Then::Job(to(&kept))))],
        });
    }
    RenameCheck::Go(to(name))
}

/// Starts a job, or says first that it will not fit.
fn start_job(shared: &Shared, key: isle_app::WindowKey, work: Work, alert: &mut State<Option<Alert>>) {
    let probe = work.clone();
    let short = isle_fs::jobs::short_of_space(&probe);
    if let Some((needed, free, into)) = short {
        alert.set(Some(Alert::no_space(&into, needed, free)));
        return;
    }
    let (title, undo_label) = crate::jobs::title(&work);
    shared.jobs.start(&shared.app(), key, work, title, undo_label);
}

/// Everything in every trash; reads them, so call it off the window's thread.
fn trash_items() -> Vec<isle_fs::trash::Trashed> {
    isle_fs::trash::all_trashes().iter().flat_map(isle_fs::trash::list).collect()
}

/// Asks before emptying, once the Trash is counted.
fn ask_empty_trash(alert: State<Option<Alert>>) {
    spawn(async move {
        if let Some(items) = off_thread(trash_items).await {
            let mut alert = alert;
            alert.set(Some(Alert::empty_trash(items)));
        }
    });
}

/// A link to an item beside it, "{name} alias" (`files.md` § Acting on files).
fn make_alias(item: &std::path::Path) {
    let (Some(dir), Some(name)) = (item.parent(), item.file_name()) else { return };
    let link = isle_fs::jobs::free_name(dir, &format!("{} alias", name.to_string_lossy()), false);
    if let Err(err) = std::os::unix::fs::symlink(item, &link) {
        tracing::warn!("no alias of {}: {err}", item.display());
    }
}

/// A view changed in a tab is its folder's from now on.
fn set_shown(shared: &Shared, mut tab: State<Tab>, next: Shown) {
    let place = {
        let mut t = tab.write();
        t.shown = next;
        t.folder.set_order(next.order());
        t.rev += 1;
        t.place.clone()
    };
    shared.keep_shown(&place, next);
}

fn open_new_tab(
    shared: &Shared,
    mut tabs: State<Vec<TabHandle>>,
    mut current: State<usize>,
    key: isle_app::WindowKey,
    place: Place,
) {
    let handle = new_tab(shared, place, None);
    tab::load(handle.tab, &handle.live, false);
    let n = {
        let mut list = tabs.write();
        list.push(handle);
        list.len() - 1
    };
    current.set(n);
    shared.remember(key, &tabs.peek(), n);
}

/// Opens items of a tab: one folder in place, several (or ⌘) in tabs.
fn open_items(
    shared: &Shared,
    tabs: State<Vec<TabHandle>>,
    current: State<usize>,
    key: isle_app::WindowKey,
    handle: &TabHandle,
    paths: Vec<PathBuf>,
    in_tabs: bool,
    open_with: State<Option<crate::open::Offer>>,
) {
    let entries: Vec<_> = {
        let t = handle.tab.peek();
        paths.iter().filter_map(|p| t.folder.get(p).cloned()).collect()
    };
    let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir()).collect();
    match (dirs.as_slice(), in_tabs) {
        ([one], false) => tab::go(handle.tab, &handle.live, Place::Folder(one.path.clone()), None),
        (many, _) => {
            for dir in many {
                open_new_tab(shared, tabs, current, key, Place::Folder(dir.path.clone()));
            }
        }
    }
    // An item in the Trash is put back or dragged out to be opened.
    let files: Vec<PathBuf> = entries.iter().filter(|e| !e.is_dir() && e.trashed.is_none()).map(|e| e.path.clone()).collect();
    if !files.is_empty() {
        open_files(shared, files, open_with);
    }
}

/// Opens files in their apps, asking with Open With for the first nothing opens by itself.
fn open_files(shared: &Shared, files: Vec<PathBuf>, open_with: State<Option<crate::open::Offer>>) {
    let bus = shared.app().bus();
    spawn(async move {
        let Some((launches, ask)) = off_thread(move || crate::open::decide(&files)).await else { return };
        if let Some(bus) = bus {
            for (app, uris) in launches {
                crate::open::launch(bus.clone(), app, uris).await;
            }
        }
        if let Some(first) = ask.into_iter().next() {
            ask_open_with(first, open_with);
        }
    });
}

/// Shows Open With for one file, once its apps are read.
fn ask_open_with(path: PathBuf, open_with: State<Option<crate::open::Offer>>) {
    spawn(async move {
        if let Some(offer) = off_thread(move || crate::open::offer(&path)).await {
            tracing::info!(path = %offer.path.display(), kind = offer.kind, apps = offer.apps.len(), "open with");
            let mut open_with = open_with;
            open_with.set(Some(offer));
        }
    });
}

fn close_tab(mut tabs: State<Vec<TabHandle>>, mut current: State<usize>, i: usize) {
    let len = tabs.peek().len();
    if len <= 1 || i >= len {
        return;
    }
    tabs.write().remove(i);
    let now = *current.peek();
    if now >= i && now > 0 {
        current.set(now - 1);
    }
}

/// The folder's content, or why there is none.
fn content(
    theme: &isle_tokens::Theme,
    handle: &TabHandle,
    rev: u64,
    focused: bool,
    held: State<Held>,
    on_pick: EventHandler<Pick>,
    interact: Interact,
) -> Element {
    let t = &theme.tokens;
    let tab = handle.tab.read();
    let message = match (&tab.status, tab.folder.is_empty()) {
        (Status::Failed(isle_fs::ListError::Denied), _) => Some(format!(
            "You don't have permission to see the contents of “{}”.",
            tab.place.title()
        )),
        (Status::Gone, _) => Some(format!("“{}” was moved or deleted.", tab.place.title())),
        (Status::Failed(_), _) => Some(format!("“{}” couldn't be read.", tab.place.title())),
        (Status::Ready, true) => Some(
            match tab.place {
                Place::Recents => "No recent files",
                Place::Trash => "No items in Trash",
                _ => "No items",
            }
            .to_owned(),
        ),
        _ => None,
    };
    drop(tab);
    match message {
        Some(message) => rect()
            .expanded()
            .center()
            .background(colour(t.colour_view))
            .child(
                rect().max_width(Size::px(400.)).child(
                    label()
                        .font_size(isle_ui::text::metrics(theme, text::Style::Body).0)
                        .color(colour(t.colour_text_2))
                        .text_align(TextAlign::Center)
                        .text(message),
                ),
            )
            .into_element(),
        None => match handle.tab.peek().shown.view {
            View::List => ListView {
                tab: handle.tab,
                rev,
                focused,
                held,
                on_pick: on_pick.clone(),
                controller: handle.scroll,
                interact,
            }
            .into_element(),
            View::Icons => IconsView {
                tab: handle.tab,
                rev,
                focused,
                held,
                on_pick: on_pick.clone(),
                controller: handle.scroll,
                interact,
            }
            .into_element(),
            View::Columns => ColumnsView {
                tab: handle.tab,
                rev,
                focused,
                held,
                on_pick: on_pick.clone(),
                interact,
            }
            .into_element(),
        },
    }
}

#[derive(PartialEq)]
struct TabBar {
    titles: Vec<String>,
    current: usize,
    on_select: EventHandler<usize>,
    on_close: EventHandler<usize>,
    on_new: EventHandler<()>,
}

impl Component for TabBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut hover = use_state(|| None::<usize>);
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_tab_bar))
            .horizontal()
            .content(Content::flex())
            .background(colour(t.colour_window_sunken))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .a11y_role(AccessibilityRole::TabList)
            .child(
                // Equal widths down to the minimum; past it the tabs are cut at the bar's end.
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .overflow(Overflow::Clip)
                    .children(self.titles.iter().enumerate().map(|(i, title)| {
                        let selected = i == self.current;
                        let (on_select, on_close) = (self.on_select.clone(), self.on_close.clone());
                        let closing = self.on_close.clone();
                        rect()
                            .key(i)
                            .width(Size::flex(1.))
                            .min_width(Size::px(t.size_tab_min))
                            .height(Size::fill())
                            .center()
                            .background(colour(if selected { t.colour_view } else { t.colour_window_sunken }))
                            .a11y_role(AccessibilityRole::Tab)
                            .a11y_alt(title.clone())
                            .a11y_builder(move |node| node.set_selected(selected))
                            .on_pointer_enter(move |_| hover.set(Some(i)))
                            .on_pointer_leave(move |_| hover.set_if_modified(None))
                            .on_press(move |_| on_select.call(i))
                            .on_mouse_down(move |e: Event<MouseEventData>| {
                                if e.button == Some(MouseButton::Middle) {
                                    on_close.call(i);
                                }
                            })
                            .maybe_child((*hover.read() == Some(i)).then(|| {
                                rect()
                                    .position(Position::new_absolute().left(t.space_xs).top(7.))
                                    .width(Size::px(16.))
                                    .height(Size::px(16.))
                                    .center()
                                    .corner_radius(t.radius_small)
                                    .a11y_role(AccessibilityRole::Button)
                                    .a11y_alt("Close Tab")
                                    .on_press(move |e: Event<PressEventData>| {
                                        e.stop_propagation();
                                        closing.call(i);
                                    })
                                    .child(Glyph::new("x", 12., colour(t.colour_text_2)))
                            }))
                            .child(text::line(
                                &theme,
                                text::Style::Callout,
                                colour(if selected { t.colour_text } else { t.colour_text_2 }),
                                title.clone(),
                            ))
                            .into_element()
                    })),
            )
            .child({
                let on_new = self.on_new.clone();
                rect()
                    .width(Size::px(t.size_tab_bar))
                    .height(Size::fill())
                    .center()
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt("New Tab")
                    .on_press(move |_| on_new.call(()))
                    .child(Glyph::new("plus", 16., colour(t.colour_text_2)))
            })
    }
}

#[derive(PartialEq)]
struct Sidebar {
    places: Places,
    shown: Place,
    volumes: Vec<isle_disks::Volume>,
    shares: Vec<PathBuf>,
    space: std::collections::HashMap<PathBuf, (u64, u64)>,
    dragging: bool,
    drop_target: State<Option<PathBuf>>,
    on_go: EventHandler<Place>,
    on_volume: EventHandler<isle_disks::Volume>,
    on_eject: EventHandler<isle_disks::Volume>,
}

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let dragging = self.dragging;
        let drop_target = self.drop_target;
        // A row: its glyph and label, selected while its place is shown, and under a drag the
        // folder (or the Trash) a drop goes into.
        let row = |key: String, glyph: &'static str, label: String, selected: bool, dimmed: bool, target: Option<PathBuf>, on_press: Box<dyn Fn()>, trailing: Option<Element>| {
            let ink = if selected { t.colour_on_accent } else if dimmed { t.colour_text_3 } else { t.colour_text };
            let glyph_ink = if selected { t.colour_on_accent } else if dimmed { t.colour_text_3 } else { t.colour_accent_text };
            let ringed = dragging && target.is_some() && drop_target.peek().as_deref() == target.as_deref();
            let (enter, leave) = (target.clone(), target.clone());
            rect()
                .key(key)
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .horizontal()
                .content(Content::flex())
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .corner_radius(t.radius_row)
                .maybe(selected, |el| el.background(colour(t.colour_accent)))
                .maybe(ringed, |el| el.border(crate::interact::drop_ring(&theme)))
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(label.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_pointer_enter(move |_| {
                    if dragging && enter.is_some() {
                        let mut drop_target = drop_target;
                        drop_target.set(enter.clone());
                    }
                })
                .on_pointer_leave(move |_| {
                    if leave.is_some() && *drop_target.peek() == leave {
                        let mut drop_target = drop_target;
                        drop_target.set(None);
                    }
                })
                .on_press(move |_| on_press())
                .child(Glyph::new(glyph, t.size_glyph, colour(glyph_ink)))
                .child(rect().width(Size::flex(1.)).child(text::line(&theme, text::Style::Body, colour(ink), label)))
                .maybe_child(trailing)
                .into_element()
        };
        let place_row = |r: &Row| {
            let on_go = self.on_go.clone();
            let place = r.place.clone();
            let target = match &r.place {
                Place::Folder(p) => Some(p.clone()),
                Place::Trash => Some(PathBuf::from(crate::interact::TRASH)),
                _ => None,
            };
            row(format!("{:?}", r.place), r.glyph, r.label.clone(), r.place == self.shown, false, target, Box::new(move || on_go.call(place.clone())), None)
        };
        let header = |title: &str| {
            rect()
                .width(Size::fill())
                .height(Size::px(24.))
                .padding((0., t.space_s))
                .main_align(Alignment::End)
                .child(text::line(&theme, text::Style::Subheadline, colour(t.colour_text_2), title.to_owned()).font_weight(600))
                .into_element()
        };
        let recents = Row {
            place: Place::Recents,
            label: "Recents".into(),
            glyph: "clock-counter-clockwise",
        };
        let volume_rows = self.volumes.iter().map(|v| {
            let mounted = v.mount_point.as_ref().map(PathBuf::from);
            let shown = mounted.as_ref().is_some_and(|m| Place::Folder(m.clone()) == self.shown);
            let (on_volume, on_eject) = (self.on_volume.clone(), self.on_eject.clone());
            let (pressed, ejecting) = (v.clone(), v.clone());
            let glyph = if v.removable || v.loop_device { "usb" } else { "hard-drive" };
            let space = mounted.as_ref().and_then(|m| self.space.get(m));
            let full = crate::volumes::nearly_full(space);
            let mut trailing: Vec<Element> = Vec::new();
            if let Some(used) = full {
                trailing.push(
                    rect()
                        .width(Size::px(t.size_capacity_mark))
                        .height(Size::px(4.))
                        .corner_radius(2.)
                        .background(colour(t.colour_fill_strong))
                        .a11y_alt(format!("{} available", crate::format::size(space.map_or(0, |s| s.0))))
                        .child(rect().width(Size::percent(used * 100.)).height(Size::fill()).corner_radius(2.).background(colour(t.colour_warn)))
                        .into_element(),
                );
            }
            if mounted.is_some() && (v.removable || v.loop_device) {
                trailing.push(
                    rect()
                        .width(Size::px(t.size_control_small))
                        .height(Size::px(t.size_control_small))
                        .center()
                        .a11y_role(AccessibilityRole::Button)
                        .a11y_alt(format!("Eject “{}”", v.label))
                        .on_press(move |e: Event<PressEventData>| {
                            e.stop_propagation();
                            on_eject.call(ejecting.clone());
                        })
                        .child(Glyph::new("eject", t.size_glyph, colour(if shown { t.colour_on_accent } else { t.colour_text_2 })))
                        .into_element(),
                );
            }
            let trailing = (!trailing.is_empty()).then(|| rect().horizontal().cross_align(Alignment::Center).spacing(t.space_s).children(trailing).into_element());
            row(
                v.path.clone(),
                glyph,
                v.label.clone(),
                shown,
                mounted.is_none(),
                mounted.clone(),
                Box::new(move || on_volume.call(pressed.clone())),
                trailing,
            )
        });
        let share_rows = self.shares.iter().map(|p| {
            let on_go = self.on_go.clone();
            let place = Place::Folder(p.clone());
            row(p.display().to_string(), "network", crate::volumes::share_name(p), place == self.shown, false, Some(p.clone()), Box::new(move || on_go.call(place.clone())), None)
        });
        let (computer, trash) = (self.places.locations.first(), self.places.locations.get(1));
        rect()
            .width(Size::px(t.size_sidebar))
            .height(Size::fill())
            .background(colour(t.colour_sidebar))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 1., bottom: 0., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .child(rect().width(Size::fill()).height(Size::px(t.size_toolbar)).toolbar())
            .child(
                isle_ui::scroll_area(&theme)
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((t.space_s, t.space_s))
                            .a11y_role(AccessibilityRole::List)
                            .child(place_row(&recents))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Favourites"))
                            .children(self.places.favourites.iter().map(place_row))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Locations"))
                            .children(computer.map(place_row))
                            .children(volume_rows)
                            .children(share_rows)
                            .children(trash.map(place_row)),
                    ),
            )
    }
}
