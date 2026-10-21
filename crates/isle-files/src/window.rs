//! A Files window: the sidebar of places, the toolbar, the tab bar and the folder shown
//! (`p1/structure/files.md` § Content; `p1/surfaces/files.md` § Anatomy).

use std::{path::PathBuf, rc::Rc};

use freya::prelude::*;
use isle_app::{Chord, ChordKey, Command, Desktop, Menu, ToolbarExt, use_commands};
use isle_ui::{Glyph, colour, text, use_theme};

use crate::{
    Shared, WindowLink,
    format,
    list_view::{Held, ListView, Pick},
    places::{self, Place, Places, Row},
    tab::{self, Live, Status, Tab},
};

/// What another part of the app asks a window to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    /// A tab at the place, the item revealed in it.
    Tab(Place, Option<PathBuf>),
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
    TabHandle {
        tab: State::create(t),
        live: Live::default(),
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

        let (back, forward, place) = {
            let t = tab.read();
            (!t.back.is_empty(), !t.forward.is_empty(), t.place.clone())
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
        let commands = vec![
            Command::new("new-window", "New Window", Menu::File, 0).chord(Chord::cmd('n')),
            Command::new("new-tab", "New Tab", Menu::File, 0).chord(Chord::cmd('t')),
            Command::new("open", "Open", Menu::File, 1)
                .chord(Chord::cmd('o'))
                .enabled(has_selection),
            Command::new("close-tab", "Close Tab", Menu::File, 2).enabled(tabs.read().len() > 1),
            Command::new("deselect", "Deselect All", Menu::Edit, 2)
                .chord(Chord::cmd('a').option())
                .enabled(has_selection),
            Command::new("as-list", "as List", Menu::View, 0)
                .chord(Chord::cmd('2'))
                .checked(true),
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
        let go = {
            let live = live.clone();
            move |place: Place| tab::go(tab, &live, place, None)
        };
        let open_new_tab = {
            let shared = shared.clone();
            move |place: Place| {
                let (mut tabs, mut current) = (tabs, current);
                let handle = new_tab(&shared, place, None);
                tab::load(handle.tab, &handle.live, false);
                let n = {
                    let mut list = tabs.write();
                    list.push(handle);
                    list.len() - 1
                };
                current.set(n);
                shared.remember(key, &tabs.peek(), n);
            }
        };
        let open_items = {
            let (go, open_new_tab) = (go.clone(), open_new_tab.clone());
            move |paths: Vec<PathBuf>, in_tabs: bool| {
                let entries: Vec<_> = {
                    let t = tab.peek();
                    paths.iter().filter_map(|p| t.folder.get(p).cloned()).collect()
                };
                let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir()).collect();
                match (dirs.as_slice(), in_tabs) {
                    ([one], false) => go(Place::Folder(one.path.clone())),
                    (many, _) => {
                        for dir in many {
                            open_new_tab(Place::Folder(dir.path.clone()));
                        }
                    }
                }
                let files: Vec<_> = entries.iter().filter(|e| !e.is_dir()).collect();
                if !files.is_empty() {
                    tracing::info!(n = files.len(), "opening files comes with 15.8");
                }
            }
        };

        use_side_effect({
            let (go, open_new_tab, open_items) = (go.clone(), open_new_tab.clone(), open_items.clone());
            let live = live.clone();
            let shared = shared.clone();
            move || {
                let chosen: Vec<String> = desktop.commands.read().clone();
                if chosen.is_empty() {
                    return;
                }
                let mut queue = desktop.commands;
                queue.write().clear();
                for id in chosen {
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
                        "new-tab" => open_new_tab(shared.new_window_place()),
                        "close-tab" => close_tab(tabs, current, *current.peek()),
                        "open" | "open-down" => {
                            let selection = tab.peek().selection.clone();
                            open_items(selection, false);
                        }
                        "deselect" => tab.write().deselect(),
                        "toggle-sidebar" => sidebar.toggle(),
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
        let on_pick = {
            let open_items = open_items.clone();
            move |pick: Pick| {
                let mut tab = tab;
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
                    open_items(vec![path], held.cmd);
                }
                }
            }
        };
        let on_key = move |e: Event<KeyboardEventData>| {
            let m = e.modifiers;
            held.set_if_modified(Held {
                cmd: m.contains(Modifiers::META),
                shift: m.contains(Modifiers::SHIFT),
                option: m.contains(Modifiers::ALT),
            });
            if m.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::ALT) {
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

        let body = content(&theme, &handle, rev, focused, held, on_pick);
        let count = tab.read().folder.len();
        let title = place.title();

        rect()
            .expanded()
            .horizontal()
            .content(Content::flex())
            .on_global_key_down(on_key)
            .on_global_key_up(on_key_up)
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
                    on_go: EventHandler::new({
                        let go = go.clone();
                        move |place: Place| go(place)
                    }),
                }
            }))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .content(Content::flex())
                    .background(colour(t.colour_view))
                    .child(toolbar(&theme, &title, count, back, forward, *sidebar.read(), {
                        let live = live.clone();
                        move |action: Nav| match action {
                            Nav::Back => tab::back(tab, &live),
                            Nav::Forward => tab::forward(tab, &live),
                            Nav::Sidebar => {
                                let mut sidebar = sidebar;
                                sidebar.toggle();
                            }
                        }
                    }))
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
                                let (open_new_tab, shared) = (open_new_tab.clone(), shared.clone());
                                move |_| open_new_tab(shared.new_window_place())
                            }),
                        }
                    }))
                    .child(rect().width(Size::fill()).height(Size::flex(1.)).child(body)),
            )
            .into_element()
    }
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

#[derive(Clone, Copy)]
enum Nav {
    Back,
    Forward,
    Sidebar,
}

/// The folder's content, or why there is none.
fn content(
    theme: &isle_tokens::Theme,
    handle: &TabHandle,
    rev: u64,
    focused: bool,
    held: State<Held>,
    on_pick: impl Fn(Pick) + 'static,
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
        None => ListView {
            tab: handle.tab,
            rev,
            focused,
            held,
            on_pick: EventHandler::new(on_pick),
            controller: handle.scroll,
        }
        .into_element(),
    }
}

fn icon_button(
    theme: &isle_tokens::Theme,
    glyph: &'static str,
    alt: &'static str,
    enabled: bool,
    on_press: impl Fn() + 'static,
) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(t.size_control_large))
        .height(Size::px(t.size_control_large))
        .center()
        .corner_radius(t.radius_control)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(alt)
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            if enabled {
                on_press();
            }
        })
        .child(Glyph::new(
            glyph,
            t.size_glyph_large,
            colour(if enabled { t.colour_text_2 } else { t.colour_text_disabled }),
        ))
        .into_element()
}

fn toolbar(
    theme: &isle_tokens::Theme,
    title: &str,
    count: usize,
    back: bool,
    forward: bool,
    sidebar: bool,
    on_nav: impl Fn(Nav) + Clone + 'static,
) -> Element {
    let t = &theme.tokens;
    let (a, b, c) = (on_nav.clone(), on_nav.clone(), on_nav);
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_toolbar))
        .padding((0., t.space_l))
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t.space_xs)
        .background(colour(t.colour_window))
        .border(
            Border::new()
                .fill(colour(t.colour_hairline))
                .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                .alignment(BorderAlignment::Inner),
        )
        .toolbar()
        .maybe_child((!sidebar).then(|| {
            rect()
                .padding((0., 0., 0., t.space_xxxl + t.space_xl))
                .child(icon_button(theme, "sidebar-simple", "Show Sidebar", true, move || c(Nav::Sidebar)))
        }))
        .child(icon_button(theme, "caret-left", "Back", back, move || a(Nav::Back)))
        .child(icon_button(theme, "caret-right", "Forward", forward, move || b(Nav::Forward)))
        .child(
            rect()
                .padding((0., 0., 0., t.space_s))
                .child(text::line(theme, text::Style::Headline, colour(t.colour_text), title.to_owned()))
                .child(text::line(theme, text::Style::Subheadline, colour(t.colour_text_2), format::items(count))),
        )
        .into_element()
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
    on_go: EventHandler<Place>,
}

impl Component for Sidebar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let row = |r: &Row| {
            let selected = r.place == self.shown;
            let on_go = self.on_go.clone();
            let place = r.place.clone();
            let ink = if selected { t.colour_on_accent } else { t.colour_text };
            let glyph = if selected { t.colour_on_accent } else { t.colour_accent_text };
            rect()
                .key(format!("{:?}", r.place))
                .width(Size::fill())
                .height(Size::px(t.size_row))
                .padding((0., t.space_s))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .corner_radius(t.radius_row)
                .maybe(selected, |el| el.background(colour(t.colour_accent)))
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(r.label.clone())
                .a11y_builder(move |node| node.set_selected(selected))
                .on_press(move |_| on_go.call(place.clone()))
                .child(Glyph::new(r.glyph, t.size_glyph, colour(glyph)))
                .child(text::line(&theme, text::Style::Body, colour(ink), r.label.clone()))
                .into_element()
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
                            .child(row(&recents))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Favourites"))
                            .children(self.places.favourites.iter().map(row))
                            .child(rect().height(Size::px(t.space_m)))
                            .child(header("Locations"))
                            .children(self.places.locations.iter().map(row)),
                    ),
            )
    }
}
