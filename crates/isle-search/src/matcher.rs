//! Whether a query matches a text, and how well: a tier first, then a score within it.

use nucleo_matcher::{
    Config, Utf32Str,
    chars::{graphemes, is_upper_case, normalize, to_lower_case},
};

/// How a query matched, best first. Only results in the same tier compare by score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// The text starts with the query.
    Prefix,
    /// A word in the text starts with the query, or each of its words starts a word in order.
    WordStart,
    /// The query is the start of the words' initials: `vsc` for Visual Studio Code.
    Initials,
    /// The query is inside the text.
    Inside,
    /// The query's letters are in the text in order.
    Fuzzy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub tier: Tier,
    /// Higher is better, within a tier only.
    pub score: u16,
    /// The matched characters, as grapheme indices into the text.
    pub indices: Vec<u32>,
}

/// A fuzzy match must score at least this per query character, so scattered letters across
/// a long text are not results.
const FUZZY_PER_CHAR: u16 = 18;

/// Holds nucleo's buffers; one per thread that searches.
pub struct Matcher {
    nucleo: nucleo_matcher::Matcher,
    query: Vec<char>,
    text: Vec<char>,
    starts: Vec<u32>,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Matcher {
    pub fn new() -> Self {
        Self {
            nucleo: nucleo_matcher::Matcher::new(Config::DEFAULT),
            query: Vec::new(),
            text: Vec::new(),
            starts: Vec::new(),
        }
    }

    /// How `query` matches `text`, ignoring case and Latin accents. An empty query matches
    /// everything as a prefix, so an empty scope lists by use and then by name.
    pub fn find(&mut self, query: &str, text: &str) -> Option<Match> {
        fold(query.trim(), &mut self.query);
        if self.query.is_empty() {
            return Some(Match {
                tier: Tier::Prefix,
                score: 0,
                indices: Vec::new(),
            });
        }
        self.starts.clear();
        self.text.clear();
        let mut previous: Option<char> = None;
        for (i, c) in graphemes(text).enumerate() {
            let starts_word = match previous {
                None => c.is_alphanumeric(),
                Some(p) => {
                    c.is_alphanumeric()
                        && (!p.is_alphanumeric()
                            || (is_upper_case(c) && !is_upper_case(p) && p.is_alphabetic()))
                }
            };
            if starts_word {
                self.starts.push(i as u32);
            }
            self.text.push(to_lower_case(normalize(c)));
            previous = Some(c);
        }
        let (q, t) = (&self.query[..], &self.text[..]);
        let len = t.len() as u16;
        if t.starts_with(q) {
            return Some(Match {
                tier: Tier::Prefix,
                score: u16::MAX - len,
                indices: (0..q.len() as u32).collect(),
            });
        }
        if let Some(indices) = word_starts(q, t, &self.starts) {
            let first = indices[0] as u16;
            return Some(Match {
                tier: Tier::WordStart,
                score: u16::MAX - first.saturating_mul(8) - len,
                indices,
            });
        }
        if q.len() >= 2 && q.len() <= self.starts.len() && !q.contains(&' ') {
            let initials = self.starts.iter().map(|&s| t[s as usize]);
            if initials.zip(q).all(|(a, b)| a == *b) {
                return Some(Match {
                    tier: Tier::Initials,
                    score: u16::MAX - len,
                    indices: self.starts[..q.len()].to_vec(),
                });
            }
        }
        if let Some(at) = t.windows(q.len()).position(|w| w == q) {
            return Some(Match {
                tier: Tier::Inside,
                score: u16::MAX - (at as u16).saturating_mul(8) - len,
                indices: (at as u32..(at + q.len()) as u32).collect(),
            });
        }
        if q.len() < 2 {
            return None;
        }
        let mut indices = Vec::new();
        let score =
            self.nucleo
                .fuzzy_indices(Utf32Str::Unicode(t), Utf32Str::Unicode(q), &mut indices)?;
        (score >= FUZZY_PER_CHAR * q.len() as u16).then_some(Match {
            tier: Tier::Fuzzy,
            score,
            indices,
        })
    }
}

fn fold(text: &str, out: &mut Vec<char>) {
    out.clear();
    out.extend(graphemes(text).map(|c| to_lower_case(normalize(c))));
}

/// The query as a whole starting a word after the first, or each of its words starting a
/// word, in order.
fn word_starts(q: &[char], t: &[char], starts: &[u32]) -> Option<Vec<u32>> {
    if let Some(&s) = starts
        .iter()
        .skip(1)
        .find(|&&s| t[s as usize..].starts_with(q))
    {
        return Some((s..s + q.len() as u32).collect());
    }
    let words: Vec<&[char]> = q.split(|c| *c == ' ').filter(|w| !w.is_empty()).collect();
    if words.len() < 2 {
        return None;
    }
    let mut indices = Vec::new();
    let mut from = 0;
    for word in words {
        let at = starts[from..]
            .iter()
            .position(|&s| t[s as usize..].starts_with(word))?;
        let s = starts[from + at];
        indices.extend(s..s + word.len() as u32);
        from += at + 1;
    }
    Some(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tier(query: &str, text: &str) -> Option<Tier> {
        Matcher::new().find(query, text).map(|m| m.tier)
    }

    #[test]
    fn tiers() {
        assert_eq!(tier("fire", "Firefox"), Some(Tier::Prefix));
        assert_eq!(tier("screen", "Lock Screen"), Some(Tier::WordStart));
        assert_eq!(tier("office", "LibreOffice"), Some(Tier::WordStart));
        assert_eq!(tier("new win", "New Private Window"), Some(Tier::WordStart));
        assert_eq!(tier("vsc", "Visual Studio Code"), Some(Tier::Initials));
        assert_eq!(tier("vs", "Visual Studio Code"), Some(Tier::Initials));
        assert_eq!(tier("efo", "Firefox"), Some(Tier::Inside));
        assert_eq!(tier("ffx", "Firefox"), Some(Tier::Fuzzy));
        assert_eq!(tier("zq", "Firefox"), None);
    }

    #[test]
    fn case_and_accents() {
        assert_eq!(tier("FIRE", "firefox"), Some(Tier::Prefix));
        assert_eq!(tier("cafe", "Café"), Some(Tier::Prefix));
    }

    #[test]
    fn empty_matches_all() {
        assert_eq!(tier("", "Anything"), Some(Tier::Prefix));
        assert_eq!(tier("  ", "Anything"), Some(Tier::Prefix));
    }

    #[test]
    fn one_letter_is_never_fuzzy() {
        assert_eq!(tier("x", "Firefox"), Some(Tier::Inside));
        assert_eq!(tier("q", "Firefox"), None);
    }

    #[test]
    fn scattered_letters_are_not_results() {
        assert_eq!(tier("aei", "pantheist mojito"), None);
        assert_eq!(tier("kty", "kitty"), Some(Tier::Fuzzy));
    }

    #[test]
    fn shorter_and_earlier_score_higher() {
        let mut m = Matcher::new();
        let files = m.find("fi", "Files").unwrap().score;
        let firefox = m.find("fi", "Firefox").unwrap().score;
        assert!(files > firefox);
        let early = m.find("tab", "New Tab").unwrap().score;
        let late = m.find("tab", "Reopen Closed Tab").unwrap().score;
        assert!(early > late);
    }

    #[test]
    fn indices_mark_the_match() {
        let mut m = Matcher::new();
        assert_eq!(m.find("scr", "Lock Screen").unwrap().indices, vec![5, 6, 7]);
        assert_eq!(
            m.find("vsc", "Visual Studio Code").unwrap().indices,
            vec![0, 7, 14]
        );
    }
}
