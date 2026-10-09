//! Shell actions with their chords, and apps' desktop-entry actions.

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

/// One of `data/spotlight/actions.kdl`'s actions.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionDef {
    pub id: String,
    pub title: String,
    pub glyph: &'static str,
    pub also: Vec<String>,
}

/// The shipped actions, in the file's order, read once.
pub fn catalogue() -> &'static [ActionDef] {
    static CATALOGUE: std::sync::OnceLock<Vec<ActionDef>> = std::sync::OnceLock::new();
    CATALOGUE.get_or_init(|| parse(include_str!("../../../../data/spotlight/actions.kdl")))
}

fn parse(text: &str) -> Vec<ActionDef> {
    let Ok(doc) = text.parse::<kdl::KdlDocument>() else {
        return Vec::new();
    };
    doc.nodes()
        .iter()
        .filter(|n| n.name().value() == "action")
        .filter_map(|n| {
            let id = n
                .entries()
                .iter()
                .find(|e| e.name().is_none())?
                .value()
                .as_string()?
                .to_owned();
            let get = |key: &str| {
                n.get(key)
                    .and_then(|v| v.as_string())
                    .unwrap_or("")
                    .to_owned()
            };
            Some(ActionDef {
                id,
                title: get("title"),
                glyph: Box::leak(get("glyph").into_boxed_str()),
                also: super::words(&get("also")),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// A shortcut's id, or a command: `sleep`, `restart`, `focus:Work`.
    pub id: String,
    pub title: String,
    pub glyph: &'static str,
    pub also: Vec<String>,
    /// Its chord as the keyboard names it.
    pub chord: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntryAction {
    pub app: String,
    pub app_name: String,
    pub action: String,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct ActionsSource {
    pub actions: Vec<Action>,
    pub entry_actions: Vec<EntryAction>,
}

impl Source for ActionsSource {
    fn group(&self) -> Group {
        Group::Actions
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::General | Scope::Actions | Scope::App(_))
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        if query.scope == Scope::General && query.text.trim().is_empty() {
            return Vec::new();
        }
        let one_app = match &query.scope {
            Scope::App(app) => Some(app.as_str()),
            _ => None,
        };
        let mut rows = Vec::new();
        if one_app.is_none() {
            for a in &self.actions {
                let also: Vec<&str> = a.also.iter().map(String::as_str).collect();
                rows.extend(super::row(
                    cx,
                    query,
                    format!("action:{}", a.id),
                    Group::Actions,
                    &a.title,
                    &also,
                    true,
                    || {
                        (
                            None,
                            a.chord.clone(),
                            Icon::Glyph(a.glyph),
                            Item::Action(a.id.clone()),
                        )
                    },
                ));
            }
        }
        for e in self
            .entry_actions
            .iter()
            .filter(|e| one_app.is_none_or(|app| app == e.app))
        {
            let title = format!("{}: {}", e.app_name, e.name);
            rows.extend(super::row(
                cx,
                query,
                format!("entry:{}:{}", e.app, e.action),
                Group::Actions,
                &title,
                &[&e.name],
                false,
                || {
                    (
                        None,
                        None,
                        Icon::App(e.app.clone()),
                        Item::EntryAction {
                            app: e.app.clone(),
                            action: e.action.clone(),
                        },
                    )
                },
            ));
        }
        super::sorted(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_parses() {
        let all = catalogue();
        assert!(all.len() > 20);
        let lock = all.iter().find(|a| a.id == "lock").unwrap();
        assert_eq!(lock.title, "Lock Screen");
        assert_eq!(lock.glyph, "lock");
        assert!(
            all.iter()
                .all(|a| !a.title.is_empty() && !a.glyph.is_empty())
        );
        let off = all.iter().find(|a| a.id == "shut-down").unwrap();
        assert!(off.also.contains(&"power off".to_string()));
    }

    #[test]
    fn actions_are_found_by_their_words_and_show_their_chord() {
        let source = ActionsSource {
            actions: vec![
                Action {
                    id: "shut-down".into(),
                    title: "Shut Down…".into(),
                    glyph: "power",
                    also: vec!["power off".into()],
                    chord: None,
                },
                Action {
                    id: "lock".into(),
                    title: "Lock Screen".into(),
                    glyph: "lock",
                    also: Vec::new(),
                    chord: Some("⌃⌘Q".into()),
                },
            ],
            entry_actions: vec![EntryAction {
                app: "firefox".into(),
                app_name: "Firefox".into(),
                action: "private".into(),
                name: "New Private Window".into(),
            }],
        };
        let mut matcher = crate::Matcher::new();
        let usage = crate::Usage::default();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 0,
            latest: None,
        };
        let q = |text: &str, scope: Scope| Query {
            text: text.into(),
            scope,
            generation: 1,
        };
        let rows = source.search(&q("power", Scope::General), &mut cx);
        assert_eq!(rows[0].item, Item::Action("shut-down".into()));
        let rows = source.search(&q("lock", Scope::General), &mut cx);
        assert_eq!(rows[0].trailing.as_deref(), Some("⌃⌘Q"));
        let rows = source.search(&q("private", Scope::General), &mut cx);
        assert_eq!(rows[0].title, "Firefox: New Private Window");
        let rows = source.search(&q("", Scope::App("firefox".into())), &mut cx);
        assert_eq!(rows.len(), 1, "an app's own actions only");
        assert_eq!(
            source.search(&q("", Scope::Actions), &mut cx).len(),
            3,
            "every action, by use"
        );
    }
}
