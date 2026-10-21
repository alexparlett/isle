//! A search and the walk that runs it (`p1/structure/files.md` § Searching; `p15-files.md` 31).

use std::path::PathBuf;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::RecvTimeoutError,
    },
    time::Duration,
};

use ignore::WalkState;
use isle_fs::{Entry, Listed};
use serde::{Deserialize, Serialize};

/// Contents are read from this many matching files at most.
const CONTENTS_LIMIT: usize = 500;
/// Names matched at most, so a search of everything stays a list.
const NAMES_LIMIT: usize = 5000;
const CONTENTS_MAX_SIZE: u64 = 20 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Everything under the folder the search began in.
    Under,
    Home,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KindIs {
    Any,
    Folder,
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Code,
    App,
}

pub const KINDS: [(KindIs, &str); 9] = [
    (KindIs::Any, "Any"),
    (KindIs::Folder, "Folder"),
    (KindIs::Document, "Document"),
    (KindIs::Image, "Image"),
    (KindIs::Audio, "Audio"),
    (KindIs::Video, "Video"),
    (KindIs::Archive, "Archive"),
    (KindIs::Code, "Code"),
    (KindIs::App, "App"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Within {
    Today,
    Week,
    Month,
    Year,
}

pub const WITHIN: [(Within, &str); 4] = [
    (Within::Today, "today"),
    (Within::Week, "the last 7 days"),
    (Within::Month, "the last 30 days"),
    (Within::Year, "this year"),
];

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Criterion {
    Kind(KindIs),
    Modified(Within),
    Contents(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Search {
    pub term: String,
    pub scope: Scope,
    /// The folder the search began in; none from a gathering.
    pub folder: Option<PathBuf>,
    pub criteria: Vec<Criterion>,
    /// A saved search's name.
    pub name: Option<String>,
}

impl Search {
    pub fn root(&self) -> Option<PathBuf> {
        match self.scope {
            Scope::Under => self.folder.clone(),
            Scope::Home => crate::places::home(),
        }
    }

    /// "Searching “Projects”", or the saved search's name.
    pub fn title(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        let folder = self.root().map(|r| crate::places::folder_name(&r)).unwrap_or_default();
        format!("Searching “{folder}”")
    }
}

/// Whether an entry's type is of a kind.
pub fn is_kind(entry: &Entry, kind: KindIs) -> bool {
    let mime = entry.mime.as_deref().unwrap_or("");
    let name = entry.name.to_lowercase();
    match kind {
        KindIs::Any => true,
        KindIs::Folder => entry.is_dir(),
        KindIs::Image => mime.starts_with("image/"),
        KindIs::Audio => mime.starts_with("audio/"),
        KindIs::Video => mime.starts_with("video/"),
        KindIs::Archive => ["zip", "tar", "compressed", "7z", "rar", "bzip", "xz", "zstd"].iter().any(|k| mime.contains(k)),
        KindIs::App => name.ends_with(".appimage") || name.ends_with(".desktop") || mime == "application/x-executable",
        KindIs::Code => {
            mime.starts_with("text/x-")
                || ["javascript", "json", "toml", "yaml", "x-shellscript", "rust", "python", "x-ruby", "xml"].iter().any(|k| mime.contains(k))
        }
        KindIs::Document => {
            !entry.is_dir()
                && (mime.starts_with("text/") || mime == "application/pdf" || mime.contains("opendocument") || mime.contains("officedocument") || mime.contains("msword"))
        }
    }
}

fn within(entry: &Entry, within: Within, now: i64) -> bool {
    let day = 86_400;
    let age = now - entry.modified;
    match within {
        Within::Today => age < day,
        Within::Week => age < 7 * day,
        Within::Month => age < 30 * day,
        Within::Year => age < 365 * day,
    }
}

/// Whether a name holds the term, case ignored.
pub fn name_matches(name: &str, term: &str) -> bool {
    term.is_empty() || name.to_lowercase().contains(&term.to_lowercase())
}

/// Walks the search's folders on threads of their own and sends what matches in batches;
/// blocks until the walk ends or `send` says no more. Hidden folders are skipped unless hidden
/// files are shown, and what `.gitignore` names, as Spotlight's index does (B76).
pub fn run(search: &Search, hidden: bool, mut send: impl FnMut(Listed) -> bool) {
    let asks = !search.term.trim().is_empty() || !search.criteria.is_empty();
    let Some(root) = search.root().filter(|_| asks) else {
        send(Listed::Done);
        return;
    };
    let contents: Option<String> = search.criteria.iter().find_map(|c| match c {
        Criterion::Contents(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        _ => None,
    });
    let walker = ignore::WalkBuilder::new(&root)
        .hidden(!hidden)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .parents(false)
        .follow_links(false)
        .require_git(false)
        .build_parallel();
    let stop = Arc::new(AtomicBool::new(false));
    let read = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Entry>(1024);
    let walking = {
        let (search, stop, root) = (search.clone(), stop.clone(), root.clone());
        let now = jiff::Timestamp::now().as_second();
        std::thread::Builder::new().name("isle-files-search".into()).spawn(move || {
            walker.run(|| {
                let (tx, stop, read, search, root) = (tx.clone(), stop.clone(), read.clone(), search.clone(), root.clone());
                let mut matcher = contents.as_deref().map(isle_search::files::Contents::new);
                Box::new(move |found| {
                    if stop.load(Ordering::Relaxed) {
                        return WalkState::Quit;
                    }
                    let Ok(found) = found else { return WalkState::Continue };
                    let path = found.path();
                    if path == root || !name_matches(&found.file_name().to_string_lossy(), search.term.trim()) {
                        return WalkState::Continue;
                    }
                    let Ok(entry) = Entry::at(path) else { return WalkState::Continue };
                    let fits = search.criteria.iter().all(|c| match c {
                        Criterion::Kind(kind) => is_kind(&entry, *kind),
                        Criterion::Modified(w) => within(&entry, *w, now),
                        Criterion::Contents(_) => true,
                    });
                    if !fits {
                        return WalkState::Continue;
                    }
                    if let Some(matcher) = matcher.as_mut() {
                        let Some(matcher) = matcher.as_mut() else { return WalkState::Continue };
                        if entry.is_dir() || entry.size > CONTENTS_MAX_SIZE {
                            return WalkState::Continue;
                        }
                        if read.fetch_add(1, Ordering::Relaxed) >= CONTENTS_LIMIT {
                            return WalkState::Quit;
                        }
                        if matcher.first_hit(path).is_none() {
                            return WalkState::Continue;
                        }
                    }
                    if tx.send(entry).is_err() { WalkState::Quit } else { WalkState::Continue }
                })
            });
        })
    };
    let (mut batch, mut sent, mut wanted) = (Vec::new(), 0usize, true);
    let mut flushed = std::time::Instant::now();
    loop {
        let next = rx.recv_timeout(Duration::from_millis(100));
        let ended = matches!(next, Err(RecvTimeoutError::Disconnected));
        if let Ok(entry) = next {
            batch.push(entry);
            sent += 1;
        }
        let due = !batch.is_empty() && (batch.len() >= 256 || flushed.elapsed() >= Duration::from_millis(100));
        if wanted && (due || (ended && !batch.is_empty())) {
            wanted = send(Listed::Batch(std::mem::take(&mut batch)));
            flushed = std::time::Instant::now();
        }
        if !wanted || sent >= NAMES_LIMIT {
            stop.store(true, Ordering::Relaxed);
        }
        if ended {
            break;
        }
    }
    if let Ok(walking) = walking {
        let _ = walking.join();
    }
    if wanted {
        send(Listed::Done);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(search: &Search) -> Vec<String> {
        let mut found = Vec::new();
        run(search, false, |l| {
            if let Listed::Batch(b) = l {
                found.extend(b.into_iter().map(|e| e.name));
            }
            true
        });
        found.sort();
        found
    }

    #[test]
    fn a_search_walks_matches_names_and_reads_contents() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/deep")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join(".gitignore"), "target\n").unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::write(root.join("src/deep/notes.md"), "the island is here\n").unwrap();
        std::fs::write(root.join("src/Notes.txt"), "nothing\n").unwrap();
        std::fs::write(root.join(".hidden/notes.md"), "island\n").unwrap();
        std::fs::write(root.join("target/notes.md"), "island\n").unwrap();
        let mut search = Search {
            term: "notes".into(),
            scope: Scope::Under,
            folder: Some(root.to_owned()),
            criteria: Vec::new(),
            name: None,
        };
        assert_eq!(collect(&search), ["Notes.txt", "notes.md"], "hidden and ignored folders skipped, case ignored");
        search.criteria.push(Criterion::Contents("island".into()));
        assert_eq!(collect(&search), ["notes.md"]);
        search.criteria = vec![Criterion::Kind(KindIs::Folder)];
        search.term = "deep".into();
        assert_eq!(collect(&search), ["deep"]);
    }
}
