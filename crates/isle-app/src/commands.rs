//! One command table per app (`apps.md` Conventions 10, 11): menu items and keys are built
//! from it. A chosen item with a chord arrives as that chord's key press, so the item and the
//! key take one path; one without a chord is run directly.

use freya::prelude::{Key, Modifiers};
use isle_menus::export::{Entry, MenuItem};

/// The menus in the order the bar shows them; App is titled with the app's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Menu {
    App,
    File,
    Edit,
    View,
    Go,
    Window,
    Help,
}

impl Menu {
    fn title(self, app: &str) -> String {
        match self {
            Menu::App => app.to_owned(),
            Menu::File => "File".into(),
            Menu::Edit => "Edit".into(),
            Menu::View => "View".into(),
            Menu::Go => "Go".into(),
            Menu::Window => "Window".into(),
            Menu::Help => "Help".into(),
        }
    }
}

/// The key of a chord: what it types without Shift, `,` or `q`, or a key that types nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChordKey {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Return,
    Space,
    Backspace,
    Delete,
    Tab,
    Escape,
}

impl ChordKey {
    /// The key press it is.
    pub fn key(self) -> Key {
        use freya::prelude::NamedKey;
        match self {
            ChordKey::Char(c) => Key::Character(c.to_string()),
            ChordKey::Space => Key::Character(" ".into()),
            ChordKey::Up => Key::Named(NamedKey::ArrowUp),
            ChordKey::Down => Key::Named(NamedKey::ArrowDown),
            ChordKey::Left => Key::Named(NamedKey::ArrowLeft),
            ChordKey::Right => Key::Named(NamedKey::ArrowRight),
            ChordKey::Return => Key::Named(NamedKey::Enter),
            ChordKey::Backspace => Key::Named(NamedKey::Backspace),
            ChordKey::Delete => Key::Named(NamedKey::Delete),
            ChordKey::Tab => Key::Named(NamedKey::Tab),
            ChordKey::Escape => Key::Named(NamedKey::Escape),
        }
    }

    /// XKB's name for it.
    fn name(self) -> String {
        match self {
            ChordKey::Char(c) => key_name(c),
            ChordKey::Up => "Up".into(),
            ChordKey::Down => "Down".into(),
            ChordKey::Left => "Left".into(),
            ChordKey::Right => "Right".into(),
            ChordKey::Return => "Return".into(),
            ChordKey::Space => "space".into(),
            ChordKey::Backspace => "BackSpace".into(),
            ChordKey::Delete => "Delete".into(),
            ChordKey::Tab => "Tab".into(),
            ChordKey::Escape => "Escape".into(),
        }
    }
}

/// A chord as the app's keys and menus name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub cmd: bool,
    pub shift: bool,
    pub option: bool,
    pub ctrl: bool,
    pub key: ChordKey,
}

impl Chord {
    pub const fn cmd(key: char) -> Self {
        Self::cmd_key(ChordKey::Char(key))
    }

    pub const fn cmd_key(key: ChordKey) -> Self {
        Self {
            cmd: true,
            shift: false,
            option: false,
            ctrl: false,
            key,
        }
    }

    /// A key alone, with no modifier: Space for Quick Look, Return to rename.
    pub const fn bare(key: ChordKey) -> Self {
        Self {
            cmd: false,
            ..Self::cmd_key(key)
        }
    }

    pub const fn shift(self) -> Self {
        Self { shift: true, ..self }
    }

    pub const fn option(self) -> Self {
        Self { option: true, ..self }
    }

    pub const fn ctrl(self) -> Self {
        Self { ctrl: true, ..self }
    }

    /// DBusMenu's names for it, as Qt writes them and Isle's menu bar reads them.
    pub fn dbusmenu(&self) -> Vec<String> {
        let mut names: Vec<String> = [
            (self.ctrl, "Control"),
            (self.option, "Alt"),
            (self.shift, "Shift"),
            (self.cmd, "Super"),
        ]
        .into_iter()
        .filter(|(held, _)| *held)
        .map(|(_, name)| name.to_owned())
        .collect();
        names.push(self.key.name());
        names
    }

    pub fn modifiers(&self) -> Modifiers {
        let mut m = Modifiers::empty();
        m.set(Modifiers::META, self.cmd);
        m.set(Modifiers::SHIFT, self.shift);
        m.set(Modifiers::ALT, self.option);
        m.set(Modifiers::CONTROL, self.ctrl);
        m
    }

    /// Whether a key press is this chord.
    pub fn matches(&self, key: &Key, modifiers: Modifiers) -> bool {
        let wanted = self.modifiers();
        let held = modifiers & (Modifiers::META | Modifiers::SHIFT | Modifiers::ALT | Modifiers::CONTROL);
        held == wanted
            && match (key, self.key) {
                (Key::Character(c), ChordKey::Char(k)) => c.to_lowercase() == k.to_lowercase().to_string(),
                (pressed, named) => *pressed == named.key(),
            }
    }
}

/// XKB's name for a key that types `c`.
fn key_name(c: char) -> String {
    match c {
        ',' => "comma",
        '.' => "period",
        '/' => "slash",
        ';' => "semicolon",
        '\'' => "apostrophe",
        '[' => "bracketleft",
        ']' => "bracketright",
        '-' => "minus",
        '=' => "equal",
        '`' => "grave",
        '\\' => "backslash",
        ' ' => "space",
        other => return other.to_string(),
    }
    .to_owned()
}

/// What choosing a command does.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub menu: Menu,
    /// Commands in a menu are grouped by this number, with separators between groups.
    pub group: u8,
    pub chord: Option<Chord>,
    pub enabled: bool,
    pub checked: Option<bool>,
    /// The focused field takes it as a key press: the edit commands.
    pub for_field: bool,
}

impl Command {
    pub fn new(id: impl Into<String>, label: impl Into<String>, menu: Menu, group: u8) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            menu,
            group,
            chord: None,
            enabled: true,
            checked: None,
            for_field: false,
        }
    }

    pub fn chord(mut self, chord: Chord) -> Self {
        self.chord = Some(chord);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// The focused text field takes it as its key press, as the edit commands are.
    pub fn field(mut self) -> Self {
        self.for_field = true;
        self
    }
}

/// The commands every Isle app has, about its own name; `settings` when it has preferences.
pub fn standard(app: &str, settings: bool) -> Vec<Command> {
    let mut list = vec![Command::new("about", format!("About {app}"), Menu::App, 0)];
    if settings {
        list.push(Command::new("settings", "Settings…", Menu::App, 1).chord(Chord::cmd(',')));
    }
    list.extend([
        Command::new("hide", format!("Hide {app}"), Menu::App, 2),
        Command::new("hide-others", "Hide Others", Menu::App, 2),
        Command::new("show-all", "Show All", Menu::App, 2),
        Command::new("quit", format!("Quit {app}"), Menu::App, 3).chord(Chord::cmd('q')),
        Command::new("undo", "Undo", Menu::Edit, 0).chord(Chord::cmd('z')).field(),
        Command::new("redo", "Redo", Menu::Edit, 0)
            .chord(Chord::cmd('z').shift())
            .field(),
        Command::new("cut", "Cut", Menu::Edit, 1).chord(Chord::cmd('x')).field(),
        Command::new("copy", "Copy", Menu::Edit, 1).chord(Chord::cmd('c')).field(),
        Command::new("paste", "Paste", Menu::Edit, 1).chord(Chord::cmd('v')).field(),
        Command::new("select-all", "Select All", Menu::Edit, 1)
            .chord(Chord::cmd('a'))
            .field(),
        Command::new("minimise", "Minimise", Menu::Window, 0),
        Command::new("zoom", "Zoom", Menu::Window, 0),
        Command::new("close", "Close Window", Menu::Window, 1).chord(Chord::cmd('w')),
    ]);
    list
}

/// The menu bar for `commands`, with each item's id its index in `commands` plus one.
pub fn menus(app: &str, commands: &[Command]) -> Vec<MenuItem> {
    let mut kinds: Vec<Menu> = commands.iter().map(|c| c.menu).collect();
    kinds.sort();
    kinds.dedup();
    kinds
        .into_iter()
        .enumerate()
        .map(|(n, menu)| {
            let mut children = Vec::new();
            let mut last_group = None;
            for (i, command) in commands.iter().enumerate().filter(|(_, c)| c.menu == menu) {
                if last_group.is_some_and(|g| g != command.group) {
                    children.push(Entry::Separator);
                }
                last_group = Some(command.group);
                children.push(Entry::Item(MenuItem {
                    enabled: command.enabled,
                    checked: command.checked,
                    shortcut: command.chord.map(|c| c.dbusmenu()),
                    ..MenuItem::new(i as i32 + 1, command.label.clone())
                }));
            }
            MenuItem {
                children,
                ..MenuItem::new(-(n as i32) - 1, menu.title(app))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_chord_is_on_two_commands() {
        let commands = standard("System Settings", true);
        let mut chords: Vec<Chord> = commands.iter().filter_map(|c| c.chord).collect();
        let all = chords.len();
        chords.dedup();
        chords.sort_by_key(|c| (c.key, c.cmd, c.shift, c.option, c.ctrl));
        chords.dedup();
        assert_eq!(chords.len(), all);
    }

    #[test]
    fn menus_group_with_separators_and_name_chords_as_qt_does() {
        let commands = standard("System Settings", true);
        let bar = menus("System Settings", &commands);
        let titles: Vec<&str> = bar.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["System Settings", "Edit", "Window"]);
        let app = &bar[0].children;
        assert!(matches!(app[1], Entry::Separator));
        let Entry::Item(quit) = app.last().unwrap() else { panic!() };
        assert_eq!(quit.label, "Quit System Settings");
        assert_eq!(quit.shortcut.as_deref(), Some(&["Super".to_owned(), "q".to_owned()][..]));
        assert_eq!(
            Chord::cmd(',').dbusmenu(),
            ["Super".to_owned(), "comma".to_owned()]
        );
    }

    #[test]
    fn a_chord_matches_its_key_press_and_no_other() {
        let redo = Chord::cmd('z').shift();
        assert!(redo.matches(&Key::Character("Z".into()), Modifiers::META | Modifiers::SHIFT));
        assert!(!redo.matches(&Key::Character("z".into()), Modifiers::META));
        assert!(!Chord::cmd('q').matches(&Key::Character("q".into()), Modifiers::CONTROL));
    }
}
