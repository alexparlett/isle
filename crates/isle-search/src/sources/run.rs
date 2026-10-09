//! Run's command and the web search's row.

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

fn plain(
    cx: &mut Context<'_>,
    key: String,
    group: Group,
    title: String,
    detail: Option<String>,
    icon: Icon,
    item: Item,
) -> Option<Row> {
    let m = cx.matcher.find("", "")?;
    Some(Row {
        key,
        group,
        title,
        detail,
        trailing: None,
        icon,
        item,
        section: None,
        rank: crate::Rank::new(&m, 0.0, false, ""),
        matched: Vec::new(),
    })
}

/// The command as typed, then recent commands that hold it, newest first.
#[derive(Clone, Debug, Default)]
pub struct RunSource {
    pub history: Vec<String>,
}

impl Source for RunSource {
    fn group(&self) -> Group {
        Group::Run
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::Run
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        let mut rows = Vec::new();
        if !text.is_empty() {
            rows.extend(plain(
                cx,
                "run:typed".into(),
                Group::Run,
                text.to_owned(),
                None,
                Icon::Glyph("terminal-window"),
                Item::Run(text.to_owned()),
            ));
        }
        for command in self
            .history
            .iter()
            .filter(|c| *c != text && c.contains(text))
        {
            rows.extend(plain(
                cx,
                format!("run:{command}"),
                Group::Run,
                command.clone(),
                None,
                Icon::Glyph("terminal-window"),
                Item::Run(command.clone()),
            ));
        }
        rows
    }
}

/// The web search for the query: always the general search's last row.
#[derive(Clone, Debug, Default)]
pub struct WebSource {
    /// The address with `{query}` where the query goes.
    pub template: String,
}

/// `text` for a URL's query string.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl Source for WebSource {
    fn group(&self) -> Group {
        Group::Web
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::General
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        if text.is_empty() {
            return Vec::new();
        }
        let url = self.template.replace("{query}", &encode(text));
        plain(
            cx,
            "web".into(),
            Group::Web,
            format!("Search the web for “{text}”"),
            None,
            Icon::Glyph("globe"),
            Item::Web(url),
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(source: &dyn Source, scope: Scope, text: &str) -> Vec<Row> {
        let mut matcher = crate::Matcher::new();
        let usage = crate::Usage::default();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 0,
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
    fn the_web_row_encodes_its_query() {
        let web = WebSource {
            template: "https://duckduckgo.com/?q={query}".into(),
        };
        let rows = search(&web, Scope::General, "rust & smithay");
        assert_eq!(
            rows[0].item,
            Item::Web("https://duckduckgo.com/?q=rust+%26+smithay".into())
        );
        assert_eq!(rows[0].title, "Search the web for “rust & smithay”");
        assert!(search(&web, Scope::General, " ").is_empty());
    }

    #[test]
    fn run_offers_the_command_then_history() {
        let run = RunSource {
            history: vec!["cargo build".into(), "cargo test".into(), "ls".into()],
        };
        let titles: Vec<String> = search(&run, Scope::Run, "cargo")
            .into_iter()
            .map(|r| r.title)
            .collect();
        assert_eq!(titles, ["cargo", "cargo build", "cargo test"]);
        let titles: Vec<String> = search(&run, Scope::Run, "")
            .into_iter()
            .map(|r| r.title)
            .collect();
        assert_eq!(
            titles,
            ["cargo build", "cargo test", "ls"],
            "recent commands for an empty query"
        );
    }
}
