//! Spotlight's query, scope, results and selection (`p1/structure/spotlight.md`), with no
//! drawing and no sources of its own, so every rule is tested alone.

use std::collections::{HashMap, HashSet};

pub use isle_search::{Group, Item, Query, Row, Scope};

/// A key, already read: the compositor decides which chords reach Spotlight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Text(String),
    Backspace,
    Delete,
    Left,
    Right,
    /// Cmd+Left and Cmd+Right.
    Home,
    End,
    /// Option+Left and Option+Right.
    WordLeft,
    WordRight,
    Up,
    Down,
    PageUp,
    PageDown,
    /// Cmd+Up and Cmd+Down: the previous or next group.
    GroupUp,
    GroupDown,
    Return,
    /// Shift+Return: copy instead of pasting, the username.
    ShiftReturn,
    /// Cmd+Return: show in Files, run in the terminal.
    CmdReturn,
    Escape,
    Tab,
    /// Cmd+1 to Cmd+4.
    Quick(u8),
    /// Cmd+K.
    Menu,
    /// Cmd+Y.
    Preview,
    /// Cmd+C.
    Copy,
    /// Cmd+V, with the clipboard's text.
    Paste(String),
    /// Cmd+A.
    SelectAll,
    /// Cmd+P.
    Pin,
    /// Cmd+Delete.
    DeleteItem,
    /// Option+Cmd+Left and Right: move the selected favourite.
    MoveFavourite(i32),
}

/// How a chosen row is acted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// Return or a click.
    Default,
    /// Shift+Return.
    Alternate,
    /// Cmd+Return.
    Reveal,
}

/// What the compositor is to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Run the sources for this query: the fast ones at once, the slow ones on the worker.
    Search(Query),
    Choose(Row, How),
    Close,
    Menu(Row),
    Preview(Row),
    Copy(Row),
    Pin(Row),
    Delete(Row),
    MoveFavourite(Row, i32),
}

/// How the scope's results are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    List,
    /// The Apps view and the emoji grid, with this many columns.
    Grid(usize),
}

/// One line of the results: a header or a row.
#[derive(Clone, Debug)]
pub enum Line {
    Header(String),
    Row(Row),
}

/// The query's text and caret, in characters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub text: String,
    pub caret: usize,
    /// The whole text is selected, as a reopened query is: typing replaces it.
    pub all: bool,
}

impl Field {
    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte(&self, at: usize) -> usize {
        self.text
            .char_indices()
            .nth(at)
            .map_or(self.text.len(), |(i, _)| i)
    }

    fn insert(&mut self, s: &str) {
        if std::mem::take(&mut self.all) {
            self.text.clear();
            self.caret = 0;
        }
        let at = self.byte(self.caret);
        self.text.insert_str(at, s);
        self.caret += s.chars().count();
    }

    fn backspace(&mut self) -> bool {
        if std::mem::take(&mut self.all) {
            let had = !self.text.is_empty();
            self.text.clear();
            self.caret = 0;
            return had;
        }
        if self.caret == 0 {
            return false;
        }
        let (from, to) = (self.byte(self.caret - 1), self.byte(self.caret));
        self.text.replace_range(from..to, "");
        self.caret -= 1;
        true
    }

    fn delete(&mut self) -> bool {
        if self.all {
            return self.backspace();
        }
        if self.caret >= self.len() {
            return false;
        }
        let (from, to) = (self.byte(self.caret), self.byte(self.caret + 1));
        self.text.replace_range(from..to, "");
        true
    }

    fn word(&self, forward: bool) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut at = self.caret;
        if forward {
            while at < chars.len() && !chars[at].is_alphanumeric() {
                at += 1;
            }
            while at < chars.len() && chars[at].is_alphanumeric() {
                at += 1;
            }
        } else {
            while at > 0 && !chars[at - 1].is_alphanumeric() {
                at -= 1;
            }
            while at > 0 && chars[at - 1].is_alphanumeric() {
                at -= 1;
            }
        }
        at
    }

    /// Moves the caret; a selected text collapses to the side moved to.
    fn go(&mut self, to: impl FnOnce(&Self) -> usize) {
        self.caret = to(self).min(self.len());
        self.all = false;
    }
}

/// The prefix that turns on a scope when typed first (`structure/spotlight.md` Scopes).
fn prefix_scope(c: char) -> Option<Scope> {
    Some(match c {
        '/' => Scope::Files { contents: false },
        '?' => Scope::Files { contents: true },
        '=' => Scope::Calculator,
        '.' => Scope::Emoji,
        ':' => Scope::Clipboard,
        '@' => Scope::Windows,
        '>' => Scope::Run,
        _ => return None,
    })
}

/// Whether the query reads as a sum or a conversion, so the calculation row is placed before
/// the calculator answers (`p10-spotlight.md` choice 14).
pub fn reads_as_sum(query: &str) -> bool {
    let q = query.trim();
    let Some(first) = q.chars().next() else {
        return false;
    };
    if first.is_ascii_digit() || matches!(first, '(' | '-' | '+' | '.' | '√' | 'π') {
        return true;
    }
    const FUNCTIONS: [&str; 10] = [
        "sqrt", "sin", "cos", "tan", "log", "ln", "abs", "exp", "pi", "e^",
    ];
    FUNCTIONS
        .iter()
        .any(|f| q.starts_with(f) && q[f.len()..].starts_with(['(', ' ']))
}

/// Spotlight from opening to closing.
#[derive(Clone, Debug)]
pub struct Spotlight {
    field: Field,
    scope: Scope,
    generation: u64,
    /// Rows by group for the current generation; a slow group's waiting rows apart.
    rows: HashMap<Group, Vec<Row>>,
    waiting: HashMap<Group, Vec<Row>>,
    /// Slow groups still searching for the current generation.
    searching: HashSet<Group>,
    /// The calculation row is placed and waits for its answer.
    calculating: bool,
    selected: Option<String>,
    /// The user moved the selection since the last keystroke; until then it follows the
    /// first row.
    moved: bool,
    /// The results panel is up; it stays at its height until the query is cleared.
    panel: bool,
    /// Return was pressed before anything could be chosen.
    pending_return: bool,
    /// The general query kept from the last time, and the one Tab left for an app.
    kept: Option<String>,
    before_app: Option<String>,
    columns: usize,
}

impl Spotlight {
    /// Opened in `scope`; the general search comes back with `kept` selected, so typing
    /// replaces it and Return repeats it.
    pub fn open(scope: Scope, kept: Option<String>, columns: usize) -> (Self, Vec<Effect>) {
        let mut field = Field::default();
        if scope == Scope::General
            && let Some(text) = kept.clone().filter(|t| !t.is_empty())
        {
            field.caret = text.chars().count();
            field.text = text;
            field.all = true;
        }
        let mut s = Self {
            field,
            scope,
            generation: 0,
            rows: HashMap::new(),
            waiting: HashMap::new(),
            searching: HashSet::new(),
            calculating: false,
            selected: None,
            moved: false,
            panel: false,
            pending_return: false,
            kept,
            before_app: None,
            columns: columns.max(1),
        };
        let effects = s.search();
        (s, effects)
    }

    pub fn field(&self) -> &Field {
        &self.field
    }

    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn query(&self) -> Query {
        Query {
            text: self.field.text.clone(),
            scope: self.scope.clone(),
            generation: self.generation,
        }
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn panel(&self) -> bool {
        self.panel
    }

    pub fn searching(&self) -> bool {
        !self.searching.is_empty()
    }

    pub fn calculating(&self) -> bool {
        self.calculating
    }

    pub fn view(&self) -> View {
        match self.scope {
            Scope::Apps | Scope::Emoji => View::Grid(self.columns),
            _ => View::List,
        }
    }

    /// The general query to keep when Spotlight closes; a scoped one is never kept.
    pub fn keep(&self) -> Option<String> {
        match self.scope {
            Scope::General => Some(self.field.text.clone()).filter(|t| !t.is_empty()),
            Scope::App(_) => self.before_app.clone(),
            _ => self.kept.clone(),
        }
    }

    /// Whether the scope token shows, and its name for the one-app scope is the app's.
    pub fn scoped(&self) -> bool {
        self.scope != Scope::General
    }

    /// A new query: everything from the old one goes, the selection goes back to the top.
    fn search(&mut self) -> Vec<Effect> {
        self.generation += 1;
        self.rows.clear();
        self.waiting.clear();
        self.searching.clear();
        self.selected = None;
        self.moved = false;
        self.pending_return = false;
        let general = self.scope == Scope::General;
        self.calculating =
            (general && reads_as_sum(&self.field.text)) || self.scope == Scope::Calculator;
        if self.field.text.is_empty() && general {
            self.panel = false;
        }
        vec![Effect::Search(self.query())]
    }

    /// Slow groups the compositor started for the current query.
    pub fn started(&mut self, generation: u64, groups: impl IntoIterator<Item = Group>) {
        if generation == self.generation {
            self.searching.extend(groups);
        }
    }

    /// A source's rows for `generation`. A fast group lands at once; a slow one lands only
    /// below the selected row, else waits for the next keystroke (`shell.md` Spotlight 5).
    pub fn answer(
        &mut self,
        generation: u64,
        group: Group,
        rows: Vec<Row>,
        slow: bool,
    ) -> Vec<Effect> {
        if generation != self.generation {
            return Vec::new();
        }
        if slow {
            self.searching.remove(&group);
        }
        if group == Group::Calculation {
            self.calculating = false;
        }
        let lands_above = slow && group != Group::Calculation && self.moved_past(group);
        if lands_above {
            self.waiting.insert(group, rows);
        } else {
            self.rows.insert(group, rows);
        }
        if !self.rows.values().all(Vec::is_empty) || self.calculating {
            self.panel = true;
        }
        if !self.moved || !self.lines_have(self.selected.as_deref().unwrap_or("")) {
            self.selected = self.first_key();
        }
        if self.pending_return
            && let Some(row) = self.selected_row()
        {
            self.pending_return = false;
            return vec![Effect::Choose(row, How::Default)];
        }
        Vec::new()
    }

    /// Whether rows of `group` would list above the selected row.
    fn moved_past(&self, group: Group) -> bool {
        let Some(selected) = &self.selected else {
            return false;
        };
        let lines = self.lines();
        let at = lines
            .iter()
            .position(|l| matches!(l, Line::Row(r) if &r.key == selected));
        let first_of_group = lines
            .iter()
            .position(|l| matches!(l, Line::Row(r) if r.group > group && r.group != Group::Web));
        match (at, first_of_group) {
            (Some(at), Some(first)) => first <= at,
            (Some(_), None) => self.rows_after(group, selected),
            _ => false,
        }
    }

    /// Whether the selected row belongs to a group listed after `group`.
    fn rows_after(&self, group: Group, selected: &str) -> bool {
        self.rows
            .iter()
            .any(|(g, rows)| *g > group && rows.iter().any(|r| r.key == selected))
    }

    fn lines_have(&self, key: &str) -> bool {
        self.lines()
            .iter()
            .any(|l| matches!(l, Line::Row(r) if r.key == key))
    }

    fn first_key(&self) -> Option<String> {
        self.selectable().first().map(|r| r.key.clone())
    }

    /// The rows in the order they list, without headers.
    pub fn selectable(&self) -> Vec<Row> {
        self.lines()
            .into_iter()
            .filter_map(|l| match l {
                Line::Row(r) => Some(r),
                Line::Header(_) => None,
            })
            .collect()
    }

    pub fn selected_row(&self) -> Option<Row> {
        let key = self.selected.as_ref()?;
        self.selectable().into_iter().find(|r| &r.key == key)
    }

    /// The results as they list (`structure/spotlight.md` The general search): the
    /// calculation, the top hit from the fast groups, each group in its fixed order without
    /// the top hit, the web row last. A scope lists its own rows under their sections.
    pub fn lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut groups: Vec<&Group> = self.rows.keys().collect();
        groups.sort();
        let general = self.scope == Scope::General;
        let mut top: Option<Row> = None;
        if general && !self.field.text.trim().is_empty() {
            top = groups
                .iter()
                .filter(|g| {
                    matches!(
                        g,
                        Group::Apps
                            | Group::Actions
                            | Group::MenuItems
                            | Group::Settings
                            | Group::Windows
                    )
                })
                .filter_map(|g| self.rows[g].first())
                .min_by(|a, b| a.rank.cmp(&b.rank))
                .cloned();
        }
        let calculation = self.rows.get(&Group::Calculation).and_then(|r| r.first());
        if self.calculating || calculation.is_some() {
            lines.push(Line::Header("Calculation".into()));
            match calculation {
                Some(row) => lines.push(Line::Row(row.clone())),
                None => lines.push(Line::Row(calculation_placeholder())),
            }
        }
        if let Some(top) = &top {
            lines.push(Line::Header("Top hit".into()));
            lines.push(Line::Row(top.clone()));
        }
        for group in groups {
            if *group == Group::Calculation {
                continue;
            }
            let mut section: Option<String> = None;
            for row in &self.rows[group] {
                if top.as_ref().is_some_and(|t| t.key == row.key) {
                    continue;
                }
                let header = row
                    .section
                    .clone()
                    .or_else(|| general.then(|| header(*group)).flatten().map(str::to_owned));
                if header != section {
                    if let Some(h) = &header {
                        lines.push(Line::Header(h.clone()));
                    }
                    section = header;
                }
                lines.push(Line::Row(row.clone()));
            }
        }
        lines
    }

    pub fn key(&mut self, key: Key) -> Vec<Effect> {
        let grid = matches!(self.view(), View::Grid(_));
        match key {
            Key::Text(text) => {
                let mut chars = text.chars();
                if self.scope == Scope::General
                    && (self.field.text.is_empty() || self.field.all)
                    && let (Some(c), None) = (chars.next(), chars.next())
                    && let Some(scope) = prefix_scope(c)
                {
                    self.field = Field::default();
                    return self.enter(scope);
                }
                self.field.insert(&text);
                self.search()
            }
            Key::Paste(text) => {
                let line = text.lines().next().unwrap_or("").to_owned();
                if line.is_empty() {
                    return Vec::new();
                }
                self.field.insert(&line);
                self.search()
            }
            Key::Backspace => {
                if self.field.text.is_empty() && self.scope != Scope::General {
                    return self.leave();
                }
                if self.field.backspace() {
                    self.search()
                } else {
                    Vec::new()
                }
            }
            Key::Delete => {
                if self.field.delete() {
                    self.search()
                } else {
                    Vec::new()
                }
            }
            Key::Left if grid => self.step_grid(-1, 0),
            Key::Right if grid => self.step_grid(1, 0),
            Key::Up if grid => self.step_grid(0, -1),
            Key::Down if grid => self.step_grid(0, 1),
            Key::Left => self.caret(|f| if f.all { 0 } else { f.caret.saturating_sub(1) }),
            Key::Right => self.caret(|f| if f.all { f.len() } else { f.caret + 1 }),
            Key::Home => self.caret(|_| 0),
            Key::End => self.caret(Field::len),
            Key::WordLeft => self.caret(|f| f.word(false)),
            Key::WordRight => self.caret(|f| f.word(true)),
            Key::SelectAll => {
                self.field.all = !self.field.text.is_empty();
                self.field.caret = self.field.len();
                Vec::new()
            }
            Key::Up => self.step(-1),
            Key::Down => self.step(1),
            Key::PageUp => self.step(-8),
            Key::PageDown => self.step(8),
            Key::GroupUp => self.step_group(false),
            Key::GroupDown => self.step_group(true),
            Key::Return => self.choose(How::Default),
            Key::ShiftReturn => self.choose(How::Alternate),
            Key::CmdReturn => self.choose(How::Reveal),
            Key::Escape => {
                if self.field.text.is_empty() && self.scope == Scope::General {
                    vec![Effect::Close]
                } else if self.field.text.is_empty() {
                    self.leave()
                } else {
                    self.field = Field::default();
                    self.search()
                }
            }
            Key::Tab => match self.selected_row() {
                Some(Row {
                    item: Item::App(app),
                    ..
                }) if self.scope == Scope::General => {
                    self.before_app = Some(self.field.text.clone());
                    self.field = Field::default();
                    self.enter(Scope::App(app))
                }
                _ => Vec::new(),
            },
            Key::Quick(n) => {
                let scope = match n {
                    1 => Scope::Apps,
                    2 => Scope::Files { contents: false },
                    3 => Scope::Actions,
                    4 => Scope::Clipboard,
                    _ => return Vec::new(),
                };
                let same = match (&self.scope, &scope) {
                    (Scope::Files { .. }, Scope::Files { .. }) => true,
                    (a, b) => a == b,
                };
                if same {
                    self.leave()
                } else {
                    self.enter(scope)
                }
            }
            Key::Menu => self.selected_row().map(Effect::Menu).into_iter().collect(),
            Key::Preview => self
                .selected_row()
                .map(Effect::Preview)
                .into_iter()
                .collect(),
            Key::Copy => self.selected_row().map(Effect::Copy).into_iter().collect(),
            Key::Pin => self.selected_row().map(Effect::Pin).into_iter().collect(),
            Key::DeleteItem => self
                .selected_row()
                .map(Effect::Delete)
                .into_iter()
                .collect(),
            Key::MoveFavourite(by) => self
                .selected_row()
                .filter(|r| r.section.as_deref() == Some("Favourites"))
                .map(|r| Effect::MoveFavourite(r, by))
                .into_iter()
                .collect(),
        }
    }

    fn caret(&mut self, to: impl FnOnce(&Field) -> usize) -> Vec<Effect> {
        self.field.go(to);
        Vec::new()
    }

    /// Into `scope`, from the general search or another scope.
    fn enter(&mut self, scope: Scope) -> Vec<Effect> {
        if self.scope == Scope::General {
            self.kept = Some(self.field.text.clone()).filter(|t| !t.is_empty());
        }
        self.scope = scope;
        self.field = Field::default();
        self.panel = false;
        self.search()
    }

    /// Back to the general search; out of an app's scope the query comes back as it was.
    fn leave(&mut self) -> Vec<Effect> {
        let text = match &self.scope {
            Scope::App(_) => self.before_app.take(),
            _ => None,
        }
        .unwrap_or_default();
        self.scope = Scope::General;
        self.field = Field {
            caret: text.chars().count(),
            text,
            all: false,
        };
        self.panel = false;
        self.search()
    }

    /// Moves the selection by `by` rows, stopping at the ends.
    fn step(&mut self, by: i32) -> Vec<Effect> {
        let rows = self.selectable();
        if rows.is_empty() {
            return Vec::new();
        }
        let at = self
            .selected
            .as_ref()
            .and_then(|k| rows.iter().position(|r| &r.key == k))
            .unwrap_or(0) as i32;
        let to = (at + by).clamp(0, rows.len() as i32 - 1) as usize;
        self.selected = Some(rows[to].key.clone());
        self.moved = true;
        Vec::new()
    }

    /// The first row of the previous or next group.
    fn step_group(&mut self, forward: bool) -> Vec<Effect> {
        let lines = self.lines();
        let mut starts = Vec::new();
        let mut after_header = false;
        for line in &lines {
            match line {
                Line::Header(_) => after_header = true,
                Line::Row(r) => {
                    if after_header || starts.is_empty() {
                        starts.push(r.key.clone());
                    }
                    after_header = false;
                }
            }
        }
        let rows = self.selectable();
        let at = self
            .selected
            .as_ref()
            .and_then(|k| rows.iter().position(|r| &r.key == k))
            .unwrap_or(0);
        let start_index = |key: &String| rows.iter().position(|r| &r.key == key).unwrap_or(0);
        let target = if forward {
            starts.iter().find(|k| start_index(k) > at)
        } else {
            starts.iter().rev().find(|k| start_index(k) < at)
        };
        if let Some(key) = target {
            self.selected = Some(key.clone());
            self.moved = true;
        }
        Vec::new()
    }

    /// The grid's arrows: across within a section's rows of `columns`, and up and down by a
    /// row, into the next section's nearest column.
    fn step_grid(&mut self, dx: i32, dy: i32) -> Vec<Effect> {
        let cells = self.grid_cells();
        if cells.is_empty() {
            return Vec::new();
        }
        let at = self
            .selected
            .as_ref()
            .and_then(|k| cells.iter().position(|(key, _, _)| key == k))
            .unwrap_or(0);
        let (_, row, col) = cells[at];
        let target = if dx != 0 {
            let to = at as i32 + dx;
            (to >= 0 && (to as usize) < cells.len() && cells[to as usize].1 == row)
                .then_some(to as usize)
        } else {
            let want = row as i32 + dy;
            let in_row: Vec<usize> = (0..cells.len())
                .filter(|i| cells[*i].1 as i32 == want)
                .collect();
            in_row
                .iter()
                .copied()
                .min_by_key(|i| (cells[*i].2 as i32 - col as i32).abs())
        };
        if let Some(to) = target {
            self.selected = Some(cells[to].0.clone());
            self.moved = true;
        }
        Vec::new()
    }

    /// Each grid cell's key, its row counted across sections, and its column.
    pub fn grid_cells(&self) -> Vec<(String, usize, usize)> {
        let mut out = Vec::new();
        let mut row = 0usize;
        let mut col = 0usize;
        let mut started = false;
        for line in self.lines() {
            match line {
                Line::Header(_) => {
                    if started && col > 0 {
                        row += 1;
                    }
                    col = 0;
                }
                Line::Row(r) => {
                    if col == self.columns {
                        row += 1;
                        col = 0;
                    }
                    out.push((r.key, row, col));
                    col += 1;
                    started = true;
                }
            }
        }
        out
    }

    fn choose(&mut self, how: How) -> Vec<Effect> {
        match self.selected_row() {
            Some(row) if row.key == CALCULATING => Vec::new(),
            Some(row) => vec![Effect::Choose(row, how)],
            None if self.searching() || self.calculating => {
                self.pending_return = true;
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    /// A pointer that moved over a row selects it.
    pub fn hover(&mut self, key: &str) {
        if self.lines_have(key) {
            self.selected = Some(key.to_owned());
            self.moved = true;
        }
    }

    /// A click on a row does it, as Return.
    pub fn click(&mut self, key: &str, how: How) -> Vec<Effect> {
        self.hover(key);
        self.choose(how)
    }

    /// The scope a quick-key button, prefix or tile set from outside: Cmd+1's button, the
    /// Apps view's key.
    pub fn set_scope(&mut self, scope: Scope) -> Vec<Effect> {
        if self.scope == scope {
            return Vec::new();
        }
        if scope == Scope::General {
            self.leave()
        } else {
            self.enter(scope)
        }
    }

    /// The Files scope's Names / Contents switch.
    pub fn set_contents(&mut self, contents: bool) -> Vec<Effect> {
        match self.scope {
            Scope::Files { contents: c } if c != contents => {
                self.scope = Scope::Files { contents };
                self.search()
            }
            _ => Vec::new(),
        }
    }

    /// The grid's column count, fixed while open; set again on the next open.
    pub fn columns(&self) -> usize {
        self.columns
    }
}

const CALCULATING: &str = "calc:pending";

fn calculation_placeholder() -> Row {
    Row {
        key: CALCULATING.into(),
        group: Group::Calculation,
        title: "=".into(),
        detail: None,
        trailing: None,
        icon: isle_search::Icon::Glyph("calculator"),
        item: Item::Answer(String::new()),
        section: None,
        rank: placeholder_rank(),
        matched: Vec::new(),
    }
}

fn placeholder_rank() -> isle_search::Rank {
    isle_search::Rank::new(
        &isle_search::Match {
            tier: isle_search::Tier::Prefix,
            score: 0,
            indices: Vec::new(),
        },
        0.0,
        false,
        "",
    )
}

/// The header a group lists under in the general search; the web row has none.
fn header(group: Group) -> Option<&'static str> {
    Some(match group {
        Group::Calculation => "Calculation",
        Group::Apps => "Apps",
        Group::Actions => "Actions",
        Group::MenuItems => "Menu items",
        Group::Settings => "Settings",
        Group::Windows => "Windows",
        Group::Files => "Files",
        Group::Web | Group::Clipboard | Group::Emoji | Group::Run => return None,
    })
}

#[cfg(test)]
mod tests {
    use isle_search::{Icon, Matcher, Rank};

    use super::*;

    fn row(group: Group, key: &str, tier_query: &str) -> Row {
        let m = Matcher::new().find(tier_query, key).unwrap();
        Row {
            key: key.into(),
            group,
            title: key.into(),
            detail: None,
            trailing: None,
            icon: Icon::None,
            item: Item::App(key.into()),
            section: None,
            rank: Rank::new(&m, 0.0, group == Group::Actions, key),
            matched: Vec::new(),
        }
    }

    fn typed(s: &mut Spotlight, text: &str) -> u64 {
        for c in text.chars() {
            s.key(Key::Text(c.to_string()));
        }
        s.generation()
    }

    fn keys(s: &Spotlight) -> Vec<String> {
        s.lines()
            .iter()
            .map(|l| match l {
                Line::Header(h) => format!("# {h}"),
                Line::Row(r) => r.key.clone(),
            })
            .collect()
    }

    fn opened() -> Spotlight {
        Spotlight::open(Scope::General, None, 8).0
    }

    #[test]
    fn every_key_searches_and_the_top_hit_leads_the_fixed_groups() {
        let mut s = opened();
        let g = typed(&mut s, "lock");
        s.answer(
            g,
            Group::Apps,
            vec![
                row(Group::Apps, "Lockpick", "lock"),
                row(Group::Apps, "Unlock Tool", "lock"),
            ],
            false,
        );
        s.answer(
            g,
            Group::Actions,
            vec![row(Group::Actions, "Lock Screen", "lock")],
            false,
        );
        s.answer(g, Group::Web, vec![row(Group::Web, "web", "")], false);
        assert_eq!(
            keys(&s),
            [
                "# Top hit",
                "Lock Screen",
                "# Apps",
                "Lockpick",
                "Unlock Tool",
                "web"
            ],
            "the action wins the tie, and is not repeated in its group"
        );
        assert_eq!(s.selected(), Some("Lock Screen"));
    }

    #[test]
    fn answers_to_an_older_query_are_dropped() {
        let mut s = opened();
        let old = typed(&mut s, "f");
        let new = typed(&mut s, "i");
        assert!(new > old);
        s.answer(
            old,
            Group::Apps,
            vec![row(Group::Apps, "Files", "f")],
            false,
        );
        assert!(s.lines().is_empty());
    }

    #[test]
    fn the_calculation_row_is_placed_before_its_answer() {
        let mut s = opened();
        let g = typed(&mut s, "12*48");
        assert!(s.calculating());
        assert_eq!(keys(&s), ["# Calculation", CALCULATING]);
        assert!(s.key(Key::Return).is_empty(), "nothing to copy yet");
        let mut answer = row(Group::Calculation, "calc", "");
        answer.title = "576".into();
        let effects = s.answer(g, Group::Calculation, vec![answer], true);
        assert_eq!(keys(&s), ["# Calculation", "calc"]);
        assert!(
            matches!(&effects[..], [Effect::Choose(r, How::Default)] if r.title == "576"),
            "Return waited for it"
        );
    }

    #[test]
    fn a_slow_answer_never_lands_above_the_selected_row() {
        let mut s = opened();
        let g = typed(&mut s, "fi");
        s.started(g, [Group::Files]);
        assert!(s.searching());
        s.answer(
            g,
            Group::Apps,
            vec![
                row(Group::Apps, "Firefox", "fi"),
                row(Group::Apps, "Fish", "fi"),
            ],
            false,
        );
        s.answer(g, Group::Web, vec![row(Group::Web, "web", "")], false);
        s.key(Key::Down);
        s.key(Key::Down);
        assert_eq!(s.selected(), Some("web"));
        s.answer(
            g,
            Group::Files,
            vec![row(Group::Files, "file.txt", "fi")],
            true,
        );
        assert!(
            !keys(&s).contains(&"file.txt".to_string()),
            "it waits for the next keystroke"
        );
        assert_eq!(s.selected(), Some("web"));
        assert!(!s.searching());

        let mut s = opened();
        let g = typed(&mut s, "fi");
        s.answer(
            g,
            Group::Apps,
            vec![row(Group::Apps, "Firefox", "fi")],
            false,
        );
        s.answer(
            g,
            Group::Files,
            vec![row(Group::Files, "file.txt", "fi")],
            true,
        );
        assert_eq!(
            keys(&s),
            ["# Top hit", "Firefox", "# Files", "file.txt"],
            "below the selection it lands"
        );
        assert_eq!(s.selected(), Some("Firefox"));
    }

    #[test]
    fn return_before_a_slow_scope_answers_acts_on_its_first_row() {
        let mut s = opened();
        s.key(Key::Text("/".into()));
        assert_eq!(s.scope(), &Scope::Files { contents: false });
        let g = typed(&mut s, "notes");
        s.started(g, [Group::Files]);
        assert!(s.key(Key::Return).is_empty());
        let effects = s.answer(
            g,
            Group::Files,
            vec![row(Group::Files, "notes.md", "notes")],
            true,
        );
        assert!(matches!(&effects[..], [Effect::Choose(r, _)] if r.key == "notes.md"));
    }

    #[test]
    fn prefixes_quick_keys_and_backspace_share_one_scope() {
        let mut s = opened();
        for (prefix, scope) in [
            ("=", Scope::Calculator),
            (".", Scope::Emoji),
            (":", Scope::Clipboard),
            ("@", Scope::Windows),
            (">", Scope::Run),
            ("?", Scope::Files { contents: true }),
        ] {
            s.key(Key::Text(prefix.into()));
            assert_eq!(s.scope(), &scope);
            assert!(
                s.field().text.is_empty(),
                "the prefix is not part of the query"
            );
            s.key(Key::Backspace);
            assert_eq!(s.scope(), &Scope::General);
        }
        s.key(Key::Text("*".into()));
        assert_eq!(s.field().text, "*", "the vault waits for the providers");
        s.key(Key::Backspace);
        s.key(Key::Quick(4));
        assert_eq!(s.scope(), &Scope::Clipboard);
        s.key(Key::Quick(4));
        assert_eq!(
            s.scope(),
            &Scope::General,
            "the same quick key again leaves"
        );
        s.key(Key::Quick(1));
        assert_eq!(s.view(), View::Grid(8));
        typed(&mut s, "ab");
        s.key(Key::Text("/".into()));
        assert_eq!(s.field().text, "ab/", "a prefix counts only first");
    }

    #[test]
    fn tab_on_an_app_scopes_to_it_and_backspace_brings_the_query_back() {
        let mut s = opened();
        let g = typed(&mut s, "fire");
        s.answer(
            g,
            Group::Apps,
            vec![row(Group::Apps, "Firefox", "fire")],
            false,
        );
        s.key(Key::Tab);
        assert_eq!(s.scope(), &Scope::App("Firefox".into()));
        assert!(s.field().text.is_empty());
        s.key(Key::Backspace);
        assert_eq!(s.scope(), &Scope::General);
        assert_eq!(s.field().text, "fire");
    }

    #[test]
    fn the_last_general_query_comes_back_selected_and_a_scoped_one_never() {
        let mut s = opened();
        typed(&mut s, "kitty");
        let kept = s.keep();
        assert_eq!(kept.as_deref(), Some("kitty"));
        let (mut s, effects) = Spotlight::open(Scope::General, kept.clone(), 8);
        assert!(matches!(&effects[..], [Effect::Search(q)] if q.text == "kitty"));
        assert!(s.field().all);
        s.key(Key::Text("f".into()));
        assert_eq!(s.field().text, "f", "typing replaces it");

        let (mut s, _) = Spotlight::open(Scope::General, kept, 8);
        s.key(Key::Quick(4));
        typed(&mut s, "secret");
        assert_eq!(
            s.keep().as_deref(),
            Some("kitty"),
            "the clipboard's query is not kept"
        );
        let (s, _) = Spotlight::open(Scope::Emoji, Some("kitty".into()), 8);
        assert!(s.field().text.is_empty());
    }

    #[test]
    fn escape_clears_then_leaves_then_closes() {
        let mut s = opened();
        s.key(Key::Quick(3));
        typed(&mut s, "x");
        s.key(Key::Escape);
        assert!(s.field().text.is_empty());
        assert_eq!(s.scope(), &Scope::Actions);
        s.key(Key::Escape);
        assert_eq!(s.scope(), &Scope::General);
        assert_eq!(s.key(Key::Escape), vec![Effect::Close]);
    }

    #[test]
    fn the_field_edits_like_a_text_field() {
        let mut s = opened();
        typed(&mut s, "helo world");
        s.key(Key::WordLeft);
        s.key(Key::Left);
        s.key(Key::Left);
        s.key(Key::Text("l".into()));
        assert_eq!(s.field().text, "hello world");
        s.key(Key::Home);
        s.key(Key::Delete);
        assert_eq!(s.field().text, "ello world");
        s.key(Key::End);
        s.key(Key::Backspace);
        assert_eq!(s.field().text, "ello worl");
        s.key(Key::SelectAll);
        s.key(Key::Backspace);
        assert_eq!(s.field().text, "");
        s.key(Key::Paste("two\nlines".into()));
        assert_eq!(s.field().text, "two");
        typed(&mut s, "é");
        s.key(Key::Left);
        s.key(Key::Backspace);
        assert_eq!(s.field().text, "twé");
    }

    #[test]
    fn the_selection_stops_at_the_ends_and_jumps_by_group() {
        let mut s = opened();
        let g = typed(&mut s, "a");
        s.answer(
            g,
            Group::Apps,
            vec![
                row(Group::Apps, "Alpha", "a"),
                row(Group::Apps, "Apex", "a"),
            ],
            false,
        );
        s.answer(
            g,
            Group::Windows,
            vec![
                row(Group::Windows, "a window", "a"),
                row(Group::Windows, "another", "a"),
            ],
            false,
        );
        let rows: Vec<String> = s.selectable().into_iter().map(|r| r.key).collect();
        assert_eq!(rows[0], s.selected().unwrap());
        s.key(Key::Up);
        assert_eq!(s.selected(), Some(rows[0].as_str()), "no wrapping");
        s.key(Key::GroupDown);
        s.key(Key::GroupDown);
        let windows_first = s.selected().unwrap().to_owned();
        assert!(
            windows_first.starts_with('a') && s.selected_row().unwrap().group == Group::Windows
        );
        s.key(Key::PageDown);
        assert_eq!(s.selected(), rows.last().map(String::as_str));
        s.key(Key::GroupUp);
        assert_eq!(s.selected(), Some(windows_first.as_str()));
    }

    #[test]
    fn the_grid_moves_within_sections() {
        let (mut s, _) = Spotlight::open(Scope::Apps, None, 3);
        let g = s.generation();
        let mut rows = Vec::new();
        for (i, section) in [
            (0, "Favourites"),
            (1, "Favourites"),
            (2, "All apps"),
            (3, "All apps"),
            (4, "All apps"),
            (5, "All apps"),
        ] {
            let mut r = row(Group::Apps, &format!("app{i}"), "");
            r.section = Some(section.into());
            rows.push(r);
        }
        s.answer(g, Group::Apps, rows, false);
        let cells = s.grid_cells();
        assert_eq!(
            cells.iter().map(|c| (c.1, c.2)).collect::<Vec<_>>(),
            [(0, 0), (0, 1), (1, 0), (1, 1), (1, 2), (2, 0)]
        );
        assert_eq!(s.selected(), Some("app0"));
        s.key(Key::Right);
        s.key(Key::Right);
        assert_eq!(s.selected(), Some("app1"), "no wrapping across a row");
        s.key(Key::Down);
        assert_eq!(s.selected(), Some("app3"));
        s.key(Key::Down);
        assert_eq!(s.selected(), Some("app5"));
        s.key(Key::MoveFavourite(1));
        s.key(Key::Up);
        s.key(Key::Up);
        assert_eq!(s.selected(), Some("app0"));
        assert!(
            matches!(&s.key(Key::MoveFavourite(1))[..], [Effect::MoveFavourite(r, 1)] if r.key == "app0")
        );
    }

    #[test]
    fn the_panel_stays_until_the_query_is_cleared() {
        let mut s = opened();
        let g = typed(&mut s, "f");
        assert!(!s.panel());
        s.answer(g, Group::Apps, vec![row(Group::Apps, "Files", "f")], false);
        assert!(s.panel());
        let g = typed(&mut s, "zz");
        s.answer(g, Group::Apps, Vec::new(), false);
        assert!(s.panel(), "narrowing to nothing keeps its height");
        s.key(Key::SelectAll);
        s.key(Key::Backspace);
        assert!(!s.panel());
    }

    #[test]
    fn sums_are_told_from_words_cheaply() {
        for q in ["12*48", "(3+4)", "-2", "5 km to miles", "sqrt(2)", ".5"] {
            assert!(reads_as_sum(q), "{q}");
        }
        for q in ["firefox", "e", "sin", "pizza"] {
            assert!(!reads_as_sum(q), "{q}");
        }
    }
}
