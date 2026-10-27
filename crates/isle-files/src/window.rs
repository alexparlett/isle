//! A Files window: the sidebar of places, the toolbar, the tab bar and the folder shown
//! (`p1/structure/files.md` § Content; `p1/surfaces/files.md` § Anatomy).

use std::{path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_app::{Desktop, use_commands};
use isle_fs::jobs::{Step, Undo, Work};
use isle_ui::{colour, text, use_theme};

use crate::{
    Shared, WindowLink,
    bars::{PathBar, StatusBar},
    columns_view::ColumnsView,
    icons_view::IconsView,
    interact::{Interact, Rename, TRASH},
    list_view::{ListView, Pick},
    places::{self, Place, Places},
    preview::Preview,
    scope_bar::{ScopeBar, Where},
    search::{Criterion, Search},
    tab::{self, Live, Status, Tab},
    jobs::Finished,
    ops::*,
    sheets::{Alert, ConflictSheet, Then, alert_sheet},
    sidebar::Sidebar,
    tabbar::TabBar,
    toolbar::{Act, Toolbar},
    view::{Shown, View},
};

/// What another part of the app asks a window to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    /// A tab at the place, the item revealed in it.
    Tab(Place, Option<PathBuf>),
    /// Open With for a file nothing opens by itself.
    OpenWith(PathBuf),
    /// An alert the app raises on its front window, as quitting with work in flight.
    Alert(Alert),
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
    /// The search field takes the keyboard once the window shows.
    pub find: bool,
}

/// Runs `f` on a thread of its own and hands its answer back.
pub async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.ok()
}

/// A tab owned by `scope`, the window's: a tab made in a view's handler would go with the view.
pub(crate) fn new_tab(shared: &Shared, place: Place, reveal: Option<PathBuf>, scope: ScopeId) -> TabHandle {
    let mut t = Tab::new(shared.next_tab(), place);
    t.reveal = reveal;
    t.folder.set_show_hidden(*shared.hidden.peek());
    let memory = crate::SHARED.with(|s| s.borrow().clone());
    run_in_scope(scope, || TabHandle {
        tab: State::create(t),
        live: match memory {
            Some(shared) => Live::new(Rc::new(move |place: &Place| shared.shown_for(place)), Some(scope)),
            None => Live::default(),
        },
        scroll: ScrollController::new(0, 0),
    })
}

impl App for FilesWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let shared = self.shared.clone();
        let key = isle_app::this_window();
        let desktop = Desktop::get();
        let scope = current_scope_id();
        let mut tabs = use_state({
            let (shared, start) = (shared.clone(), self.tabs.clone());
            move || {
                start
                    .into_iter()
                    .map(|(place, reveal)| new_tab(&shared, place, reveal, scope))
                    .collect::<Vec<_>>()
            }
        });
        let mut current = use_state(|| self.current.min(self.tabs.len().saturating_sub(1)));
        let mut places = use_state(Places::default);
        let inbox = use_state(Vec::<Open>::new);
        let sidebar = use_state(|| true);
        let preview = use_state(|| false);
        let path_bar = use_state(|| false);
        let status_bar = use_state(|| false);
        let options = use_state(|| false);
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
        let mut query = use_state(String::new);
        let find = use_a11y();
        let mut saving = use_state(|| false);
        // The query a move keeps, as the scope bar's This folder keeps the term.
        let mut keep_query = use_state(|| None::<String>);
        let mut seen_place = use_state(|| None::<(u64, Place)>);
        let mut found_field = use_state(|| !self.find);
        use_side_effect(move || {
            if !*found_field.read() {
                found_field.set(true);
                find.request_focus();
            }
        });
        let drag = isle_app::use_drag();
        let surface = isle_app::this_surface();
        let data = shared.app().data();

        use_hook({
            let shared = shared.clone();
            move || {
                for handle in tabs.peek().iter() {
                    tab::load(handle.tab, &handle.live, false);
                }
                shared.register(key, WindowLink { inbox, scope });
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

        // Files' settings as the compositor has them; a change to how names show or sort
        // draws every tab again.
        use_side_effect({
            let shared = shared.clone();
            move || {
                let next = crate::prefs::Prefs::read(&desktop);
                if shared.prefs() == next {
                    return;
                }
                if shared.take_prefs(next) {
                    for handle in tabs.peek().iter() {
                        let mut tab = handle.tab;
                        let mut t = tab.write();
                        let order = t.shown.order();
                        t.folder.set_order(order);
                        t.rev += 1;
                    }
                }
            }
        });

        // The focused window's selection is the one the Inspector shows.
        use_side_effect({
            let shared = shared.clone();
            move || {
                if !*Platform::get().is_app_focused.read() {
                    return;
                }
                let at = *current.read();
                let Some(handle) = tabs.read().get(at).cloned() else { return };
                let t = handle.tab.read();
                let shown = if t.selection.is_empty() {
                    t.place.folder().map(PathBuf::from).into_iter().collect()
                } else {
                    t.selection.clone()
                };
                drop(t);
                shared.bring_front(key);
                // Quick Look follows the selection it was opened on.
                let cursor = {
                    let t = handle.tab.peek();
                    t.cursor.as_ref().and_then(|c| shown.iter().position(|p| p == c)).unwrap_or(0)
                };
                shared.quick_look_follow(crate::quicklook::Caller::Window(key), shown.clone(), cursor);
                let mut selection = shared.selection;
                if *selection.peek() != shown {
                    selection.set(shown);
                }
            }
        });

        // The search field follows the tab and its place; what it holds filters the items shown.
        use_side_effect(move || {
            let at = *current.read();
            let Some(handle) = tabs.read().get(at).cloned() else { return };
            let mut tab = handle.tab;
            let here = {
                let t = tab.read();
                (t.id, t.place.clone())
            };
            if seen_place.peek().as_ref() != Some(&here) {
                let next = keep_query.write().take().unwrap_or_else(|| match &here.1 {
                    Place::Search(s) => s.term.clone(),
                    _ => String::new(),
                });
                seen_place.set(Some(here));
                if *query.peek() != next {
                    query.set(next);
                }
            }
            let typed = query.read().trim().to_lowercase();
            let want = (!typed.is_empty()).then_some(typed);
            if tab.peek().folder.filter() != want.as_deref() {
                let mut t = tab.write();
                t.folder.set_filter(want.as_deref());
                t.prune();
                t.rev += 1;
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
                        Open::Alert(raised) => alert.set(Some(raised)),
                        // A tab already showing the place is chosen rather than another added.
                        Open::Tab(place, reveal) if let Some(n) = tabs.peek().iter().position(|h| h.tab.peek().place == place) => {
                            if let Some(path) = reveal {
                                let mut tab = tabs.peek()[n].tab;
                                tab.write().reveal(path);
                            }
                            current.set(n);
                        }
                        Open::Tab(place, reveal) => {
                            let handle = new_tab(&shared, place, reveal, scope);
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
        let win = crate::commands::Win {
            key,
            tabs,
            current,
            places,
            sidebar,
            preview,
            path_bar,
            status_bar,
            options,
            undo_stack,
            alert,
            renaming,
            rename_text,
            rename_error,
            rename_many,
            open_with,
            unlocking,
            saving,
            find,
            desktop,
        };
        use_commands(crate::commands::table(win, &shared));

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
            move || crate::commands::chosen(win, &shared)
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
        let shared_keys = shared.clone();
        let on_key = move |e: Event<KeyboardEventData>| {
            let m = e.modifiers;
            if m.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::ALT) || renaming.peek().is_some() || find.is_focused() {
                return;
            }
            if e.key == Key::Named(NamedKey::Escape)
                && shared_keys.quick_look_open().is_some_and(|q| q.caller == crate::quicklook::Caller::Window(key))
            {
                shared_keys.close_quick_look();
                return;
            }
            if e.key == Key::Character(" ".into()) {
                let selection = tab.peek().selection.clone();
                shared_keys.toggle_quick_look(selection, 0, crate::quicklook::Caller::Window(key));
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

        // Made once, so the rows they reach stay equal while a name is typed: a new handler
        // each render would rebuild the row and the field with it.
        let (on_commit, on_cancel) = use_hook({
            let shared = shared.clone();
            move || {
                let commit = EventHandler::new(move |text: String| {
                    let Some(path) = renaming.peek().clone() else { return };
                    let Some(handle) = tabs.peek().get(*current.peek()).cloned() else { return };
                    let check = check_rename(&handle.tab.peek(), &path, &text, shared.prefs().warn_extension_change);
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
            chooses: None,
        };
        let body = content(&theme, &handle, rev, focused, on_pick, interact.clone());
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
        let action = match selected_items.as_slice() {
            [_] if place == Place::Trash => Some(("Put Back", EventHandler::new(move |_| {
                let mut commands = desktop.commands;
                commands.write().push("put-back".into());
            }))),
            [one] if one.mime.as_deref().is_some_and(crate::markup::editable) => {
                let (shared, path) = (shared.clone(), one.path.clone());
                Some(("Markup", EventHandler::new(move |_| {
                    let over = shared.app().export(key);
                    shared.show_quick_look_in(vec![path.clone()], 0, crate::quicklook::Caller::Window(key), over, "markup");
                })))
            }
            _ => None,
        };
        let pane = (*preview.read() && shown.view != View::Columns).then(|| Preview {
            items: selected_items.clone(),
            folder: tab.read().here.clone(),
            width: t.size_inspector,
            pane: true,
            action,
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
                Act::EmptyTrash if shared.prefs().warn_empty_trash => ask_empty_trash(alert),
                Act::EmptyTrash => {
                    let shared = shared.clone();
                    spawn(async move {
                        if let Some(items) = off_thread(trash_items).await {
                            let mut alert = alert;
                            start_job(&shared, key, Work::EmptyTrash { items }, &mut alert);
                        }
                    });
                }
                Act::Search(text) => {
                    let term = text.trim().to_owned();
                    if term.is_empty() {
                        return;
                    }
                    let place = tab.peek().place.clone();
                    match place {
                        Place::Search(s) => tab::replace(tab, &live, Place::Search(Box::new(Search { term, name: None, ..*s }))),
                        other => {
                            let mut search = search_from(&other, term, Vec::new());
                            if search.folder.is_some() {
                                search.scope = shared.prefs().scope(shared.last_scope.get());
                            }
                            shared.last_scope.set(Some(search.scope));
                            tab::go(tab, &live, Place::Search(Box::new(search)), None)
                        }
                    }
                }
                Act::EndSearch => {
                    find.request_unfocus();
                    let mut query = query;
                    query.set(String::new());
                    end_search(tab, &live);
                }
            }
        };
        let on_where = {
            let (live, shared) = (live.clone(), shared.clone());
            move |chosen: Where| {
                let place = tab.peek().place.clone();
                match (chosen, place) {
                    (Where::ThisFolder, Place::Search(s)) => {
                        let mut keep_query = keep_query;
                        keep_query.set(Some(s.term.clone()));
                        end_search(tab, &live);
                    }
                    (Where::ThisFolder, _) => {}
                    (Where::Search(scope), Place::Search(s)) => {
                        shared.last_scope.set(Some(scope));
                        tab::replace(tab, &live, Place::Search(Box::new(Search { scope, name: None, ..*s })))
                    }
                    (Where::Search(scope), other) => {
                        shared.last_scope.set(Some(scope));
                        let mut search = search_from(&other, query.peek().trim().to_owned(), Vec::new());
                        search.scope = scope;
                        tab::go(tab, &live, Place::Search(Box::new(search)), None);
                    }
                }
            }
        };
        let on_criteria = {
            let live = live.clone();
            move |criteria: Vec<Criterion>| {
                let place = tab.peek().place.clone();
                match place {
                    Place::Search(s) => tab::replace(tab, &live, Place::Search(Box::new(Search { criteria, name: None, ..*s }))),
                    other => {
                        let search = search_from(&other, query.peek().trim().to_owned(), criteria);
                        tab::go(tab, &live, Place::Search(Box::new(search)), None);
                    }
                }
            }
        };
        let searching = matches!(place, Place::Search(_));
        let scope_bar = (searching || !query.read().trim().is_empty()).then(|| {
            let (search, this_folder, folder) = match &place {
                Place::Search(s) => (Some((**s).clone()), s.folder.is_some(), s.folder.as_deref().map(places::folder_name)),
                Place::Folder(f) => (None, true, Some(places::folder_name(f))),
                _ => (None, true, None),
            };
            ScopeBar {
                search,
                this_folder,
                folder,
                on_where: EventHandler::new(on_where),
                on_criteria: EventHandler::new(on_criteria),
                on_save: EventHandler::new(move |_| saving.set(true)),
            }
        });

        rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .on_global_key_down(on_key)
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
                    saved: shared.saved.read().clone(),
                    prefs: shared.prefs(),
                    width: t.size_sidebar,
                    under_toolbar: true,
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
                        query,
                        find,
                        searching,
                        glyph: match place {
                            _ if searching => Some("magnifying-glass"),
                            Place::Trash => Some("trash"),
                            Place::Recents => Some("clock-counter-clockwise"),
                            _ => None,
                        },
                        proxy: tab.read().here.clone(),
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
                    .maybe_child(scope_bar)
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
            .maybe_child(question.map(|q| ConflictSheet { question: q, jobs, alone: false }.into_element()))
            .maybe_child((*saving.read()).then(|| {
                let shared = shared.clone();
                let live = live.clone();
                let suggested = match &place {
                    Place::Search(s) => s.name.clone().unwrap_or_else(|| s.term.clone()),
                    _ => String::new(),
                };
                crate::sheets::SaveSearch {
                    suggested,
                    on_done: EventHandler::new(move |name: Option<String>| {
                        saving.set(false);
                        let (Some(name), Place::Search(s)) = (name, tab.peek().place.clone()) else { return };
                        let named = Search { name: Some(name), ..*s };
                        shared.save_search(named.clone());
                        tab::replace(tab, &live, Place::Search(Box::new(named)));
                    }),
                }
                .into_element()
            }))
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
                        Some(Then::Quit) => shared.quit_now(),
                        None => {}
                    }
                })
            }))
            .into_element()
    }
}

/// The folder's content, or why there is none.
pub(crate) fn content(
    theme: &isle_tokens::Theme,
    handle: &TabHandle,
    rev: u64,
    focused: bool,
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
        (Status::Ready, true) => Some(match (&tab.place, tab.folder.filter()) {
            (Place::Search(s), _) if !s.term.is_empty() => format!("No results for “{}”", s.term),
            (Place::Search(_), _) => "No results".to_owned(),
            (_, Some(typed)) => format!("No results for “{typed}”"),
            (Place::Recents, _) => "No recent files".to_owned(),
            (Place::Trash, _) => "No items in Trash".to_owned(),
            _ => "No items".to_owned(),
        }),
        _ => None,
    };
    drop(tab);
    match message {
        Some(message) => rect()
            .expanded()
            .center()
            .background(colour(t.colour_view))
            .child(
                rect().width(Size::fill()).max_width(Size::px(400.)).child(
                    label()
                        .width(Size::fill())
                        .font_size(isle_ui::text::metrics(theme, text::Style::Body).0)
                        .color(colour(t.colour_text_2))
                        .text_align(TextAlign::Center)
                        .text(message),
                ),
            )
            .into_element(),
        None => match handle.tab.peek().shown.view {
            // Columns turns into List while a search stands.
            View::Columns if matches!(handle.tab.peek().place, Place::Search(_)) => ListView {
                tab: handle.tab,
                rev,
                focused,
                on_pick: on_pick.clone(),
                controller: handle.scroll,
                interact,
            }
            .into_element(),
            View::List => ListView {
                tab: handle.tab,
                rev,
                focused,
                on_pick: on_pick.clone(),
                controller: handle.scroll,
                interact,
            }
            .into_element(),
            View::Icons => IconsView {
                tab: handle.tab,
                rev,
                focused,
                on_pick: on_pick.clone(),
                controller: handle.scroll,
                interact,
            }
            .into_element(),
            View::Columns => ColumnsView {
                tab: handle.tab,
                rev,
                focused,
                on_pick: on_pick.clone(),
                on_go: EventHandler::new({
                    let (tab, live) = (handle.tab, handle.live.clone());
                    move |(folder, reveal): (PathBuf, Option<PathBuf>)| tab::go(tab, &live, Place::Folder(folder), reveal)
                }),
                interact,
            }
            .into_element(),
        },
    }
}
