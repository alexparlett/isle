//! The translation tables (B21): shared rule sets, groups of apps and single apps, each a
//! list of Mac chords and what an app is sent instead, with the app's own close and quit
//! chords. Read from the shipped `data/keys/translation.kdl` with the user's file over it.

use std::fmt;

use crate::{
    chord::Chord,
    engine::{Rules, Stroke},
};

pub const SHIPPED: &str = include_str!("../../../data/keys/translation.kdl");

#[derive(Clone, Debug, PartialEq)]
pub struct TableError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

#[derive(Clone, Debug, Default)]
struct Scope {
    name: String,
    /// The apps a group or app table applies to; empty for a shared set.
    apps: Vec<String>,
    /// Shared sets this scope's apps do without.
    skip: Option<Vec<String>>,
    rules: Rules,
    close: Option<Vec<Stroke>>,
    quit: Option<Vec<Stroke>>,
    commands: Vec<Command>,
}

/// A rule that names itself for the fallback menus: the app's command under a menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub menu: String,
    pub label: String,
    pub chord: Chord,
}

#[derive(Clone, Debug, Default)]
pub struct Tables {
    shared: Vec<Scope>,
    groups: Vec<Scope>,
    apps: Vec<Scope>,
}

/// What an app's keys go through, most specific first.
pub struct AppRules<'a> {
    pub rules: Vec<&'a Rules>,
    /// The app's own chord for the desktop's close and quit, when it has one.
    pub close: Option<&'a [Stroke]>,
    pub quit: Option<&'a [Stroke]>,
}

/// Whether an app's names match a pattern: case-insensitive, with a trailing `*`.
pub fn matches(pattern: &str, names: &[&str]) -> bool {
    let pattern = pattern.to_lowercase();
    names.iter().any(|name| {
        let name = name.to_lowercase();
        match pattern.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix),
            None => name == pattern,
        }
    })
}

/// One rule as the tests and the bus list it: where it is, an app it applies to, the
/// chord and what is sent.
pub struct Listed<'a> {
    pub scope: &'a str,
    pub app: Option<&'a str>,
    pub chord: &'a Chord,
    pub strokes: &'a [Stroke],
}

impl Tables {
    pub fn shipped() -> Self {
        Self::parse(SHIPPED).expect("the shipped translation.kdl parses")
    }

    pub fn parse(text: &str) -> Result<Self, TableError> {
        let at = |offset: usize, message: String| {
            let before = &text[..offset.min(text.len())];
            TableError {
                line: before.matches('\n').count() + 1,
                column: before
                    .rfind('\n')
                    .map_or(before.len(), |n| before.len() - n - 1)
                    + 1,
                message,
            }
        };
        let doc: kdl::KdlDocument = text.parse().map_err(|err: kdl::KdlError| {
            let first = err.diagnostics.first();
            at(
                first.map_or(0, |d| d.span.offset()),
                first
                    .and_then(|d| d.message.clone().or_else(|| d.label.clone()))
                    .unwrap_or_else(|| "this is not valid KDL".into()),
            )
        })?;
        let mut tables = Tables::default();
        for node in doc.nodes() {
            let kind = node.name().value();
            if kind == "version" {
                match node.entries().first().and_then(|e| e.value().as_integer()) {
                    Some(1) => continue,
                    _ => return Err(at(node.span().offset(), "this Isle reads version 1".into())),
                }
            }
            let name = node
                .entries()
                .first()
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| at(node.span().offset(), format!("`{kind}` takes a name")))?
                .to_owned();
            let mut scope = Scope {
                name: name.clone(),
                ..Scope::default()
            };
            if kind == "app" {
                scope.apps = vec![name];
            }
            for child in node.children().map_or(&[][..], |c| c.nodes()) {
                let key = child.name().value();
                let property = |name: &str| {
                    child
                        .entries()
                        .iter()
                        .find(|e| e.name().is_some_and(|n| n.value() == name))
                        .and_then(|e| e.value().as_string())
                        .map(str::to_owned)
                };
                let strings = || -> Result<Vec<String>, TableError> {
                    child
                        .entries()
                        .iter()
                        .filter(|e| e.name().is_none())
                        .map(|e| {
                            e.value().as_string().map(str::to_owned).ok_or_else(|| {
                                at(e.span().offset(), format!("`{key}` takes strings"))
                            })
                        })
                        .collect()
                };
                let strokes = || -> Result<Vec<Stroke>, TableError> {
                    let texts = strings()?;
                    if texts.is_empty() {
                        return Err(at(child.span().offset(), format!("`{key}` sends nothing")));
                    }
                    texts
                        .iter()
                        .map(|t| {
                            Stroke::parse(t).ok_or_else(|| {
                                at(child.span().offset(), format!("`{t}` is not a chord"))
                            })
                        })
                        .collect()
                };
                match key {
                    "apps" if kind == "group" => scope.apps = strings()?,
                    "skip" if kind != "shared" => scope.skip = Some(strings()?),
                    "close" if kind != "shared" => scope.close = Some(strokes()?),
                    "quit" if kind != "shared" => scope.quit = Some(strokes()?),
                    chord => {
                        let parsed = Chord::parse(chord).ok_or_else(|| {
                            at(child.span().offset(), format!("`{chord}` is not a chord"))
                        })?;
                        let named = property("menu").zip(property("label"));
                        // A command with nothing to send goes by the general rule.
                        if named.is_none() || !strings()?.is_empty() {
                            scope.rules.push(parsed, strokes()?);
                        }
                        if let Some((menu, label)) = named {
                            scope.commands.push(Command {
                                menu,
                                label,
                                chord: parsed,
                            });
                        }
                    }
                }
            }
            match kind {
                "shared" => tables.shared.push(scope),
                "group" => tables.groups.push(scope),
                "app" => tables.apps.push(scope),
                other => {
                    return Err(at(
                        node.span().offset(),
                        format!("`{other}` is not shared, group or app"),
                    ));
                }
            }
        }
        Ok(tables)
    }

    /// The user's tables laid over these: a scope of the same kind and name gains the
    /// user's rules chord by chord, and takes the user's apps, skips, close and quit when
    /// the user gives them; new scopes are added.
    pub fn over(mut self, user: Tables) -> Self {
        fn lay(base: &mut Vec<Scope>, user: Vec<Scope>) {
            for scope in user {
                match base.iter_mut().find(|s| s.name == scope.name) {
                    Some(existing) => {
                        if !scope.apps.is_empty() {
                            existing.apps = scope.apps;
                        }
                        if scope.skip.is_some() {
                            existing.skip = scope.skip;
                        }
                        if scope.close.is_some() {
                            existing.close = scope.close;
                        }
                        if scope.quit.is_some() {
                            existing.quit = scope.quit;
                        }
                        for command in scope.commands {
                            existing.commands.retain(|c| c.chord != command.chord);
                            existing.commands.push(command);
                        }
                        for (chord, strokes) in scope.rules.iter() {
                            existing.rules.push(*chord, strokes.to_vec());
                        }
                    }
                    None => base.push(scope),
                }
            }
        }
        lay(&mut self.shared, user.shared);
        lay(&mut self.groups, user.groups);
        lay(&mut self.apps, user.apps);
        self
    }

    /// The rules for an app known by `names` (its desktop id, app id or X11 class).
    pub fn for_app(&self, names: &[&str]) -> AppRules<'_> {
        let scopes: Vec<&Scope> = self
            .apps
            .iter()
            .chain(&self.groups)
            .filter(|s| s.apps.iter().any(|p| matches(p, names)))
            .collect();
        let skipped = |shared: &Scope| {
            scopes
                .iter()
                .filter_map(|s| s.skip.as_ref())
                .any(|skip| skip.contains(&shared.name))
        };
        let mut rules: Vec<&Rules> = scopes.iter().map(|s| &s.rules).collect();
        rules.extend(self.shared.iter().filter(|s| !skipped(s)).map(|s| &s.rules));
        AppRules {
            rules,
            close: scopes.iter().find_map(|s| s.close.as_deref()),
            quit: scopes.iter().find_map(|s| s.quit.as_deref()),
        }
    }

    /// The commands an app's tables name for the fallback menus, most specific first, one
    /// per chord.
    pub fn commands(&self, names: &[&str]) -> Vec<Command> {
        let mut out: Vec<Command> = Vec::new();
        for scope in self
            .apps
            .iter()
            .chain(&self.groups)
            .filter(|s| s.apps.iter().any(|p| matches(p, names)))
        {
            for command in &scope.commands {
                if !out.iter().any(|c| c.chord == command.chord) {
                    out.push(command.clone());
                }
            }
        }
        out
    }

    /// The Mac chord that sends an app `sent`, for a menu item the app declares by it: a
    /// rule giving exactly that one stroke, else Cmd for Ctrl by the general rule when no
    /// rule takes that Cmd chord for something else.
    pub fn mac_chord(&self, names: &[&str], sent: &Stroke) -> Option<Chord> {
        let app = self.for_app(names);
        for rules in &app.rules {
            if let Some((chord, _)) = rules.iter().find(|(_, s)| *s == std::slice::from_ref(sent)) {
                return Some(*chord);
            }
        }
        let m = sent.mods;
        let crate::engine::StrokeKey::Sym(sym) = sent.key else {
            return None;
        };
        if !m.ctrl || m.super_ || m.level3 {
            return None;
        }
        let chord = Chord::new(
            crate::chord::Mods {
                cmd: true,
                shift: m.shift,
                option: m.alt,
                ctrl: false,
            },
            sym,
        );
        let taken = app.rules.iter().any(|r| r.lookup(&chord).is_some());
        (!taken).then_some(chord)
    }

    /// What a Mac chord sends an app: its rule, else Cmd as Ctrl and Option as Alt.
    pub fn strokes_for(&self, names: &[&str], chord: &Chord) -> Vec<Stroke> {
        let app = self.for_app(names);
        if let Some(strokes) = app.rules.iter().find_map(|r| r.lookup(chord)) {
            return strokes.to_vec();
        }
        let m = chord.mods;
        vec![Stroke {
            mods: crate::engine::Sent {
                ctrl: m.cmd || m.ctrl,
                shift: m.shift,
                alt: m.option,
                super_: false,
                level3: false,
            },
            key: crate::engine::StrokeKey::Sym(chord.sym),
        }]
    }

    /// Every rule with an app it applies to; shared rules name none.
    pub fn list(&self) -> Vec<Listed<'_>> {
        let mut out = Vec::new();
        for scope in self.apps.iter().chain(&self.groups).chain(&self.shared) {
            let app = scope
                .apps
                .iter()
                .find(|p| !p.ends_with('*'))
                .map(String::as_str);
            for (chord, strokes) in scope.rules.iter() {
                out.push(Listed {
                    scope: &scope.name,
                    app,
                    chord,
                    strokes,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> Chord {
        Chord::parse(text).unwrap()
    }

    #[test]
    fn named_rules_are_commands_and_one_with_nothing_to_send_takes_the_general_rule() {
        let tables = Tables::parse(
            "group \"browsers\" {\n    apps \"chromium\"\n    \"Cmd+T\" menu=\"File\" label=\"New Tab\"\n    \"Cmd+[\" \"Alt+Left\" menu=\"History\" label=\"Back\"\n}\n",
        )
        .unwrap();
        let commands = tables.commands(&["chromium"]);
        assert_eq!(commands.len(), 2);
        assert_eq!((commands[0].menu.as_str(), commands[0].label.as_str()), ("File", "New Tab"));
        assert_eq!(
            tables.strokes_for(&["chromium"], &chord("Cmd+T")),
            [Stroke::parse("Ctrl+T").unwrap()]
        );
        assert_eq!(
            tables.strokes_for(&["chromium"], &chord("Cmd+[")),
            [Stroke::parse("Alt+Left").unwrap()]
        );
        assert!(tables.commands(&["kitty"]).is_empty());
        assert!(!Tables::shipped().commands(&["code"]).is_empty());
    }

    #[test]
    fn the_shipped_tables_parse() {
        let tables = Tables::shipped();
        assert!(tables.list().len() > 40);
    }

    #[test]
    fn an_app_takes_its_groups_and_the_shared_sets_it_does_not_skip() {
        let tables = Tables::shipped();
        let kitty = tables.for_app(&["kitty"]);
        let copy = kitty.rules.iter().find_map(|r| r.lookup(&chord("Cmd+C")));
        assert_eq!(copy, Some(&[Stroke::parse("Ctrl+Shift+C").unwrap()][..]));
        assert!(
            kitty
                .rules
                .iter()
                .all(|r| r.lookup(&chord("Ctrl+A")).is_none()),
            "terminals keep Ctrl"
        );
        assert_eq!(
            kitty.close,
            Some(&[Stroke::parse("Ctrl+Shift+W").unwrap()][..])
        );

        let gedit = tables.for_app(&["org.gnome.TextEditor"]);
        assert!(
            gedit
                .rules
                .iter()
                .any(|r| r.lookup(&chord("Ctrl+A")).is_some())
        );
        assert!(
            gedit.close.is_none(),
            "no chord of its own: the desktop closes"
        );

        let firefox = tables.for_app(&["org.mozilla.firefox"]);
        assert_eq!(firefox.quit, Some(&[Stroke::parse("Ctrl+Q").unwrap()][..]));
    }

    #[test]
    fn a_user_file_lies_over_the_shipped_one() {
        let user = Tables::parse(
            "version 1\ngroup \"terminals\" {\n    \"Cmd+K\" \"Ctrl+L\"\n}\napp \"my.App\" {\n    close \"Ctrl+F4\"\n}\n",
        )
        .unwrap();
        let tables = Tables::shipped().over(user);
        let kitty = tables.for_app(&["kitty"]);
        assert!(
            kitty
                .rules
                .iter()
                .any(|r| r.lookup(&chord("Cmd+K")).is_some())
        );
        assert!(
            kitty
                .rules
                .iter()
                .any(|r| r.lookup(&chord("Cmd+C")).is_some())
        );
        assert!(tables.for_app(&["my.app"]).close.is_some());
    }

    #[test]
    fn a_broken_file_names_its_place() {
        let err =
            Tables::parse("version 1\ngroup \"x\" {\n    \"Cmd+Nope\" \"Home\"\n}\n").unwrap_err();
        assert_eq!((err.line, err.column), (3, 5));
        assert!(err.message.contains("Cmd+Nope"));
    }

    #[test]
    fn the_inverse_gives_the_mac_chord_for_a_menu_item() {
        let tables = Tables::shipped();
        let sent = Stroke::parse("Ctrl+Shift+C").unwrap();
        assert_eq!(tables.mac_chord(&["kitty"], &sent), Some(chord("Cmd+C")));
        let save = Stroke::parse("Ctrl+S").unwrap();
        assert_eq!(
            tables.mac_chord(&["org.gnome.TextEditor"], &save),
            Some(chord("Cmd+S"))
        );
        let home = Stroke::parse("Home").unwrap();
        assert_eq!(
            tables.mac_chord(&["org.gnome.TextEditor"], &home),
            Some(chord("Cmd+Left"))
        );
    }
}
