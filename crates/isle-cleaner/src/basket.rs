//! The basket (`apps.md` Cleaner 10): what the user picked to go, each path once, a folder
//! taking the place of anything under it.

use std::path::{Path, PathBuf};

use isle_scan::Kind;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
    pub kind: Kind,
    /// The device and inode the scan saw, checked again before moving.
    pub dev: u64,
    pub ino: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Basket {
    items: Vec<Item>,
}

impl Basket {
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Puts `item` in, unless a folder in the basket already holds it; anything under it goes.
    pub fn add(&mut self, item: Item) {
        if self.covers(&item.path) {
            return;
        }
        self.items.retain(|i| !i.path.starts_with(&item.path));
        self.items.push(item);
    }

    pub fn remove(&mut self, path: &Path) {
        self.items.retain(|i| i.path != path);
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.items.iter().any(|i| i.path == path)
    }

    /// The path is in the basket, or a folder above it is.
    pub fn covers(&self, path: &Path) -> bool {
        self.items.iter().any(|i| path.starts_with(&i.path))
    }

    pub fn total(&self) -> u64 {
        self.items.iter().map(|i| i.bytes).sum()
    }

    /// Each item's bytes as `bytes_of` gives them, where it does.
    pub fn resize(&mut self, bytes_of: impl Fn(&Path) -> Option<u64>) {
        for i in &mut self.items {
            if let Some(b) = bytes_of(&i.path) {
                i.bytes = b;
            }
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str, bytes: u64) -> Item {
        Item {
            path: path.into(),
            name: path.into(),
            bytes,
            kind: Kind::Other,
            dev: 0,
            ino: 0,
        }
    }

    #[test]
    fn a_folder_takes_the_place_of_what_is_under_it() {
        let mut b = Basket::default();
        b.add(item("/h/.cache/a/x", 1));
        b.add(item("/h/.cache/b", 2));
        b.add(item("/h/.cache", 10));
        assert_eq!(b.items().len(), 1);
        assert_eq!(b.total(), 10);
        b.add(item("/h/.cache/c", 3));
        assert_eq!(b.items().len(), 1, "already held by its folder");
        assert!(b.covers(Path::new("/h/.cache/c")) && !b.contains(Path::new("/h/.cache/c")));
        b.add(item("/h/.cachex", 4));
        assert_eq!(
            b.items().len(),
            2,
            "a sibling with a longer name is not under it"
        );
    }
}
