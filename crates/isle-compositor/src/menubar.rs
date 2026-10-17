//! The menu bar on the loop: what the front app's bar holds and what each item does, the
//! reveal and the leave, the window menu and Isle's About panel (`p6-island-and-menus.md`
//! choices 13 and 18–23).

use std::{collections::HashMap, rc::Rc, time::Instant};

use isle_keys::{chord::Chord, engine::Stroke, profile::Profile};
use isle_shell::{
    menubar::{BarAction, BarFrame, BarMenuModel, BarMenuSurface, MenuBarSurface, Title, TitleKind},
    placed_menu::{About, AboutPanel, MenuModel, WindowMenu},
    prompts::session::Choice,
    transition::Motion,
};
use isle_ui::{
    MenuEntry, MenuEvent, MenuItem,
    menu::{Hint, Mark},
};
use smithay::{output::Output, reexports::calloop::RegistrationToken, utils::{Logical, Point, Rectangle, Size}};
use tracing::{info, warn};

use crate::{
    focus::Focus,
    state::{Isle, State},
    wm::{Mode, geometry::Zone},
};

/// What a menu item does.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Sleep,
    LockScreen,
    /// The session dialog's confirmation for this action.
    EndSession(isle_shell::prompts::session::Choice),
    ForceQuit,
    /// System Settings, at a pane when one is named.
    Settings(Option<&'static str>),
    About,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    /// The app's own chord, sent to it.
    Strokes(Vec<Stroke>),
    EntryAction(String),
    Link(String),
    Window(u64, WindowCommand),
    /// An item of the app's own menus, by window and label path.
    Remote(u64, Vec<String>),
    /// A submenu of the app's own menus to fill as it opens.
    Fill(u64, Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowCommand {
    Minimise,
    Zoom,
    Tile(Zone),
    Restore,
    FullScreen,
    ToDesktop(usize),
    NewDesktop,
    ToDisplay(i32),
    KeepOnTop,
    BringAllToFront,
    Focus,
    Move,
    Resize,
    Close,
}

pub struct BarState {
    pub surface: Rc<MenuBarSurface>,
    /// The open menu under its title.
    pub menu: Rc<BarMenuSurface>,
    /// Where the open menu's surface is, relative to its output.
    menu_at: Option<Rectangle<i32, Logical>>,
    pub window_menu: Rc<WindowMenu>,
    pub about: Rc<AboutPanel>,
    /// The output the bar is revealed on.
    pub output: Option<String>,
    shown: Motion,
    last: Instant,
    pub open: Option<usize>,
    pub focused: Option<usize>,
    pub by_key: bool,
    pub titles: Vec<Title>,
    commands: HashMap<u64, Command>,
    next_id: u64,
    generation: u64,
    pub leave: Option<RegistrationToken>,
    /// Travel into the top edge outside the island's zone.
    pub push: f64,
    pub dwell: Option<RegistrationToken>,
    /// The window menu's window and its commands.
    window_menu_for: Option<u64>,
    window_commands: HashMap<u64, Command>,
    /// A window moved or resized from the keyboard.
    pub keyboard_move: Option<(u64, bool, Rectangle<i32, smithay::utils::Logical>)>,
    /// Help and website links by app, read on the worker.
    links: HashMap<String, isle_apps::metainfo::Links>,
    /// The window the revealed bar speaks for: the one focused before the bar took the keys.
    pub window: Option<u64>,
    /// The output the bar is sliding away from.
    pub leaving_on: Option<Output>,
}

impl BarState {
    pub fn new() -> Self {
        Self {
            surface: MenuBarSurface::new(),
            menu: BarMenuSurface::new(),
            menu_at: None,
            window_menu: WindowMenu::new(),
            about: AboutPanel::new(),
            output: None,
            shown: Motion::at(0.0),
            last: Instant::now(),
            open: None,
            focused: None,
            by_key: false,
            titles: Vec::new(),
            commands: HashMap::new(),
            next_id: 1,
            generation: 0,
            leave: None,
            push: 0.0,
            dwell: None,
            window_menu_for: None,
            window_commands: HashMap::new(),
            keyboard_move: None,
            links: HashMap::new(),
            window: None,
            leaving_on: None,
        }
    }
}

/// How the typing keyboard's modifier keys are labelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Labels {
    Mac,
    Pc,
    /// A PC keyboard with Alt and Super swapped: Cmd is the key labelled Alt.
    PcSwapped,
}

/// A chord as the menu shows it: macOS's symbols and the key, or a PC keyboard's words for
/// the keys the user presses.
pub fn hint(chord: &Chord, labels: Labels) -> Hint {
    if labels != Labels::Mac {
        let (cmd, option) = match labels {
            Labels::PcSwapped => ("Alt", "Super"),
            _ => ("Super", "Alt"),
        };
        let words = chord.words().replace("Cmd", cmd).replace("Option", option);
        return Hint {
            modifiers: String::new(),
            key: words,
        };
    }
    let m = chord.mods;
    let mut modifiers = String::new();
    for (on, symbol) in [(m.ctrl, "⌃"), (m.option, "⌥"), (m.shift, "⇧"), (m.cmd, "⌘")] {
        if on {
            modifiers.push_str(symbol);
        }
    }
    use smithay::input::keyboard::keysyms as k;
    let key = match chord.sym.raw() {
        k::KEY_Return => "↩".into(),
        k::KEY_Escape => "⎋".into(),
        k::KEY_BackSpace => "⌫".into(),
        k::KEY_Delete => "⌦".into(),
        k::KEY_Tab => "⇥".into(),
        k::KEY_Left => "←".into(),
        k::KEY_Right => "→".into(),
        k::KEY_Up => "↑".into(),
        k::KEY_Down => "↓".into(),
        k::KEY_space => "Space".into(),
        _ => isle_keys::chord::key_name(chord.sym),
    };
    Hint { modifiers, key }
}

/// A row of the Window menu before it is numbered.
struct Row {
    label: String,
    command: Option<Command>,
    chord: Option<Chord>,
    mark: Mark,
    children: Option<Vec<Row>>,
}

impl Row {
    fn new(label: impl Into<String>, command: Option<Command>, chord: Option<Chord>) -> Self {
        Self {
            label: label.into(),
            command,
            chord,
            mark: Mark::None,
            children: None,
        }
    }

    fn separator() -> Self {
        Self::new("", None, None)
    }

    fn sub(label: impl Into<String>, children: Vec<Row>) -> Self {
        Self {
            children: Some(children),
            ..Self::new(label, None, None)
        }
    }

    fn mark(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }
}

/// Builds menus, numbering their items and keeping what each does.
struct Builder<'a> {
    commands: &'a mut HashMap<u64, Command>,
    next: &'a mut u64,
    labels: Labels,
}

impl Builder<'_> {
    fn item(
        &mut self,
        label: impl Into<String>,
        command: Option<Command>,
        chord: Option<Chord>,
    ) -> MenuEntry {
        let id = *self.next;
        *self.next += 1;
        let enabled = command.is_some();
        if let Some(command) = command {
            self.commands.insert(id, command);
        }
        MenuEntry::Item(
            MenuItem::new(id, label)
                .enabled(enabled)
                .hint(chord.map(|c| hint(&c, self.labels))),
        )
    }

    fn rows(&mut self, rows: Vec<Row>) -> Vec<MenuEntry> {
        rows.into_iter()
            .map(|row| match row.children {
                Some(children) => {
                    let kids = self.rows(children);
                    self.submenu(row.label, kids)
                }
                None if row.label.is_empty() => MenuEntry::Separator,
                None => match self.item(row.label, row.command, row.chord) {
                    MenuEntry::Item(item) => MenuEntry::Item(item.mark(row.mark)),
                    other => other,
                },
            })
            .collect()
    }

    /// A submenu, disabled when nothing in it is enabled.
    fn submenu(&mut self, label: impl Into<String>, entries: Vec<MenuEntry>) -> MenuEntry {
        let id = *self.next;
        *self.next += 1;
        let enabled = entries.iter().filter_map(MenuEntry::item).any(|i| i.enabled);
        MenuEntry::Item(MenuItem::new(id, label).enabled(enabled).submenu(entries))
    }

    /// The app's own menu nodes, each item running itself by its path.
    fn remote(
        &mut self,
        window: u64,
        nodes: &[isle_menus::Node],
        path: &mut Vec<String>,
        hints: &HashMap<Vec<String>, Chord>,
    ) -> Vec<MenuEntry> {
        let mut out = Vec::new();
        for node in nodes {
            match node {
                isle_menus::Node::Separator => out.push(MenuEntry::Separator),
                isle_menus::Node::Section(label) => out.push(MenuEntry::Section(label.clone())),
                isle_menus::Node::Item(item) => {
                    path.push(item.label.clone());
                    let mark = match item.toggle {
                        isle_menus::Toggle::Check(Some(true)) | isle_menus::Toggle::Radio(true) => Mark::Check,
                        isle_menus::Toggle::Check(None) => Mark::Mixed,
                        _ => Mark::None,
                    };
                    let entry = match &item.children {
                        Some(children) => {
                            let kids = self.remote(window, children, path, hints);
                            let id = *self.next;
                            *self.next += 1;
                            self.commands.insert(id, Command::Fill(window, path.clone()));
                            MenuEntry::Item(MenuItem::new(id, item.label.clone()).enabled(item.enabled).submenu(kids))
                        }
                        None => match self.item(
                            item.label.clone(),
                            item.enabled.then(|| Command::Remote(window, path.clone())),
                            hints.get(path.as_slice()).copied(),
                        ) {
                            MenuEntry::Item(i) => MenuEntry::Item(i.mark(mark)),
                            other => other,
                        },
                    };
                    out.push(entry);
                    path.pop();
                }
            }
        }
        out
    }
}

impl Isle {
    /// The desktop chord of a shortcut, for a hint.
    fn shortcut_chord(&self, id: &str) -> Option<Chord> {
        let s = &self.settings.shortcuts;
        let chords = match id {
            "hide" => &s.hide,
            "hide-others" => &s.hide_others,
            "quit-app" => &s.quit_app,
            "minimise" => &s.minimise,
            "full-screen" => &s.full_screen,
            "close-window" => &s.close_window,
            "tile-left" => &s.tile_left,
            "tile-right" => &s.tile_right,
            "tile-centre" => &s.tile_centre,
            "fill" => &s.fill,
            "restore" => &s.restore,
            _ => return None,
        };
        chords.first().and_then(|c| Chord::parse(c))
    }

    /// The record of the window the bar speaks for, or of the window a dialog belongs to,
    /// since the bar names the app whose menus it shows.
    fn bar_record(&self) -> Option<&crate::wm::Record> {
        let mut record = match self.bar.window {
            Some(id) if self.bar.output.is_some() => self.wm.get(id)?,
            _ => {
                let focus = self.seat.get_keyboard()?.current_focus()?;
                self.wm.for_surface(focus.surface()?)?
            }
        };
        for _ in 0..8 {
            match record.parent.and_then(|p| self.wm.get(p)) {
                Some(parent) => record = parent,
                None => break,
            }
        }
        Some(record)
    }

    /// The app the bar speaks for: the window it speaks for's, else Files.
    pub fn bar_app(&self) -> (String, String) {
        if let Some(record) = self.bar_record() {
            let app = record.app.clone();
            return (app.clone(), self.app_name(&app));
        }
        let files = [
            "org.gnome.Nautilus",
            "org.kde.dolphin",
            "thunar",
            "nemo",
            "pcmanfm",
        ]
        .into_iter()
        .find(|id| self.apps.get(id).is_some())
        .unwrap_or("");
        (files.to_owned(), "Files".to_owned())
    }
}

impl State {
    /// The window the bar speaks for: while it is down, the one focused before it.
    fn bar_window(&self) -> Option<u64> {
        if self.isle.bar.output.is_some() {
            self.isle.bar.window
        } else {
            self.focused_window()
        }
    }

    pub fn labels(&self) -> Labels {
        match self.isle.keyboards.last_device() {
            Some(d) if d.profile == Profile::Pc && d.swap => Labels::PcSwapped,
            Some(d) if d.profile == Profile::Pc => Labels::Pc,
            _ => Labels::Mac,
        }
    }

    /// The front app's titles and their menus, with every item's command.
    pub fn build_bar(&mut self) -> Vec<Title> {
        let (app, name) = self.isle.bar_app();
        let labels = self.labels();
        let game = self.focus_is_game();
        let names: Vec<String> = {
            let mut n = self.focus_names();
            if n.is_empty() {
                n.push(app.clone());
            }
            n
        };
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let table_commands = if game {
            Vec::new()
        } else {
            self.isle.keyboards.tables().commands(&refs)
        };
        let strokes: Vec<(isle_keys::tables::Command, Vec<Stroke>)> = table_commands
            .into_iter()
            .map(|c| {
                let s = self.isle.keyboards.tables().strokes_for(&refs, &c.chord);
                (c, s)
            })
            .collect();
        let chord = |id: &str| {
            if game {
                None
            } else {
                self.isle.shortcut_chord(id)
            }
        };
        let (hide, hide_others, quit, minimise, full, close) = (
            chord("hide"),
            chord("hide-others"),
            chord("quit-app"),
            chord("minimise"),
            chord("full-screen"),
            chord("close-window"),
        );
        let entry = self.isle.apps.get(&app).cloned();
        let focused = self.bar_window();
        let menus_of = focused.map(|id| self.menus_window(id));
        let real: Option<isle_menus::Tree> = menus_of.and_then(|id| match self.isle.menus.trees.get(&id) {
            Some(crate::menus::Found::Menus(_, tree)) => Some(tree.clone()),
            _ => None,
        });
        // Each shortcut the app declares, as the Mac chord the user presses for it.
        let mut hints: HashMap<Vec<String>, Chord> = HashMap::new();
        if let Some(tree) = &real
            && !game
        {
            for (path, item) in tree.walk() {
                let Some(words) = item.shortcut.as_ref().and_then(crate::menus::stroke_words) else {
                    continue;
                };
                let Some(stroke) = Stroke::parse(&words) else {
                    continue;
                };
                let chord = self
                    .isle
                    .keyboards
                    .tables()
                    .mac_chord(&refs, &stroke)
                    .or_else(|| Chord::parse(&words));
                if let Some(chord) = chord {
                    hints.insert(path, chord);
                }
            }
        }
        let strokes = if real.is_some() { Vec::new() } else { strokes };
        let find_real = |labels: &[&str]| -> Option<Vec<String>> {
            let tree = real.as_ref()?;
            tree.walk()
                .into_iter()
                .find(|(_, item)| labels.iter().any(|l| item.label.starts_with(l)) && item.children.is_none())
                .map(|(path, _)| path)
        };
        let real_about = find_real(&["About"]);
        let real_settings = find_real(&["Settings", "Preferences"]);
        if !self.isle.bar.links.contains_key(&app) && !app.is_empty() {
            self.fetch_links(&app);
        }
        let links = self.isle.bar.links.get(&app).cloned().unwrap_or_default();
        let window_entries = focused.map(|id| self.window_entries(id, true));

        let bar = &mut self.isle.bar;
        bar.commands.clear();
        let mut b = Builder {
            commands: &mut bar.commands,
            next: &mut bar.next_id,
            labels,
        };
        let mut titles = Vec::new();

        let system = vec![
            b.item("About This Computer", Some(Command::Settings(Some("general"))), None),
            MenuEntry::Separator,
            b.item("System Settings…", Some(Command::Settings(None)), None),
            b.item("Software Update…", None, None),
            MenuEntry::Separator,
            b.item(
                "Force Quit…",
                Some(Command::ForceQuit),
                Chord::parse("Option+Cmd+Escape"),
            ),
            MenuEntry::Separator,
            b.item("Sleep", Some(Command::Sleep), None),
            b.item(
                "Restart…",
                Some(Command::EndSession(Choice::Restart)),
                None,
            ),
            b.item(
                "Shut Down…",
                Some(Command::EndSession(Choice::ShutDown)),
                None,
            ),
            MenuEntry::Separator,
            b.item("Lock Screen", Some(Command::LockScreen), Chord::parse("Ctrl+Cmd+Q")),
            b.item(
                format!("Log Out {}…", whoami()),
                Some(Command::EndSession(Choice::LogOut)),
                Chord::parse("Shift+Cmd+Q"),
            ),
        ];
        titles.push(Title {
            label: "System".into(),
            kind: TitleKind::System,
            enabled: true,
            menu: Rc::new(system),
        });

        let settings = strokes
            .iter()
            .find(|(c, _)| c.chord == Chord::parse("Cmd+,").unwrap())
            .map(|(_, s)| Command::Strokes(s.clone()));
        let about = match (menus_of, real_about) {
            (Some(id), Some(path)) => Command::Remote(id, path),
            _ => Command::About,
        };
        let settings = match (menus_of, real_settings) {
            (Some(id), Some(path)) => Some(Command::Remote(id, path)),
            _ => settings,
        };
        let mut app_menu = vec![
            b.item(format!("About {name}"), Some(about), None),
            MenuEntry::Separator,
            b.item("Settings…", settings, Chord::parse("Cmd+,")),
            MenuEntry::Separator,
        ];
        if let Some(entry) = &entry {
            for action in &entry.actions {
                app_menu.push(b.item(
                    action.name.clone(),
                    Some(Command::EntryAction(action.id.clone())),
                    None,
                ));
            }
            app_menu.push(MenuEntry::Separator);
        }
        app_menu.extend([
            b.item(format!("Hide {name}"), focused.map(|_| Command::Hide), hide),
            b.item("Hide Others", Some(Command::HideOthers), hide_others),
            b.item("Show All", Some(Command::ShowAll), None),
            MenuEntry::Separator,
            b.item(format!("Quit {name}"), focused.map(|_| Command::Quit), quit),
        ]);
        titles.push(Title {
            label: name.clone(),
            kind: TitleKind::App,
            enabled: true,
            menu: Rc::new(app_menu),
        });

        // The app's commands, under their menus in the order the tables name them.
        let mut order: Vec<String> = Vec::new();
        for (c, _) in &strokes {
            if c.menu != "App" && !order.contains(&c.menu) {
                order.push(c.menu.clone());
            }
        }
        for menu in order {
            let mut entries: Vec<MenuEntry> = strokes
                .iter()
                .filter(|(c, _)| c.menu == menu)
                .map(|(c, s)| {
                    b.item(
                        c.label.clone(),
                        Some(Command::Strokes(s.clone())),
                        Some(c.chord),
                    )
                })
                .collect();
            if menu == "File"
                && let Some(id) = focused
            {
                entries.push(MenuEntry::Separator);
                entries.push(b.item(
                    "Close Window",
                    Some(Command::Window(id, WindowCommand::Close)),
                    close,
                ));
            }
            titles.push(Title {
                label: menu,
                kind: TitleKind::Menu,
                enabled: true,
                menu: Rc::new(entries),
            });
        }

        // The app's own menus, but Window and Help, which join Isle's.
        let mut real_window = Vec::new();
        let mut real_help = Vec::new();
        if let (Some(tree), Some(id)) = (&real, menus_of) {
            for menu in &tree.menus {
                let mut path = vec![menu.label.clone()];
                let entries = b.remote(id, menu.children.as_deref().unwrap_or(&[]), &mut path, &hints);
                match menu.label.as_str() {
                    "Window" => real_window = entries,
                    "Help" => real_help = entries,
                    _ => titles.push(Title {
                        label: menu.label.clone(),
                        kind: TitleKind::Menu,
                        enabled: menu.enabled,
                        menu: Rc::new(entries),
                    }),
                }
            }
        }

        let window = match window_entries {
            Some(rows) => {
                let mut entries = b.rows(rows);
                if !real_window.is_empty() {
                    entries.push(MenuEntry::Separator);
                    entries.extend(real_window);
                }
                entries
            }
            None => vec![
                b.item("Minimise", None, minimise),
                b.item("Zoom", None, None),
                b.item("Enter Full Screen", None, full),
            ],
        };
        titles.push(Title {
            label: "Window".into(),
            kind: TitleKind::Menu,
            enabled: true,
            menu: Rc::new(window),
        });

        let help = if real_help.is_empty() {
            vec![
                b.item(format!("{name} Help"), links.help.map(Command::Link), None),
                b.item(format!("{name} Website"), links.homepage.map(Command::Link), None),
            ]
        } else {
            real_help
        };
        titles.push(Title {
            label: "Help".into(),
            kind: TitleKind::Menu,
            enabled: true,
            menu: Rc::new(help),
        });
        titles
    }

    /// The Window menu's rows for `id`; `bar` adds the app's windows and Bring All to Front,
    /// the title bar's adds Move, Resize and Close.
    fn window_entries(&mut self, id: u64, bar: bool) -> Vec<Row> {
        let chord = |s: &Self, n: &str| s.isle.shortcut_chord(n);
        let w = |c: WindowCommand| Some(Command::Window(id, c));
        let full_screen = matches!(
            self.isle.wm.get(id).map(|r| &r.mode),
            Some(Mode::FullScreen { .. })
        );
        let keep = self.isle.wm.get(id).is_some_and(|r| r.keep_on_top);
        let mut out = vec![
            Row::new(
                "Minimise",
                w(WindowCommand::Minimise),
                chord(self, "minimise"),
            ),
            Row::new("Zoom", w(WindowCommand::Zoom), None),
            Row::new(
                "Fill",
                w(WindowCommand::Tile(Zone::Fill)),
                chord(self, "fill"),
            ),
            Row::new(
                "Centre",
                w(WindowCommand::Tile(Zone::Centre)),
                chord(self, "tile-centre"),
            ),
        ];
        let zones = [
            ("Left half", Zone::LeftHalf, chord(self, "tile-left")),
            ("Right half", Zone::RightHalf, chord(self, "tile-right")),
            ("Left third", Zone::LeftThird, None),
            ("Centre third", Zone::CentreThird, None),
            ("Right third", Zone::RightThird, None),
            ("Left two-thirds", Zone::LeftTwoThirds, None),
            ("Right two-thirds", Zone::RightTwoThirds, None),
            ("Top left", Zone::TopLeft, None),
            ("Top right", Zone::TopRight, None),
            ("Bottom left", Zone::BottomLeft, None),
            ("Bottom right", Zone::BottomRight, None),
        ];
        let mut sub: Vec<Row> = zones
            .iter()
            .map(|(l, z, c)| Row::new(*l, w(WindowCommand::Tile(*z)), *c))
            .collect();
        sub.push(Row::separator());
        sub.push(Row::new(
            "Return to previous size",
            w(WindowCommand::Restore),
            chord(self, "restore"),
        ));
        out.push(Row::sub("Move and resize", sub));
        out.push(Row::sub(
            "Full screen tile",
            vec![
                Row::new("Left of screen", None, None),
                Row::new("Right of screen", None, None),
            ],
        ));
        out.push(Row::new(
            if full_screen {
                "Exit Full Screen"
            } else {
                "Enter Full Screen"
            },
            w(WindowCommand::FullScreen),
            chord(self, "full-screen"),
        ));
        out.push(Row::separator());
        if let Some(output) = self.isle.output_of(id) {
            let identity = Isle::output_identity(&output);
            let row = self.isle.desktops.rows.get(&identity);
            let count = row.map_or(1, |r| r.desktops.len());
            let current = row.map_or(0, |r| r.current);
            let mut desktops: Vec<Row> = (0..count)
                .map(|i| {
                    Row::new(
                        format!("Desktop {}", i + 1),
                        w(WindowCommand::ToDesktop(i)),
                        None,
                    )
                    .mark(if i == current {
                        Mark::Check
                    } else {
                        Mark::None
                    })
                })
                .collect();
            desktops.push(Row::separator());
            desktops.push(Row::new("New desktop", w(WindowCommand::NewDesktop), None));
            out.push(Row::sub("Move to desktop", desktops));
            let others: Vec<String> = self
                .isle
                .space
                .outputs()
                .filter(|o| *o != &output)
                .map(|o| o.name())
                .collect();
            for (i, name) in others.iter().enumerate() {
                out.push(Row::new(
                    format!("Move to {name}"),
                    w(WindowCommand::ToDisplay(i as i32 + 1)),
                    None,
                ));
            }
        }
        out.push(
            Row::new("Keep on top", w(WindowCommand::KeepOnTop), None).mark(if keep {
                Mark::Check
            } else {
                Mark::None
            }),
        );
        if bar {
            out.push(Row::separator());
            out.push(Row::new(
                "Bring All to Front",
                w(WindowCommand::BringAllToFront),
                None,
            ));
            let app = self
                .isle
                .wm
                .get(id)
                .map(|r| r.app.clone())
                .unwrap_or_default();
            let windows: Vec<(u64, String, bool)> = self
                .isle
                .wm
                .records
                .iter()
                .filter(|r| r.app == app && r.mapped && r.parent.is_none())
                .map(|r| (r.id, r.names().1, r.minimised))
                .collect();
            if !windows.is_empty() {
                out.push(Row::separator());
            }
            for (wid, title, minimised) in windows {
                let mark = if wid == id {
                    Mark::Check
                } else if minimised {
                    Mark::Diamond
                } else {
                    Mark::None
                };
                out.push(
                    Row::new(
                        title,
                        Some(Command::Window(wid, WindowCommand::Focus)),
                        None,
                    )
                    .mark(mark),
                );
            }
        } else {
            out.push(Row::separator());
            out.push(Row::new("Move", w(WindowCommand::Move), None));
            out.push(Row::new("Resize", w(WindowCommand::Resize), None));
            out.push(Row::new(
                "Close",
                w(WindowCommand::Close),
                chord(self, "close-window"),
            ));
        }
        out
    }

    fn fetch_links(&mut self, app: &str) {
        self.isle
            .bar
            .links
            .insert(app.to_owned(), Default::default());
        let (Some(runtime), Some(to_loop)) = (self.isle.worker.clone(), self.isle.to_loop.clone())
        else {
            return;
        };
        let app = app.to_owned();
        runtime.spawn(async move {
            let id = app.clone();
            let links = tokio::task::spawn_blocking(move || {
                isle_apps::metainfo::links(&id, &isle_apps::metainfo::dirs())
            })
            .await
            .unwrap_or_default();
            to_loop.post(move |state| {
                state.isle.bar.links.insert(app, links);
                if state.isle.bar.output.is_some() {
                    state.refresh_bar();
                }
            });
        });
    }

    /// Reveals the bar on `output`; from the keyboard with `open` and focus on the titles.
    pub fn reveal_bar(&mut self, output: &Output, by_key: bool, open: Option<usize>) {
        if self.isle.island.panel.is_some() {
            self.close_control_centre();
        }
        let name = output.name();
        if self.isle.bar.output.as_ref().is_some_and(|o| o != &name) {
            self.hide_bar();
        }
        if self.isle.bar.output.is_none() {
            self.isle.bar.window = self.focused_window();
        }
        if let Some(leaving) = self.isle.bar.leaving_on.take()
            && leaving.name() != name
        {
            self.isle.shell.set_open_on("menu-bar", &leaving, false);
        }
        self.isle.bar.titles = self.build_bar();
        self.isle.bar.output = Some(name);
        self.isle.bar.by_key = by_key;
        self.isle.bar.open = open;
        self.isle.bar.focused = if by_key {
            Some(open.unwrap_or(0))
        } else {
            None
        };
        self.isle.bar.generation += 1;
        let theme = self.isle.shell.theme().clone();
        let instant = !self.isle.shell.animated() || self.isle.island_game_mode();
        if instant || theme.appearance.reduce_motion {
            self.isle.bar.shown = Motion::at(1.0);
        } else {
            self.isle.bar.shown.spring_to(1.0, theme.spring_open());
        }
        self.isle.bar.last = Instant::now();
        self.isle
            .bar
            .surface
            .set_height(isle_shell::menubar::band_height(&theme));
        self.isle.shell.set_open_on("menu-bar", output, true);
        if by_key || open.is_some() {
            self.focus_bar(output);
        }
        self.push_bar_frame();
    }

    /// The bar's menus again with a menu kept open, as the app filled one of its submenus.
    pub fn refresh_bar_keeping_open(&mut self) {
        self.refresh_bar();
    }

    /// The bar's menus again, as the front app changed or its links arrived.
    pub fn refresh_bar(&mut self) {
        if self.isle.bar.output.is_none() {
            return;
        }
        self.isle.bar.titles = self.build_bar();
        self.push_bar_frame();
    }

    pub fn hide_bar(&mut self) {
        let Some(output) = self.bar_output() else {
            return;
        };
        if let Some(token) = self.isle.bar.leave.take() {
            self.isle.loop_handle.remove(token);
        }
        self.isle.bar.open = None;
        self.isle.bar.focused = None;
        self.close_bar_menu();
        self.unfocus_bar();
        let theme = self.isle.shell.theme().clone();
        let instant =
            !self.isle.shell.animated() || self.isle.island_game_mode() || theme.appearance.reduce_motion;
        if instant {
            self.isle.bar.shown = Motion::at(0.0);
        } else {
            self.isle.bar.shown.spring_to(0.0, theme.spring_exit());
        }
        self.isle.bar.last = Instant::now();
        self.push_bar_frame();
        self.isle.bar.output = None;
        if instant {
            self.isle.shell.set_open_on("menu-bar", &output, false);
        } else {
            self.isle.bar.leaving_on = Some(output);
        }
    }

    fn focus_bar(&mut self, output: &Output) {
        let Some(focus) = self.isle.shell.instance_focus("menu-bar", output) else {
            return;
        };
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let previous = keyboard.current_focus();
        if previous == Some(Focus::Shell(focus.clone())) {
            return;
        }
        self.isle
            .shell_keyboard_return
            .push((focus.clone(), previous));
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        self.focus_keyboard(Some(Focus::Shell(focus)), serial);
    }

    fn unfocus_bar(&mut self) {
        let keyboard = self.isle.seat.get_keyboard().unwrap();
        let Some(Focus::Shell(focus)) = keyboard.current_focus() else {
            return;
        };
        if !matches!(self.isle.shell.name_of(&focus), Some("menu-bar" | "bar-menu")) {
            return;
        }
        match self
            .isle
            .shell_keyboard_return
            .iter()
            .position(|(f, _)| *f == focus)
            .map(|i| self.isle.shell_keyboard_return.remove(i).1)
        {
            Some(Some(previous)) if smithay::utils::IsAlive::alive(&previous) => {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.focus_keyboard(Some(previous), serial);
            }
            _ => self.focus_next(),
        }
    }

    fn push_bar_frame(&mut self) {
        let theme = self.isle.shell.theme().clone();
        let t = &theme.tokens;
        let solid = theme.appearance.reduce_transparency || self.isle.island_game_mode();
        let Some(name) = self.isle.bar.output.clone() else {
            return;
        };
        let width = self
            .isle
            .space
            .outputs()
            .find(|o| o.name() == name)
            .and_then(|o| self.isle.space.output_geometry(o))
            .map_or(1920.0, |g| g.size.w as f32);
        let reserve_w = t.size_island_max + 2.0 * t.size_island_guard;
        let reserve = ((width - reserve_w) / 2.0, (width + reserve_w) / 2.0);
        let titles = self.fold_titles(&theme, width, reserve);
        let frame = BarFrame {
            shown: self.isle.bar.shown.value() as f32,
            titles,
            open: self.isle.bar.open,
            focused: self.isle.bar.focused,
            by_key: self.isle.bar.by_key,
            reserve,
            solid,
        };
        let surface = self.isle.bar.surface.clone();
        surface.set_model(&name, frame.clone());
        self.isle.shell.with_roots("menu-bar", |output| {
            if output.name() == name {
                surface.set(&name, frame.clone());
            }
        });
        let output = self.isle.space.outputs().find(|o| o.name() == name).cloned();
        if let Some(output) = output {
            self.isle.queue_redraw(&output);
            self.sync_bar_menu(&theme, &output, frame);
        }
    }

    /// The open menu's surface under its title, opened, moved or closed to match `frame`.
    fn sync_bar_menu(&mut self, theme: &isle_tokens::Theme, output: &Output, frame: BarFrame) {
        let Some(open) = frame.open.filter(|i| *i < frame.titles.len()) else {
            self.close_bar_menu();
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(output) else {
            return;
        };
        let t = &theme.tokens;
        let labels: Vec<(String, TitleKind)> = frame
            .titles
            .iter()
            .map(|t| (t.label.clone(), t.kind))
            .collect();
        let (cells, _) = isle_shell::menubar::layout(theme, &labels, 0.0, frame.reserve);
        let m = isle_shell::placed_menu::menu_margin(theme);
        let top = (t.size_bar_height + t.space_xs).round() as i32;
        let help = isle_shell::menubar::is_help(&frame.titles, open);
        let size = isle_shell::menubar::bar_menu_size(theme, &frame.titles[open], help, screen.size, top);
        let x = (cells[open].0.round() as i32 - m)
            .min(screen.size.w - size.w)
            .max(0);
        let geometry = Rectangle::new((x, top - m).into(), size);
        let model = BarMenuModel {
            titles: frame.titles,
            open,
            solid: frame.solid,
            by_key: frame.by_key,
            generation: self.isle.bar.generation,
        };
        let menu = self.isle.bar.menu.clone();
        menu.set_model(model.clone());
        self.isle.shell.with_root("bar-menu", || menu.set(model));
        if self.isle.bar.menu_at != Some(geometry) || !self.isle.shell.is_open("bar-menu") {
            let place = isle_shell::OutputPlace {
                output: output.clone(),
                geometry: screen,
                scale: output.current_scale().fractional_scale(),
            };
            self.isle.shell.open_placed("bar-menu", &place, geometry);
            self.isle.bar.menu_at = Some(geometry);
            self.open_surface("bar-menu");
        }
    }

    fn close_bar_menu(&mut self) {
        if self.isle.bar.menu_at.take().is_some() && self.isle.shell.is_open("bar-menu") {
            self.close_surface("bar-menu");
        }
    }

    /// The titles with the app's menus that reach the island's reserve folded into ».
    fn fold_titles(
        &self,
        theme: &isle_tokens::Theme,
        width: f32,
        reserve: (f32, f32),
    ) -> Vec<Title> {
        let titles = &self.isle.bar.titles;
        let labels: Vec<(String, TitleKind)> =
            titles.iter().map(|t| (t.label.clone(), t.kind)).collect();
        let (_, fold) = isle_shell::menubar::layout(theme, &labels, width, reserve);
        let Some(fold) = fold else {
            return titles.clone();
        };
        // System, app, and the last two (Window, Help) never fold: make room for them and ».
        let keep_tail = 2;
        let tail_start = titles.len().saturating_sub(keep_tail);
        let mut first_folded = fold.clamp(2, tail_start);
        loop {
            let mut probe: Vec<(String, TitleKind)> = labels[..first_folded].to_vec();
            probe.push(("»".into(), TitleKind::Overflow));
            probe.extend(labels[tail_start..].iter().cloned());
            let (_, again) = isle_shell::menubar::layout(theme, &probe, width, reserve);
            if again.is_none() || first_folded <= 2 {
                break;
            }
            first_folded -= 1;
        }
        let mut out: Vec<Title> = titles[..first_folded].to_vec();
        let folded: Vec<MenuEntry> = titles[first_folded..tail_start]
            .iter()
            .enumerate()
            .map(|(i, t)| {
                MenuEntry::Item(
                    MenuItem::new(u64::MAX - i as u64, t.label.clone()).submenu((*t.menu).clone()),
                )
            })
            .collect();
        if !folded.is_empty() {
            out.push(Title {
                label: "More menus".into(),
                kind: TitleKind::Overflow,
                enabled: true,
                menu: Rc::new(folded),
            });
        }
        out.extend(titles[tail_start..].iter().cloned());
        out
    }

    /// Moves the bar's slide on to `now`; true while it moves.
    pub fn step_bar(&mut self) -> bool {
        if self.isle.bar.shown.settled() {
            return false;
        }
        let now = Instant::now();
        let dt = now
            .duration_since(self.isle.bar.last)
            .as_secs_f64()
            .min(0.05);
        self.isle.bar.last = now;
        self.isle.bar.shown.step(now, dt);
        match (
            self.isle.bar.output.is_some(),
            self.isle.bar.leaving_on.clone(),
        ) {
            (true, _) => self.push_bar_frame(),
            (false, Some(output)) => {
                // Leaving: the band rides up on its spring, then the surface closes.
                self.isle.bar.output = Some(output.name());
                self.push_bar_frame();
                self.isle.bar.output = None;
                if self.isle.bar.shown.settled() {
                    self.isle.shell.set_open_on("menu-bar", &output, false);
                    self.isle.bar.leaving_on = None;
                }
            }
            (false, None) => {}
        }
        !self.isle.bar.shown.settled()
    }

    pub fn bar_moving(&self) -> bool {
        !self.isle.bar.shown.settled()
    }

    /// Ctrl+F2: the bar on the focused window's screen, the system menu open.
    pub fn menu_bar_chord(&mut self) {
        if self.focus_is_game() {
            return;
        }
        if let Some(output) = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output())
        {
            self.reveal_bar(&output, true, Some(0));
        }
    }

    /// Help's search is opening: the submenus the app fills only when asked are asked for,
    /// a few levels down and at most fifty, so search finds what is in them.
    pub fn crawl_menus(&mut self) {
        let Some(id) = self.bar_window().map(|id| self.menus_window(id)) else {
            return;
        };
        let Some(crate::menus::Found::Menus(_, tree)) = self.isle.menus.trees.get(&id) else {
            return;
        };
        let fills: Vec<Vec<String>> = tree
            .walk()
            .into_iter()
            .filter(|(path, item)| item.fill.is_some() && path.len() <= 4)
            .map(|(path, _)| path)
            .take(50)
            .collect();
        for path in fills {
            self.fill_remote(id, path);
        }
    }

    /// Shift+Cmd+/: the bar with Help open.
    pub fn help_chord(&mut self) {
        self.crawl_menus();
        if let Some(output) = self
            .isle
            .output_with_focus()
            .or_else(|| self.isle.active_output())
        {
            self.reveal_bar(&output, true, None);
            let help = self.isle.bar.titles.len().saturating_sub(1);
            self.isle.bar.open = Some(help);
            self.isle.bar.focused = Some(help);
            self.focus_bar(&output);
            self.push_bar_frame();
        }
    }

    pub fn menu_bar_action(&mut self, action: BarAction) {
        let count = self.isle.bar.titles.len();
        match action {
            BarAction::Title(i) => {
                let open = if self.isle.bar.open == Some(i) {
                    None
                } else {
                    Some(i)
                };
                if open.is_some() && i + 1 == count {
                    self.crawl_menus();
                }
                self.isle.bar.open = open;
                self.isle.bar.focused = Some(i);
                self.isle.bar.by_key = false;
                self.isle.bar.generation += 1;
                if open.is_some()
                    && let Some(output) = self.bar_output()
                {
                    self.focus_bar(&output);
                }
            }
            BarAction::Hover(i) => {
                if self.isle.bar.open.is_some() && self.isle.bar.open != Some(i) {
                    self.isle.bar.open = Some(i);
                    self.isle.bar.focused = Some(i);
                    self.isle.bar.by_key = false;
                    self.isle.bar.generation += 1;
                }
                if let Some(token) = self.isle.bar.leave.take() {
                    self.isle.loop_handle.remove(token);
                }
            }
            BarAction::Step(step) => {
                let at = self.isle.bar.focused.unwrap_or(0) as i32;
                self.isle.bar.focused = Some((at + step).rem_euclid(count.max(1) as i32) as usize);
            }
            BarAction::Close => self.hide_bar(),
            BarAction::Menu(MenuEvent::Chosen(id)) => {
                let command = self.isle.bar.commands.get(&id).cloned();
                self.hide_bar();
                if let Some(command) = command {
                    self.run_command(command);
                }
                return;
            }
            BarAction::Menu(MenuEvent::Opening(id)) => {
                if let Some(Command::Fill(window, path)) = self.isle.bar.commands.get(&id).cloned() {
                    self.fill_remote(window, path);
                }
            }
            BarAction::Menu(MenuEvent::Back) => {
                if self.isle.bar.by_key {
                    // Left at the top level moves to the neighbour; Escape closes the menu.
                    self.isle.bar.open = None;
                } else {
                    self.hide_bar();
                    return;
                }
            }
            BarAction::Menu(MenuEvent::Forward) => {
                let next = match self.isle.bar.open {
                    Some(i) => (i + 1) % count.max(1),
                    None => self.isle.bar.focused.unwrap_or(0),
                };
                self.isle.bar.open = Some(next);
                self.isle.bar.focused = Some(next);
                self.isle.bar.by_key = true;
                self.isle.bar.generation += 1;
            }
        }
        self.push_bar_frame();
    }

    fn bar_output(&self) -> Option<Output> {
        let name = self.isle.bar.output.as_ref()?;
        self.isle
            .space
            .outputs()
            .find(|o| &o.name() == name)
            .cloned()
    }

    fn run_command(&mut self, command: Command) {
        info!(?command, "menu");
        let app = self.isle.bar_app().0;
        match command {
            Command::Sleep => self.isle.suspend(),
            Command::LockScreen => self.lock("system menu"),
            Command::EndSession(choice) => self.open_session_confirm(choice),
            Command::ForceQuit => self.open_force_quit(),
            Command::Settings(pane) => {
                self.open_settings(pane, None);
            }
            Command::About => self.open_about(&app),
            Command::Hide => {
                self.set_app_hidden(&app, true);
            }
            Command::HideOthers => self.hide_others(),
            Command::ShowAll => {
                let apps: Vec<String> = self
                    .isle
                    .wm
                    .records
                    .iter()
                    .filter(|r| r.hidden)
                    .map(|r| r.app.clone())
                    .collect();
                for app in apps {
                    self.set_app_hidden(&app, false);
                }
            }
            Command::Quit => {
                self.quit_app(&app);
            }
            Command::Strokes(strokes) => self.send_strokes(&strokes),
            Command::EntryAction(action) => {
                self.launch(crate::launch::Launch::App {
                    app,
                    action: Some(action),
                    uris: Vec::new(),
                });
            }
            Command::Link(url) => {
                self.launch(crate::launch::Launch::Open(url));
            }
            Command::Window(id, command) => self.window_command(id, command),
            Command::Remote(id, path) => self.activate_remote(id, &path),
            Command::Fill(..) => {}
        }
    }

    fn window_command(&mut self, id: u64, command: WindowCommand) {
        match command {
            WindowCommand::Minimise => {
                self.minimise(id);
            }
            WindowCommand::Zoom => self.toggle_fill(id),
            WindowCommand::Tile(Zone::Fill) => self.toggle_fill(id),
            WindowCommand::Tile(zone) => {
                self.tile(id, zone);
            }
            WindowCommand::Restore => {
                self.restore_box(id);
            }
            WindowCommand::FullScreen => self.toggle_full_screen(id),
            WindowCommand::ToDesktop(index) => {
                self.move_to_desktop(id, index);
            }
            WindowCommand::NewDesktop => {
                if let Some(output) = self.isle.output_of(id)
                    && let Some(index) = self.add_desktop(&output)
                {
                    self.move_to_desktop(id, index);
                }
            }
            WindowCommand::ToDisplay(step) => {
                self.activate(id);
                self.move_to_display(step);
            }
            WindowCommand::KeepOnTop => {
                if let Some(record) = self.isle.wm.get_mut(id) {
                    record.keep_on_top = !record.keep_on_top;
                }
                self.isle.sync_space();
            }
            WindowCommand::BringAllToFront => {
                if let Some(app) = self.isle.wm.get(id).map(|r| r.app.clone()) {
                    self.activate_app(&app);
                }
            }
            WindowCommand::Focus => {
                if self.isle.wm.get(id).is_some_and(|r| r.minimised) {
                    self.unminimise(id);
                }
                self.activate(id);
            }
            WindowCommand::Move | WindowCommand::Resize => {
                if let Some(frame) = self.isle.frame(id) {
                    self.activate(id);
                    self.isle.bar.keyboard_move =
                        Some((id, command == WindowCommand::Resize, frame));
                }
            }
            WindowCommand::Close => {
                self.close_window(id);
            }
        }
    }

    /// The window menu for `id` at `at` in the global space, or by its top-left corner.
    pub fn open_window_menu(
        &mut self,
        id: u64,
        at: Option<smithay::utils::Point<i32, smithay::utils::Logical>>,
        by_key: bool,
    ) {
        let (Some(frame), Some(output)) = (self.isle.frame(id), self.isle.output_of(id)) else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let labels = self.labels();
        let items = self.window_entries(id, false);
        let bar = &mut self.isle.bar;
        bar.window_commands.clear();
        let mut b = Builder {
            commands: &mut bar.window_commands,
            next: &mut bar.next_id,
            labels,
        };
        let entries = b.rows(items);
        bar.window_menu_for = Some(id);
        bar.generation += 1;
        let rows = entries.len();
        let model = MenuModel {
            entries: Rc::new(entries),
            by_key,
            generation: bar.generation,
        };
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::placed_menu::window_menu_size(&theme, rows);
        let at = at.unwrap_or_else(|| {
            (
                frame.loc.x + 8,
                frame.loc.y + self.isle.bar_height(id).max(8),
            )
                .into()
        });
        let rect = crate::wm::geometry::clamp_whole(Rectangle::new(at, size), screen);
        let menu = self.isle.bar.window_menu.clone();
        menu.set_model(model.clone());
        self.isle.shell.with_root("window-menu", || menu.set(model));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        // The surface leaves room round the menu for its shadow.
        let m = isle_shell::placed_menu::menu_margin(&theme);
        self.isle.shell.open_placed(
            "window-menu",
            &place,
            Rectangle::new(
                rect.loc - screen.loc - Point::<i32, Logical>::from((m, m)),
                rect.size + Size::<i32, Logical>::from((2 * m, 2 * m)),
            ),
        );
        self.open_surface("window-menu");
    }

    /// `entries` in the window menu's surface at `at` in the global space, for an owner
    /// other than a window, who reads the choice before the window menu does.
    pub fn open_menu_at(
        &mut self,
        entries: Vec<isle_ui::MenuEntry>,
        output: &Output,
        at: smithay::utils::Point<i32, smithay::utils::Logical>,
    ) {
        let Some(screen) = self.isle.space.output_geometry(output) else {
            return;
        };
        let bar = &mut self.isle.bar;
        bar.window_commands.clear();
        bar.window_menu_for = None;
        bar.generation += 1;
        let rows = entries.len();
        let model = MenuModel {
            entries: Rc::new(entries),
            by_key: false,
            generation: bar.generation,
        };
        let theme = self.isle.shell.theme().clone();
        let size = isle_shell::placed_menu::window_menu_size(&theme, rows);
        let rect = crate::wm::geometry::clamp_whole(Rectangle::new(at, size), screen);
        let menu = self.isle.bar.window_menu.clone();
        menu.set_model(model.clone());
        self.isle.shell.with_root("window-menu", || menu.set(model));
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        let m = isle_shell::placed_menu::menu_margin(&theme);
        self.isle.shell.open_placed(
            "window-menu",
            &place,
            Rectangle::new(
                rect.loc - screen.loc - Point::<i32, Logical>::from((m, m)),
                rect.size + Size::<i32, Logical>::from((2 * m, 2 * m)),
            ),
        );
        self.open_surface("window-menu");
    }

    pub fn window_menu_event(&mut self, event: MenuEvent) {
        if self.thumbnail_menu_event(&event) || self.toolbar_menu_event(&event) {
            return;
        }
        match event {
            MenuEvent::Chosen(id) => {
                let command = self.isle.bar.window_commands.get(&id).cloned();
                self.close_surface("window-menu");
                if let Some(command) = command {
                    self.run_command(command);
                }
            }
            MenuEvent::Back => {
                self.close_surface("window-menu");
            }
            MenuEvent::Opening(_) | MenuEvent::Forward => {}
        }
    }

    /// Isle's About panel for `app`, centred on the main screen.
    fn open_about(&mut self, app: &str) {
        let Some(output) = self.isle.main_output() else {
            return;
        };
        let Some(screen) = self.isle.space.output_geometry(&output) else {
            return;
        };
        let entry = self.isle.apps.get(app).cloned();
        let icon = self.isle.app_icon(app, 64);
        let about = About {
            name: self.isle.app_name(app),
            icon,
            version: None,
            package: None,
            comment: entry.as_ref().and_then(|e| e.comment.clone()),
        };
        let panel = self.isle.bar.about.clone();
        panel.set_model(about.clone());
        self.isle
            .shell
            .with_root("about", || panel.set(about.clone()));
        let size = AboutPanel::size();
        let place = isle_shell::OutputPlace {
            output: output.clone(),
            geometry: screen,
            scale: output.current_scale().fractional_scale(),
        };
        let m = isle_shell::placed_menu::about_margin(self.isle.shell.theme());
        self.isle.shell.open_placed(
            "about",
            &place,
            Rectangle::new(
                ((screen.size.w - size.w) / 2 - m, (screen.size.h - size.h) / 3 - m).into(),
                size + Size::<i32, Logical>::from((2 * m, 2 * m)),
            ),
        );
        self.open_surface("about");
        // The package and its version come from pacman, on the worker.
        if let (Some(entry), Some(runtime), Some(to_loop)) =
            (entry, self.isle.worker.clone(), self.isle.to_loop.clone())
        {
            runtime.spawn(async move {
                let out = tokio::process::Command::new("pacman")
                    .args(["-Qo", "--quiet"])
                    .arg(&entry.path)
                    .output()
                    .await;
                let Ok(out) = out else {
                    return;
                };
                let package = String::from_utf8_lossy(&out.stdout).trim().to_owned();
                if package.is_empty() {
                    return;
                }
                let version = tokio::process::Command::new("pacman")
                    .args(["-Q", &package])
                    .output()
                    .await
                    .ok()
                    .and_then(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .split_whitespace()
                            .nth(1)
                            .map(str::to_owned)
                    });
                to_loop.post(move |state| {
                    let panel = state.isle.bar.about.clone();
                    let about = About {
                        version,
                        package: Some(package),
                        ..about
                    };
                    panel.set_model(about.clone());
                    state.isle.shell.with_root("about", || panel.set(about));
                });
            });
        }
    }

    /// A key while a window is moved or resized from the keyboard: arrows move it, Return
    /// keeps it, Escape puts it back. True when the key was taken.
    pub fn keyboard_move_key(&mut self, code: u32) -> bool {
        let Some((id, resize, start)) = self.isle.bar.keyboard_move else {
            return false;
        };
        let step = 16;
        let (dx, dy) = match code {
            105 => (-step, 0),
            106 => (step, 0),
            103 => (0, -step),
            108 => (0, step),
            28 | 96 => {
                self.isle.bar.keyboard_move = None;
                return true;
            }
            1 => {
                self.isle.bar.keyboard_move = None;
                self.set_frame(id, start);
                return true;
            }
            _ => return true,
        };
        if let Some(mut frame) = self.isle.frame(id) {
            if resize {
                frame.size.w = (frame.size.w + dx).max(120);
                frame.size.h = (frame.size.h + dy).max(80);
            } else {
                frame.loc.x += dx;
                frame.loc.y += dy;
            }
            self.set_frame(id, frame);
        }
        true
    }
}

impl Isle {
    /// Sleep through logind, on the worker.
    pub fn suspend(&self) {
        let Some(runtime) = self.worker.clone() else {
            return;
        };
        runtime.spawn(async move {
            let result: zbus::Result<()> = async {
                let bus = zbus::Connection::system().await?;
                bus.call_method(
                    Some("org.freedesktop.login1"),
                    "/org/freedesktop/login1",
                    Some("org.freedesktop.login1.Manager"),
                    "Suspend",
                    &(true,),
                )
                .await?;
                Ok(())
            }
            .await;
            if let Err(err) = result {
                warn!("could not sleep: {err}");
            }
        });
    }
}

fn whoami() -> String {
    std::env::var("USER").unwrap_or_default()
}

/// A menu's entries as JSON, for the bus.
fn entries_json(entries: &[MenuEntry]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|e| match e {
            MenuEntry::Separator => serde_json::json!({ "separator": true }),
            MenuEntry::Section(s) => serde_json::json!({ "section": s }),
            MenuEntry::Item(item) => serde_json::json!({
                "id": item.id,
                "label": item.label,
                "enabled": item.enabled,
                "mark": format!("{:?}", item.mark).to_lowercase(),
                "hint": item.hint.as_ref().map(|h| format!("{}{}", h.modifiers, h.key)),
                "items": item.submenu.as_ref().map(|s| entries_json(s)),
            }),
        })
        .collect()
}

/// The item at `path` of labels in `entries`: a title's menu, then submenus.
fn find_item(entries: &[MenuEntry], path: &[&str]) -> Option<MenuItem> {
    let (first, rest) = path.split_first()?;
    let item = entries
        .iter()
        .filter_map(MenuEntry::item)
        .find(|i| i.label == *first)?;
    if rest.is_empty() {
        return Some(item.clone());
    }
    find_item(item.submenu.as_deref()?, rest)
}

impl State {
    /// The front app's menu items whose labels hold `query`, with their paths and hints, as
    /// JSON: Help's search and Spotlight's menu-item source read the same index.
    pub fn menu_search(&mut self, query: &str) -> String {
        if self.isle.bar.output.is_none() {
            self.isle.bar.titles = self.build_bar();
        }
        let last = self.isle.bar.titles.len().saturating_sub(1);
        let menus: Vec<(String, Rc<Vec<MenuEntry>>)> = self
            .isle
            .bar
            .titles
            .iter()
            .enumerate()
            .filter(|(i, t)| *i != last && t.kind != TitleKind::System)
            .map(|(_, t)| (t.label.clone(), t.menu.clone()))
            .collect();
        let found: Vec<serde_json::Value> = isle_ui::menu::search(&menus, query)
            .into_iter()
            .map(|(path, item)| {
                serde_json::json!({
                    "path": path,
                    "label": item.label,
                    "enabled": item.enabled,
                    "hint": item.hint.map(|h| format!("{}{}", h.modifiers, h.key)),
                })
            })
            .collect();
        serde_json::Value::Array(found).to_string()
    }

    /// The front app's bar as JSON: its titles and their menus, built afresh.
    pub fn bar_json(&mut self) -> String {
        let titles = match self.bar_output() {
            Some(output) => {
                let theme = self.isle.shell.theme().clone();
                let t = &theme.tokens;
                let width = self
                    .isle
                    .space
                    .output_geometry(&output)
                    .map_or(1920.0, |g| g.size.w as f32);
                let reserve_w = t.size_island_max + 2.0 * t.size_island_guard;
                self.fold_titles(
                    &theme,
                    width,
                    ((width - reserve_w) / 2.0, (width + reserve_w) / 2.0),
                )
            }
            None => self.build_bar(),
        };
        serde_json::json!({
            "output": self.isle.bar.output,
            "open": self.isle.bar.open,
            "focused": self.isle.bar.focused,
            "app": self.isle.bar_app().1,
            "titles": titles.iter().map(|t| serde_json::json!({
                "label": t.label,
                "kind": format!("{:?}", t.kind).to_lowercase(),
                "items": entries_json(&t.menu),
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Chooses the item at `path` (a title, then labels down its menus) as a click would;
    /// false when there is none or it is disabled.
    pub fn bar_activate(&mut self, path: &[&str]) -> bool {
        let Some((title, rest)) = path.split_first() else {
            return false;
        };
        if self.isle.bar.output.is_none() {
            self.isle.bar.titles = self.build_bar();
        }
        let Some(menu) = self
            .isle
            .bar
            .titles
            .iter()
            .find(|t| t.label == *title)
            .map(|t| t.menu.clone())
        else {
            return false;
        };
        let Some(item) = find_item(&menu, rest).filter(|i| i.enabled && i.submenu.is_none()) else {
            return false;
        };
        let Some(command) = self.isle.bar.commands.get(&item.id).cloned() else {
            return false;
        };
        self.hide_bar();
        self.run_command(command);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_name_the_keys_the_user_presses() {
        let chord = Chord::parse("Option+Cmd+Escape").unwrap();
        assert_eq!(hint(&chord, Labels::Mac).modifiers, "⌥⌘");
        assert_eq!(hint(&chord, Labels::Pc).key, "Alt+Super+Esc");
        assert_eq!(hint(&chord, Labels::PcSwapped).key, "Super+Alt+Esc");
    }
}
