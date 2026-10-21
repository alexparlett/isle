//! File types from shared-mime-info: by name first, by content only when asked
//! (`p15-files.md` choice 11), with each type's description and icons.

use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use ::mime;
use xdg_mime::SharedMimeInfo;

pub const DIRECTORY: &str = "inode/directory";
pub const UNKNOWN: &str = "application/octet-stream";
pub const PLAIN: &str = "text/plain";
/// An empty file, which shared-mime-info names by its size.
pub const EMPTY: &str = "application/x-zerosize";

/// The bytes read to sniff a file's type; shared-mime-info's rules look no further.
const SNIFF: usize = 4096;

/// The globs in the data directories' `globs2` that are not a plain `*.suffix`.
fn special_globs(dirs: &[PathBuf]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for dir in dirs {
        let Ok(text) = std::fs::read_to_string(dir.join("mime/globs2")) else { continue };
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let Some(pattern) = line.split(':').nth(2) else { continue };
            let plain = pattern.strip_prefix("*.").is_some_and(|rest| !rest.contains(['*', '?', '[']));
            let pattern = pattern.to_lowercase();
            if !plain && pattern != "__noglobs__" && !out.contains(&pattern) {
                out.push(pattern);
            }
        }
    }
    out
}

/// A shell glob of `*`, `?` and `[…]` against a whole name.
fn glob_matches(glob: &[u8], name: &[u8]) -> bool {
    match glob.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => (0..=name.len()).any(|i| glob_matches(rest, &name[i..])),
        Some((b'?', rest)) => !name.is_empty() && glob_matches(rest, &name[1..]),
        Some((b'[', rest)) => {
            let Some(end) = rest.iter().position(|b| *b == b']') else { return false };
            let Some((&c, name_rest)) = name.split_first() else { return false };
            let (set, after) = (&rest[..end], &rest[end + 1..]);
            let mut hit = false;
            let mut i = 0;
            while i < set.len() {
                if i + 2 < set.len() && set[i + 1] == b'-' {
                    hit |= (set[i]..=set[i + 2]).contains(&c);
                    i += 3;
                } else {
                    hit |= set[i] == c;
                    i += 1;
                }
            }
            hit && glob_matches(after, name_rest)
        }
        Some((&b, rest)) => name.first() == Some(&b) && glob_matches(rest, &name[1..]),
    }
}

/// The database, read once.
pub struct Types {
    db: SharedMimeInfo,
    /// A name's type by everything after its first dot, or by the whole name when it has none.
    by_name: Mutex<HashMap<String, Option<String>>>,
    /// The globs that are not a plain `*.suffix`, lower-cased: a name one matches is asked of
    /// the database itself, as its answer is not its suffix's.
    special: OnceLock<Vec<String>>,
    comments: Mutex<HashMap<String, String>>,
    dirs: Vec<PathBuf>,
}

/// The session's database.
pub fn types() -> &'static Types {
    static TYPES: OnceLock<Types> = OnceLock::new();
    TYPES.get_or_init(|| Types::with(SharedMimeInfo::new(), data_dirs()))
}

fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["/usr/local/share".into(), "/usr/share".into()]);
    home.into_iter().chain(dirs).collect()
}

impl Types {
    fn with(db: SharedMimeInfo, dirs: Vec<PathBuf>) -> Self {
        Self {
            db,
            by_name: Mutex::default(),
            special: OnceLock::new(),
            comments: Mutex::default(),
            dirs,
        }
    }

    /// The database under one data directory alone, for tests.
    pub fn in_dir(dir: &Path) -> Self {
        Self::with(SharedMimeInfo::new_for_directory(dir.join("mime")), vec![dir.to_owned()])
    }

    /// The type a name says, if it says one.
    pub fn by_name(&self, name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        let special = self.special.get_or_init(|| special_globs(&self.dirs));
        let ask = || {
            self.db
                .get_mime_types_from_file_name(name)
                .into_iter()
                .next()
                .map(|m| self.canonical(&m))
                .filter(|m| m != UNKNOWN)
        };
        if special.iter().any(|g| glob_matches(g.as_bytes(), lower.as_bytes())) {
            return ask();
        }
        let shown = name.trim_start_matches('.');
        let key = match shown.split_once('.') {
            Some((_, rest)) if !rest.is_empty() => format!(".{rest}"),
            _ => name.to_owned(),
        };
        if let Some(known) = self.by_name.lock().unwrap().get(&key) {
            return known.clone();
        }
        let found = ask();
        self.by_name.lock().unwrap().insert(key, found.clone());
        found
    }

    /// The type a file's content says, read from its first bytes; for a name that says none.
    /// Never call it on a remote mount or the UI thread.
    pub fn sniff(&self, path: &Path) -> String {
        let mut head = Vec::with_capacity(SNIFF);
        let read = std::fs::File::open(path).and_then(|f| f.take(SNIFF as u64).read_to_end(&mut head));
        match read {
            Ok(0) => EMPTY.to_owned(),
            Ok(_) => match self.db.get_mime_type_for_data(&head) {
                Some((mime, _)) => self.canonical(&mime),
                None if looks_like_text(&head) => PLAIN.to_owned(),
                None => UNKNOWN.to_owned(),
            },
            Err(_) => UNKNOWN.to_owned(),
        }
    }

    /// A type by its own name rather than an alias.
    fn canonical(&self, mime: &mime::Mime) -> String {
        self.db
            .unalias_mime_type(mime)
            .unwrap_or_else(|| mime.clone())
            .essence_str()
            .to_owned()
    }

    /// Whether `kind` is `base` or one of its descendants: `text/markdown` is `text/plain`.
    pub fn is_a(&self, kind: &str, base: &str) -> bool {
        if kind == base {
            return true;
        }
        if base.ends_with("/*") {
            return kind.split('/').next() == base.split('/').next();
        }
        match (kind.parse(), base.parse()) {
            (Ok(kind), Ok(base)) => self.db.mime_type_subclass(&kind, &base),
            _ => false,
        }
    }

    /// The icon names for a type, most specific first.
    pub fn icons(&self, kind: &str) -> Vec<String> {
        if kind == DIRECTORY {
            return vec!["folder".into()];
        }
        match kind.parse() {
            Ok(mime) => self.db.lookup_icon_names(&mime),
            Err(_) => vec!["application-octet-stream".into()],
        }
    }

    /// The type's description in English, "Markdown document"; the type itself when the
    /// database has none.
    pub fn comment(&self, kind: &str) -> String {
        if let Some(known) = self.comments.lock().unwrap().get(kind) {
            return known.clone();
        }
        let found = self
            .dirs
            .iter()
            .find_map(|dir| std::fs::read_to_string(dir.join("mime").join(format!("{kind}.xml"))).ok())
            .and_then(|xml| comment_of(&xml))
            .unwrap_or_else(|| kind.to_owned());
        self.comments.lock().unwrap().insert(kind.to_owned(), found.clone());
        found
    }
}

/// The first `<comment>` without a language.
fn comment_of(xml: &str) -> Option<String> {
    let mut rest = xml;
    while let Some(at) = rest.find("<comment") {
        rest = &rest[at + "<comment".len()..];
        let close = rest.find('>')?;
        let attrs = &rest[..close];
        let body = &rest[close + 1..];
        let end = body.find("</comment>")?;
        if !attrs.contains("xml:lang") {
            return Some(unescape(&body[..end]));
        }
        rest = &body[end..];
    }
    None
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Text when it decodes as UTF-8 and holds no NUL, as GLib guesses.
fn looks_like_text(head: &[u8]) -> bool {
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        // Cut in the middle of a character at the read's end.
        Err(e) => e.error_len().is_none() && head.len() == SNIFF,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_is_the_one_without_a_language() {
        let xml = r#"<mime-type type="text/markdown"><comment xml:lang="de">Markdown-Dokument</comment><comment>Markdown document</comment></mime-type>"#;
        assert_eq!(comment_of(xml).as_deref(), Some("Markdown document"));
        assert_eq!(comment_of("<mime-type/>"), None);
    }

    #[test]
    fn text_is_utf8_without_nul() {
        assert!(looks_like_text(b"fn main() {}\n"));
        assert!(!looks_like_text(b"\x7fELF\x00\x01"));
        assert!(!looks_like_text(&[0xff, 0xfe, 0x41]));
    }

    #[test]
    fn the_system_database_names_and_sniffs() {
        let types = types();
        if types.by_name("a.png").is_none() {
            return; // no shared-mime-info here
        }
        assert_eq!(types.by_name("Shot.PNG").as_deref(), Some("image/png"));
        assert_eq!(types.by_name("notes.md").as_deref(), Some("text/markdown"));
        assert_eq!(types.by_name("Makefile").as_deref(), Some("text/x-makefile"));
        assert_eq!(types.by_name("README"), types.by_name("README"));
        // A whole-name glob answers for its name alone, never for its suffix.
        assert_eq!(types.by_name("todo.txt").as_deref(), Some("text/x-todo-txt"));
        assert_eq!(types.by_name("notebook.txt").as_deref(), Some(PLAIN));
        assert_eq!(types.by_name("a.tar.gz").as_deref(), Some("application/x-compressed-tar"));
        assert_eq!(types.by_name("b.gz").as_deref(), Some("application/gzip"));
        assert!(types.is_a("text/markdown", PLAIN));
        assert!(types.is_a("image/png", "image/*"));
        assert!(!types.is_a("image/png", PLAIN));
        assert_eq!(types.comment("text/markdown"), "Markdown document");
        assert!(types.icons("image/png").contains(&"image-png".to_owned()));

        let dir = std::env::temp_dir().join(format!("isle-mime-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("script"), "#!/bin/sh\necho hi\n").unwrap();
        std::fs::write(dir.join("empty"), "").unwrap();
        std::fs::write(dir.join("notes"), "plain words\n").unwrap();
        assert_eq!(types.sniff(&dir.join("script")), "text/x-shellscript");
        assert_eq!(types.sniff(&dir.join("empty")), EMPTY);
        assert_eq!(types.sniff(&dir.join("notes")), PLAIN);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
