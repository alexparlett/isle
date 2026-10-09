//! Emoji and symbols from the generated `data/emoji/emoji.tsv` (`p10-spotlight.md` choice 15).

use std::sync::OnceLock;

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source, Tier};

#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub text: String,
    pub name: String,
    /// Its group's name: "Smileys & Emotion", "Arrows".
    pub group: String,
    /// Its subgroup's words; empty for a symbol.
    pub sub: String,
    pub keywords: Vec<String>,
    /// Skin-tone forms: each one's text and what sets it apart.
    pub variants: Vec<(String, String)>,
}

/// Every emoji, then every symbol, in the file's order.
pub fn table() -> &'static [Glyph] {
    static TABLE: OnceLock<Vec<Glyph>> = OnceLock::new();
    TABLE.get_or_init(|| parse(include_str!("../../../../data/emoji/emoji.tsv")))
}

/// The generator's lines: `group` and `symbols` name a group, `e` is an emoji with its name,
/// subgroup and keywords, `v` a variant of the emoji above, `s` a symbol.
fn parse(text: &str) -> Vec<Glyph> {
    let mut out: Vec<Glyph> = Vec::new();
    let mut group = String::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["group" | "symbols", title] => group = (*title).to_owned(),
            ["e", text, name, sub, keywords] => out.push(Glyph {
                text: (*text).to_owned(),
                name: (*name).to_owned(),
                group: group.clone(),
                sub: sub.replace('-', " "),
                keywords: super::words(keywords),
                variants: Vec::new(),
            }),
            ["v", text, name] => {
                if let Some(last) = out.last_mut() {
                    last.variants.push(((*text).to_owned(), (*name).to_owned()));
                }
            }
            ["s", text, name] => out.push(Glyph {
                text: (*text).to_owned(),
                name: (*name).to_owned(),
                group: group.clone(),
                sub: String::new(),
                keywords: Vec::new(),
                variants: Vec::new(),
            }),
            _ => {}
        }
    }
    out
}

/// The glyphs the colour font can draw, by their place in `table()`, found once by `check`
/// when first needed and kept across searches.
#[derive(Clone, Debug)]
pub struct EmojiSource {
    pub drawable: std::sync::Arc<OnceLock<Vec<usize>>>,
    pub check: fn() -> Vec<usize>,
}

impl Default for EmojiSource {
    fn default() -> Self {
        Self {
            drawable: Default::default(),
            check: || (0..table().len()).collect(),
        }
    }
}

impl EmojiSource {
    fn glyphs(&self) -> impl Iterator<Item = &'static Glyph> + '_ {
        let all = table();
        self.drawable
            .get_or_init(self.check)
            .iter()
            .filter_map(move |i| all.get(*i))
    }
}

/// The most recent picks an empty query leads with.
const RECENT: usize = 24;

fn row(glyph: &Glyph, rank: crate::Rank, section: String) -> Row {
    Row {
        key: format!("emoji:{}", glyph.text),
        group: Group::Emoji,
        title: glyph.name.clone(),
        detail: Some(glyph.name.clone()),
        trailing: None,
        icon: Icon::Text(glyph.text.clone()),
        item: Item::Emoji(glyph.text.clone()),
        section: Some(section),
        rank,
        matched: Vec::new(),
    }
}

impl Source for EmojiSource {
    fn group(&self) -> Group {
        Group::Emoji
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::Emoji
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        if text.is_empty() {
            let mut recent: Vec<(f64, &Glyph)> = self
                .glyphs()
                .filter_map(|g| {
                    let used = crate::usage::score(cx.usage, &format!("emoji:{}", g.text), cx.now);
                    (used > 0.0).then_some((used, g))
                })
                .collect();
            recent.sort_by(|a, b| b.0.total_cmp(&a.0));
            let m = cx.matcher.find("", "").expect("an empty query matches");
            let mut rows: Vec<Row> = recent
                .into_iter()
                .take(RECENT)
                .map(|(_, g)| {
                    let mut r = row(g, crate::Rank::new(&m, 0.0, false, ""), "Recent".into());
                    r.key = format!("recent:{}", g.text);
                    r
                })
                .collect();
            rows.extend(
                self.glyphs()
                    .map(|g| row(g, crate::Rank::new(&m, 0.0, false, ""), g.group.clone())),
            );
            return rows;
        }
        let mut rows: Vec<Row> = self
            .glyphs()
            .filter_map(|g| {
                let mut also: Vec<&str> = g.keywords.iter().map(String::as_str).collect();
                also.push(&g.sub);
                also.push(&g.group);
                let (rank, _) =
                    cx.rank(text, &format!("emoji:{}", g.text), &g.name, &also, false)?;
                (rank.tier != Tier::Fuzzy).then(|| row(g, rank, String::new()))
            })
            .collect();
        rows.sort_by(|a, b| a.rank.cmp(&b.rank));
        for r in &mut rows {
            r.section = None;
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_emoji_variants_and_symbols() {
        let all = table();
        assert!(all.len() > 3000, "{}", all.len());
        let wave = all.iter().find(|g| g.name == "waving hand").unwrap();
        assert_eq!(wave.variants.len(), 5);
        assert_eq!(wave.group, "People & Body");
        assert!(all.iter().any(|g| g.text == "→" && g.group == "Arrows"));
    }

    #[test]
    fn names_rank_first_and_use_leads_an_empty_query() {
        let source = EmojiSource::default();
        let mut usage = crate::Usage::default();
        crate::usage::record(&mut usage, "emoji:🎉", 10);
        let mut matcher = crate::Matcher::new();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 10,
            latest: None,
        };
        let q = |text: &str| Query {
            text: text.into(),
            scope: Scope::Emoji,
            generation: 1,
        };
        let rows = source.search(&q("grinning"), &mut cx);
        let first: Vec<&str> = rows.iter().take(2).map(|r| r.title.as_str()).collect();
        assert!(
            first.contains(&"grinning face") && first.contains(&"grinning cat"),
            "{first:?}"
        );
        let rows = source.search(&q(""), &mut cx);
        assert_eq!(rows[0].item, Item::Emoji("🎉".into()));
        assert_eq!(rows[0].section.as_deref(), Some("Recent"));
        assert_eq!(rows[1].section.as_deref(), Some("Smileys & Emotion"));
        let rows = source.search(&q("arrow"), &mut cx);
        assert!(rows.iter().any(|r| r.icon == Icon::Text("→".into())));
    }
}
