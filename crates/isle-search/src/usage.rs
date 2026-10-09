//! How much a result has been chosen: a count that halves every 30 days, with a bonus that
//! fades over the week after the last choice (`p10-spotlight.md` choice 5).

use isle_state::domains::{Usage, Use};

const HALF_LIFE: f64 = 30.0 * 86_400.0;
const WEEK: f64 = 7.0 * 86_400.0;
/// A use whose count has decayed below this is forgotten when the next is recorded.
const FORGET: f64 = 0.05;

fn decayed(u: &Use, now: u64) -> f64 {
    let age = now.saturating_sub(u.last) as f64;
    u.weight * 0.5f64.powf(age / HALF_LIFE)
}

/// The result's use at `now`; zero for one never chosen.
pub fn score(usage: &Usage, key: &str, now: u64) -> f64 {
    usage.uses.get(key).map_or(0.0, |u| {
        let age = now.saturating_sub(u.last) as f64;
        decayed(u, now) + (1.0 - age / WEEK).max(0.0)
    })
}

/// Counts a choice of the result at `now`, forgetting results long unused.
pub fn record(usage: &mut Usage, key: &str, now: u64) {
    let weight = usage.uses.get(key).map_or(0.0, |u| decayed(u, now)) + 1.0;
    usage
        .uses
        .retain(|k, u| k == key || decayed(u, now) >= FORGET);
    usage.uses.insert(key.to_owned(), Use { weight, last: now });
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400;
    const NOW: u64 = 1_000 * DAY;

    #[test]
    fn counts_and_decays() {
        let mut usage = Usage::default();
        record(&mut usage, "a", NOW);
        record(&mut usage, "a", NOW);
        assert!((score(&usage, "a", NOW) - 3.0).abs() < 1e-9);
        assert!((score(&usage, "a", NOW + 30 * DAY) - 1.0).abs() < 1e-9);
        assert_eq!(score(&usage, "b", NOW), 0.0);
    }

    #[test]
    fn recent_beats_old_heavy_use() {
        let mut usage = Usage::default();
        for _ in 0..10 {
            record(&mut usage, "old", NOW - 365 * DAY);
        }
        record(&mut usage, "new", NOW - DAY);
        assert!(score(&usage, "new", NOW) > score(&usage, "old", NOW));
    }

    #[test]
    fn forgets_the_long_unused() {
        let mut usage = Usage::default();
        record(&mut usage, "old", NOW - 200 * DAY);
        record(&mut usage, "new", NOW);
        assert!(!usage.uses.contains_key("old"));
        assert!(usage.uses.contains_key("new"));
    }
}
