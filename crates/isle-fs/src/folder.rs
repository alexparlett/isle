//! A folder's entries, or a gathering's from many folders, in their order, changed by batches and by single entries, never
//! rebuilt: what is shown keeps its place, its selection and its scroll (`p15-files.md`
//! choice 9).

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

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
    /// Items not shown: names beginning with a dot while hidden files are not shown, and
    /// names the filter leaves out.
    aside: Vec<Entry>,
    show_hidden: bool,
    /// Shows only names holding this, lowercased.
    filter: Option<String>,
    /// Each entry's place in `entries`, by path.
    at: HashMap<PathBuf, usize>,
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

    pub fn position(&self, path: &Path) -> Option<usize> {
        self.at.get(path).copied()
    }

    pub fn get(&self, path: &Path) -> Option<&Entry> {
        self.position(path).map(|i| &self.entries[i])
    }

    /// Shows or hides the items whose names begin with a dot.
    pub fn set_show_hidden(&mut self, show: bool) {
        if show != self.show_hidden {
            self.show_hidden = show;
            self.sort_out();
        }
    }

    /// Shows only the items whose names hold `filter`, case ignored; `None` shows them all.
    pub fn set_filter(&mut self, filter: Option<&str>) {
        let filter = filter.filter(|f| !f.is_empty()).map(str::to_lowercase);
        if filter != self.filter {
            self.filter = filter;
            self.sort_out();
        }
    }

    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    fn shows(&self, entry: &Entry) -> bool {
        (self.show_hidden || !entry.hidden()) && self.filter.as_ref().is_none_or(|f| entry.name.to_lowercase().contains(f))
    }

    /// Divides every item again into those shown and those set aside.
    fn sort_out(&mut self) {
        let mut all = std::mem::take(&mut self.entries);
        all.append(&mut self.aside);
        let (mut shown, aside): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| self.shows(e));
        let order = self.order;
        shown.sort_by(|a, b| order.compare(a, b));
        self.entries = shown;
        self.aside = aside;
        self.at.clear();
        self.reindex(0);
    }

    pub fn shows_hidden(&self) -> bool {
        self.show_hidden
    }

    /// Takes a batch's items aside while they are not shown.
    fn set_aside(&mut self, batch: &mut Vec<Entry>) {
        if self.show_hidden && self.filter.is_none() {
            return;
        }
        let (shown, aside): (Vec<_>, Vec<_>) = std::mem::take(batch).into_iter().partition(|e| self.shows(e));
        if !aside.is_empty() {
            let paths: HashSet<&Path> = aside.iter().map(|e| e.path.as_path()).collect();
            self.aside.retain(|e| !paths.contains(e.path.as_path()));
        }
        self.aside.extend(aside);
        *batch = shown;
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
    /// path is here already replaces it.
    pub fn merge(&mut self, mut batch: Vec<Entry>) {
        self.set_aside(&mut batch);
        let order = self.order;
        {
            let replaced: HashSet<&Path> = batch
                .iter()
                .filter(|e| self.at.contains_key(&e.path))
                .map(|e| e.path.as_path())
                .collect();
            if !replaced.is_empty() {
                self.entries.retain(|e| !replaced.contains(e.path.as_path()));
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

    /// Puts one entry in its place, replacing the one at its path; its index.
    pub fn upsert(&mut self, entry: Entry) -> Option<usize> {
        self.remove(&entry.path.clone());
        if !self.shows(&entry) {
            self.aside.push(entry);
            return None;
        }
        let order = self.order;
        let i = self.entries.partition_point(|e| order.compare(e, &entry).is_lt());
        self.entries.insert(i, entry);
        self.reindex(i);
        Some(i)
    }

    /// Takes an entry out; where it was.
    pub fn remove(&mut self, path: &Path) -> Option<usize> {
        self.aside.retain(|e| e.path != path);
        let i = self.at.remove(path)?;
        self.entries.remove(i);
        self.reindex(i);
        Some(i)
    }

    /// Keeps only the paths listed: the end of a listing read again.
    pub fn keep_only(&mut self, paths: &HashSet<PathBuf>) {
        self.entries.retain(|e| paths.contains(&e.path));
        self.aside.retain(|e| paths.contains(&e.path));
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
                    let path = dir.join(name);
                    match crate::Entry::at(&path) {
                        Ok(entry) => {
                            self.upsert(entry);
                        }
                        Err(_) => {
                            self.remove(&path);
                        }
                    }
                }
                Applied::Done
            }
        }
    }

    fn reindex(&mut self, from: usize) {
        for (i, e) in self.entries.iter().enumerate().skip(from) {
            self.at.insert(e.path.clone(), i);
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
            trashed: None,
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
        assert_eq!(f.position(Path::new("/x/b2")), Some(3));
    }

    #[test]
    fn hidden_items_wait_aside_until_shown() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry(".git", true), entry("src", true), entry(".env", false)]);
        assert_eq!(names(&f), ["src"]);
        assert_eq!(f.upsert(entry(".cache", true)), None);
        f.set_show_hidden(true);
        assert_eq!(names(&f), [".cache", ".git", "src", ".env"]);
        f.set_show_hidden(false);
        f.remove(Path::new("/x/.git"));
        f.set_show_hidden(true);
        assert_eq!(names(&f), [".cache", "src", ".env"]);
    }

    #[test]
    fn a_filter_sets_aside_what_it_leaves_out() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("Notes", false), entry("src", true), entry(".notes", false)]);
        f.set_filter(Some("NOT"));
        assert_eq!(names(&f), ["Notes"]);
        f.merge(vec![entry("notebook", false), entry("other", false)]);
        assert_eq!(f.upsert(entry("x", false)), None);
        assert_eq!(names(&f), ["notebook", "Notes"]);
        f.set_show_hidden(true);
        assert_eq!(names(&f), [".notes", "notebook", "Notes"]);
        f.set_filter(None);
        assert_eq!(names(&f), ["src", ".notes", "notebook", "Notes", "other", "x"]);
    }

    #[test]
    fn a_batch_replaces_an_entry_of_its_name() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("a", false), entry("b", false)]);
        let mut grown = entry("a", false);
        grown.size = 9;
        f.merge(vec![grown]);
        assert_eq!(names(&f), ["a", "b"]);
        assert_eq!(f.get(Path::new("/x/a")).unwrap().size, 9);
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
        assert_eq!(f.get(&dir.path().join("a")).unwrap().size, 5);
        std::fs::remove_file(dir.path().join("a")).unwrap();
        f.apply(dir.path(), &crate::watch::Change::Touched(["a".into()].into()));
        assert_eq!(names(&f), ["b"]);
    }

    #[test]
    fn single_changes_keep_the_index() {
        let mut f = Folder::new(Order::default());
        f.merge(vec![entry("a", false), entry("c", false)]);
        assert_eq!(f.upsert(entry("b", false)), Some(1));
        assert_eq!(f.position(Path::new("/x/c")), Some(2));
        assert_eq!(f.remove(Path::new("/x/a")), Some(0));
        assert_eq!(f.position(Path::new("/x/c")), Some(1));
        assert_eq!(f.remove(Path::new("/x/zz")), None);
    }
}
