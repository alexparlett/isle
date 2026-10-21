//! A folder's entries in their order, changed by batches and by single entries, never
//! rebuilt: what is shown keeps its place, its selection and its scroll (`p15-files.md`
//! choice 9).

use std::collections::HashMap;

use crate::{Entry, Order};

/// What applying a change left for the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    Done,
    /// List the folder again.
    Reread,
    /// The folder was removed or moved away.
    Gone,
}

#[derive(Clone, Debug, Default)]
pub struct Folder {
    order: Order,
    entries: Vec<Entry>,
    /// Each entry's place in `entries`, by name.
    at: HashMap<String, usize>,
}

impl Folder {
    pub fn new(order: Order) -> Self {
        Self {
            order,
            ..Default::default()
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn order(&self) -> Order {
        self.order
    }

    pub fn position(&self, name: &str) -> Option<usize> {
        self.at.get(name).copied()
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.position(name).map(|i| &self.entries[i])
    }

    /// Sorts again by another order.
    pub fn set_order(&mut self, order: Order) {
        if order == self.order {
            return;
        }
        self.order = order;
        self.entries.sort_by(|a, b| order.compare(a, b));
        self.reindex(0);
    }

    /// Merges a batch, sorted on its own and then into the rest in one pass; an entry whose
    /// name is here already replaces it.
    pub fn merge(&mut self, mut batch: Vec<Entry>) {
        let order = self.order;
        {
            let replaced: std::collections::HashSet<&str> = batch
                .iter()
                .filter(|e| self.at.contains_key(&e.name))
                .map(|e| e.name.as_str())
                .collect();
            if !replaced.is_empty() {
                self.entries.retain(|e| !replaced.contains(e.name.as_str()));
            }
        }
        batch.sort_by(|a, b| order.compare(a, b));
        let old = std::mem::take(&mut self.entries);
        let mut merged = Vec::with_capacity(old.len() + batch.len());
        let (mut a, mut b) = (old.into_iter().peekable(), batch.into_iter().peekable());
        loop {
            let take_a = match (a.peek(), b.peek()) {
                (Some(x), Some(y)) => order.compare(x, y).is_le(),
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };
            merged.push(if take_a { a.next() } else { b.next() }.unwrap());
        }
        self.entries = merged;
        self.reindex(0);
    }

    /// Puts one entry in its place, replacing the one of its name; its index.
    pub fn upsert(&mut self, entry: Entry) -> usize {
        self.remove(&entry.name);
        let order = self.order;
        let i = self.entries.partition_point(|e| order.compare(e, &entry).is_lt());
        self.entries.insert(i, entry);
        self.reindex(i);
        i
    }

    /// Takes an entry out by name; where it was.
    pub fn remove(&mut self, name: &str) -> Option<usize> {
        let i = self.at.remove(name)?;
        self.entries.remove(i);
        self.reindex(i);
        Some(i)
    }

    /// Keeps only the names listed: the end of a listing read again.
    pub fn keep_only(&mut self, names: &std::collections::HashSet<String>) {
        self.entries.retain(|e| names.contains(&e.name));
        self.at.clear();
        self.reindex(0);
    }

    /// Reads each touched name again and puts it in its place, or takes it out; blocks on
    /// the filesystem, so run it where the listing runs. What the view must do instead, when
    /// the change is more than names.
    pub fn apply(&mut self, dir: &std::path::Path, change: &crate::watch::Change) -> Applied {
        use crate::watch::Change;
        match change {
            Change::Gone => Applied::Gone,
            Change::Overflow => Applied::Reread,
            Change::Touched(names) => {
                for name in names {
                    let shown = name.to_string_lossy();
                    match crate::Entry::at(&dir.join(name)) {
                        Ok(entry) => {
                            self.upsert(entry);
                        }
                        Err(_) => {
                            self.remove(&shown);
                        }
                    }
                }
                Applied::Done
            }
        }
    }

    fn reindex(&mut self, from: usize) {
        for (i, e) in self.entries.iter().enumerate().skip(from) {
            self.at.insert(e.name.clone(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, natural_key};

    fn entry(name: &str, dir: bool) -> Entry {
        Entry {
            name: name.into(),
            path: format!("/x/{name}").into(),
            kind: if dir { Kind::Dir } else { Kind::File },
            size: 0,
            modified: 0,
            created: None,
            accessed: 0,
            mode: 0o644,
            uid: 0,
            mime: None,
            key: natural_key(name),
        }
    }

    fn names(f: &Folder) -> Vec<&str> {
        f.entries().iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn batches_merge_in_order_with_folders_first() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("b10", false), entry("src", true), entry("a", false)]);
        f.merge(vec![entry("b2", false), entry("docs", true)]);
        assert_eq!(names(&f), ["docs", "src", "a", "b2", "b10"]);
        assert_eq!(f.position("b2"), Some(3));
    }

    #[test]
    fn a_batch_replaces_an_entry_of_its_name() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("a", false), entry("b", false)]);
        let mut grown = entry("a", false);
        grown.size = 9;
        f.merge(vec![grown]);
        assert_eq!(names(&f), ["a", "b"]);
        assert_eq!(f.get("a").unwrap().size, 9);
    }

    #[test]
    fn a_touched_name_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), "1").unwrap();
        std::fs::write(dir.path().join("b"), "").unwrap();
        let mut f = Folder::new(Order::default());
        f.merge(vec![crate::Entry::at(&dir.path().join("a")).unwrap()]);
        std::fs::write(dir.path().join("a"), "12345").unwrap();
        let touched = crate::watch::Change::Touched(["a".into(), "b".into(), "c".into()].into());
        assert_eq!(f.apply(dir.path(), &touched), Applied::Done);
        assert_eq!(names(&f), ["a", "b"]);
        assert_eq!(f.get("a").unwrap().size, 5);
        std::fs::remove_file(dir.path().join("a")).unwrap();
        f.apply(dir.path(), &crate::watch::Change::Touched(["a".into()].into()));
        assert_eq!(names(&f), ["b"]);
    }

    #[test]
    fn single_changes_keep_the_index() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("a", false), entry("c", false)]);
        assert_eq!(f.upsert(entry("b", false)), 1);
        assert_eq!(f.position("c"), Some(2));
        assert_eq!(f.remove("a"), Some(0));
        assert_eq!(f.position("c"), Some(1));
        assert_eq!(f.remove("zz"), None);
    }
}
