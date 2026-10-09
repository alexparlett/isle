//! The sources: each holds what it searches, handed over by the compositor when Spotlight
//! opens, so each is tested without one.

pub mod actions;
pub mod apps;
pub mod calculator;
pub mod emoji;
pub mod menus;
pub mod run;
pub mod settings;
pub mod windows;

use crate::{Context, Group, Icon, Item, Query, Row};

/// A row for `name` under `key` when `query` matches it; see `Context::rank`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn row(
    cx: &mut Context<'_>,
    query: &Query,
    key: String,
    group: Group,
    name: &str,
    also: &[&str],
    action: bool,
    build: impl FnOnce() -> (Option<String>, Option<String>, Icon, Item),
) -> Option<Row> {
    let (rank, matched) = cx.rank(&query.text, &key, name, also, action)?;
    let (detail, trailing, icon, item) = build();
    Some(Row {
        key,
        group,
        title: name.to_owned(),
        detail,
        trailing,
        icon,
        item,
        section: None,
        rank,
        matched,
    })
}

/// Rows best first.
pub(crate) fn sorted(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|a, b| a.rank.cmp(&b.rank));
    rows
}

/// The `;`-separated words of a data file's value.
pub(crate) fn words(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}
