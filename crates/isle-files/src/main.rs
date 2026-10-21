//! Files (`docs/design/apps.md` Files; `p1/structure/files.md`).

mod bars;
mod chooser;
mod columns_view;
mod files1;
mod fm1;
mod format;
mod icons;
mod icons_view;
mod info;
mod look;
mod markup;
mod markup_view;
mod interact;
mod jobs;
mod list_view;
mod sheets;
mod preview;
mod toolbar;
mod transfer;
mod memory;
mod open;
mod panel;
mod places;
mod prefs;
mod ql1;
mod quicklook;
mod scope_bar;
mod search;
mod settings_window;
mod tab;
mod trim;
mod thumbnail;
mod thumbs;
mod view;
mod volumes;
mod window;

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
};

use freya::prelude::*;
use isle_app::{AppHandle, Delegate, Request, Spec, WindowKey, WindowSpec};
use tracing::info;

use crate::{
    memory::{Memory, WindowMemory},
    places::Place,
    window::{FilesWindow, Open, TabHandle},
};

const SPEC: Spec = Spec {
    id: "io.github.alexparlett.Isle.Files",
    title: "Files",
    size: (1400.0, 860.0),
    min_size: (560.0, 400.0),
    max_size: None,
    // The lights' 32 point group centred in the 52 point toolbar.
    traffic_light_inset: (0.0, 10.0),
    preferences: true,
    backdrop: None,
};

/// How a window reaches its own state from the app.
#[derive(Clone, Copy)]
pub struct WindowLink {
    pub inbox: State<Vec<Open>>,
}

/// What the app keeps above its windows.
pub struct Shared {
    app: RefCell<Option<AppHandle>>,
    windows: RefCell<BTreeMap<WindowKey, WindowLink>>,
    /// The windows in the order they were last used, the front one last.
    front: RefCell<Vec<WindowKey>>,
    memory: RefCell<Memory>,
    next_tab: Cell<u64>,
    /// Hidden files shown, in every window (`files.md` § The content: views).
    pub hidden: State<bool>,
    pub jobs: jobs::Jobs,
    /// Items cut and not yet pasted, drawn dimmed.
    pub cut: State<Vec<PathBuf>>,
    pub locations: volumes::Locations,
    /// Saved searches, as Favourites lists them.
    pub saved: State<Vec<search::Search>>,
    /// The front window's selection, which the Inspector follows.
    pub selection: State<Vec<PathBuf>>,
    /// Get Info's sections closed, in every Get Info window.
    info_collapsed: State<Vec<String>>,
    /// The Inspector's window while it is open.
    inspector: Cell<Option<WindowKey>>,
    /// Files' settings as the windows last read them.
    prefs: RefCell<prefs::Prefs>,
    /// The settings window while it is open, and the pane it shows.
    settings: Cell<Option<WindowKey>>,
    settings_at: State<settings_window::At>,
    /// The scope the last search used, for Use the previous scope.
    pub last_scope: Cell<Option<search::Scope>>,
    /// The Trash has been cleared of old items this launch.
    purged: Cell<bool>,
    /// The panels open, by their request.
    panels: RefCell<BTreeMap<u64, (WindowKey, String)>>,
    /// Requests waiting for their parent's panel to answer.
    waiting: RefCell<Vec<chooser::Ask>>,
    /// Quick Look while it is open.
    quick_look: RefCell<Option<quicklook::Open>>,
}

impl Shared {
    pub fn app(&self) -> AppHandle {
        self.app.borrow().clone().expect("the app has started")
    }

    pub fn next_tab(&self) -> u64 {
        let n = self.next_tab.get() + 1;
        self.next_tab.set(n);
        n
    }

    /// Where a new window or tab opens (`files.md` § Tabs and windows), as General sets it.
    pub fn new_window_place(&self) -> Place {
        let home = places::home().unwrap_or_else(|| PathBuf::from("/"));
        let dir = |key: &str, name: &str| {
            let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
            std::fs::read_to_string(config.join("user-dirs.dirs"))
                .ok()
                .and_then(|t| places::user_dirs(&t, &home).into_iter().find(|(k, _)| k == key).map(|(_, p)| p))
                .unwrap_or_else(|| home.join(name))
        };
        let place = match self.prefs.borrow().new_window.as_str() {
            "desktop" => Place::Folder(dir("XDG_DESKTOP_DIR", "Desktop")),
            "documents" => Place::Folder(dir("XDG_DOCUMENTS_DIR", "Documents")),
            "downloads" => Place::Folder(dir("XDG_DOWNLOAD_DIR", "Downloads")),
            "recents" => Place::Recents,
            "computer" => Place::Computer,
            _ => Place::Folder(home.clone()),
        };
        match place {
            Place::Folder(p) if !p.is_dir() => Place::Folder(home),
            other => other,
        }
    }

    pub fn prefs(&self) -> prefs::Prefs {
        self.prefs.borrow().clone()
    }

    /// Takes the settings as a window read them; whether the views must draw again.
    pub fn take_prefs(&self, next: prefs::Prefs) -> bool {
        let redraw = next.apply();
        if next.purge_trash && !self.purged.replace(true) {
            std::thread::spawn(purge_trash);
        }
        *self.prefs.borrow_mut() = next;
        redraw
    }

    /// Opens Files' settings window, or shows `at` in the one open.
    pub fn open_settings(&self, at: Option<settings_window::At>, token: Option<String>) {
        let app = self.app();
        let mut shown = self.settings_at;
        if let Some(at) = at {
            shown.set(at);
        }
        match self.settings.get().filter(|k| app.windows().contains(k)) {
            Some(key) => {
                if let Some(token) = token {
                    app.activate(key, &token);
                }
            }
            None => {
                let spec = WindowSpec::new("Files Settings", (560., 520.), (480., 360.)).token(token);
                self.settings.set(Some(app.open(spec, settings_window::SettingsWindow { at: shown })));
            }
        }
    }

    pub fn register(&self, key: WindowKey, link: WindowLink) {
        self.windows.borrow_mut().insert(key, link);
        self.bring_front(key);
    }

    pub fn unregister(&self, key: WindowKey) {
        self.windows.borrow_mut().remove(&key);
        self.front.borrow_mut().retain(|k| *k != key);
        // The last window's tabs are kept to open again at the next launch.
        if !self.windows.borrow().is_empty() {
            self.memory.borrow_mut().windows.remove(&key.number());
            self.save();
        }
    }

    pub fn window_keys(&self) -> Vec<WindowKey> {
        self.windows.borrow().keys().copied().collect()
    }

    pub fn front_window(&self) -> Option<WindowKey> {
        self.front.borrow().last().copied()
    }

    pub fn bring_front(&self, key: WindowKey) {
        let mut front = self.front.borrow_mut();
        front.retain(|k| *k != key);
        front.push(key);
    }

    /// Keeps a window's tabs to open again (`apps.md` Conventions 6).
    pub fn remember(&self, key: WindowKey, tabs: &[TabHandle], current: usize) {
        let window = WindowMemory {
            tabs: tabs.iter().map(|h| h.tab.peek().place.clone()).collect(),
            current,
        };
        self.memory.borrow_mut().windows.insert(key.number(), window);
        self.save();
    }

    /// A place's remembered view.
    pub fn shown_for(&self, place: &Place) -> view::Shown {
        self.memory.borrow().shown(place)
    }

    pub fn keep_shown(&self, place: &Place, shown: view::Shown) {
        self.memory.borrow_mut().keep(place, shown);
        self.save();
    }

    /// Saves a search under its name, replacing one of the same name.
    pub fn save_search(&self, search: search::Search) {
        let mut saved = self.saved;
        saved.write().retain(|s| s.name != search.name);
        saved.write().push(search);
        self.memory.borrow_mut().saved = saved.peek().clone();
        self.save();
    }

    pub fn collapsed_info(&self) -> Vec<String> {
        self.info_collapsed.read().clone()
    }

    pub fn toggle_info_section(&self, title: &str) {
        let mut collapsed = self.info_collapsed;
        let mut list = collapsed.write();
        match list.iter().position(|c| c == title) {
            Some(i) => {
                list.remove(i);
            }
            None => list.push(title.to_owned()),
        }
        self.memory.borrow_mut().info_collapsed = list.clone();
        drop(list);
        self.save();
    }

    /// Get Info for each item, or one summary of them all.
    pub fn get_info(&self, items: Vec<PathBuf>, summary: bool, token: Option<String>) {
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let app = self.app();
        let open = |of: info::InfoOf| {
            let spec = WindowSpec::new(of.title(), (300., 620.), (260., 320.)).token(token.clone());
            app.open(spec, info::InfoWindow { shared: shared.clone(), of })
        };
        if summary && items.len() > 1 {
            open(info::InfoOf::Summary(items));
        } else {
            for item in items {
                open(info::InfoOf::One(item));
            }
        }
    }

    /// Opens the Inspector, or closes it when it is open.
    pub fn toggle_inspector(&self) {
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let app = self.app();
        match self.inspector.get().filter(|k| app.windows().contains(k)) {
            Some(key) => {
                app.close(key);
                self.inspector.set(None);
            }
            None => {
                let of = info::InfoOf::Inspector;
                let spec = WindowSpec::new(of.title(), (300., 620.), (260., 320.));
                self.inspector.set(Some(app.open(spec, info::InfoWindow { shared, of })));
            }
        }
    }

    pub fn panel_memory(&self, app: &str) -> memory::PanelMemory {
        self.memory.borrow().panels.get(app).cloned().unwrap_or_default()
    }

    pub fn keep_panel(&self, app: &str, kept: memory::PanelMemory) {
        self.memory.borrow_mut().panels.insert(app.to_owned(), kept);
        self.save();
    }

    /// Shows a request's panel, or holds it while its parent has one open.
    pub fn choose(&self, ask: chooser::Ask) {
        let busy = !ask.parent.is_empty() && self.panels.borrow().values().any(|(_, p)| *p == ask.parent);
        if busy {
            self.waiting.borrow_mut().push(ask);
            return;
        }
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let sizes = panel::sizes();
        let save = ask.kind == chooser::Kind::Save && !self.panel_memory(&ask.app_id).expanded;
        let size = if save { sizes.save } else { sizes.open };
        let title = match (ask.title.is_empty(), ask.kind) {
            (false, _) => ask.title.clone(),
            (true, chooser::Kind::Open) => "Open".into(),
            (true, _) => "Save".into(),
        };
        // An X11 parent cannot be an xdg-foreign parent: its panel is a window of its own.
        let handle = ask.parent.strip_prefix("wayland:").map(str::to_owned);
        let spec = WindowSpec::new(title, size, sizes.min).parent(handle);
        let (id, parent) = (ask.id, ask.parent.clone());
        let key = self.app().open(spec, panel::PanelWindow { shared, ask });
        self.panels.borrow_mut().insert(id, (key, parent));
    }

    /// Answers a request once, closes its panel and shows the next its parent holds.
    pub fn answer_choose(&self, id: u64, answer: chooser::Answer) {
        let open = self.panels.borrow_mut().remove(&id);
        let held = {
            let mut waiting = self.waiting.borrow_mut();
            waiting.iter().position(|a| a.id == id).map(|i| waiting.remove(i))
        };
        if open.is_none() && held.is_none() {
            return;
        }
        let app = self.app();
        if let Some(bus) = app.bus() {
            app.worker().spawn(files1::answer(bus, id, answer));
        }
        if let Some((key, parent)) = open {
            app.close(key);
            let next = {
                let mut waiting = self.waiting.borrow_mut();
                waiting.iter().position(|a| a.parent == parent).map(|i| waiting.remove(i))
            };
            if let Some(next) = next {
                self.choose(next);
            }
        }
    }

    /// Opens Quick Look on `items` at `index`, or closes it when the same caller has it open.
    pub fn toggle_quick_look(&self, items: Vec<PathBuf>, index: usize, caller: quicklook::Caller) {
        if self.quick_look.borrow().is_some_and(|q| q.caller == caller) {
            self.close_quick_look();
            return;
        }
        let over = match caller {
            quicklook::Caller::Window(key) => self.app().export(key),
            quicklook::Caller::Bus => None,
        };
        self.show_quick_look(items, index, caller, over);
    }

    /// Shows `items` in Quick Look, in the window open or a new one, floating over `over`: an
    /// xdg-foreign handle, `wayland:` and one, or `isle:` and a shell surface's name.
    pub fn show_quick_look(&self, items: Vec<PathBuf>, index: usize, caller: quicklook::Caller, over: Option<String>) {
        self.show_quick_look_in(items, index, caller, over, "none");
    }

    /// As `show_quick_look`, in Markup or Trim when `mode` names it.
    pub fn show_quick_look_in(&self, items: Vec<PathBuf>, index: usize, caller: quicklook::Caller, over: Option<String>, mode: &str) {
        let (markup, trim) = (mode == "markup", mode == "trim");
        if items.is_empty() {
            return;
        }
        let open = *self.quick_look.borrow();
        if let Some(mut open) = open {
            open.items.set(items);
            open.index.set(index);
            open.marking.set(markup);
            open.trimming.set(trim);
            open.caller = caller;
            *self.quick_look.borrow_mut() = Some(open);
            return;
        }
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let (items, index) = (State::create_global(items), State::create_global(index));
        let (marking, trimming) = (State::create_global(markup), State::create_global(trim));
        let float = over.filter(|o| !o.is_empty()).map(|o| isle_app::Float {
            over: o.strip_prefix("wayland:").map(str::to_owned).unwrap_or(o),
            on_request: false,
            from: None,
        });
        let spec = WindowSpec::new("Quick Look", (900., 680.), (360., 280.)).lights((0., 10.)).float(float);
        let key = self.app().open(spec, quicklook::QuickLookWindow { shared, items, index, marking, trimming });
        *self.quick_look.borrow_mut() = Some(quicklook::Open { key, items, index, marking, trimming, caller });
        ql1::VISIBLE.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Follows the caller's selection while Quick Look is open from it.
    pub fn quick_look_follow(&self, caller: quicklook::Caller, items: Vec<PathBuf>, index: usize) {
        let open = *self.quick_look.borrow();
        let Some(mut open) = open.filter(|o| o.caller == caller) else { return };
        if items.is_empty() {
            return;
        }
        if *open.items.peek() != items {
            open.items.set(items);
        }
        if *open.index.peek() != index {
            open.index.set(index);
        }
    }

    pub fn close_quick_look(&self) {
        let open = *self.quick_look.borrow();
        if let Some(open) = open {
            self.app().close(open.key);
        }
    }

    /// The window has gone, however it closed: a caller over the bus hears which item it
    /// left on.
    pub fn quick_look_closed(&self) {
        ql1::VISIBLE.store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(open) = self.quick_look.borrow_mut().take()
            && open.caller == quicklook::Caller::Bus
        {
            self.tell_caller(*open.index.peek(), true);
        }
    }

    /// The item shown changed: a caller over the bus follows it.
    pub fn quick_look_at(&self, index: usize) {
        if self.quick_look.borrow().is_some_and(|o| o.caller == quicklook::Caller::Bus) {
            self.tell_caller(index, false);
        }
    }

    fn tell_caller(&self, index: usize, closed: bool) {
        let app = self.app();
        if let Some(bus) = app.bus() {
            app.worker().spawn(ql1::tell(bus, index as u32, closed));
        }
    }

    pub fn quick_look_open(&self) -> Option<quicklook::Open> {
        *self.quick_look.borrow()
    }

    pub fn set_default_shown(&self, shown: view::Shown) {
        self.memory.borrow_mut().default_view = shown;
        self.save();
    }

    fn save(&self) {
        let memory = self.memory.borrow().clone();
        std::thread::spawn(move || memory::save(&memory));
    }


    pub fn open_window(&self, tabs: Vec<(Place, Option<PathBuf>)>, current: usize, token: Option<String>) {
        self.open_window_with(tabs, current, token, false);
    }

    /// As `open_window`, its search field taking the keyboard when `find`.
    pub fn open_window_with(&self, tabs: Vec<(Place, Option<PathBuf>)>, current: usize, token: Option<String>, find: bool) {
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let app = self.app();
        let spec = WindowSpec::main(app.spec()).token(token);
        app.open(spec, FilesWindow { shared, tabs, current, find });
    }

    /// Asks Open With for a file in the front window, opening one at its folder if none is open.
    fn ask(&self, path: PathBuf, token: Option<String>) {
        let front = self.front.borrow().last().copied();
        match front.and_then(|k| self.windows.borrow().get(&k).copied().map(|l| (k, l))) {
            Some((key, mut link)) => {
                link.inbox.write().push(Open::OpenWith(path));
                if let Some(token) = token {
                    self.app().activate(key, &token);
                }
            }
            None => {
                let folder = path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                self.open_window(vec![(Place::Folder(folder), Some(path))], 0, token);
            }
        }
    }

    /// Shows a place in a new tab of the front window, or a new window when none is open.
    fn show(&self, place: Place, reveal: Option<PathBuf>, token: Option<String>) {
        let front = self.front.borrow().last().copied();
        let link = front.and_then(|k| self.windows.borrow().get(&k).copied());
        match (front, link) {
            (Some(key), Some(mut link)) => {
                link.inbox.write().push(Open::Tab(place, reveal));
                if let Some(token) = token {
                    self.app().activate(key, &token);
                }
            }
            _ => self.open_window(vec![(place, reveal)], 0, token),
        }
    }
}

thread_local! {
    pub(crate) static SHARED: RefCell<Option<Rc<Shared>>> = const { RefCell::new(None) };
}

/// A `file://` URI's path.
pub fn path_of(uri: &str) -> Option<PathBuf> {
    match uri.strip_prefix("file://") {
        Some(rest) => {
            // A host before the path names this machine or none.
            let path = rest.find('/').map(|at| &rest[at..])?;
            Some(PathBuf::from(places::percent_decode(path)))
        }
        None if uri.starts_with('/') => Some(PathBuf::from(uri)),
        None => None,
    }
}

struct Files {
    shared: Rc<Shared>,
}

impl Delegate for Files {
    fn started(&self, app: &AppHandle) {
        *self.shared.app.borrow_mut() = Some(app.clone());
        self.shared.locations.follow(app);
        if let Some(bus) = app.bus() {
            app.worker().spawn(files1::serve(bus.clone(), app.requester()));
            app.worker().spawn(ql1::serve(bus.clone(), app.requester()));
            app.worker().spawn(fm1::serve(bus, app.requester()));
        }
    }

    fn request(&self, app: &AppHandle, request: Request) {
        let first = app.windows().is_empty();
        if first {
            let memory = self.shared.memory.borrow().clone();
            let reopened: Vec<WindowMemory> = memory.windows.into_values().filter(|w| !w.tabs.is_empty()).collect();
            self.shared.memory.borrow_mut().windows.clear();
            let asked_for_more = !matches!(request, Request::Activate { .. });
            if !reopened.is_empty() && !asked_for_more {
                for window in reopened {
                    let tabs = window.tabs.into_iter().map(|p| (p, None)).collect();
                    self.shared.open_window(tabs, window.current, request.token().map(str::to_owned));
                }
                return;
            }
        }
        let token = request.token().map(str::to_owned);
        match request {
            Request::Activate { .. } => match self.shared.front.borrow().last().copied() {
                Some(key) => {
                    if let Some(token) = token {
                        app.activate(key, &token);
                    }
                }
                None => self.shared.open_window(vec![(self.shared.new_window_place(), None)], 0, token),
            },
            Request::Open { uris, .. } => {
                let (links, uris): (Vec<_>, Vec<_>) = uris.iter().partition(|u| u.starts_with("isle-settings://"));
                for at in links.iter().filter_map(|u| settings_window::At::from_uri(u)) {
                    self.shared.open_settings(Some(at), token.clone());
                }
                let (asks, uris): (Vec<_>, Vec<_>) = uris.into_iter().partition(|u| u.starts_with("isle-open-with:"));
                for path in asks.iter().filter_map(|u| path_of(u.trim_start_matches("isle-open-with:"))) {
                    self.shared.ask(path, token.clone());
                }
                for path in uris.iter().filter_map(|u| path_of(u)) {
                    if path.is_dir() {
                        self.shared.show(Place::Folder(path), None, token.clone());
                    } else if let Some(parent) = path.parent() {
                        self.shared.show(Place::Folder(parent.to_owned()), Some(path.clone()), token.clone());
                    }
                }
            }
            Request::Action { name, params, .. } => match name.as_str() {
                isle_app::SERVICE => {}
                // shortcuts.files: a new window with its search field focused, as Finder's
                // search window.
                "find" => self.shared.open_window_with(vec![(self.shared.new_window_place(), None)], 0, token.clone(), true),
                ql1::SHOW => {
                    let Some(show) = params.first().and_then(|j| serde_json::from_str::<ql1::Show>(j).ok()) else { return };
                    let items: Vec<PathBuf> = show.uris.iter().filter_map(|u| path_of(u)).collect();
                    self.shared.show_quick_look_in(items, show.index as usize, quicklook::Caller::Bus, Some(show.caller), &show.mode);
                }
                ql1::SELECT => {
                    if let Some(open) = self.shared.quick_look_open().filter(|o| o.caller == quicklook::Caller::Bus)
                        && let Some(i) = params.first().and_then(|p| p.parse::<usize>().ok())
                    {
                        let mut index = open.index;
                        index.set(i);
                    }
                }
                ql1::CLOSE => self.shared.close_quick_look(),
                files1::CHOOSE => {
                    if let Some(ask) = params.first().and_then(|j| serde_json::from_str::<chooser::Ask>(j).ok()) {
                        self.shared.choose(ask);
                    }
                }
                files1::CANCEL => {
                    if let Some(id) = params.first().and_then(|p| p.parse().ok()) {
                        self.shared.answer_choose(id, chooser::Answer::cancelled());
                    }
                }
                fm1::SHOW_ITEMS => {
                    for path in params.iter().filter_map(|u| path_of(u)) {
                        let parent = path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                        self.shared.show(Place::Folder(parent), Some(path), token.clone());
                    }
                }
                fm1::SHOW_FOLDERS => {
                    for path in params.iter().filter_map(|u| path_of(u)) {
                        self.shared.show(Place::Folder(path), None, token.clone());
                    }
                }
                // Spotlight's "Show all in Files": the query and a folder's URI, or anything
                // else for Home.
                "search" => {
                    let Some(term) = params.first().filter(|t| !t.trim().is_empty()) else { return };
                    let folder = params.get(1).and_then(|s| path_of(s)).filter(|p| p.is_dir());
                    let search = search::Search {
                        term: term.trim().to_owned(),
                        scope: if folder.is_some() { search::Scope::Under } else { search::Scope::Home },
                        folder,
                        criteria: Vec::new(),
                        name: None,
                    };
                    self.shared.show(Place::Search(Box::new(search)), None, token.clone());
                }
                fm1::SHOW_PROPERTIES => {
                    let items = params.iter().filter_map(|u| path_of(u)).collect();
                    self.shared.get_info(items, false, token.clone());
                }
                other => info!(action = other, "not built yet"),
            },
        }
    }
}

/// Files serves the Isle desktop alone; elsewhere it takes no name and leaves.
fn in_isle() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|d| d.split(':').any(|d| d.eq_ignore_ascii_case("Isle")))
}

/// Erases what has been in any Trash for more than thirty days (Advanced's setting).
fn purge_trash() {
    let cutoff = jiff::Zoned::now().datetime() - jiff::Span::new().days(30);
    for trash in isle_fs::trash::all_trashes() {
        for item in isle_fs::trash::list(&trash) {
            let old = item.deleted.parse::<jiff::civil::DateTime>().is_ok_and(|d| d < cutoff);
            if old && let Err(err) = isle_fs::trash::erase(&item) {
                tracing::warn!("{} is not erased: {err}", item.file.display());
            }
        }
    }
}

fn main() {
    // A maintainer's command, writing into the checkout it was built from.
    #[cfg(debug_assertions)]
    if std::env::args().nth(1).as_deref() == Some("--write-index") {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/files/settings-index.kdl");
        std::fs::write(path, prefs::index()).expect("data/files/settings-index.kdl");
        return;
    }
    isle_app::init_logging(module_path!());
    if !in_isle() {
        info!("not an Isle session; Files is Isle's");
        return;
    }
    let memory = memory::load();
    let shared = Rc::new(Shared {
        app: RefCell::new(None),
        windows: RefCell::default(),
        front: RefCell::default(),
        saved: State::create_global(memory.saved.clone()),
        selection: State::create_global(Vec::new()),
        info_collapsed: State::create_global(memory.info_collapsed.clone()),
        inspector: Cell::new(None),
        prefs: RefCell::new(prefs::Prefs::default()),
        settings: Cell::new(None),
        settings_at: State::create_global(settings_window::At { pane: "general".into(), row: None }),
        last_scope: Cell::new(None),
        purged: Cell::new(false),
        panels: RefCell::default(),
        waiting: RefCell::default(),
        quick_look: RefCell::default(),
        memory: RefCell::new(memory),
        next_tab: Cell::new(0),
        hidden: State::create_global(false),
        jobs: jobs::Jobs::new(),
        cut: State::create_global(Vec::new()),
        locations: volumes::Locations::new(),
    });
    SHARED.with(|s| *s.borrow_mut() = Some(shared.clone()));
    isle_app::run_with(SPEC, Files { shared });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_name_paths() {
        assert_eq!(path_of("file:///home/me/My%20Doc.md"), Some(PathBuf::from("/home/me/My Doc.md")));
        assert_eq!(path_of("file://localhost/etc/hosts"), Some(PathBuf::from("/etc/hosts")));
        assert_eq!(path_of("/tmp"), Some(PathBuf::from("/tmp")));
        assert_eq!(path_of("https://example.org"), None);
    }
}
