//! Spotlight on the loop: the model, fed keys from the chord's press; the fast sources run in
//! the keystroke's frame and the slow ones on the worker; what a choice does once focus has gone
//! back (`p10-spotlight.md` choices 1–4).

use std::{
    collections::BTreeMap,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use isle_keys::keyinfo::Role;
use isle_search::{Context, Group, Icon, Item, Matcher, Query, Row, Scope, Source, Usage};
use isle_shell::spotlight::{
    Frame, SpotlightAction, SpotlightSurface,
    model::{Effect, How, Key, Line, Spotlight, View},
    surface::{Layout, LineView, RowView, Slot},
};
use isle_state::{Loaded, StateDir};
use smithay::{
    input::keyboard::{Keycode, keysyms},
    reexports::calloop::RegistrationToken,
    utils::Rectangle,
};
use tracing::{debug, info, warn};

use crate::{
    focus::Focus,
    state::{Isle, State},
    state_writer::Saver,
};

/// The sources a search runs, built when Spotlight opens from what the compositor knows then.
#[derive(Default)]
pub struct Sources {
    /// Apps apart, as favourites and the category filter change while Spotlight is open.
    pub apps: isle_search::sources::apps::AppsSource,
    /// Clipboard history apart, as pins and deletions change it while open.
    pub clipboard: isle_search::sources::clipboard::ClipboardSource,
    pub fast: Vec<Box<dyn Source>>,
    pub slow: Vec<Arc<dyn Source>>,
}

/// What a row's or tile's menu item does.
#[derive(Clone, Debug)]
enum MenuCommand {
    Choose(Row),
    EntryAction {
        app: String,
        action: String,
    },
    AddFavourite(String),
    RemoveFavourite(String),
    KeepRunning(String, bool),
    ShowInLauncher(String),
    /// An emoji's variant, put in as the emoji is.
    Insert(String),
    Quit(String),
    Copy(Row),
    Category(Option<String>),
    Pin(u64),
    Delete(u64),
    /// Asks before clearing history.
    ClearAsk,
    Clear,
}

struct OpenMenu {
    view: isle_shell::spotlight::surface::MenuView,
    commands: std::collections::HashMap<u64, MenuCommand>,
}

pub struct SpotlightState {
    pub surface: Rc<SpotlightSurface>,
    pub model: Option<Spotlight>,
    sources: Sources,
    matcher: Matcher,
    /// The newest query's generation, which slow sources on the worker check.
    latest: Arc<AtomicU64>,
    /// The general query kept from the last time.
    kept: Option<String>,
    /// The window that had focus when Spotlight opened: where a pick is pasted, whose app's
    /// menus are searched.
    pub previous: Option<u64>,
    /// The key held for repeat, and its timer.
    repeat: Option<(u32, RegistrationToken)>,
    pub usage: Usage,
    usage_saver: Option<Saver<Usage>>,
    commands: isle_state::domains::Commands,
    commands_saver: Option<Saver<isle_state::domains::Commands>>,
    /// The Apps view's panel on the screen Spotlight opened on.
    apps_panel: (f32, f32),
    menu: Option<OpenMenu>,
    /// A clipboard item shown whole over the list.
    preview: Option<isle_shell::spotlight::surface::Preview>,
    /// The tile a press went down on, which a release elsewhere drops.
    drag_from: Option<String>,
    /// How long each search's fast sources took, in microseconds, newest last.
    pub search_us: Vec<u64>,
    /// Spotlight's surface in the global space, which a drag of a row leaves to begin.
    rect: Rectangle<f64, smithay::utils::Logical>,
}

impl SpotlightState {
    pub fn new() -> Self {
        Self {
            surface: SpotlightSurface::new(),
            model: None,
            sources: Sources::default(),
            matcher: Matcher::new(),
            latest: Arc::new(AtomicU64::new(0)),
            kept: None,
            previous: None,
            repeat: None,
            usage: Usage::default(),
            usage_saver: None,
            commands: Default::default(),
            commands_saver: None,
            apps_panel: (0., 0.),
            menu: None,
            preview: None,
            drag_from: None,
            rect: Rectangle::default(),
            search_us: Vec::new(),
        }
    }

    pub fn open(&self) -> bool {
        self.model.is_some()
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The field's placeholder in each scope (`structure/spotlight.md` Copy).
fn placeholder(scope: &Scope) -> &'static str {
    match scope {
        Scope::General | Scope::App(_) => "Search",
        Scope::Apps => "Search apps",
        Scope::Files { .. } => "Search files",
        Scope::Actions => "Search actions",
        Scope::Clipboard => "Search clipboard",
        Scope::Calculator => "Calculate",
        Scope::Emoji => "Search emoji",
        Scope::Windows => "Search windows",
        Scope::Run => "Run a command",
    }
}

fn token_name(scope: &Scope) -> Option<&'static str> {
    Some(match scope {
        Scope::General | Scope::App(_) => return None,
        Scope::Apps => "Apps",
        Scope::Files { .. } => "Files",
        Scope::Actions => "Actions",
        Scope::Clipboard => "Clipboard",
        Scope::Calculator => "Calculator",
        Scope::Emoji => "Emoji",
        Scope::Windows => "Windows",
        Scope::Run => "Run",
    })
}

/// What an empty scope says it holds none of.
fn empty_line(scope: &Scope, query: &str) -> String {
    if !query.is_empty() {
        return format!("No results for “{query}”");
    }
    match scope {
        Scope::Clipboard => "No copied items",
        Scope::Windows => "No open windows",
        _ => "No results",
    }
    .to_owned()
}

impl Isle {
    pub fn load_usage(&mut self, runtime: &tokio::runtime::Handle) {
        let dir = StateDir::new(StateDir::default_dir());
        let (usage, loaded) = dir.load::<Usage>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "Spotlight's use");
        }
        self.spotlight.usage = usage;
        self.spotlight.usage_saver = Some(Saver::spawn(runtime, dir.clone()));
        self.spotlight.commands = dir.load().0;
        self.spotlight.commands_saver = Some(Saver::spawn(runtime, dir));
    }

    /// Counts a choice of `key` for ranking.
    pub fn record_use(&mut self, key: &str) {
        isle_search::usage::record(&mut self.spotlight.usage, key, now());
        if let Some(saver) = &self.spotlight.usage_saver {
            saver.save(self.spotlight.usage.clone());
        }
    }

    /// The model as JSON, for the tests.
    pub fn spotlight_status(&self) -> String {
        let Some(model) = &self.spotlight.model else {
            return serde_json::json!({ "open": false, "kept": self.spotlight.kept }).to_string();
        };
        let lines: Vec<serde_json::Value> = model
            .lines()
            .iter()
            .map(|l| match l {
                Line::Header(h) => serde_json::json!({ "header": h }),
                Line::Row(r) => serde_json::json!({
                    "key": r.key, "title": r.title, "detail": r.detail, "trailing": r.trailing,
                    "group": format!("{:?}", r.group),
                }),
            })
            .collect();
        fn labels(entries: &[isle_ui::MenuEntry], out: &mut Vec<String>) {
            for item in entries.iter().filter_map(isle_ui::MenuEntry::item) {
                out.push(item.label.clone());
                if let Some(sub) = &item.submenu {
                    labels(sub, out);
                }
            }
        }
        let mut menu = Vec::new();
        if let Some(m) = &self.spotlight.menu {
            labels(&m.view.entries, &mut menu);
        }
        let badges: serde_json::Map<String, serde_json::Value> = self
            .spotlight_frame()
            .map(|f| {
                f.lines
                    .into_iter()
                    .filter_map(|l| match l {
                        isle_shell::spotlight::surface::LineView::Row(r) => {
                            r.badge.map(|b| (r.key, b.into()))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::json!({
            "open": true,
            "menu": self.spotlight.menu.as_ref().map(|_| menu),
            "badges": badges,
            "search_us": self.spotlight.search_us,
            "category": self.spotlight.sources.apps.category,
            "text": model.field().text,
            "caret": model.field().caret,
            "scope": format!("{:?}", model.scope()),
            "selected": model.selected(),
            "searching": model.searching(),
            "panel": model.panel(),
            "lines": lines,
        })
        .to_string()
    }

    fn row_slot(&self, icon: &Icon) -> Slot {
        match icon {
            Icon::App(app) => self
                .app_icon(app, self.shell.theme().tokens.size_app_icon_large as u32)
                .map_or(Slot::Glyph("app-window"), |p| {
                    if self.games.for_app(app).is_some() {
                        Slot::Picture(p)
                    } else {
                        Slot::Icon(p)
                    }
                }),
            Icon::Path(p) => Slot::Picture(p.clone()),
            Icon::Glyph(g) => Slot::Glyph(g),
            Icon::Text(t) => Slot::Text(t.clone()),
            Icon::None => Slot::None,
        }
    }

    /// The frame the surface draws, from the model.
    fn spotlight_frame(&self) -> Option<Frame> {
        let model = self.spotlight.model.as_ref()?;
        let mut lines = Vec::new();
        let mut strong = false;
        for line in model.lines() {
            match line {
                Line::Header(h) => {
                    strong = h == "Top hit" || h == "Calculation";
                    lines.push(LineView::Header(h));
                }
                Line::Row(r) => {
                    lines.push(LineView::Row(RowView {
                        slot: self.row_slot(&r.icon),
                        badge: match &r.item {
                            Item::App(app) => self
                                .badges
                                .get(app)
                                .filter(|b| b.count_visible)
                                .map(|b| b.count),
                            _ => None,
                        },
                        key: r.key,
                        title: r.title,
                        detail: r.detail,
                        trailing: r.trailing,
                        strong,
                    }));
                    strong = false;
                }
            }
        }
        let scope = model.scope();
        let token = match scope {
            Scope::App(app) => Some((self.app_name(app), self.app_icon(app, 16))),
            other => token_name(other).map(|n| (n.to_owned(), None)),
        };
        let has_rows = lines.iter().any(|l| matches!(l, LineView::Row(_)));
        let mac = self.keyboards.last_profile() == isle_keys::profile::Profile::Mac;
        let chord = |n: u8| {
            if mac {
                format!("⌘{n}")
            } else {
                format!("Cmd+{n}")
            }
        };
        let keyboard_focus = self.seat.get_keyboard().and_then(|k| k.current_focus());
        let ours = self.shell.keyboard_taker("spotlight").map(Focus::Shell);
        Some(Frame {
            text: model.field().text.clone(),
            caret: model.field().caret,
            all: model.field().all,
            placeholder: placeholder(scope).to_owned(),
            token,
            quick: match scope {
                Scope::Apps => Some(1),
                Scope::Files { .. } => Some(2),
                Scope::Actions => Some(3),
                Scope::Clipboard => Some(4),
                _ => None,
            },
            panel: model.panel() || (scope != &Scope::General && !model.searching()),
            layout: match model.view() {
                View::Grid(columns) if *scope == Scope::Apps => Layout::Apps(columns),
                View::Grid(_) => {
                    Layout::Emoji(self.shell.theme().tokens.limit_apps_columns as usize * 2)
                }
                View::List => Layout::List,
            },
            files: match scope {
                Scope::Files { contents } => Some(*contents),
                _ => None,
            },
            empty: (!has_rows && !model.searching())
                .then(|| empty_line(scope, &model.field().text)),
            unfocused: ours.is_some() && keyboard_focus != ours,
            solid: self.game_mode_on(),
            quick_chords: [chord(1), chord(2), chord(3), chord(4)],
            apps_panel: self.spotlight.apps_panel,
            apps_title: match &self.spotlight.sources.apps.category {
                Some(c) => c.clone(),
                None if model.field().text.is_empty()
                    && !self.spotlight.sources.apps.favourites.is_empty() =>
                {
                    "Favourites".into()
                }
                None => "All apps".into(),
            },
            filter: self
                .spotlight
                .sources
                .apps
                .category
                .clone()
                .unwrap_or_else(|| "All apps".into()),
            menu: self.spotlight.menu.as_ref().map(|m| m.view.clone()),
            preview: self.spotlight.preview.clone(),
            dragging: self.spotlight.drag_from.clone(),
            lines,
            selected: model.selected().map(str::to_owned),
            searching: model.searching(),
        })
    }
}

impl State {
    /// Cmd+Space, Ctrl+Cmd+Space and the Launchpad key: Spotlight in `scope` on the main
    /// screen, or closed when it is already open in that way.
    pub fn toggle_spotlight(&mut self, scope: Scope) {
        if let Some(model) = &self.isle.spotlight.model {
            if scope == Scope::General || model.scope() == &scope {
                return self.close_spotlight();
            }
            let effects = self.isle.spotlight.model.as_mut().unwrap().set_scope(scope);
            return self.spotlight_effects(effects);
        }
        self.open_spotlight(scope);
    }

    pub fn open_spotlight(&mut self, scope: Scope) {
        if self.isle.lock.locked || self.isle.spotlight.open() || self.isle.prompts_hold_keyboard()
        {
            return;
        }
        let Some(output) = self.isle.main_output() else {
            return;
        };
        self.cancel_switcher();
        self.close_session_dialog();
        self.close_force_quit();
        self.close_menu_bar_group();
        self.isle.spotlight.previous = self.focused_window();
        self.isle.spotlight.sources = self.spotlight_sources();
        let columns = self.isle.shell.theme().tokens.limit_apps_columns as usize;
        let (model, effects) = Spotlight::open(scope, self.isle.spotlight.kept.clone(), columns);
        self.isle.spotlight.model = Some(model);
        info!(previous = ?self.isle.spotlight.previous, "spotlight");

        let theme = self.isle.shell.theme().clone();
        let screen = self.isle.space.output_geometry(&output).unwrap_or_default();
        let t = &theme.tokens;
        let top = (screen.size.h as f32 * t.size_spotlight_top_scale).round();
        let size = SpotlightSurface::size_for(&theme, columns, top, screen.size.h as f32);
        let margin = SpotlightSurface::margin(&theme);
        let geometry = Rectangle::new(
            ((screen.size.w - size.w) / 2, (top - margin).round() as i32).into(),
            size,
        );
        self.isle.spotlight.surface.set_size(size);
        self.isle.spotlight.rect =
            Rectangle::new(screen.loc + geometry.loc, geometry.size).to_f64();
        self.isle.spotlight.apps_panel =
            SpotlightSurface::apps_panel(&theme, columns, top, screen.size.h as f32);
        self.spotlight_effects(effects);
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        self.isle.shell.open_placed("spotlight", &place, geometry);
        if let Some(focus) = self.isle.shell.keyboard_taker("spotlight") {
            let keyboard = self.isle.seat.get_keyboard().unwrap();
            let previous = keyboard.current_focus();
            self.isle
                .shell_keyboard_return
                .push((focus.clone(), previous));
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.focus_keyboard(Some(Focus::Shell(focus)), serial);
        }
        self.put_spotlight_frame();
        self.dispatch_shell();
        self.isle.queue_redraw_all();
    }

    pub fn close_spotlight(&mut self) {
        let Some(model) = self.isle.spotlight.model.take() else {
            return;
        };
        self.isle.spotlight.kept = model.keep();
        self.isle.spotlight.sources = Sources::default();
        self.isle.spotlight.menu = None;
        self.isle.spotlight.preview = None;
        self.isle.spotlight.drag_from = None;
        self.isle.spotlight.latest.fetch_add(1, Ordering::Relaxed);
        self.stop_spotlight_repeat();
        self.close_surface("spotlight");
    }

    fn put_spotlight_frame(&mut self) {
        if let Some(frame) = self.isle.spotlight_frame() {
            self.isle.spotlight.surface.set(frame);
            self.dispatch_shell();
        }
    }

    /// Whether keys go to Spotlight: it is open and holds the keyboard, which a prompt over
    /// it takes away.
    fn spotlight_has_keys(&self) -> bool {
        if !self.isle.spotlight.open() {
            return false;
        }
        let ours = self
            .isle
            .shell
            .keyboard_taker("spotlight")
            .map(Focus::Shell);
        ours.is_some()
            && self
                .isle
                .seat
                .get_keyboard()
                .and_then(|k| k.current_focus())
                == ours
    }

    /// A key pressed while Spotlight holds the keyboard; false for the keys it leaves to the
    /// desktop: modifiers, its own chords and every chord it does not use.
    pub fn spotlight_key(&mut self, code: u32) -> bool {
        if !self.spotlight_has_keys() {
            return false;
        }
        let (role, sym) = self.isle.keyboards.with_engine(false, &[], |_, ctx| {
            (ctx.keys.role(code), ctx.keys.sym(code))
        });
        if !matches!(role, Role::Key) {
            return false;
        }
        let mods = self.held_mods();
        let raw = sym.map_or(0, |s| s.raw());
        let key = if mods.ctrl {
            None
        } else if mods.cmd {
            match raw {
                keysyms::KEY_1..=keysyms::KEY_4 => Some(Key::Quick((raw - keysyms::KEY_0) as u8)),
                keysyms::KEY_k => Some(Key::Menu),
                keysyms::KEY_y => Some(Key::Preview),
                keysyms::KEY_c => Some(Key::Copy),
                keysyms::KEY_a => Some(Key::SelectAll),
                keysyms::KEY_p => Some(Key::Pin),
                keysyms::KEY_v => {
                    self.spotlight_paste();
                    return true;
                }
                keysyms::KEY_BackSpace | keysyms::KEY_Delete => Some(Key::DeleteItem),
                keysyms::KEY_Return | keysyms::KEY_KP_Enter => Some(Key::CmdReturn),
                keysyms::KEY_Left if mods.option => Some(Key::MoveFavourite(-1)),
                keysyms::KEY_Right if mods.option => Some(Key::MoveFavourite(1)),
                keysyms::KEY_Left => Some(Key::Home),
                keysyms::KEY_Right => Some(Key::End),
                keysyms::KEY_Up => Some(Key::GroupUp),
                keysyms::KEY_Down => Some(Key::GroupDown),
                _ => None,
            }
        } else {
            match raw {
                keysyms::KEY_BackSpace => Some(Key::Backspace),
                keysyms::KEY_Delete => Some(Key::Delete),
                keysyms::KEY_Left if mods.option => Some(Key::WordLeft),
                keysyms::KEY_Right if mods.option => Some(Key::WordRight),
                keysyms::KEY_Left => Some(Key::Left),
                keysyms::KEY_Right => Some(Key::Right),
                keysyms::KEY_Up => Some(Key::Up),
                keysyms::KEY_Down => Some(Key::Down),
                keysyms::KEY_Home => Some(Key::Home),
                keysyms::KEY_End => Some(Key::End),
                keysyms::KEY_Page_Up => Some(Key::PageUp),
                keysyms::KEY_Page_Down => Some(Key::PageDown),
                keysyms::KEY_Return | keysyms::KEY_KP_Enter if mods.shift => Some(Key::ShiftReturn),
                keysyms::KEY_Return | keysyms::KEY_KP_Enter => Some(Key::Return),
                keysyms::KEY_Escape => Some(Key::Escape),
                keysyms::KEY_Tab => Some(Key::Tab),
                _ => self.typed_text(code).map(Key::Text),
            }
        };
        let Some(key) = key else {
            return false;
        };
        if self.isle.spotlight.menu.is_some() {
            return false;
        }
        if self.isle.spotlight.preview.is_some() {
            if matches!(key, Key::Escape | Key::Preview) {
                self.isle.spotlight.preview = None;
                self.put_spotlight_frame();
            }
            return true;
        }
        let repeats = matches!(
            key,
            Key::Text(_)
                | Key::Backspace
                | Key::Delete
                | Key::Left
                | Key::Right
                | Key::Up
                | Key::Down
                | Key::WordLeft
                | Key::WordRight
        );
        self.spotlight_apply_key(key.clone());
        if repeats {
            self.start_spotlight_repeat(code, key);
        }
        true
    }

    /// The text `code` types under the seat's layout and modifiers, if any.
    fn typed_text(&mut self, code: u32) -> Option<String> {
        let keyboard = self.isle.seat.get_keyboard()?;
        let text = keyboard.with_xkb_state(self, |context| {
            let xkb = context.xkb().lock().unwrap();
            // Safety: read only; nothing here changes the state.
            unsafe { xkb.state() }.key_get_utf8(Keycode::new(code + 8))
        });
        (!text.is_empty() && !text.chars().any(char::is_control)).then_some(text)
    }

    fn spotlight_apply_key(&mut self, key: Key) {
        let Some(model) = self.isle.spotlight.model.as_mut() else {
            return;
        };
        let effects = model.key(key);
        self.spotlight_effects(effects);
        self.put_spotlight_frame();
    }

    fn start_spotlight_repeat(&mut self, code: u32, key: Key) {
        use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
        self.stop_spotlight_repeat();
        let keyboard = &self.isle.settings.keyboard;
        let delay = std::time::Duration::from_millis(keyboard.repeat_delay as u64);
        let every = std::time::Duration::from_secs_f64(1.0 / keyboard.repeat_rate.max(1) as f64);
        let token =
            self.isle
                .loop_handle
                .insert_source(Timer::from_duration(delay), move |_, _, state| {
                    if !state.spotlight_has_keys() {
                        state.isle.spotlight.repeat = None;
                        return TimeoutAction::Drop;
                    }
                    state.spotlight_apply_key(key.clone());
                    TimeoutAction::ToDuration(every)
                });
        if let Ok(token) = token {
            self.isle.spotlight.repeat = Some((code, token));
        }
    }

    fn stop_spotlight_repeat(&mut self) {
        if let Some((_, token)) = self.isle.spotlight.repeat.take() {
            self.isle.loop_handle.remove(token);
        }
    }

    /// A key released: ends its repeat.
    pub fn spotlight_key_released(&mut self, code: u32) {
        if self
            .isle
            .spotlight
            .repeat
            .as_ref()
            .is_some_and(|(c, _)| *c == code)
        {
            self.stop_spotlight_repeat();
        }
    }

    /// Cmd+V in the field: the clipboard's text, read off the loop.
    fn spotlight_paste(&mut self) {
        self.read_clipboard_text(|state, text| {
            if let Some(text) = text {
                state.spotlight_apply_key(Key::Paste(text));
            }
        });
    }

    pub fn spotlight_action(&mut self, action: SpotlightAction) {
        let Some(model) = self.isle.spotlight.model.as_mut() else {
            return;
        };
        let effects = match action {
            SpotlightAction::Hover(key) => {
                model.hover(&key);
                Vec::new()
            }
            SpotlightAction::Click(key) => {
                let how = if self.isle.keyboards.engine.mods().cmd {
                    How::Reveal
                } else {
                    How::Default
                };
                self.isle.spotlight.model.as_mut().unwrap().click(&key, how)
            }
            SpotlightAction::Secondary(key) => {
                model.hover(&key);
                model.key(Key::Menu)
            }
            SpotlightAction::Quick(n) => model.key(Key::Quick(n)),
            SpotlightAction::Contents(on) => model.set_contents(on),
            SpotlightAction::DragFrom(key) => {
                self.isle.spotlight.drag_from = Some(key);
                Vec::new()
            }
            SpotlightAction::DragTo(key) => {
                let from = self.isle.spotlight.drag_from.take();
                match from.filter(|f| *f != key) {
                    Some(from) => self.spotlight_drop(&from, &key),
                    None => Vec::new(),
                }
            }
            SpotlightAction::Filter => {
                self.spotlight_filter_menu(false);
                Vec::new()
            }
            SpotlightAction::Menu(event) => self.spotlight_menu_event(event),
        };
        self.spotlight_effects(effects);
        self.put_spotlight_frame();
    }

    fn spotlight_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Search(query) => self.spotlight_search(query),
                Effect::Close => self.close_spotlight(),
                Effect::Choose(row, how) => self.spotlight_choose(row, how),
                Effect::Menu(row) => self.spotlight_menu(row),
                Effect::Preview(row) => self.spotlight_preview(row),
                Effect::Copy(row) => self.spotlight_copy(row),
                Effect::Pin(row) => self.spotlight_pin(row),
                Effect::Delete(row) => self.spotlight_delete(row),
                Effect::MoveFavourite(row, by) => self.spotlight_move_favourite(row, by),
                Effect::Filter => self.spotlight_filter_menu(true),
            }
        }
    }

    /// Runs the fast sources now and starts the slow ones on the worker.
    fn spotlight_search(&mut self, query: Query) {
        let started = std::time::Instant::now();
        let spotlight = &mut self.isle.spotlight;
        spotlight.latest.store(query.generation, Ordering::Relaxed);
        let now = now();
        let mut fast: BTreeMap<Group, Vec<Row>> = BTreeMap::new();
        {
            let mut cx = Context {
                matcher: &mut spotlight.matcher,
                usage: &spotlight.usage,
                now,
                latest: None,
            };
            if spotlight.sources.apps.answers(&query.scope) {
                let rows = spotlight.sources.apps.search(&query, &mut cx);
                fast.entry(Group::Apps).or_default().extend(rows);
            }
            if spotlight.sources.clipboard.answers(&query.scope) {
                let rows = spotlight.sources.clipboard.search(&query, &mut cx);
                fast.entry(Group::Clipboard).or_default().extend(rows);
            }
            for source in spotlight
                .sources
                .fast
                .iter()
                .filter(|s| s.answers(&query.scope))
            {
                let rows = source.search(&query, &mut cx);
                fast.entry(source.group()).or_default().extend(rows);
            }
        }
        let slow: Vec<Arc<dyn Source>> = spotlight
            .sources
            .slow
            .iter()
            .filter(|s| s.answers(&query.scope))
            .cloned()
            .collect();
        let Some(model) = spotlight.model.as_mut() else {
            return;
        };
        model.started(query.generation, slow.iter().map(|s| s.group()));
        let took = started.elapsed().as_micros() as u64;
        let times = &mut self.isle.spotlight.search_us;
        times.push(took);
        if times.len() > 256 {
            times.remove(0);
        }
        let Some(model) = self.isle.spotlight.model.as_mut() else {
            return;
        };
        let mut effects = Vec::new();
        for (group, rows) in fast {
            effects.extend(model.answer(query.generation, group, rows, false));
        }
        let (Some(worker), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return self.spotlight_effects(effects);
        };
        for source in slow {
            let (query, latest, usage, to_loop) = (
                query.clone(),
                self.isle.spotlight.latest.clone(),
                self.isle.spotlight.usage.clone(),
                to_loop.clone(),
            );
            worker.spawn_blocking(move || {
                if latest.load(Ordering::Relaxed) != query.generation {
                    return;
                }
                let mut matcher = Matcher::new();
                let mut cx = Context {
                    matcher: &mut matcher,
                    usage: &usage,
                    now,
                    latest: Some(&latest),
                };
                let rows = source.search(&query, &mut cx);
                if cx.stale(&query) {
                    return;
                }
                let group = source.group();
                to_loop.post(move |state| {
                    let Some(model) = state.isle.spotlight.model.as_mut() else {
                        return;
                    };
                    let effects = model.answer(query.generation, group, rows, true);
                    state.spotlight_effects(effects);
                    state.put_spotlight_frame();
                });
            });
        }
        self.spotlight_effects(effects);
    }

    /// Closes Spotlight, gives focus back, then does what the row is for (`shell.md` Shell
    /// runtime 11, v1 D63).
    fn spotlight_choose(&mut self, row: Row, how: How) {
        let previous = self.isle.spotlight.previous;
        self.close_spotlight();
        debug!(key = row.key, ?how, "spotlight choice");
        let counted = !matches!(
            row.item,
            Item::Web(_) | Item::Answer(_) | Item::Run(_) | Item::Clip(_) | Item::File(_)
        );
        if counted {
            let key = match &row.item {
                Item::Emoji(text) => format!("emoji:{text}"),
                _ => row.key.clone(),
            };
            self.isle.record_use(&key);
        }
        match row.item {
            Item::App(app) => {
                if !self.activate_app(&app) {
                    self.launch(crate::launch::Launch::App {
                        app,
                        action: None,
                        uris: Vec::new(),
                    });
                }
            }
            Item::Game(key) => {
                if let Some(id) = isle_games::GameId::parse(&key) {
                    let running = match &id {
                        isle_games::GameId::Steam(n) => {
                            self.activate_app(&format!("steam_app_{n}"))
                        }
                        _ => false,
                    };
                    if !running {
                        self.launch_game(&id);
                    }
                }
            }
            Item::EntryAction { app, action } => {
                self.launch(crate::launch::Launch::App {
                    app,
                    action: Some(action),
                    uris: Vec::new(),
                });
            }
            Item::Action(id) => self.run_spotlight_action(&id),
            Item::Window(id) => {
                self.activate(id);
            }
            Item::MenuItem(path) => {
                if let Some(id) = previous {
                    self.activate_menu_item(id, &path);
                }
            }
            Item::Setting { pane, row } => self.open_settings_at(&pane, &row),
            Item::File(path) => match how {
                How::Reveal => self.show_in_files(&path),
                _ => {
                    self.launch(crate::launch::Launch::Open(
                        path.to_string_lossy().into_owned(),
                    ));
                }
            },
            Item::Answer(answer) => self.copy_text(answer),
            Item::Web(url) => {
                self.launch(crate::launch::Launch::Open(url));
            }
            Item::Run(command) => self.spotlight_run(command, how),
            Item::Emoji(text) => self.insert_pick(
                previous,
                crate::clipboard::Pick::text(text),
                how == How::Alternate,
                true,
            ),
            Item::Clip(id) => self.insert_clip(previous, id, how == How::Alternate),
        }
    }
}

impl Isle {
    /// A system prompt is on screen and holds the keyboard.
    pub fn prompts_hold_keyboard(&self) -> bool {
        self.prompt_showing().is_some()
    }
}

impl State {
    /// The sources for a search opened now, from what the compositor knows: the app index,
    /// the game library, the shortcut set, the windows and the front app's menus.
    fn spotlight_sources(&mut self) -> Sources {
        use isle_search::sources::{actions, apps, settings, windows};
        let isle = &self.isle;
        let listed: Vec<&isle_apps::Entry> = isle
            .apps
            .entries()
            .iter()
            .filter(|e| e.listed())
            .filter(|e| {
                e.exec
                    .as_deref()
                    .and_then(|x| isle.games.for_exec(x))
                    .is_none()
            })
            .collect();
        let apps_source = apps::AppsSource {
            apps: listed
                .iter()
                .map(|e| apps::App {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    also: e
                        .generic_name
                        .iter()
                        .chain(e.keywords.iter())
                        .cloned()
                        .collect(),
                    category: e.category().label().to_owned(),
                })
                .collect(),
            games: isle
                .games
                .games()
                .iter()
                .map(|g| apps::Game {
                    key: g.id.to_string(),
                    name: g.name.clone(),
                    cover: g.cover.clone(),
                })
                .collect(),
            favourites: isle.settings.spotlight.favourites.clone(),
            category: None,
        };

        let labels = self.labels();
        let chords = isle.settings.shortcuts.chords();
        let chord_of = |id: &str| {
            chords
                .iter()
                .find(|(i, _)| *i == id)
                .and_then(|(_, c)| c.first())
                .and_then(|c| isle_keys::chord::Chord::parse(c))
                .map(|c| {
                    let hint = crate::menubar::hint(&c, labels);
                    format!("{}{}", hint.modifiers, hint.key)
                })
        };
        const COMMANDS: [&str; 4] = ["sleep", "restart", "shut-down", "dark-mode"];
        let mut action_list: Vec<actions::Action> = actions::catalogue()
            .iter()
            .filter(|a| COMMANDS.contains(&a.id.as_str()) || crate::binds::built(&a.id))
            .map(|a| actions::Action {
                id: a.id.clone(),
                title: a.title.clone(),
                glyph: a.glyph,
                also: a.also.clone(),
                chord: chord_of(&a.id),
            })
            .collect();
        for mode in isle.settings.focus_mode.keys() {
            action_list.push(actions::Action {
                id: format!("focus:{mode}"),
                title: mode.clone(),
                glyph: "moon",
                also: vec!["focus".into()],
                chord: None,
            });
        }
        let entry_actions = listed
            .iter()
            .flat_map(|e| {
                e.actions.iter().map(|a| actions::EntryAction {
                    app: e.id.clone(),
                    app_name: e.name.clone(),
                    action: a.id.clone(),
                    name: a.name.clone(),
                })
            })
            .collect();

        let windows_source = windows::WindowsSource {
            windows: isle
                .window_infos()
                .into_iter()
                .map(|w| windows::Window {
                    app_name: isle.app_name(&w.app),
                    desktop: format!("Desktop {}", w.desktop),
                    id: w.id,
                    title: w.title,
                    app: w.app,
                })
                .collect(),
        };

        let menus_source = self.spotlight_menu_items();
        let settings_rows = std::fs::read_to_string(settings_index())
            .map(|t| settings::parse(&t))
            .unwrap_or_default();
        let clipboard = isle_search::sources::clipboard::ClipboardSource {
            clips: self.isle.clipboard.clips(|app| self.isle.app_name(app)),
        };
        Sources {
            apps: apps_source,
            clipboard,
            fast: vec![
                Box::new(actions::ActionsSource {
                    actions: action_list,
                    entry_actions,
                }),
                Box::new(windows_source),
                Box::new(menus_source),
                Box::new(settings::SettingsSource {
                    rows: settings_rows,
                }),
                Box::new(isle_search::sources::run::RunSource {
                    history: self.isle.spotlight.commands.run.clone(),
                }),
                Box::new(isle_search::sources::emoji::EmojiSource {
                    drawable: self.isle.emoji_drawable.clone(),
                    check: isle_shell::spotlight::drawable_emoji,
                }),
                Box::new(isle_search::sources::run::WebSource {
                    template: self.isle.settings.spotlight.web_search.clone(),
                }),
            ],
            slow: match &self.isle.files {
                Some(index) => vec![
                    Arc::new(isle_search::files::FilesSource {
                        index: index.clone(),
                        xbel: isle_apps::appimage::data_home()
                            .map(|d| d.join("recently-used.xbel")),
                        app_names: self
                            .isle
                            .apps
                            .entries()
                            .iter()
                            .map(|e| {
                                let command = e
                                    .exec
                                    .as_deref()
                                    .and_then(|x| x.split_whitespace().next())
                                    .map(|c| c.rsplit('/').next().unwrap_or(c).to_owned());
                                (
                                    e.id.clone(),
                                    std::iter::once(e.name.clone()).chain(command).collect(),
                                )
                            })
                            .collect(),
                    }) as Arc<dyn Source>,
                    Arc::new(isle_search::files::ContentsSource {
                        index: index.clone(),
                    }),
                    Arc::new(isle_search::sources::calculator::CalculatorSource),
                ],
                None => {
                    vec![Arc::new(isle_search::sources::calculator::CalculatorSource)
                        as Arc<dyn Source>]
                }
            },
        }
    }

    /// The front app's enabled menu items, from the same titles Help's search reads; submenus
    /// the app fills on demand are asked for, for the next search.
    fn spotlight_menu_items(&mut self) -> isle_search::sources::menus::MenusSource {
        use isle_search::sources::menus::{MenuItem, MenusSource};
        self.crawl_menus();
        let Some(app) = self.isle.front_app() else {
            return MenusSource::default();
        };
        if self.isle.bar.output.is_none() {
            self.isle.bar.titles = self.build_bar();
        }
        let titles = &self.isle.bar.titles;
        let last = titles.len().saturating_sub(1);
        fn walk(entries: &[isle_ui::MenuEntry], path: &mut Vec<String>, out: &mut Vec<MenuItem>) {
            for item in entries.iter().filter_map(isle_ui::MenuEntry::item) {
                path.push(item.label.clone());
                match &item.submenu {
                    Some(children) => walk(children, path, out),
                    None if item.enabled && !item.label.is_empty() => out.push(MenuItem {
                        path: path.clone(),
                        hint: item
                            .hint
                            .as_ref()
                            .map(|h| format!("{}{}", h.modifiers, h.key)),
                    }),
                    None => {}
                }
                path.pop();
            }
        }
        let mut items = Vec::new();
        for (i, title) in titles.iter().enumerate() {
            if i == last || title.kind == isle_shell::menubar::TitleKind::System {
                continue;
            }
            let mut path = vec![title.label.clone()];
            walk(&title.menu, &mut path, &mut items);
        }
        MenusSource { app, items }
    }

    /// A Spotlight action: a shortcut's, or a command with no chord.
    fn run_spotlight_action(&mut self, id: &str) {
        use isle_shell::prompts::session::Choice;
        match id {
            "sleep" => self.isle.suspend(),
            "restart" => self.open_session_confirm(Choice::Restart),
            "shut-down" => self.open_session_confirm(Choice::ShutDown),
            "dark-mode" => {
                let dark = self.isle.shell.theme().appearance.mode == isle_tokens::Mode::Dark;
                self.isle.write_setting(
                    "appearance.mode",
                    isle_settings::Value::Str(if dark { "light" } else { "dark" }.into()),
                );
            }
            _ => match id.strip_prefix("focus:") {
                Some(mode) => {
                    let mode = mode.to_owned();
                    self.focus_chord(&mode);
                }
                None => self.run_shortcut(id),
            },
        }
    }

    /// A menu item of the window `id`'s app, by its labels from the title down.
    fn activate_menu_item(&mut self, id: u64, path: &[String]) {
        if self.focused_window() != Some(id) {
            self.activate(id);
        }
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        self.bar_activate(&path);
    }

    /// Settings at a pane and row; Settings comes with P13.
    fn open_settings_at(&mut self, pane: &str, row: &str) {
        let target = format!("isle-settings://{pane}/{row}");
        self.launch(crate::launch::Launch::Open(target));
    }

    /// The file selected in whatever owns `org.freedesktop.FileManager1`.
    fn show_in_files(&mut self, path: &std::path::Path) {
        let uri = format!("file://{}", path.display());
        if let Some(worker) = &self.isle.worker {
            worker.spawn(async move {
                let Ok(conn) = isle_bus::session().await else {
                    return;
                };
                let call = conn
                    .call_method(
                        Some("org.freedesktop.FileManager1"),
                        "/org/freedesktop/FileManager1",
                        Some("org.freedesktop.FileManager1"),
                        "ShowItems",
                        &(vec![uri], ""),
                    )
                    .await;
                if let Err(err) = call {
                    warn!("could not show a file: {err}");
                }
            });
        }
    }

    /// Spotlight's Run: in a scope of its own, or in the user's terminal with Cmd+Return;
    /// the last fifty commands are kept.
    fn spotlight_run(&mut self, command: String, how: How) {
        let history = &mut self.isle.spotlight.commands.run;
        history.retain(|c| *c != command);
        history.insert(0, command.clone());
        history.truncate(50);
        if let Some(saver) = &self.isle.spotlight.commands_saver {
            saver.save(self.isle.spotlight.commands.clone());
        }
        let line = match how {
            How::Reveal => format!("xdg-terminal-exec sh -c {}", shell_quote(&command)),
            _ => command,
        };
        self.launch(crate::launch::Launch::Command(line));
    }

    /// Cmd+Y on a clipboard item: the whole text or the image over the list.
    fn spotlight_preview(&mut self, row: Row) {
        use isle_shell::spotlight::surface::Preview;
        let Item::Clip(id) = row.item else {
            return;
        };
        let Some(item) = self.isle.clipboard.items.iter().find(|i| i.id == id) else {
            return;
        };
        self.isle.spotlight.preview = Some(match item.image(self.isle.clipboard.dir()) {
            Some(path) => Preview::Image(path),
            None => Preview::Text(
                self.isle
                    .clipboard
                    .pick(id)
                    .and_then(|p| {
                        p.types
                            .iter()
                            .find(|(t, _)| t.starts_with("text/"))
                            .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
                    })
                    .unwrap_or_default(),
            ),
        });
    }

    /// History changed while open: the clipboard scope lists it again.
    fn refresh_clipboard_scope(&mut self) {
        let clips = self.isle.clipboard.clips(|app| self.isle.app_name(app));
        self.isle.spotlight.sources.clipboard.clips = clips;
        let effects = self
            .isle
            .spotlight
            .model
            .as_mut()
            .map(Spotlight::refresh)
            .unwrap_or_default();
        self.spotlight_effects(effects);
    }

    /// Cmd+C on a row: its answer, path or text.
    fn spotlight_copy(&mut self, row: Row) {
        if let Item::Clip(id) = row.item {
            self.close_spotlight();
            self.insert_clip(None, id, true);
            return;
        }
        let text = match row.item {
            Item::Answer(a) => a,
            Item::File(p) => p.to_string_lossy().into_owned(),
            Item::Emoji(e) => e,
            Item::Web(_) | Item::Run(_) => row.title,
            _ => return,
        };
        self.close_spotlight();
        self.copy_text(text);
    }

    fn spotlight_pin(&mut self, row: Row) {
        if let Item::Clip(id) = row.item {
            self.clipboard_pin(id);
            self.refresh_clipboard_scope();
        }
    }

    fn spotlight_delete(&mut self, row: Row) {
        if let Item::Clip(id) = row.item {
            self.clipboard_delete(id);
            self.refresh_clipboard_scope();
        }
    }
}

/// Where Settings installs its row index: beside its data, else the checkout's empty one.
fn settings_index() -> std::path::PathBuf {
    let installed = std::path::Path::new("/usr/share/isle/settings/index.kdl");
    if installed.is_file() {
        return installed.to_owned();
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/settings/index.kdl")
}

/// `text` as one word for `sh`.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

impl State {
    /// A row's or tile's menu (`structure/spotlight.md`, the tile's menu).
    fn spotlight_menu(&mut self, row: Row) {
        use isle_ui::{MenuEntry, MenuItem, menu::Mark};
        let mut commands = std::collections::HashMap::new();
        let mut next = 1u64;
        let mut item = |label: &str, command: MenuCommand| {
            let id = next;
            next += 1;
            commands.insert(id, command);
            MenuItem::new(id, label)
        };
        let mut groups: Vec<Vec<MenuEntry>> = Vec::new();
        let target = match &row.item {
            Item::App(app) => Some(app.clone()),
            Item::Game(key) => Some(key.clone()),
            _ => None,
        };
        if let Item::Clip(id) = row.item {
            let pinned = self
                .isle
                .clipboard
                .items
                .iter()
                .any(|i| i.id == id && i.pinned);
            groups.push(vec![
                MenuEntry::Item(item("Paste", MenuCommand::Choose(row.clone()))),
                MenuEntry::Item(item("Copy", MenuCommand::Copy(row.clone()))),
            ]);
            groups.push(vec![
                MenuEntry::Item(item(
                    if pinned { "Unpin" } else { "Pin" },
                    MenuCommand::Pin(id),
                )),
                MenuEntry::Item(item("Delete", MenuCommand::Delete(id))),
            ]);
            groups.push(vec![MenuEntry::Item(item(
                "Clear clipboard history…",
                MenuCommand::ClearAsk,
            ))]);
        } else {
            groups.push(vec![MenuEntry::Item(item(
                "Open",
                MenuCommand::Choose(row.clone()),
            ))]);
        }
        if let Item::App(app) = &row.item
            && let Some(entry) = self.isle.apps.get(app).cloned()
            && !entry.actions.is_empty()
        {
            groups.push(
                entry
                    .actions
                    .iter()
                    .map(|a| {
                        MenuEntry::Item(item(
                            &a.name,
                            MenuCommand::EntryAction {
                                app: app.clone(),
                                action: a.id.clone(),
                            },
                        ))
                    })
                    .collect(),
            );
        }
        if let Some(target) = &target {
            let favourite = self.isle.spotlight.sources.apps.favourites.contains(target);
            groups.push(vec![MenuEntry::Item(if favourite {
                item(
                    "Remove from favourites",
                    MenuCommand::RemoveFavourite(target.clone()),
                )
            } else {
                item(
                    "Add to favourites",
                    MenuCommand::AddFavourite(target.clone()),
                )
            })]);
        }
        let mut options = Vec::new();
        if let Item::App(app) = &row.item {
            let on = self.isle.settings.windows.keep_running.contains(app);
            let keep = item(
                "Keep running after last window",
                MenuCommand::KeepRunning(app.clone(), !on),
            )
            .mark(if on { Mark::Check } else { Mark::None });
            options.push(MenuEntry::Item(
                item("Options", MenuCommand::Category(None)).submenu(vec![MenuEntry::Item(keep)]),
            ));
        }
        if let Item::Game(key) = &row.item
            && let Some(game) =
                isle_games::GameId::parse(key).and_then(|id| self.isle.games.get(&id).cloned())
        {
            options.push(MenuEntry::Item(item(
                &format!("Show in {}", game.launcher.name()),
                MenuCommand::ShowInLauncher(key.clone()),
            )));
        }
        if let Item::Emoji(text) = &row.item
            && let Some(glyph) = isle_search::sources::emoji::table()
                .iter()
                .find(|g| &g.text == text)
            && !glyph.variants.is_empty()
        {
            groups.push(
                glyph
                    .variants
                    .iter()
                    .map(|(v, name)| {
                        MenuEntry::Item(item(
                            &format!("{v}  {name}"),
                            MenuCommand::Insert(v.clone()),
                        ))
                    })
                    .collect(),
            );
        }
        if matches!(row.item, Item::Answer(_) | Item::File(_) | Item::Emoji(_)) {
            options.push(MenuEntry::Item(item(
                "Copy",
                MenuCommand::Copy(row.clone()),
            )));
        }
        if !options.is_empty() {
            groups.push(options);
        }
        let app_of = |target: &str| match target.strip_prefix("steam:") {
            Some(n) => format!("steam_app_{n}"),
            None => target.to_owned(),
        };
        let running = target.as_deref().is_some_and(|t| {
            self.isle
                .wm
                .records
                .iter()
                .any(|r| r.app == app_of(t) && r.mapped)
        });
        if running && let Some(target) = &target {
            groups.push(vec![MenuEntry::Item(item(
                "Quit",
                MenuCommand::Quit(app_of(target)),
            ))]);
        }
        let mut entries = Vec::new();
        for (i, group) in groups.into_iter().enumerate() {
            if i > 0 {
                entries.push(MenuEntry::Separator);
            }
            entries.extend(group);
        }
        self.isle.spotlight.menu = Some(OpenMenu {
            view: isle_shell::spotlight::surface::MenuView {
                entries: Rc::new(entries),
                anchor: isle_shell::spotlight::surface::MenuAnchor::Selected,
                by_key: true,
            },
            commands,
        });
    }

    /// The Apps view's category filter: All apps, then each category that has apps.
    fn spotlight_filter_menu(&mut self, by_key: bool) {
        use isle_ui::{MenuEntry, MenuItem, menu::Mark};
        let apps = &self.isle.spotlight.sources.apps;
        let present: Vec<String> = isle_apps::Category::ALL
            .iter()
            .map(|c| c.label().to_owned())
            .filter(|label| {
                apps.apps.iter().any(|a| &a.category == label)
                    || (label == isle_search::sources::apps::GAMES && !apps.games.is_empty())
            })
            .collect();
        let current = apps.category.clone();
        let mut commands = std::collections::HashMap::new();
        commands.insert(1, MenuCommand::Category(None));
        let check = |on: bool| if on { Mark::Check } else { Mark::None };
        let mut entries = vec![
            MenuEntry::Item(MenuItem::new(1, "All apps").mark(check(current.is_none()))),
            MenuEntry::Separator,
        ];
        for (i, label) in present.into_iter().enumerate() {
            let id = i as u64 + 2;
            entries.push(MenuEntry::Item(
                MenuItem::new(id, label.clone()).mark(check(current.as_ref() == Some(&label))),
            ));
            commands.insert(id, MenuCommand::Category(Some(label)));
        }
        self.isle.spotlight.menu = Some(OpenMenu {
            view: isle_shell::spotlight::surface::MenuView {
                entries: Rc::new(entries),
                anchor: isle_shell::spotlight::surface::MenuAnchor::Filter,
                by_key,
            },
            commands,
        });
    }

    fn spotlight_menu_event(&mut self, event: isle_ui::MenuEvent) -> Vec<Effect> {
        use isle_ui::MenuEvent;
        match event {
            MenuEvent::Chosen(id) => {
                let command = self
                    .isle
                    .spotlight
                    .menu
                    .take()
                    .and_then(|m| m.commands.get(&id).cloned());
                match command {
                    Some(command) => self.spotlight_menu_command(command),
                    None => Vec::new(),
                }
            }
            MenuEvent::Back => {
                self.isle.spotlight.menu = None;
                Vec::new()
            }
            MenuEvent::Opening(_) | MenuEvent::Forward => Vec::new(),
        }
    }

    fn spotlight_menu_command(&mut self, command: MenuCommand) -> Vec<Effect> {
        match command {
            MenuCommand::Choose(row) => vec![Effect::Choose(row, How::Default)],
            MenuCommand::EntryAction { app, action } => {
                self.close_spotlight();
                self.launch(crate::launch::Launch::App {
                    app,
                    action: Some(action),
                    uris: Vec::new(),
                });
                Vec::new()
            }
            MenuCommand::AddFavourite(target) => {
                let mut list = self.isle.spotlight.sources.apps.favourites.clone();
                if !list.contains(&target) {
                    list.push(target);
                }
                self.set_favourites(list)
            }
            MenuCommand::RemoveFavourite(target) => {
                let mut list = self.isle.spotlight.sources.apps.favourites.clone();
                list.retain(|f| *f != target);
                self.set_favourites(list)
            }
            MenuCommand::KeepRunning(app, on) => {
                let mut list = self.isle.settings.windows.keep_running.clone();
                list.retain(|a| *a != app);
                if on {
                    list.push(app);
                }
                self.isle
                    .write_setting("windows.keep-running", isle_settings::Value::List(list));
                Vec::new()
            }
            MenuCommand::ShowInLauncher(key) => {
                self.close_spotlight();
                let game = isle_games::GameId::parse(&key)
                    .and_then(|id| self.isle.games.get(&id).cloned());
                if let Some(game) = game {
                    match game.page_url() {
                        Some(url) => {
                            self.launch(crate::launch::Launch::Open(url));
                        }
                        None => {
                            let entry = game.launcher.desktop_id(game.flatpak).to_owned();
                            if !self.activate_app(&entry) {
                                self.launch(crate::launch::Launch::App {
                                    app: entry,
                                    action: None,
                                    uris: Vec::new(),
                                });
                            }
                        }
                    }
                }
                Vec::new()
            }
            MenuCommand::Quit(app) => {
                self.quit_app(&app);
                Vec::new()
            }
            MenuCommand::Insert(text) => {
                let previous = self.isle.spotlight.previous;
                self.close_spotlight();
                self.isle.record_use(&format!("emoji:{text}"));
                self.insert_pick(previous, crate::clipboard::Pick::text(text), false, true);
                Vec::new()
            }
            MenuCommand::Copy(row) => {
                self.spotlight_copy(row);
                Vec::new()
            }
            MenuCommand::Pin(id) => {
                self.clipboard_pin(id);
                self.refresh_clipboard_scope();
                Vec::new()
            }
            MenuCommand::Delete(id) => {
                self.clipboard_delete(id);
                self.refresh_clipboard_scope();
                Vec::new()
            }
            MenuCommand::ClearAsk => {
                use isle_ui::{MenuEntry, MenuItem};
                let n = self
                    .isle
                    .clipboard
                    .items
                    .iter()
                    .filter(|i| !i.pinned)
                    .count();
                let mut commands = std::collections::HashMap::new();
                commands.insert(1, MenuCommand::Clear);
                commands.insert(
                    2,
                    MenuCommand::Category(self.isle.spotlight.sources.apps.category.clone()),
                );
                self.isle.spotlight.menu = Some(OpenMenu {
                    view: isle_shell::spotlight::surface::MenuView {
                        entries: Rc::new(vec![
                            MenuEntry::Section(
                                "Clear clipboard history? Pinned items are kept.".into(),
                            ),
                            MenuEntry::Item(MenuItem::new(1, format!("Clear {n} items"))),
                            MenuEntry::Item(MenuItem::new(2, "Cancel")),
                        ]),
                        anchor: isle_shell::spotlight::surface::MenuAnchor::Selected,
                        by_key: true,
                    },
                    commands,
                });
                Vec::new()
            }
            MenuCommand::Clear => {
                self.clipboard_clear();
                self.refresh_clipboard_scope();
                Vec::new()
            }
            MenuCommand::Category(category) => {
                self.isle.spotlight.sources.apps.category = category;
                self.isle
                    .spotlight
                    .model
                    .as_mut()
                    .map(Spotlight::refresh)
                    .unwrap_or_default()
            }
        }
    }

    /// Writes the favourites and shows them at once.
    fn set_favourites(&mut self, list: Vec<String>) -> Vec<Effect> {
        self.isle.write_setting(
            "spotlight.favourites",
            isle_settings::Value::List(list.clone()),
        );
        self.isle.spotlight.sources.apps.favourites = list;
        self.isle
            .spotlight
            .model
            .as_mut()
            .map(Spotlight::refresh)
            .unwrap_or_default()
    }

    /// Whether a tile's key is a favourite's, and the app or game it names.
    fn tile_target(key: &str) -> Option<(bool, String)> {
        let (favourite, rest) = match key.strip_prefix("fav:") {
            Some(rest) => (true, rest),
            None => (false, key),
        };
        let target = rest
            .strip_prefix("app:")
            .or_else(|| rest.strip_prefix("game:"))?;
        Some((favourite, target.to_owned()))
    }

    /// A tile dropped on another: onto a favourite it goes before that one, joining the
    /// favourites if it was not among them; a favourite dropped on all apps leaves them.
    fn spotlight_drop(&mut self, from: &str, to: &str) -> Vec<Effect> {
        let (Some((from_fav, from_target)), Some((to_fav, to_target))) =
            (Self::tile_target(from), Self::tile_target(to))
        else {
            return Vec::new();
        };
        let mut list = self.isle.spotlight.sources.apps.favourites.clone();
        match (from_fav, to_fav) {
            (_, true) => {
                list.retain(|f| *f != from_target);
                let at = list
                    .iter()
                    .position(|f| *f == to_target)
                    .unwrap_or(list.len());
                list.insert(at, from_target);
            }
            (true, false) => list.retain(|f| *f != from_target),
            (false, false) => return Vec::new(),
        }
        self.set_favourites(list)
    }

    /// Option+Cmd+Left and Right on a favourite.
    fn spotlight_move_favourite(&mut self, row: Row, by: i32) {
        let Some((_, target)) = Self::tile_target(&row.key) else {
            return;
        };
        let mut list = self.isle.spotlight.sources.apps.favourites.clone();
        let Some(at) = list.iter().position(|f| *f == target) else {
            return;
        };
        let to = (at as i32 + by).clamp(0, list.len() as i32 - 1) as usize;
        let moved = list.remove(at);
        list.insert(to, moved);
        let effects = self.set_favourites(list);
        self.spotlight_effects(effects);
    }
}

#[cfg(feature = "test-hooks")]
impl State {
    /// Chooses the open menu's item labelled `label`, as a click would.
    pub fn spotlight_menu_choose(&mut self, label: &str) {
        fn find(entries: &[isle_ui::MenuEntry], label: &str) -> Option<u64> {
            entries
                .iter()
                .filter_map(isle_ui::MenuEntry::item)
                .find_map(|item| {
                    if item.label == label {
                        return Some(item.id);
                    }
                    item.submenu.as_ref().and_then(|sub| find(sub, label))
                })
        }
        let id = self
            .isle
            .spotlight
            .menu
            .as_ref()
            .and_then(|m| find(&m.view.entries, label));
        if let Some(id) = id {
            self.spotlight_action(SpotlightAction::Menu(isle_ui::MenuEvent::Chosen(id)));
        }
    }
}

/// A drag out of Spotlight: Isle offers the row's types and writes them to the drop target.
struct DragSource {
    pick: crate::clipboard::Pick,
}

impl smithay::utils::IsAlive for DragSource {
    fn alive(&self) -> bool {
        true
    }
}

impl smithay::input::dnd::Source for DragSource {
    fn metadata(&self) -> Option<smithay::input::dnd::SourceMetadata> {
        Some(smithay::input::dnd::SourceMetadata {
            mime_types: self.pick.types.iter().map(|(t, _)| t.clone()).collect(),
            dnd_actions: [smithay::input::dnd::DndAction::Copy].into_iter().collect(),
        })
    }

    fn send(&self, mime_type: &str, fd: std::os::fd::OwnedFd) {
        if let Some((_, bytes)) = self.pick.types.iter().find(|(t, _)| t == mime_type) {
            let bytes = bytes.clone();
            std::thread::spawn(move || {
                use std::io::Write;
                let _ = std::fs::File::from(fd).write_all(&bytes);
            });
        }
    }

    fn choose_action(&self, _action: smithay::input::dnd::DndAction) {}

    fn drop_performed(&self) {}

    fn cancel(&self) {}

    fn finished(&self) {}
}

impl State {
    /// The pointer moved with a press held on a file or clipboard row: once it leaves
    /// Spotlight, the row is dragged and Spotlight goes, so the target shows.
    pub fn spotlight_drag_check(
        &mut self,
        location: smithay::utils::Point<f64, smithay::utils::Logical>,
    ) {
        let Some(from) = self.isle.spotlight.drag_from.clone() else {
            return;
        };
        let pointer = self.isle.seat.get_pointer().unwrap();
        if !pointer.is_grabbed() {
            self.isle.spotlight.drag_from = None;
            return;
        }
        if self.isle.spotlight.rect.contains(location) {
            return;
        }
        let item = self
            .isle
            .spotlight
            .model
            .as_ref()
            .and_then(|m| m.selectable().into_iter().find(|r| r.key == from))
            .map(|r| r.item);
        let pick = match item {
            Some(Item::File(path)) => {
                let uri = format!("file://{}\r\n", path.display());
                let path = path.display().to_string();
                Some(crate::clipboard::Pick {
                    types: vec![
                        ("text/uri-list".into(), Arc::new(uri.into_bytes())),
                        (
                            "text/plain;charset=utf-8".into(),
                            Arc::new(path.clone().into_bytes()),
                        ),
                        ("text/plain".into(), Arc::new(path.into_bytes())),
                    ],
                })
            }
            Some(Item::Clip(id)) => self.isle.clipboard.pick(id),
            _ => None,
        };
        let Some(pick) = pick else {
            return;
        };
        let Some(mut start) = pointer.grab_start_data() else {
            return;
        };
        start.focus = None;
        self.close_spotlight();
        self.drag_out(pick, start);
        info!(from, "a drag out of Spotlight");
    }

    /// Starts a drag whose source is Isle, offering `pick`'s types to the drop target.
    pub fn drag_out(
        &mut self,
        pick: crate::clipboard::Pick,
        start: smithay::input::pointer::GrabStartData<State>,
    ) {
        let pointer = self.isle.seat.get_pointer().unwrap();
        let grab = smithay::input::dnd::DnDGrab::new_pointer(
            &self.isle.display_handle,
            start,
            DragSource { pick },
            self.isle.seat.clone(),
        );
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Clear);
    }
}

/// Runs every source that answers `scope` for `text`, fast and slow, without Spotlight open:
/// `isle search`. The rows as JSON, grouped in the order Spotlight lists them.
pub fn search_all(sources: Sources, usage: Usage, text: String, scope: Scope) -> String {
    let query = Query {
        text,
        scope,
        generation: 1,
    };
    let mut matcher = Matcher::new();
    let mut cx = Context {
        matcher: &mut matcher,
        usage: &usage,
        now: now(),
        latest: None,
    };
    let mut rows: Vec<Row> = Vec::new();
    if sources.apps.answers(&query.scope) {
        rows.extend(sources.apps.search(&query, &mut cx));
    }
    if sources.clipboard.answers(&query.scope) {
        rows.extend(sources.clipboard.search(&query, &mut cx));
    }
    for source in sources.fast.iter().filter(|s| s.answers(&query.scope)) {
        rows.extend(source.search(&query, &mut cx));
    }
    for source in sources.slow.iter().filter(|s| s.answers(&query.scope)) {
        rows.extend(source.search(&query, &mut cx));
    }
    rows.sort_by_key(|r| r.group);
    let json: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "group": format!("{:?}", r.group),
                "key": r.key,
                "title": r.title,
                "detail": r.detail,
                "trailing": r.trailing,
                "section": r.section,
            })
        })
        .collect();
    serde_json::Value::Array(json).to_string()
}

impl State {
    /// The sources a search would run now, for `isle search`.
    pub fn search_sources(&mut self) -> (Sources, Usage) {
        (self.spotlight_sources(), self.isle.spotlight.usage.clone())
    }
}
