//! Apps and library games: the general search's Apps group and the Apps view.

use std::path::PathBuf;

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    /// The desktop id.
    pub id: String,
    pub name: String,
    /// Its generic name and keywords.
    pub also: Vec<String>,
    /// The Apps view's category, by its label.
    pub category: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    /// `steam:570` and the like.
    pub key: String,
    pub name: String,
    pub cover: Option<PathBuf>,
}

pub const GAMES: &str = "Games";

#[derive(Clone, Debug, Default)]
pub struct AppsSource {
    pub apps: Vec<App>,
    pub games: Vec<Game>,
    /// Desktop ids and game keys in the user's order.
    pub favourites: Vec<String>,
    /// The Apps view's category filter, by label.
    pub category: Option<String>,
}

enum Kind<'a> {
    App(&'a App),
    Game(&'a Game),
}

impl Kind<'_> {
    fn id(&self) -> &str {
        match self {
            Kind::App(a) => &a.id,
            Kind::Game(g) => &g.key,
        }
    }

    fn name(&self) -> &str {
        match self {
            Kind::App(a) => &a.name,
            Kind::Game(g) => &g.name,
        }
    }

    fn category(&self) -> &str {
        match self {
            Kind::App(a) => &a.category,
            Kind::Game(_) => GAMES,
        }
    }

    fn key(&self) -> String {
        match self {
            Kind::App(a) => format!("app:{}", a.id),
            Kind::Game(g) => format!("game:{}", g.key),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Kind::App(a) => Icon::App(a.id.clone()),
            Kind::Game(g) => match &g.cover {
                Some(cover) => Icon::Path(cover.clone()),
                None => Icon::Glyph("game-controller"),
            },
        }
    }

    fn item(&self) -> Item {
        match self {
            Kind::App(a) => Item::App(a.id.clone()),
            Kind::Game(g) => Item::Game(g.key.clone()),
        }
    }
}

impl AppsSource {
    fn all(&self) -> impl Iterator<Item = Kind<'_>> {
        self.apps
            .iter()
            .map(Kind::App)
            .chain(self.games.iter().map(Kind::Game))
    }

    fn build(
        &self,
        cx: &mut Context<'_>,
        query: &Query,
        kind: &Kind<'_>,
        section: Option<&str>,
    ) -> Option<Row> {
        let also: Vec<&str> = match kind {
            Kind::App(a) => a.also.iter().map(String::as_str).collect(),
            Kind::Game(_) => Vec::new(),
        };
        let mut row = super::row(
            cx,
            query,
            kind.key(),
            Group::Apps,
            kind.name(),
            &also,
            false,
            || (None, None, kind.icon(), kind.item()),
        )?;
        row.section = section.map(str::to_owned);
        Some(row)
    }
}

impl Source for AppsSource {
    fn group(&self) -> Group {
        Group::Apps
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::General | Scope::Apps)
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let browsing = query.scope == Scope::Apps && query.text.trim().is_empty();
        if !browsing {
            if query.text.trim().is_empty() {
                return Vec::new();
            }
            let rows = self
                .all()
                .filter_map(|k| self.build(cx, query, &k, None))
                .collect();
            return super::sorted(rows);
        }
        // The Apps view: favourites in the user's order, then every app by name; a category
        // shows its apps alone.
        let mut by_name: Vec<Kind<'_>> = self.all().collect();
        by_name.sort_by_key(|k| k.name().to_lowercase());
        if let Some(category) = &self.category {
            return by_name
                .iter()
                .filter(|k| k.category() == category)
                .filter_map(|k| self.build(cx, query, k, Some(category)))
                .collect();
        }
        let mut rows: Vec<Row> = self
            .favourites
            .iter()
            .filter_map(|f| by_name.iter().find(|k| k.id() == f))
            .filter_map(|k| self.build(cx, query, k, Some("Favourites")))
            .map(|mut r| {
                r.key = format!("fav:{}", r.key);
                r
            })
            .collect();
        rows.extend(
            by_name
                .iter()
                .filter_map(|k| self.build(cx, query, k, Some("All apps"))),
        );
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Matcher, Usage, usage};

    fn source() -> AppsSource {
        let app = |id: &str, name: &str, also: &[&str], category: &str| App {
            id: id.into(),
            name: name.into(),
            also: also.iter().map(|s| s.to_string()).collect(),
            category: category.into(),
        };
        AppsSource {
            apps: vec![
                app(
                    "org.mozilla.firefox",
                    "Firefox",
                    &["Web Browser"],
                    "Internet",
                ),
                app("kitty", "kitty", &["Terminal"], "Utilities"),
                app("org.gnome.Nautilus", "Files", &["folder"], "Utilities"),
            ],
            games: vec![Game {
                key: "steam:570".into(),
                name: "Dota 2".into(),
                cover: None,
            }],
            favourites: vec!["kitty".into(), "steam:570".into()],
            category: None,
        }
    }

    fn search(source: &AppsSource, scope: Scope, text: &str, usage: &Usage) -> Vec<Row> {
        let mut matcher = Matcher::new();
        let mut cx = Context {
            matcher: &mut matcher,
            usage,
            now: 100,
            latest: None,
        };
        source.search(
            &Query {
                text: text.into(),
                scope,
                generation: 1,
            },
            &mut cx,
        )
    }

    #[test]
    fn apps_and_games_rank_together_by_use() {
        let mut used = Usage::default();
        usage::record(&mut used, "app:org.gnome.Nautilus", 100);
        let rows = search(&source(), Scope::General, "f", &used);
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["Files", "Firefox"]);
        let rows = search(&source(), Scope::General, "dota", &Usage::default());
        assert_eq!(rows[0].item, Item::Game("steam:570".into()));
        let rows = search(&source(), Scope::General, "browser", &Usage::default());
        assert_eq!(rows[0].title, "Firefox", "by its generic name");
        assert!(search(&source(), Scope::General, "", &Usage::default()).is_empty());
    }

    #[test]
    fn the_apps_view_lists_favourites_then_everything_by_name() {
        let rows = search(&source(), Scope::Apps, "", &Usage::default());
        let listed: Vec<(Option<&str>, &str)> = rows
            .iter()
            .map(|r| (r.section.as_deref(), r.title.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                (Some("Favourites"), "kitty"),
                (Some("Favourites"), "Dota 2"),
                (Some("All apps"), "Dota 2"),
                (Some("All apps"), "Files"),
                (Some("All apps"), "Firefox"),
                (Some("All apps"), "kitty"),
            ]
        );
        assert_ne!(
            rows[0].key, rows[5].key,
            "a favourite and its place in all apps are told apart"
        );
        let mut games = source();
        games.category = Some(GAMES.into());
        let rows = search(&games, Scope::Apps, "", &Usage::default());
        assert_eq!(rows.len(), 1);
        let rows = search(&source(), Scope::Apps, "fi", &Usage::default());
        assert!(
            rows.iter().all(|r| r.section.is_none()),
            "typing drops the sections"
        );
    }
}
