//! Files by name from Isle's own index, and by content on demand (`p10-spotlight.md`
//! choices 12 and 13).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

/// What the index walks: roots, without hidden folders, what `.gitignore` files leave out,
/// and the folders excluded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    pub roots: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,
}

impl Reach {
    fn walker(&self, from: &Path, depth: Option<usize>) -> ignore::Walk {
        let exclude = self.exclude.clone();
        ignore::WalkBuilder::new(from)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .require_git(false)
            .follow_links(false)
            .max_depth(depth)
            .filter_entry(move |e| !exclude.iter().any(|x| e.path().starts_with(x)))
            .build()
    }

    /// Whether `path` is one the index keeps: under a root, not excluded, not hidden below
    /// the root. `.gitignore` is checked by the walk.
    pub fn covers(&self, path: &Path) -> bool {
        let Some(root) = self.roots.iter().find(|r| path.starts_with(r)) else {
            return false;
        };
        if self.exclude.iter().any(|x| path.starts_with(x)) {
            return false;
        }
        let below = path.strip_prefix(root).unwrap_or(path);
        !below
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    }
}

/// The index: every file and folder in reach, by folder.
#[derive(Clone, Debug, Default)]
pub struct FileIndex {
    reach: Reach,
    /// Each folder's entries by name.
    folders: BTreeMap<PathBuf, BTreeMap<String, Entry>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    dir: bool,
    /// The name lower-cased, for a cheap test before the matcher.
    folded: String,
}

impl Entry {
    fn new(name: &str, dir: bool) -> Self {
        Self {
            dir,
            folded: name.to_lowercase(),
        }
    }
}

pub type Shared = Arc<RwLock<FileIndex>>;

impl FileIndex {
    /// Walks `reach` whole; blocking, so on the worker.
    pub fn build(reach: Reach) -> Self {
        let mut index = Self {
            reach: reach.clone(),
            folders: BTreeMap::new(),
        };
        for root in &reach.roots {
            index.walk(root);
        }
        index
    }

    pub fn reach(&self) -> &Reach {
        &self.reach
    }

    /// The folders the index holds, each to be watched.
    pub fn folders(&self) -> impl Iterator<Item = &PathBuf> {
        self.folders.keys()
    }

    pub fn len(&self) -> usize {
        self.folders.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn walk(&mut self, from: &Path) {
        self.folders.entry(from.to_owned()).or_default();
        for entry in self.reach.walker(from, None).flatten() {
            let path = entry.path();
            if path == from {
                continue;
            }
            let dir = entry.file_type().is_some_and(|t| t.is_dir());
            if dir {
                self.folders.entry(path.to_owned()).or_default();
            }
            if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
                let name = name.to_string_lossy().into_owned();
                let entry = Entry::new(&name, dir);
                self.folders
                    .entry(parent.to_owned())
                    .or_default()
                    .insert(name, entry);
            }
        }
    }

    /// Reads `folder` again after a change in it: new entries join, new folders are walked,
    /// gone ones leave with all they held. The folders added, to be watched.
    pub fn sync(&mut self, folder: &Path) -> Vec<PathBuf> {
        if !self.reach.covers(folder) && !self.reach.roots.iter().any(|r| r == folder) {
            return Vec::new();
        }
        if !folder.is_dir() {
            self.forget(folder);
            return Vec::new();
        }
        let mut now: BTreeMap<String, Entry> = BTreeMap::new();
        for entry in self.reach.walker(folder, Some(1)).flatten() {
            if entry.path() == folder {
                continue;
            }
            if let Some(name) = entry.path().file_name() {
                let name = name.to_string_lossy().into_owned();
                let dir = entry.file_type().is_some_and(|t| t.is_dir());
                now.insert(name.clone(), Entry::new(&name, dir));
            }
        }
        let before = self.folders.get(folder).cloned().unwrap_or_default();
        let is_dir =
            |map: &BTreeMap<String, Entry>, name: &str| map.get(name).is_some_and(|e| e.dir);
        for (name, entry) in &before {
            if entry.dir && !is_dir(&now, name) {
                self.forget(&folder.join(name));
            }
        }
        let mut added = Vec::new();
        for (name, entry) in &now {
            if entry.dir && !is_dir(&before, name) {
                let path = folder.join(name);
                let had: BTreeSet<PathBuf> = self.folders.keys().cloned().collect();
                self.walk(&path);
                added.extend(self.folders.keys().filter(|k| !had.contains(*k)).cloned());
            }
        }
        self.folders.insert(folder.to_owned(), now);
        added
    }

    /// Drops `folder` and everything under it.
    fn forget(&mut self, folder: &Path) {
        let gone: Vec<PathBuf> = self
            .folders
            .range(folder.to_owned()..)
            .take_while(|(k, _)| k.starts_with(folder))
            .map(|(k, _)| k.clone())
            .collect();
        for k in gone {
            self.folders.remove(&k);
        }
        if let (Some(parent), Some(name)) = (folder.parent(), folder.file_name())
            && let Some(entries) = self.folders.get_mut(parent)
        {
            entries.remove(&*name.to_string_lossy());
        }
    }

    /// Every entry: its folder, name and whether it is a folder.
    pub fn entries(&self) -> impl Iterator<Item = (&Path, &str, bool)> {
        self.folders.iter().flat_map(|(dir, names)| {
            names
                .iter()
                .map(move |(n, e)| (dir.as_path(), n.as_str(), e.dir))
        })
    }

    /// Every entry whose lower-cased name holds `folded`.
    fn holding<'a>(&'a self, folded: &'a str) -> impl Iterator<Item = (&'a Path, &'a str, bool)> {
        self.folders.iter().flat_map(move |(dir, names)| {
            names
                .iter()
                .filter(move |(_, e)| e.folded.contains(folded))
                .map(move |(n, e)| (dir.as_path(), n.as_str(), e.dir))
        })
    }
}

/// `path` with the home folder as `~`.
pub fn tilde(path: &Path) -> String {
    match std::env::var_os("HOME").map(PathBuf::from) {
        Some(home) if path.starts_with(&home) => {
            format!("~/{}", path.strip_prefix(&home).unwrap_or(path).display())
                .trim_end_matches('/')
                .to_owned()
        }
        _ => path.display().to_string(),
    }
}

/// Recently used files from `recently-used.xbel`, newest first.
pub fn recent(xbel: &str, limit: usize) -> Vec<PathBuf> {
    recent_by(xbel, limit, &[])
}

/// As `recent`, only the files one of `apps` used, by the names the file gives them: an
/// app's name or its command. All files when `apps` is empty.
pub fn recent_by(xbel: &str, limit: usize, apps: &[String]) -> Vec<PathBuf> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for chunk in xbel.split("<bookmark ").skip(1) {
        let attr = |name: &str| {
            let at = chunk.find(&format!("{name}=\""))? + name.len() + 2;
            Some(&chunk[at..at + chunk[at..].find('"')?])
        };
        let (Some(href), modified) = (attr("href"), attr("modified").or_else(|| attr("visited")))
        else {
            continue;
        };
        let Some(path) = href.strip_prefix("file://") else {
            continue;
        };
        if !apps.is_empty() {
            let used: Vec<&str> = chunk
                .split("<bookmark:application ")
                .skip(1)
                .flat_map(|app| {
                    let attr = |name: &str| {
                        let at = app.find(&format!("{name}=\""))? + name.len() + 2;
                        Some(&app[at..at + app[at..].find('"')?])
                    };
                    let exec = attr("exec").map(|e| {
                        e.trim_matches(|c| c == '\'' || c == '&' || c == ';' || c == ' ')
                            .trim_start_matches("apos;")
                    });
                    [attr("name"), exec.and_then(|e| e.split_whitespace().next())]
                })
                .flatten()
                .collect();
            if !used.iter().any(|u| apps.iter().any(|a| a == u)) {
                continue;
            }
        }
        let path = PathBuf::from(percent_decode(path));
        if path.exists() {
            found.push((modified.unwrap_or("").to_owned(), path));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.dedup_by(|a, b| a.1 == b.1);
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn file_row(
    key: String,
    path: &Path,
    dir: bool,
    detail: String,
    rank: crate::Rank,
    matched: Vec<u32>,
) -> Row {
    Row {
        key,
        group: Group::Files,
        title: path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        detail: Some(detail),
        trailing: None,
        icon: Icon::Glyph(if dir { "folder" } else { "file" }),
        item: Item::File(path.to_owned()),
        section: None,
        rank,
        matched,
    }
}

/// Files by name: a handful in the general search, more in the Files scope, recent files
/// for an empty one.
pub struct FilesSource {
    pub index: Shared,
    /// `recently-used.xbel`, read for an empty Files scope and an app's recent files.
    pub xbel: Option<PathBuf>,
    /// Each app's names a recent file may give it, by desktop id: its name and command.
    pub app_names: std::collections::HashMap<String, Vec<String>>,
}

pub const GENERAL_LIMIT: usize = 5;
pub const SCOPE_LIMIT: usize = 50;

impl Source for FilesSource {
    fn group(&self) -> Group {
        Group::Files
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(
            scope,
            Scope::General | Scope::Files { contents: false } | Scope::App(_)
        )
    }

    fn slow(&self) -> bool {
        true
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        if let Scope::App(app) = &query.scope {
            let names = self.app_names.get(app).cloned().unwrap_or_default();
            if names.is_empty() {
                return Vec::new();
            }
            let xbel = self
                .xbel
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            let m = cx.matcher.find("", "").expect("an empty query matches");
            return recent_by(&xbel, GENERAL_LIMIT, &names)
                .into_iter()
                .filter(|p| {
                    text.is_empty()
                        || p.file_name().is_some_and(|n| {
                            n.to_string_lossy()
                                .to_lowercase()
                                .contains(&text.to_lowercase())
                        })
                })
                .map(|p| {
                    let parent = p.parent().map(tilde).unwrap_or_default();
                    file_row(
                        format!("file:{}", p.display()),
                        &p,
                        p.is_dir(),
                        parent,
                        crate::Rank::new(&m, 0.0, false, ""),
                        Vec::new(),
                    )
                })
                .collect();
        }
        if text.is_empty() {
            if query.scope == Scope::General {
                return Vec::new();
            }
            let text = self
                .xbel
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            return recent(&text, SCOPE_LIMIT)
                .iter()
                .filter_map(|p| {
                    let m = cx.matcher.find("", "")?;
                    let parent = p.parent().map(tilde).unwrap_or_default();
                    Some(file_row(
                        format!("file:{}", p.display()),
                        p,
                        p.is_dir(),
                        parent,
                        crate::Rank::new(&m, 0.0, false, ""),
                        Vec::new(),
                    ))
                })
                .collect();
        }
        let limit = if query.scope == Scope::General {
            GENERAL_LIMIT
        } else {
            SCOPE_LIMIT
        };
        let Ok(index) = self.index.read() else {
            return Vec::new();
        };
        let folded = text.to_lowercase();
        let mut rows: Vec<Row> = Vec::new();
        for (n, (dir, name, is_dir)) in index.holding(&folded).enumerate() {
            if n % 4096 == 0 && cx.stale(query) {
                return Vec::new();
            }
            let Some(m) = cx.matcher.find(text, name) else {
                continue;
            };
            if m.tier == crate::Tier::Fuzzy {
                continue;
            }
            let path = dir.join(name);
            let rank = crate::Rank::new(&m, 0.0, false, name);
            rows.push(file_row(
                format!("file:{}", path.display()),
                &path,
                is_dir,
                tilde(dir),
                rank,
                m.indices,
            ));
        }
        rows.sort_by(|a, b| {
            a.rank.cmp(&b.rank).then(
                a.detail
                    .as_ref()
                    .map(String::len)
                    .cmp(&b.detail.as_ref().map(String::len)),
            )
        });
        rows.truncate(limit);
        rows
    }
}

/// Files by content, searched on demand once typing pauses, at most twenty.
pub struct ContentsSource {
    pub index: Shared,
}

pub const CONTENTS_LIMIT: usize = 20;
pub const CONTENTS_PAUSE: Duration = Duration::from_millis(250);
const CONTENTS_MAX_SIZE: u64 = 20 << 20;

impl Source for ContentsSource {
    fn group(&self) -> Group {
        Group::Files
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::Files { contents: true }
    }

    fn slow(&self) -> bool {
        true
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        let text = query.text.trim();
        if text.chars().count() < 2 {
            return Vec::new();
        }
        std::thread::sleep(CONTENTS_PAUSE);
        if cx.stale(query) {
            return Vec::new();
        }
        let Some(mut contents) = Contents::new(text) else {
            return Vec::new();
        };
        let files: Vec<PathBuf> = match self.index.read() {
            Ok(index) => index
                .entries()
                .filter(|e| !e.2)
                .map(|(d, n, _)| d.join(n))
                .collect(),
            Err(_) => return Vec::new(),
        };
        let mut rows = Vec::new();
        for path in files {
            if rows.len() >= CONTENTS_LIMIT || cx.stale(query) {
                break;
            }
            if std::fs::metadata(&path).map_or(true, |m| m.len() > CONTENTS_MAX_SIZE) {
                continue;
            }
            if let Some((line, text)) = contents.first_hit(&path) {
                let m = cx.matcher.find("", "").expect("an empty query matches");
                let detail = format!(
                    "{text} · {}:{line}",
                    path.parent().map(tilde).unwrap_or_default()
                );
                rows.push(file_row(
                    format!("file:{}", path.display()),
                    &path,
                    false,
                    detail,
                    crate::Rank::new(&m, 0.0, false, ""),
                    Vec::new(),
                ));
            }
        }
        rows
    }
}

fn regex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{Matcher, Usage};

    fn tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("isle-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["notes/work", ".hidden", "code/target", "code/src", "skip"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("notes/plan.md"), "the plan\nquarterly numbers\n").unwrap();
        fs::write(root.join("notes/work/budget.txt"), "budget for Q3\n").unwrap();
        fs::write(root.join(".hidden/secret.md"), "plan").unwrap();
        fs::write(root.join("code/.gitignore"), "target/\n").unwrap();
        fs::write(root.join("code/target/plan.o"), "x").unwrap();
        fs::write(root.join("code/src/planner.rs"), "fn plan() {}\n").unwrap();
        fs::write(root.join("skip/plan.txt"), "plan").unwrap();
        root
    }

    fn reach(root: &Path) -> Reach {
        Reach {
            roots: vec![root.to_owned()],
            exclude: vec![root.join("skip")],
        }
    }

    fn names(index: &FileIndex) -> BTreeSet<String> {
        index.entries().map(|(_, n, _)| n.to_owned()).collect()
    }

    #[test]
    fn the_walk_leaves_out_hidden_ignored_and_excluded() {
        let root = tree("walk");
        let index = FileIndex::build(reach(&root));
        let names = names(&index);
        assert!(
            names.contains("plan.md")
                && names.contains("budget.txt")
                && names.contains("planner.rs")
        );
        assert!(!names.contains("secret.md"), "hidden");
        assert!(!names.contains("plan.o"), "gitignored");
        assert!(!names.contains("plan.txt"), "excluded");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_folder_read_again_follows_creates_and_deletes() {
        let root = tree("sync");
        let mut index = FileIndex::build(reach(&root));
        fs::write(root.join("notes/new.md"), "").unwrap();
        fs::create_dir_all(root.join("notes/fresh/deeper")).unwrap();
        fs::write(root.join("notes/fresh/deeper/inside.md"), "").unwrap();
        let added = index.sync(&root.join("notes"));
        assert!(added.contains(&root.join("notes/fresh/deeper")));
        assert!(names(&index).contains("new.md") && names(&index).contains("inside.md"));
        fs::remove_dir_all(root.join("notes/work")).unwrap();
        index.sync(&root.join("notes"));
        assert!(!names(&index).contains("budget.txt"));
        assert!(
            index
                .folders()
                .all(|f| !f.starts_with(root.join("notes/work")))
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn search(source: &dyn Source, scope: Scope, text: &str) -> Vec<Row> {
        let mut matcher = Matcher::new();
        let usage = Usage::default();
        let mut cx = Context {
            matcher: &mut matcher,
            usage: &usage,
            now: 0,
            latest: None,
        };
        source.search(
            &Query {
                text: text.into(),
                scope,
                generation: 1,
            },
            &mut cx,
        )
    }

    #[test]
    fn names_rank_by_tier_and_contents_find_lines() {
        let root = tree("search");
        let index: Shared = Arc::new(RwLock::new(FileIndex::build(reach(&root))));
        let files = FilesSource {
            index: index.clone(),
            xbel: None,
            app_names: Default::default(),
        };
        let rows = search(&files, Scope::Files { contents: false }, "plan");
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, ["plan.md", "planner.rs"]);
        let contents = ContentsSource { index };
        let rows = search(&contents, Scope::Files { contents: true }, "quarterly");
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0]
                .detail
                .as_ref()
                .unwrap()
                .starts_with("quarterly numbers ·"),
            "{:?}",
            rows[0].detail
        );
        assert!(rows[0].detail.as_ref().unwrap().ends_with(":2"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recent_files_are_read_newest_first() {
        let root = tree("recent");
        let a = root.join("notes/plan.md");
        let b = root.join("notes/work/budget.txt");
        let xbel = format!(
            "<xbel><bookmark href=\"file://{}\" modified=\"2026-10-01T10:00:00Z\"/><bookmark href=\"file://{}\" modified=\"2026-10-08T10:00:00Z\"/><bookmark href=\"file:///gone\" modified=\"2026-10-09T00:00:00Z\"/></xbel>",
            a.display(),
            b.display().to_string().replace(' ', "%20")
        );
        assert_eq!(recent(&xbel, 10), [b.clone(), a.clone()]);
        let by_app = format!(
            "<xbel><bookmark href=\"file://{}\" modified=\"2026-10-01T10:00:00Z\"><info><metadata><bookmark:applications><bookmark:application name=\"Text Editor\" exec=\"&apos;gnome-text-editor %u&apos;\"/></bookmark:applications></metadata></info></bookmark><bookmark href=\"file://{}\" modified=\"2026-10-02T10:00:00Z\"><info><metadata><bookmark:applications><bookmark:application name=\"Firefox\" exec=\"firefox %u\"/></bookmark:applications></metadata></info></bookmark></xbel>",
            a.display(),
            b.display()
        );
        assert_eq!(
            recent_by(&by_app, 10, &["gnome-text-editor".into()]),
            [a.clone()]
        );
        assert_eq!(recent_by(&by_app, 10, &["Firefox".into()]), [b]);
        fs::remove_dir_all(root).unwrap();
    }
}

/// A plain-text search of files' contents, as Spotlight's Contents and Files' Contents contain
/// both use: the text as typed, case ignored unless it has a capital, binary files skipped.
pub struct Contents {
    matcher: grep_regex::RegexMatcher,
    searcher: grep_searcher::Searcher,
}

impl Contents {
    pub fn new(text: &str) -> Option<Self> {
        use grep_regex::RegexMatcherBuilder;
        use grep_searcher::{BinaryDetection, SearcherBuilder};
        let smart = text.chars().all(|c| !c.is_uppercase());
        let matcher = RegexMatcherBuilder::new().case_insensitive(smart).build(&regex_escape(text)).ok()?;
        let searcher = SearcherBuilder::new().binary_detection(BinaryDetection::quit(0)).line_number(true).build();
        Some(Self { matcher, searcher })
    }

    /// The first line of `path` holding the text, and its number.
    pub fn first_hit(&mut self, path: &Path) -> Option<(u64, String)> {
        use grep_searcher::sinks::UTF8;
        let mut hit: Option<(u64, String)> = None;
        let _ = self.searcher.search_path(
            &self.matcher,
            path,
            UTF8(|line, text| {
                hit = Some((line, text.trim().chars().take(200).collect()));
                Ok(false)
            }),
        );
        hit
    }
}
