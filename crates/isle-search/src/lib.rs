//! Spotlight's sources, the matcher and the ranking, with no compositor in them.

pub mod matcher;
pub mod rank;
pub mod usage;

use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

pub use isle_state::domains::Usage;
pub use matcher::{Match, Matcher, Tier};
pub use rank::Rank;

/// What the search is narrowed to (`structure/spotlight.md` Scopes).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    General,
    Apps,
    Files {
        contents: bool,
    },
    Actions,
    Clipboard,
    Calculator,
    Emoji,
    Windows,
    Run,
    /// One app's windows, actions, quicklist and recent files, by desktop id.
    App(String),
}

/// The groups of the general search, in the order they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Calculation,
    Apps,
    Actions,
    MenuItems,
    Settings,
    Windows,
    Files,
    Web,
    Clipboard,
    Emoji,
    Run,
}

/// What choosing a row does; the compositor carries it out.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Raise the app if it runs, else launch it.
    App(String),
    /// A library game: `steam:570`, `heroic:<runner>:<app>`, `lutris:<id>`.
    Game(String),
    /// A shell action by its shortcut id.
    Action(String),
    /// A desktop entry's action: the app's id and the action's.
    EntryAction {
        app: String,
        action: String,
    },
    Window(u64),
    /// A menu item of the app in front, by its labels from the title down.
    MenuItem(Vec<String>),
    Setting {
        pane: String,
        row: String,
    },
    File(PathBuf),
    /// Copy the answer.
    Answer(String),
    Web(String),
    Run(String),
    Emoji(String),
    Clip(u64),
}

/// The leading slot of a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Icon {
    /// An app's icon by its desktop id.
    App(String),
    /// A picture on disk: a game's cover, a thumbnail.
    Path(PathBuf),
    /// A Phosphor glyph by name.
    Glyph(&'static str),
    /// Drawn as text: an emoji.
    Text(String),
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Stable across queries, so the selection can follow a row; also its key in `Usage`.
    pub key: String,
    pub group: Group,
    pub title: String,
    pub detail: Option<String>,
    /// A chord or a mark at the trailing end.
    pub trailing: Option<String>,
    pub icon: Icon,
    pub item: Item,
    /// The header it lists under in place of its group's: "Pinned", "Favourites".
    pub section: Option<String>,
    pub rank: Rank,
    /// The title's matched graphemes.
    pub matched: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub scope: Scope,
    /// Rises with every keystroke; an answer to an older query is dropped.
    pub generation: u64,
}

/// What a source searches with.
pub struct Context<'a> {
    pub matcher: &'a mut Matcher,
    pub usage: &'a Usage,
    /// Unix seconds.
    pub now: u64,
    /// The newest query's generation, for a slow source to give up on a stale one.
    pub latest: Option<&'a AtomicU64>,
}

impl Context<'_> {
    /// Whether a newer query has been typed since `query`.
    pub fn stale(&self, query: &Query) -> bool {
        self.latest
            .is_some_and(|l| l.load(Ordering::Relaxed) != query.generation)
    }

    /// `name`'s rank for `query` under `key`'s use, if it matches; see `rank::best`.
    pub fn rank(
        &mut self,
        query: &str,
        key: &str,
        name: &str,
        also: &[&str],
        action: bool,
    ) -> Option<(Rank, Vec<u32>)> {
        let m = rank::best(self.matcher, query, name, also)?;
        let usage = usage::score(self.usage, key, self.now);
        Some((Rank::new(&m, usage, action, name), m.indices))
    }
}

/// One kind of result. Fast sources answer from memory in the keystroke's frame on the loop;
/// slow ones run on the worker and check `Context::stale` as they go.
pub trait Source: Send + Sync {
    fn group(&self) -> Group;
    /// Whether it answers in `scope`.
    fn answers(&self, scope: &Scope) -> bool;
    fn slow(&self) -> bool {
        false
    }
    /// Its rows for the query, best first.
    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Names(Vec<&'static str>);

    impl Source for Names {
        fn group(&self) -> Group {
            Group::Apps
        }

        fn answers(&self, scope: &Scope) -> bool {
            matches!(scope, Scope::General | Scope::Apps)
        }

        fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
            let mut rows: Vec<Row> = self
                .0
                .iter()
                .filter_map(|name| {
                    let key = format!("app:{name}");
                    let (rank, matched) = cx.rank(&query.text, &key, name, &[], false)?;
                    Some(Row {
                        key,
                        group: Group::Apps,
                        title: name.to_string(),
                        detail: None,
                        trailing: None,
                        icon: Icon::None,
                        item: Item::App(name.to_string()),
                        section: None,
                        rank,
                        matched,
                    })
                })
                .collect();
            rows.sort_by(|a, b| a.rank.cmp(&b.rank));
            rows
        }
    }

    fn query(text: &str, generation: u64) -> Query {
        Query {
            text: text.into(),
            scope: Scope::General,
            generation,
        }
    }

    #[test]
    fn a_source_ranks_by_use() {
        let source = Names(vec!["Files", "Firefox", "Fish"]);
        let mut usage = Usage::default();
        usage::record(&mut usage, "app:Firefox", 100);
        let mut matcher = Matcher::new();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 100,
            latest: None,
        };
        let rows = source.search(&query("fi", 1), &mut cx);
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["Firefox", "Fish", "Files"]);
    }

    #[test]
    fn stale_after_a_newer_query() {
        let latest = AtomicU64::new(2);
        let mut matcher = Matcher::new();
        let usage = Usage::default();
        let cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 0,
            latest: Some(&latest),
        };
        assert!(cx.stale(&query("a", 1)));
        assert!(!cx.stale(&query("ab", 2)));
    }
}
