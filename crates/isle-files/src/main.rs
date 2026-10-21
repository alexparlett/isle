//! Files (`docs/design/apps.md` Files; `p1/structure/files.md`).

mod bars;
mod columns_view;
mod fm1;
mod format;
mod icons;
mod icons_view;
mod jobs;
mod list_view;
mod sheets;
mod preview;
mod toolbar;
mod memory;
mod places;
mod tab;
mod thumbnail;
mod thumbs;
mod view;
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

    /// Where a new window or tab opens (`files.md` § Tabs and windows): Home.
    pub fn new_window_place(&self) -> Place {
        Place::Folder(places::home().unwrap_or_else(|| PathBuf::from("/")))
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

    pub fn set_default_shown(&self, shown: view::Shown) {
        self.memory.borrow_mut().default_view = shown;
        self.save();
    }

    fn save(&self) {
        let memory = self.memory.borrow().clone();
        std::thread::spawn(move || memory::save(&memory));
    }


    pub fn open_window(&self, tabs: Vec<(Place, Option<PathBuf>)>, current: usize, token: Option<String>) {
        let Some(shared) = SHARED.with(|s| s.borrow().clone()) else { return };
        let app = self.app();
        let spec = WindowSpec::main(app.spec()).token(token);
        app.open(spec, FilesWindow { shared, tabs, current });
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
        if let Some(bus) = app.bus() {
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
                for path in uris.iter().filter_map(|u| path_of(u)) {
                    if path.is_dir() {
                        self.shared.show(Place::Folder(path), None, token.clone());
                    } else if let Some(parent) = path.parent() {
                        self.shared.show(Place::Folder(parent.to_owned()), Some(path.clone()), token.clone());
                    }
                }
            }
            Request::Action { name, params, .. } => match name.as_str() {
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

fn main() {
    isle_app::init_logging(module_path!());
    if !in_isle() {
        info!("not an Isle session; Files is Isle's");
        return;
    }
    let shared = Rc::new(Shared {
        app: RefCell::new(None),
        windows: RefCell::default(),
        front: RefCell::default(),
        memory: RefCell::new(memory::load()),
        next_tab: Cell::new(0),
        hidden: State::create_global(false),
        jobs: jobs::Jobs::new(),
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
