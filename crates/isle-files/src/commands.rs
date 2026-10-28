//! Files' commands: the table the menu bar and the chords come from, and what each does.

use std::{path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_app::{Chord, ChordKey, Command, Desktop, Menu};
use isle_fs::{
    SortBy,
    jobs::Work,
};

use crate::{
    Shared,
    places::{self, Place, Places},
    sheets::Alert,
    tab,
    ops::*,
    view::{GroupBy, View},
    window::*,
};

/// The window's state the commands read and change.
#[derive(Clone, Copy)]
pub(crate) struct Win {
    pub key: isle_app::WindowKey,
    pub tabs: State<Vec<TabHandle>>,
    pub current: State<usize>,
    pub places: State<Places>,
    pub sidebar: State<bool>,
    pub preview: State<bool>,
    pub path_bar: State<bool>,
    pub status_bar: State<bool>,
    pub options: State<bool>,
    pub history: State<crate::jobs::History>,
    pub alert: State<Option<Alert>>,
    pub renaming: State<Option<PathBuf>>,
    pub rename_text: State<String>,
    pub rename_error: State<Option<String>>,
    pub rename_many: State<Option<Vec<PathBuf>>>,
    pub going: State<Option<String>>,
    /// The items a context menu's command acts on in place of the selection.
    pub acting: State<Option<Vec<PathBuf>>>,
    pub open_with: State<Option<crate::open::Offer>>,
    pub unlocking: State<Option<(isle_disks::Volume, Option<String>)>>,
    pub saving: State<bool>,
    pub find: AccessibilityId,
    pub desktop: Desktop,
}

/// The commands as the window's state has them now: names, chords, enabled and checked.
pub(crate) fn table(win: Win, shared: &Shared) -> Vec<Command> {
    let Win { tabs, current, places, sidebar, preview, path_bar, status_bar, history, renaming, find, .. } = win;
    let at = (*current.read()).min(tabs.read().len().saturating_sub(1));
    let Some(handle) = tabs.read().get(at).cloned() else { return Vec::new() };
    let tab = handle.tab;
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
    let editing = renaming.read().is_some() || find.is_focused();
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
    let undo_label = history.read().undo.last().map(|(l, _)| format!("Undo {l}"));
    let redo_label = history.read().redo.last().map(|(l, _)| format!("Redo {l}"));
    let can_make = place.folder().is_some() && !in_trash;
    let tab_bar_shown = *shared.tab_bar.read() || tabs.read().len() > 1;
    let (selected_alias, sidebar_target) = {
        let t = tab.read();
        let one = (t.selection.len() == 1).then(|| t.folder.get(&t.selection[0])).flatten();
        let alias = one.is_some_and(|e| matches!(e.kind, isle_fs::Kind::Link { broken: false, .. }));
        let folder = match one {
            Some(e) if e.is_dir() => Some(e.path.clone()),
            Some(_) => None,
            None if t.selection.is_empty() => place.folder().map(PathBuf::from),
            None => None,
        };
        (alias, folder)
    };
    let ejectable = ejectable(shared, &tab.read());
    let eject_label = match &ejectable {
        Some(v) => format!("Eject “{}”", v.label),
        None => "Eject".to_owned(),
    };
    let templates = if can_make { crate::places::templates() } else { Vec::new() };
    // With templates New file is a submenu: an empty file, then each template.
    let mut new_file = vec![
        Command::new("new-file", if templates.is_empty() { "New file" } else { "Empty File" }, Menu::File, 0)
            .chord(Chord::cmd('n').option().shift())
            .enabled(can_make),
    ];
    for (i, template) in templates.iter().enumerate() {
        let name = template.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        new_file.push(Command::new(format!("new-file:{i}"), name, Menu::File, 0));
    }
    if !templates.is_empty() {
        new_file = new_file.into_iter().map(|c| c.under("New file")).collect();
    }
    let mut table = vec![
        Command::new("undo", undo_label.clone().unwrap_or_else(|| "Undo".into()), Menu::Edit, 0)
            .chord(Chord::cmd('z'))
            .enabled(undo_label.is_some()),
        Command::new("redo", redo_label.clone().unwrap_or_else(|| "Redo".into()), Menu::Edit, 0)
            .chord(Chord::cmd('z').shift())
            .enabled(redo_label.is_some()),
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
        Command::new("get-info", "Get Info", Menu::File, 2).chord(Chord::cmd('i')),
        Command::new("show-original", "Show Original", Menu::File, 3)
            .chord(Chord::cmd('r'))
            .enabled(selected_alias),
        Command::new("add-to-sidebar", "Add to Sidebar", Menu::File, 3)
            .chord(Chord::cmd('t').ctrl())
            .enabled(sidebar_target.is_some()),
        Command::new("eject", eject_label, Menu::File, 3)
            .chord(Chord::cmd('e'))
            .enabled(ejectable.is_some()),
        Command::new("quick-look", "Quick Look", Menu::File, 2).chord(Chord::cmd('y')).enabled(has_selection),
        Command::new("summary", "Get Summary Info", Menu::File, 2)
            .chord(Chord::cmd('i').ctrl())
            .enabled(selected_count > 1),
        Command::new("inspector", "Show Inspector", Menu::File, 2).chord(Chord::cmd('i').option()),
        Command::new("open-terminal", "Open in terminal", Menu::File, 1).enabled(place.folder().is_some()),
        Command::new("set-wallpaper", "Set as wallpaper", Menu::File, 3).enabled(selected_image),
        Command::new("rename", if selected_count > 1 { format!("Rename {selected_count} Items…") } else { "Rename".into() }, Menu::File, 3)
            .enabled(has_selection && !in_trash),
        Command::new("new-window", "New Window", Menu::File, 0).chord(Chord::cmd('n')),
        Command::new("find", "Find", Menu::File, 5).chord(Chord::cmd('f')),
        Command::new("new-tab", "New Tab", Menu::File, 0).chord(Chord::cmd('t')),
        Command::new("open", "Open", Menu::File, 1)
            .chord(Chord::cmd('o'))
            .enabled(has_selection),
        // ⌘W closes the tab, or the window with one; the standard close is ⇧⌘W.
        Command::new("close-tab", "Close Tab", Menu::File, 2).chord(Chord::cmd('w')),
        Command::new("close", "Close Window", Menu::File, 2).chord(Chord::cmd('w').shift()),
        Command::new("close-all", "Close All", Menu::File, 2).chord(Chord::cmd('w').option()),
        Command::new("show-tab-bar", if tab_bar_shown { "Hide Tab Bar" } else { "Show Tab Bar" }, Menu::View, 3)
            .chord(Chord::cmd('t').shift())
            .enabled(tabs.read().len() == 1),
        Command::new("previous-tab", "Show Previous Tab", Menu::Window, 2)
            .chord(Chord::bare(ChordKey::Tab).ctrl().shift())
            .enabled(tabs.read().len() > 1),
        Command::new("next-tab", "Show Next Tab", Menu::Window, 2)
            .chord(Chord::bare(ChordKey::Tab).ctrl())
            .enabled(tabs.read().len() > 1),
        Command::new("move-tab", "Move Tab to New Window", Menu::Window, 2).enabled(tabs.read().len() > 1),
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
        // Group By and Sort By take ⌃⌘ and ⌃⌥⌘ with digits in their order, as Finder's.
        Command::new("group-kind", "Kind", Menu::View, 1)
            .chord(Chord::cmd('1').ctrl())
            .checked(shown.group == Some(GroupBy::Kind))
            .under("Group By"),
        Command::new("group-modified", "Date Modified", Menu::View, 1)
            .chord(Chord::cmd('2').ctrl())
            .checked(shown.group == Some(GroupBy::Modified))
            .under("Group By"),
        Command::new("group-size", "Size", Menu::View, 1)
            .chord(Chord::cmd('3').ctrl())
            .checked(shown.group == Some(GroupBy::Size))
            .under("Group By"),
        Command::new("sort-name", "Name", Menu::View, 1)
            .chord(Chord::cmd('1').ctrl().option())
            .checked(shown.sort == SortBy::Name)
            .under("Sort By"),
        Command::new("sort-kind", "Kind", Menu::View, 1)
            .chord(Chord::cmd('2').ctrl().option())
            .checked(shown.sort == SortBy::Kind)
            .under("Sort By"),
        Command::new("sort-modified", "Date Modified", Menu::View, 1)
            .chord(Chord::cmd('3').ctrl().option())
            .checked(shown.sort == SortBy::Modified)
            .under("Sort By"),
        Command::new("sort-size", "Size", Menu::View, 1)
            .chord(Chord::cmd('4').ctrl().option())
            .checked(shown.sort == SortBy::Size)
            .under("Sort By"),
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
        Command::new("go-to-folder", "Go to Folder…", Menu::Go, 2).chord(Chord::cmd('g').shift()),
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
    let at = table.iter().position(|c| c.id == "new-folder").map_or(0, |i| i + 1);
    table.splice(at..at, new_file);
    for (i, folder) in shared.recent_folders().iter().enumerate() {
        table.push(Command::new(format!("recent-folder:{i}"), places::folder_name(folder), Menu::Go, 2).under("Recent Folders"));
    }
    table
}

/// Carries out a context menu's row `act`.
pub(crate) fn run_menu(win: Win, shared: &Rc<Shared>, open: &crate::menus::Open, act: &crate::menus::Act) {
    use crate::menus::Act;
    let Win { key, tabs, mut current, mut places, mut acting, mut desktop, alert, .. } = win;
    let place_of = |path: &std::path::Path| {
        if path == std::path::Path::new(crate::interact::TRASH) { Place::Trash } else { Place::Folder(path.to_owned()) }
    };
    match act {
        Act::Command(id) => {
            acting.set(open.items.clone());
            desktop.commands.write().push(id.clone());
        }
        Act::OpenWith(app) => {
            let (Some(bus), Some(items)) = (shared.app().bus(), open.items.clone()) else { return };
            spawn(crate::open::launch(bus, app.clone(), items.iter().map(|p| isle_thumbnail::uri(p)).collect()));
        }
        Act::OpenInTab(path) => open_new_tab(shared, tabs, current, key, place_of(path)),
        Act::OpenInWindow(path) => shared.open_window(vec![(place_of(path), None)], 0, None),
        Act::ShowEnclosing(path) => {
            let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
            if let Some(parent) = path.parent() {
                tab::go(handle.tab, &handle.live, Place::Folder(parent.to_owned()), Some(path.clone()));
            }
        }
        Act::RemoveFromSidebar(place, label) => {
            // A place of the sidebar's own turns off in Settings; a bookmark is dropped.
            let id = label.to_lowercase();
            if crate::prefs::sidebar_row(&id) {
                desktop.set(crate::prefs::key(&format!("sidebar.{id}")), isle_settings::Value::Bool(false));
                return;
            }
            let Some(folder) = place.folder().map(PathBuf::from) else { return };
            spawn(async move {
                let read = off_thread(move || {
                    if let Err(err) = places::remove_bookmark(&folder) {
                        tracing::warn!("{} was not removed from the sidebar: {err}", folder.display());
                    }
                    places::read()
                });
                if let Some(read) = read.await {
                    places.set(read);
                }
            });
        }
        Act::Eject(volume) => eject(shared, volume.clone(), false, alert),
        Act::NewTab => open_new_tab(shared, tabs, current, key, shared.new_window_place()),
        Act::CloseTab(i) if tabs.peek().len() <= 1 => {
            let _ = i;
            shared.app().request_close(key);
        }
        Act::CloseTab(i) => close_tab(tabs, current, *i),
        Act::CloseOtherTabs(i) => {
            let mut tabs = tabs;
            let kept = tabs.peek().get(*i).cloned();
            if let Some(kept) = kept {
                tabs.set(vec![kept]);
                current.set(0);
                shared.remember(key, &tabs.peek(), 0);
            }
        }
        Act::MoveTab(i) => {
            current.set(*i);
            desktop.commands.write().push("move-tab".into());
        }
        Act::GoTo(folder) => {
            let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
            tab::go(handle.tab, &handle.live, Place::Folder(folder.clone()), None);
        }
        Act::Column(column) => {
            let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
            let mut next = handle.tab.peek().shown;
            next.hidden.toggle(*column);
            set_shown(shared, handle.tab, next);
        }
    }
}

/// The volume ⌘E ejects: the one selected, else the one the folder shown is on.
fn ejectable(shared: &Shared, tab: &crate::tab::Tab) -> Option<isle_disks::Volume> {
    let volumes = shared.locations.volumes.read();
    let on = |path: &std::path::Path| {
        volumes
            .iter()
            .filter(|v| v.removable || v.loop_device)
            .find(|v| v.mount_point.as_deref().is_some_and(|m| path.starts_with(m)))
            .cloned()
    };
    match tab.selection.as_slice() {
        [one] => on(one),
        _ => tab.place.folder().and_then(on),
    }
}

/// Carries out the commands chosen since last, each on the tab current when it is taken.
pub(crate) fn chosen(win: Win, shared: &Rc<Shared>) {
    let Win { key, tabs, current, places, mut sidebar, mut preview, mut path_bar, mut status_bar, mut options, mut history, mut alert, renaming, rename_text, rename_error, rename_many, mut going, mut acting, open_with, unlocking, saving, find, desktop } = win;
    let jobs = shared.jobs;
    let chosen: Vec<String> = desktop.commands.read().clone();
    if chosen.is_empty() {
        return;
    }
    let mut queue = desktop.commands;
    queue.write().clear();
    // A sheet holds the window: only what opens another window gets past it.
    let sheet_up = alert.peek().is_some()
        || rename_many.peek().is_some()
        || going.peek().is_some()
        || open_with.peek().is_some()
        || unlocking.peek().is_some()
        || *saving.peek()
        || jobs.questions.peek().iter().any(|q| q.window == key);
    for id in chosen {
        if sheet_up && !matches!(id.as_str(), "new-window" | "settings") {
            acting.write().take();
            continue;
        }
        let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
        let (mut tab, live) = (handle.tab, handle.live.clone());
        // A context menu's command acts on its items, the selection put back after.
        let mut menu_tab = tab;
        let kept = acting.write().take().map(|items| {
            let place = menu_tab.peek().place.clone();
            (place, std::mem::replace(&mut menu_tab.write().selection, items))
        });
        let go = |place: Place| tab::go(tab, &live, place, None);
        let fav = |label: &str| {
            places
                .peek()
                .favourites
                .iter()
                .find(|r| r.label == label)
                .map(|r| r.place.clone())
        };
        'run: {
            match id.as_str() {
                "new-window" => shared.open_window(vec![(shared.new_window_place(), None)], 0, None),
                "new-tab" => open_new_tab(&shared, tabs, current, key, shared.new_window_place()),
                "close-tab" if tabs.peek().len() <= 1 => shared.app().request_close(key),
                "close-tab" => {
                    let at = *current.peek();
                    close_tab(tabs, current, at);
                }
                "close-all" => {
                    let app = shared.app();
                    for window in shared.window_keys() {
                        app.request_close(window);
                    }
                }
                "show-tab-bar" => {
                    let show = !*shared.tab_bar.peek();
                    shared.set_tab_bar(show);
                }
                "previous-tab" | "next-tab" => {
                    let n = tabs.peek().len();
                    if n > 1 {
                        let by = if id == "next-tab" { 1 } else { n - 1 };
                        let to = (*current.peek() + by) % n;
                        let mut current = current;
                        current.set(to);
                        shared.remember(key, &tabs.peek(), to);
                    }
                }
                "move-tab" if tabs.peek().len() > 1 => {
                    let at = *current.peek();
                    let place = tab.peek().place.clone();
                    close_tab(tabs, current, at);
                    shared.remember(key, &tabs.peek(), *current.peek());
                    shared.open_window(vec![(place, None)], 0, None);
                }
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
                        break 'run;
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
                "go-to-folder" => going.set(Some(String::new())),
                id if id.starts_with("recent-folder:") => {
                    let at = id["recent-folder:".len()..].parse::<usize>().ok();
                    if let Some(folder) = at.and_then(|i| shared.recent_folders().into_iter().nth(i)) {
                        go(Place::Folder(folder));
                    }
                }
                "show-original" => {
                    let one = tab.peek().selection.first().cloned();
                    let Some(alias) = one else { break 'run };
                    let Ok(to) = std::fs::read_link(&alias) else { break 'run };
                    let target = alias.parent().map_or(to.clone(), |dir| dir.join(&to));
                    if let Some(folder) = target.parent() {
                        tab::go(tab, &live, Place::Folder(folder.to_owned()), Some(target.clone()));
                    }
                }
                "add-to-sidebar" => {
                    let t = tab.peek();
                    let folder = match t.selection.as_slice() {
                        [one] if t.folder.get(one).is_some_and(|e| e.is_dir()) => Some(one.clone()),
                        [] => t.place.folder().map(PathBuf::from),
                        _ => None,
                    };
                    drop(t);
                    let Some(folder) = folder else { break 'run };
                    let mut places = places;
                    spawn(async move {
                        let read = off_thread(move || {
                            if let Err(err) = places::add_bookmark(&folder) {
                                tracing::warn!("{} was not added to the sidebar: {err}", folder.display());
                            }
                            places::read()
                        });
                        if let Some(read) = read.await {
                            places.set(read);
                        }
                    });
                }
                "eject" => {
                    if let Some(volume) = ejectable(shared, &tab.peek()) {
                        crate::ops::eject(shared, volume, false, alert);
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
                "find" => find.request_focus(),
                "settings" => shared.open_settings(None, None),
                "get-info" | "summary" => {
                    let t = tab.peek();
                    let items = if t.selection.is_empty() {
                        t.place.folder().map(PathBuf::from).into_iter().collect()
                    } else {
                        t.selection.clone()
                    };
                    drop(t);
                    shared.get_info(items, id == "summary", None);
                }
                "inspector" => shared.toggle_inspector(),
                "quick-look" => {
                    let selection = tab.peek().selection.clone();
                    shared.toggle_quick_look(selection, 0, crate::quicklook::Caller::Window(key));
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
                    let last = history.write().undo.pop();
                    if let Some((label, steps)) = last {
                        jobs.start_doing(&shared.app(), key, Work::Undo { steps }, format!("Undoing {label}"), label, crate::jobs::Doing::Undo);
                    }
                }
                "redo" => {
                    let last = history.write().redo.pop();
                    if let Some((label, steps)) = last {
                        jobs.start_doing(&shared.app(), key, Work::Undo { steps }, format!("Redoing {label}"), label, crate::jobs::Doing::Redo);
                    }
                }
                id if id == "new-file" || id.starts_with("new-file:") => {
                    let Some(into) = tab.peek().place.folder().map(PathBuf::from) else { break 'run };
                    let template = id
                        .strip_prefix("new-file:")
                        .and_then(|i| i.parse::<usize>().ok())
                        .and_then(|i| crate::places::templates().into_iter().nth(i));
                    let name = template
                        .as_deref()
                        .and_then(|t| t.file_name())
                        .map_or_else(|| "untitled".to_owned(), |n| n.to_string_lossy().into_owned());
                    start_job(shared, key, Work::NewFile { into, name, template }, &mut alert);
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
                        "empty-trash" if shared.prefs().warn_empty_trash => {
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
                    let show = !*shared.hidden.peek();
                    shared.set_hidden(show);
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
                        _ => break 'run,
                    };
                    if let Some(place) = fav(label) {
                        go(place);
                    }
                }
            }
        }
        if let Some((place, selection)) = kept
            && menu_tab.peek().place == place
        {
            menu_tab.write().selection = selection;
        }
    }
    shared.remember(key, &tabs.peek(), *current.peek());
}
