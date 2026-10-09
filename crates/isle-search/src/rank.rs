//! The order results are listed in: tier, then use, then a shell action before anything
//! else, then the score within the tier, then the name (`p10-spotlight.md` choice 5).

use std::cmp::Ordering;

use crate::matcher::{Match, Matcher, Tier};

#[derive(Clone, Debug)]
pub struct Rank {
    pub tier: Tier,
    pub usage: f64,
    pub action: bool,
    pub score: u16,
    /// The name folded for comparison.
    pub name: String,
}

impl Rank {
    pub fn new(m: &Match, usage: f64, action: bool, name: &str) -> Self {
        Self {
            tier: m.tier,
            usage,
            action,
            score: m.score,
            name: name.to_lowercase(),
        }
    }
}

impl Ord for Rank {
    /// Best first: `a < b` when `a` lists above `b`.
    fn cmp(&self, other: &Self) -> Ordering {
        self.tier
            .cmp(&other.tier)
            .then_with(|| other.usage.total_cmp(&self.usage))
            .then_with(|| other.action.cmp(&self.action))
            .then_with(|| other.score.cmp(&self.score))
            .then_with(|| self.name.cmp(&other.name))
    }
}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Rank {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Rank {}

/// The best match of `query` against a result's name and the other words it is known by
/// (a generic name, keywords, synonyms). A match on another word ranks below the same match
/// on the name: a start there counts as inside, inside as fuzzy, and fuzzy not at all. Indices
/// are given only for a match on the name.
pub fn best(matcher: &mut Matcher, query: &str, name: &str, also: &[&str]) -> Option<Match> {
    let mut found = matcher.find(query, name);
    if found.as_ref().is_some_and(|m| m.tier <= Tier::Initials) {
        return found;
    }
    for word in also {
        let Some(m) = matcher.find(query, word) else {
            continue;
        };
        let tier = match m.tier {
            Tier::Prefix | Tier::WordStart | Tier::Initials => Tier::Inside,
            Tier::Inside => Tier::Fuzzy,
            Tier::Fuzzy => continue,
        };
        let demoted = Match {
            tier,
            score: m.score,
            indices: Vec::new(),
        };
        if found.as_ref().is_none_or(|f| {
            (demoted.tier, std::cmp::Reverse(demoted.score)) < (f.tier, std::cmp::Reverse(f.score))
        }) {
            found = Some(demoted);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank(query: &str, name: &str, usage: f64, action: bool) -> Rank {
        let m = Matcher::new().find(query, name).unwrap();
        Rank::new(&m, usage, action, name)
    }

    #[test]
    fn tier_beats_use() {
        assert!(rank("lock", "Lock Screen", 0.0, true) < rank("lock", "Unlock Tool", 50.0, false));
    }

    #[test]
    fn use_decides_within_a_tier() {
        assert!(rank("fi", "Firefox", 3.0, false) < rank("fi", "Files", 0.0, false));
    }

    #[test]
    fn an_action_wins_a_tie() {
        assert!(rank("lock", "Lock Screen", 0.0, true) < rank("lock", "Lockpick", 0.0, false));
    }

    #[test]
    fn then_the_name() {
        let mut ranks = [rank("", "beta", 0.0, false), rank("", "Alpha", 0.0, false)];
        ranks.sort();
        assert_eq!(ranks[0].name, "alpha");
    }

    #[test]
    fn other_words_rank_below_the_name() {
        let mut m = Matcher::new();
        let keyword = best(&mut m, "browser", "Firefox", &["Web Browser", "internet"]).unwrap();
        assert_eq!(keyword.tier, Tier::Inside);
        assert!(keyword.indices.is_empty());
        let name = best(&mut m, "fire", "Firefox", &["Web Browser"]).unwrap();
        assert_eq!(name.tier, Tier::Prefix);
        assert!(best(&mut m, "wbr", "Firefox", &["Web Browser"]).is_none());
    }
}
