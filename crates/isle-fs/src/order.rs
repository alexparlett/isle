//! The order a folder's entries are shown in: by a column, folders first while sorting by
//! name, names compared naturally.

use std::cmp::Ordering;

use crate::Entry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortBy {
    #[default]
    Name,
    Kind,
    Modified,
    Created,
    Size,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub by: SortBy,
    pub reverse: bool,
    /// Folders before files when sorting by name (Finder's "Keep folders on top").
    pub folders_first: bool,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            by: SortBy::Name,
            reverse: false,
            folders_first: true,
        }
    }
}

impl Order {
    pub fn compare(&self, a: &Entry, b: &Entry) -> Ordering {
        if self.folders_first && self.by == SortBy::Name {
            match (a.is_dir(), b.is_dir()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }
        let by = match self.by {
            SortBy::Name => Ordering::Equal,
            SortBy::Kind => a.kind_name().cmp(b.kind_name()),
            // Newest and largest first, as Finder's columns sort on a first click.
            SortBy::Modified => b.modified.cmp(&a.modified),
            SortBy::Created => b.created.cmp(&a.created),
            SortBy::Size => b.size.cmp(&a.size),
        };
        let ordered = by.then_with(|| a.key.cmp(&b.key)).then_with(|| a.name.cmp(&b.name));
        if self.reverse { ordered.reverse() } else { ordered }
    }
}

/// A name's key for natural order as plain bytes: case folded, each run of digits compared
/// by its value, so "file2" comes before "File10".
pub fn natural_key(name: &str) -> Box<[u8]> {
    let mut key = Vec::with_capacity(name.len() + 4);
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut digits = String::from(c);
            while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                digits.push(d);
                chars.next();
            }
            let value = digits.trim_start_matches('0');
            // Below every printable byte, so a number sorts before a word in its place; its
            // length first, so a longer number is the larger.
            key.push(0x01);
            key.push(value.len().min(0xff) as u8);
            key.extend_from_slice(value.as_bytes());
        } else {
            for lower in c.to_lowercase() {
                let mut buf = [0; 4];
                key.extend_from_slice(lower.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    key.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<_> = names.iter().map(|n| (natural_key(n), n.to_string())).collect();
        names.sort();
        names.into_iter().map(|(_, n)| n).collect()
    }

    #[test]
    fn numbers_sort_by_value_and_case_is_ignored() {
        assert_eq!(
            sorted(&["file10", "File2", "file1", "file02b", "alpha", "Beta"]),
            ["alpha", "Beta", "file1", "File2", "file02b", "file10"]
        );
        assert_eq!(sorted(&["a b", "a1"]), ["a1", "a b"]);
        assert_eq!(sorted(&["v1.10", "v1.9"]), ["v1.9", "v1.10"]);
    }
}
