//! The password generator: rejection sampling over `getrandom`, so each character of the
//! chosen sets is equally likely, with at least one from each set that is on (choice 20).

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const MIN: usize = 8;
pub const MAX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipe {
    pub length: usize,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            length: 20,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
        }
    }
}

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

impl Recipe {
    fn sets(&self) -> Vec<&'static [u8]> {
        let mut sets = Vec::new();
        for (on, set) in [
            (self.upper, UPPER),
            (self.lower, LOWER),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ] {
            if on {
                sets.push(set);
            }
        }
        if sets.is_empty() {
            sets.push(LOWER);
        }
        sets
    }
}

/// A uniform index below `n`, drawn from `getrandom` and rejected above the last whole range.
fn below(n: usize) -> usize {
    let n = n as u32;
    let limit = u32::MAX - u32::MAX % n;
    loop {
        let mut b = [0u8; 4];
        getrandom::fill(&mut b).expect("the kernel's random numbers");
        let v = u32::from_ne_bytes(b);
        if v < limit {
            return (v % n) as usize;
        }
    }
}

/// A password made to `recipe`; the length is held between `MIN` and `MAX`.
pub fn generate(recipe: &Recipe) -> Zeroizing<String> {
    let sets = recipe.sets();
    let length = recipe.length.clamp(MIN, MAX);
    let all: Vec<u8> = sets.iter().flat_map(|s| s.iter().copied()).collect();
    loop {
        let bytes: Zeroizing<Vec<u8>> =
            Zeroizing::new((0..length).map(|_| all[below(all.len())]).collect());
        // Every set on is present; drawing again keeps each character uniform.
        if sets.iter().all(|set| bytes.iter().any(|b| set.contains(b))) {
            return Zeroizing::new(String::from_utf8(bytes.to_vec()).expect("ASCII"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_sets_hold() {
        for length in [1, 8, 20, 64, 200] {
            let p = generate(&Recipe {
                length,
                ..Recipe::default()
            });
            assert_eq!(p.len(), length.clamp(MIN, MAX));
        }
        let digits = Recipe {
            upper: false,
            lower: false,
            symbols: false,
            ..Recipe::default()
        };
        assert!(generate(&digits).bytes().all(|b| b.is_ascii_digit()));
        for _ in 0..200 {
            let p = generate(&Recipe {
                length: 8,
                ..Recipe::default()
            });
            assert!(p.bytes().any(|b| UPPER.contains(&b)));
            assert!(p.bytes().any(|b| LOWER.contains(&b)));
            assert!(p.bytes().any(|b| DIGITS.contains(&b)));
            assert!(p.bytes().any(|b| SYMBOLS.contains(&b)));
        }
    }

    #[test]
    fn characters_are_even() {
        let recipe = Recipe {
            length: 64,
            upper: false,
            lower: false,
            symbols: false,
            ..Recipe::default()
        };
        let mut counts = [0u32; 10];
        for _ in 0..500 {
            for b in generate(&recipe).bytes() {
                counts[(b - b'0') as usize] += 1;
            }
        }
        let expected = 500.0 * 64.0 / 10.0;
        for c in counts {
            assert!(
                (f64::from(c) - expected).abs() < expected * 0.1,
                "{counts:?}"
            );
        }
    }
}
