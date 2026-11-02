//! The lock's and the greeter's field as text: a caret, a selection and the edits a macOS
//! text field makes, by character. A secure field moves and deletes by its whole text where
//! a plain one goes by words, as `NSSecureTextField` does.

use zeroize::{Zeroize, Zeroizing};

/// The longest text the field takes, in bytes.
pub const LIMIT: usize = 512;

pub struct Field {
    /// Its capacity is set once, so an edit never reallocates and leaves a copy behind.
    text: Zeroizing<String>,
    /// The caret, in characters.
    caret: usize,
    /// The selection's other end, when there is one.
    anchor: Option<usize>,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            text: Zeroizing::new(String::with_capacity(LIMIT + 4)),
            caret: 0,
            anchor: None,
        }
    }
}

impl Zeroize for Field {
    fn zeroize(&mut self) {
        self.text.zeroize();
        self.caret = 0;
        self.anchor = None;
    }
}

/// Where a move goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum To {
    Left,
    Right,
    WordLeft,
    WordRight,
    Start,
    End,
}

impl Field {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn chars(&self) -> usize {
        self.text.chars().count()
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The selected characters, when any are.
    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        let anchor = self.anchor?;
        (anchor != self.caret).then(|| anchor.min(self.caret)..anchor.max(self.caret))
    }

    fn byte(&self, at: usize) -> usize {
        self.text
            .char_indices()
            .nth(at)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Where `to` lands from the caret.
    fn target(&self, to: To, secure: bool) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let word = |c: &char| c.is_alphanumeric();
        match to {
            To::Left => self.caret.saturating_sub(1),
            To::Right => (self.caret + 1).min(chars.len()),
            To::Start => 0,
            To::End => chars.len(),
            To::WordLeft if secure => 0,
            To::WordRight if secure => chars.len(),
            To::WordLeft => {
                let mut at = self.caret;
                while at > 0 && !word(&chars[at - 1]) {
                    at -= 1;
                }
                while at > 0 && word(&chars[at - 1]) {
                    at -= 1;
                }
                at
            }
            To::WordRight => {
                let mut at = self.caret;
                while at < chars.len() && !word(&chars[at]) {
                    at += 1;
                }
                while at < chars.len() && word(&chars[at]) {
                    at += 1;
                }
                at
            }
        }
    }

    /// Moves the caret; `extend` grows the selection instead. A plain Left or Right with a
    /// selection lands on its edge.
    pub fn move_caret(&mut self, to: To, secure: bool, extend: bool) {
        if !extend && let Some(range) = self.selection() {
            self.anchor = None;
            self.caret = match to {
                To::Left => range.start,
                To::Right => range.end,
                _ => self.target(to, secure),
            };
            return;
        }
        let target = self.target(to, secure);
        if extend {
            self.anchor.get_or_insert(self.caret);
        } else {
            self.anchor = None;
        }
        self.caret = target;
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.caret = self.chars();
    }

    /// Takes out the selection; whether there was one.
    fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let (start, end) = (self.byte(range.start), self.byte(range.end));
        self.text.replace_range(start..end, "");
        self.caret = range.start;
        self.anchor = None;
        true
    }

    /// Deletes the selection, else from the caret to where `to` lands.
    pub fn delete(&mut self, to: To, secure: bool) {
        if self.delete_selection() {
            return;
        }
        let target = self.target(to, secure);
        let (from, until) = (self.caret.min(target), self.caret.max(target));
        let (start, end) = (self.byte(from), self.byte(until));
        self.text.replace_range(start..end, "");
        self.caret = from;
    }

    /// Types `chars` at the caret over the selection, up to the limit; whether any went in.
    pub fn insert(&mut self, chars: impl Iterator<Item = char>) -> bool {
        self.delete_selection();
        let mut took = false;
        for c in chars {
            if self.text.len() + c.len_utf8() > LIMIT {
                break;
            }
            let at = self.byte(self.caret);
            self.text.insert(at, c);
            self.caret += 1;
            took = true;
        }
        took
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(text: &str) -> Field {
        let mut f = Field::default();
        f.insert(text.chars());
        f
    }

    #[test]
    fn typing_goes_in_at_the_caret() {
        let mut f = field("helo");
        f.move_caret(To::Left, true, false);
        f.insert("l".chars());
        assert_eq!((f.as_str(), f.caret()), ("hello", 4));
    }

    #[test]
    fn shift_selects_and_typing_replaces_it() {
        let mut f = field("hunter2");
        f.move_caret(To::Left, true, true);
        f.move_caret(To::Left, true, true);
        assert_eq!(f.selection(), Some(5..7));
        f.insert("99".chars());
        assert_eq!(f.as_str(), "hunte99");
        assert_eq!(f.selection(), None);
    }

    #[test]
    fn left_with_a_selection_lands_on_its_start() {
        let mut f = field("abc");
        f.select_all();
        f.move_caret(To::Left, true, false);
        assert_eq!((f.caret(), f.selection()), (0, None));
    }

    #[test]
    fn a_secure_field_goes_by_its_whole_text_a_plain_one_by_words() {
        let mut f = field("one two");
        f.move_caret(To::WordLeft, true, false);
        assert_eq!(f.caret(), 0);
        f.move_caret(To::End, true, false);
        f.move_caret(To::WordLeft, false, false);
        assert_eq!(f.caret(), 4);
        f.delete(To::WordLeft, false);
        assert_eq!(f.as_str(), "two");
    }

    #[test]
    fn deleting_backward_forward_and_to_the_ends() {
        let mut f = field("abcdef");
        f.move_caret(To::Left, true, false);
        f.move_caret(To::Left, true, false);
        f.delete(To::Left, true);
        assert_eq!(f.as_str(), "abcef");
        f.delete(To::Right, true);
        assert_eq!(f.as_str(), "abcf");
        f.delete(To::End, true);
        assert_eq!(f.as_str(), "abc");
        f.delete(To::Start, true);
        assert_eq!((f.as_str(), f.caret()), ("", 0));
    }

    #[test]
    fn characters_beyond_ascii_are_whole() {
        let mut f = field("pä§");
        f.move_caret(To::Left, true, false);
        f.delete(To::Left, true);
        assert_eq!(f.as_str(), "p§");
    }

    #[test]
    fn the_limit_holds_and_nothing_reallocates() {
        let mut f = Field::default();
        let before = f.text.capacity();
        f.insert(std::iter::repeat_n('x', LIMIT + 10));
        assert_eq!(f.as_str().len(), LIMIT);
        assert_eq!(f.text.capacity(), before);
    }
}
