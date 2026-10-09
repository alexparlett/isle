//! Clipboard history and its pins, only in their own scope (`structure/spotlight.md`
//! Clipboard).

use std::path::PathBuf;

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source, Tier};

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// The text's first line and how many more follow.
    Text { first: String, more: usize },
    /// An image kept as a file, with its size when known.
    Image {
        path: PathBuf,
        size: Option<(u32, u32)>,
    },
    /// Copied files: the first one's name and how many.
    Files { first: String, count: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: u64,
    pub kind: Kind,
    /// The whole text, for searching.
    pub text: String,
    /// The app it was copied from, by name.
    pub app: String,
    pub pinned: bool,
}

/// Items newest first, pins in the order pinned.
#[derive(Clone, Debug, Default)]
pub struct ClipboardSource {
    pub clips: Vec<Clip>,
}

fn describe(clip: &Clip) -> (String, Option<String>, Icon) {
    match &clip.kind {
        Kind::Text { first, more } => (
            first.clone(),
            (*more > 0).then(|| format!("{more} more line{}", if *more == 1 { "" } else { "s" })),
            Icon::Glyph("text-align-left"),
        ),
        Kind::Image { path, size } => (
            "Image".into(),
            Some(match size {
                Some((w, h)) => format!("Image · {w} × {h} · {}", clip.app),
                None => format!("Image · {}", clip.app),
            }),
            Icon::Path(path.clone()),
        ),
        Kind::Files { first, count } => (
            first.clone(),
            (*count > 1).then(|| format!("and {} more", count - 1)),
            Icon::Glyph("file"),
        ),
    }
}

impl Source for ClipboardSource {
    fn group(&self) -> Group {
        Group::Clipboard
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::Clipboard
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        let Some(any) = cx.matcher.find("", "") else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for pinned in [true, false] {
            for clip in self.clips.iter().filter(|c| c.pinned == pinned) {
                if !text.is_empty() {
                    match cx.matcher.find(text, &clip.text) {
                        Some(m) if m.tier != Tier::Fuzzy => {}
                        _ => continue,
                    }
                }
                let (title, detail, icon) = describe(clip);
                rows.push(Row {
                    key: format!("clip:{}", clip.id),
                    group: Group::Clipboard,
                    title,
                    detail,
                    trailing: None,
                    icon,
                    item: Item::Clip(clip.id),
                    section: Some(if pinned { "Pinned" } else { "Recent" }.into()),
                    rank: crate::Rank::new(&any, 0.0, false, ""),
                    matched: Vec::new(),
                });
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: u64, text: &str, pinned: bool) -> Clip {
        let mut lines = text.lines();
        Clip {
            id,
            kind: Kind::Text {
                first: lines.next().unwrap_or("").into(),
                more: lines.count(),
            },
            text: text.into(),
            app: "kitty".into(),
            pinned,
        }
    }

    #[test]
    fn pins_first_then_recent_and_a_query_narrows() {
        let source = ClipboardSource {
            clips: vec![
                clip(3, "newest\nsecond line", false),
                clip(2, "pinned thing", true),
                clip(1, "older", false),
            ],
        };
        let mut matcher = crate::Matcher::new();
        let usage = crate::Usage::default();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 0,
            latest: None,
        };
        let q = |text: &str| Query {
            text: text.into(),
            scope: Scope::Clipboard,
            generation: 1,
        };
        let rows = source.search(&q(""), &mut cx);
        let listed: Vec<(&str, Option<&str>)> = rows
            .iter()
            .map(|r| (r.title.as_str(), r.section.as_deref()))
            .collect();
        assert_eq!(
            listed,
            [
                ("pinned thing", Some("Pinned")),
                ("newest", Some("Recent")),
                ("older", Some("Recent"))
            ]
        );
        assert_eq!(rows[1].detail.as_deref(), Some("1 more line"));
        let rows = source.search(&q("second"), &mut cx);
        assert_eq!(rows.len(), 1, "the whole text is searched");
        assert!(
            !source.answers(&Scope::General),
            "never in the general search"
        );
    }
}
