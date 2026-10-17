//! Where Settings shows, how it got there, and where a link or a search sends it
//! (`p1/structure/settings.md`, Opening at a pane and Search).

use isle_search::matcher::{Matcher, Tier};

use crate::panes::{PANES, Pane, RowDef};

/// A pane, a sub-page within it, and a row to mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub pane: String,
    pub sub: Option<String>,
    pub row: Option<String>,
}

impl Location {
    pub fn pane(pane: &str) -> Self {
        Self {
            pane: pane.to_owned(),
            sub: None,
            row: None,
        }
    }

    /// `isle-settings://<pane>[/<row>]`, a row on a sub-page opening the sub-page; `None`
    /// for a pane Settings does not have.
    pub fn from_uri(uri: &str) -> Option<Self> {
        let rest = uri.strip_prefix("isle-settings://")?.trim_end_matches('/');
        let (pane, row) = match rest.split_once('/') {
            Some((pane, row)) => (pane, Some(row)),
            None => (rest, None),
        };
        Self::at(pane, row)
    }

    /// A pane and a row by their ids, as the `open` action names them.
    pub fn at(pane: &str, row: Option<&str>) -> Option<Self> {
        let found = crate::panes::find(pane)?;
        let row = row.filter(|r| !r.is_empty());
        let def = row.and_then(|r| found.rows.iter().find(|d| d.id == r));
        Some(Self {
            pane: found.id.to_owned(),
            sub: def.and_then(|d| d.within).map(str::to_owned),
            row: def.map(|d| d.id.to_owned()),
        })
    }
}

/// Back and forward through what was shown, as a browser's history.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    back: Vec<Location>,
    forward: Vec<Location>,
}

impl History {
    /// Moving from `from` somewhere new: it can be gone back to; forward is forgotten.
    pub fn left(&mut self, from: Location) {
        if self.back.last() != Some(&from) {
            self.back.push(from);
        }
        self.forward.clear();
    }

    pub fn back(&mut self, current: Location) -> Option<Location> {
        let to = self.back.pop()?;
        self.forward.push(current);
        Some(to)
    }

    pub fn forward(&mut self, current: Location) -> Option<Location> {
        let to = self.forward.pop()?;
        self.back.push(current);
        Some(to)
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

/// One search result: a pane, or a row with its path.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub pane: &'static Pane,
    pub row: Option<&'static RowDef>,
    tier: Tier,
    score: u16,
}

impl Found {
    pub fn label(&self) -> &'static str {
        self.row.map_or(self.pane.name, |r| r.label)
    }

    /// Where it is: "Displays › Night Light".
    pub fn path(&self) -> String {
        match self.row {
            Some(row) => match row.within {
                Some(within) => format!("{} › {within}", self.pane.name),
                None => self.pane.name.to_owned(),
            },
            None => String::new(),
        }
    }

    pub fn location(&self) -> Location {
        Location {
            pane: self.pane.id.to_owned(),
            sub: self.row.and_then(|r| r.within).map(str::to_owned),
            row: self.row.map(|r| r.id.to_owned()),
        }
    }
}

/// Every pane and row the query finds by label or keyword, grouped by pane in the sidebar's
/// order with the panes whose name matches first; within a pane, best match first.
pub fn search(query: &str) -> Vec<Found> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let mut matcher = Matcher::new();
    let mut best = |texts: &mut dyn Iterator<Item = &str>| {
        texts
            .filter_map(|t| matcher.find(query, t))
            .map(|m| (m.tier, u16::MAX - m.score))
            .min()
    };
    let mut groups: Vec<(bool, usize, Vec<Found>)> = Vec::new();
    for (order, pane) in PANES.iter().enumerate() {
        let mut found = Vec::new();
        let named = best(&mut std::iter::once(pane.name).chain(pane.keywords.iter().copied()));
        if let Some((tier, inverse)) = named {
            found.push(Found {
                pane,
                row: None,
                tier,
                score: u16::MAX - inverse,
            });
        }
        for row in pane.rows {
            let hit = best(&mut std::iter::once(row.label).chain(row.keywords.iter().copied()));
            if let Some((tier, inverse)) = hit {
                found.push(Found {
                    pane,
                    row: Some(row),
                    tier,
                    score: u16::MAX - inverse,
                });
            }
        }
        if found.is_empty() {
            continue;
        }
        found[usize::from(named.is_some())..].sort_by(|a, b| a.tier.cmp(&b.tier).then(b.score.cmp(&a.score)));
        groups.push((named.is_some(), order, found));
    }
    groups.sort_by_key(|(named, order, _)| (!named, *order));
    groups.into_iter().flat_map(|(_, _, found)| found).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uri_names_a_pane_and_an_unknown_one_nothing() {
        assert_eq!(Location::from_uri("isle-settings://displays"), Some(Location::pane("displays")));
        assert_eq!(Location::from_uri("isle-settings://displays/"), Some(Location::pane("displays")));
        assert_eq!(Location::from_uri("isle-settings://printers"), None);
        assert_eq!(Location::from_uri("https://displays"), None);
    }

    #[test]
    fn history_goes_back_and_forward_and_a_new_move_forgets_forward() {
        let (a, b, c) = (Location::pane("general"), Location::pane("displays"), Location::pane("sound"));
        let mut history = History::default();
        history.left(a.clone());
        history.left(b.clone());
        assert_eq!(history.back(c.clone()), Some(b.clone()));
        assert_eq!(history.forward(b.clone()), Some(c.clone()));
        assert_eq!(history.back(c.clone()), Some(b.clone()));
        history.left(b);
        assert!(!history.can_forward());
        assert_eq!(history.back(Location::pane("focus")).map(|l| l.pane), Some("displays".into()));
    }

    #[test]
    fn panes_whose_name_matches_come_first_and_keywords_find_without_showing() {
        let found = search("dis");
        assert_eq!(found[0].pane.id, "displays");
        let dark = search("dark mode");
        assert_eq!(dark[0].pane.id, "appearance");
        assert_eq!(dark[0].label(), "Appearance");
        assert!(search("zzzz").is_empty());
        assert!(search("  ").is_empty());
    }
}
