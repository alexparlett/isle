//! `settings.kdl` held in force: hand edits applied or rejected whole, Sets validated and
//! written back into the file's own text (`platform.md`, Settings).

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{Ctx, Leaf, Section, Seg, Value, line_column, parse_path, text};

/// Why the file's contents are not in force.
#[derive(Clone, Debug, PartialEq)]
pub struct FileError {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum Reload {
    /// The file holds what is already in force, Isle's own last write included.
    Unchanged,
    /// The keys whose values changed.
    Applied(Vec<String>),
    Rejected(FileError),
}

#[derive(Debug, PartialEq)]
pub enum SetError {
    /// A rejected hand edit stands; the file is left as typed (B27).
    Refused(FileError),
    Invalid(String),
    Io(String),
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetError::Refused(err) => write!(
                f,
                "settings.kdl has an error, so nothing is written until it is fixed: {err}"
            ),
            SetError::Invalid(message) => write!(f, "{message}"),
            SetError::Io(message) => write!(f, "could not write settings.kdl: {message}"),
        }
    }
}

pub struct Store<S: Section> {
    path: PathBuf,
    /// The file's text as last read or written; empty while there is no file.
    text: String,
    current: S,
    error: Option<FileError>,
}

impl<S: Section> Store<S> {
    /// Reads `path`. A missing file means every default; a broken one leaves the defaults
    /// in force with its error standing.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let mut store = Self {
            path: path.into(),
            text: String::new(),
            current: S::default(),
            error: None,
        };
        let text = read(&store.path);
        let _ = store.reload_text(text);
        store
    }

    /// Defaults only, the file never read: the safe start after repeated crashes. The
    /// file is not written either, since it may be what stopped Isle, until `use_file`.
    pub fn defaults(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let error = FileError {
            file: path.clone(),
            line: 0,
            column: 0,
            message: "Isle started on the default settings because it stopped soon after \
                      starting three times; settings.kdl was not read"
                .into(),
        };
        Self {
            path,
            text: String::new(),
            current: S::default(),
            error: Some(error),
        }
    }

    /// Reads the file and puts it in force, after a safe start.
    pub fn use_file(&mut self) -> Reload {
        let text = read(&self.path).unwrap_or_default();
        self.text = String::from("\0");
        let reload = self.reload_text(Some(text.clone()));
        if text.is_empty() && matches!(reload, Reload::Applied(_)) {
            self.text = text;
        }
        reload
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn current(&self) -> &S {
        &self.current
    }

    pub fn error(&self) -> Option<&FileError> {
        self.error.as_ref()
    }

    /// Reads the file again after it changed on disk.
    pub fn reload(&mut self) -> Reload {
        let text = read(&self.path);
        self.reload_text(text)
    }

    /// Takes `text` as the file's new contents; `None` when the file is gone.
    pub fn reload_text(&mut self, text: Option<String>) -> Reload {
        let text = text.unwrap_or_default();
        if text == self.text {
            return Reload::Unchanged;
        }
        self.text = text;
        match self.parse(&self.text) {
            Ok(new) => {
                let changed = changed_keys(&self.current, &new);
                self.current = new;
                self.error = None;
                Reload::Applied(changed)
            }
            Err(err) => {
                self.error = Some(err.clone());
                Reload::Rejected(err)
            }
        }
    }

    pub fn get(&self, key: &str) -> Result<Value, String> {
        self.values()
            .remove(key)
            .ok_or_else(|| format!("`{key}` is not set and has no default"))
    }

    /// Every key's value in force, defaults included; an optional key left unset has none.
    pub fn values(&self) -> BTreeMap<String, Value> {
        let mut all = BTreeMap::new();
        self.current.flatten("", &mut all);
        all
    }

    /// Validates and applies `value`, then writes it into the file. Returns the changed keys.
    pub fn set(&mut self, key: &str, value: Value) -> Result<Vec<String>, SetError> {
        if let Some(err) = &self.error {
            return Err(SetError::Refused(err.clone()));
        }
        let path = parse_path(key).map_err(SetError::Invalid)?;
        let mut new = self.current.clone();
        new.set(&path, value)
            .map_err(|e| SetError::Invalid(format!("{key}: {e}")))?;
        new.check().map_err(|e| SetError::Invalid(e.message))?;
        let leaf = new
            .kdl_args(&path)
            .ok_or_else(|| SetError::Invalid(format!("`{key}` is not a setting")))?;
        let doc = self.doc()?;
        let text = match leaf {
            Leaf::Args(args) => text::set_leaf(&self.text, &doc, &path, &args),
            Leaf::Unset => text::remove_node(&self.text, &doc, &path).unwrap_or_else(|| self.text.clone()),
        };
        match self.parse(&text) {
            Ok(written) if written == new => {}
            _ => {
                return Err(SetError::Io(format!(
                    "could not write `{key}` into the file"
                )));
            }
        }
        self.write(text, new)
    }

    /// Takes the node at `key` out of the file: a leaf goes back to its default, or unset
    /// when it has none; a section or a keyed entry goes back to all its defaults.
    pub fn reset(&mut self, key: &str) -> Result<Vec<String>, SetError> {
        if let Some(err) = &self.error {
            return Err(SetError::Refused(err.clone()));
        }
        let path = parse_path(key).map_err(SetError::Invalid)?;
        if !self.known(&path) {
            return Err(SetError::Invalid(format!("`{key}` is not a setting")));
        }
        let doc = self.doc()?;
        let Some(text) = text::remove_node(&self.text, &doc, &path) else {
            return Ok(Vec::new());
        };
        let new = self
            .parse(&text)
            .map_err(|e| SetError::Invalid(format!("{key}: {}", e.message)))?;
        self.write(text, new)
    }

    /// Deletes a keyed entry: a Focus mode, an app's notification rule, a display.
    pub fn remove(&mut self, entry: &str) -> Result<Vec<String>, SetError> {
        let path = parse_path(entry).map_err(SetError::Invalid)?;
        match path.last() {
            Some(seg) if seg.key.is_some() && seg.name != "app" => self.reset(entry),
            _ => Err(SetError::Invalid(format!("`{entry}` is not an entry"))),
        }
    }

    /// Whether `path` names a key, a section or an entry the schema has.
    fn known(&self, path: &[Seg]) -> bool {
        let pattern = path
            .iter()
            .map(|seg| match &seg.key {
                Some(key) if seg.name == "app" => format!("app[{key}]"),
                Some(_) => format!("{}[*]", seg.name),
                None => seg.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(".");
        let mut keys = Vec::new();
        S::keys("", &mut keys);
        keys.iter().any(|k| {
            k.key == pattern
                || k.key
                    .strip_prefix(&pattern)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
    }

    fn doc(&self) -> Result<kdl::KdlDocument, SetError> {
        self.text
            .parse()
            .map_err(|e| SetError::Io(format!("the file no longer parses: {e}")))
    }

    fn write(&mut self, text: String, new: S) -> Result<Vec<String>, SetError> {
        write_atomic(&self.path, &text).map_err(|e| SetError::Io(e.to_string()))?;
        let changed = changed_keys(&self.current, &new);
        self.text = text;
        self.current = new;
        Ok(changed)
    }

    fn parse(&self, text: &str) -> Result<S, FileError> {
        let at = |offset: usize, message: String| {
            let (line, column) = line_column(text, offset);
            FileError {
                file: self.path.clone(),
                line,
                column,
                message,
            }
        };
        let doc: kdl::KdlDocument = text.parse().map_err(|err: kdl::KdlError| {
            let first = err.diagnostics.first();
            let offset = first.map_or(0, |d| d.span.offset());
            let message = first
                .and_then(|d| d.message.clone().or_else(|| d.label.clone()))
                .unwrap_or_else(|| "this is not valid KDL".into());
            at(offset, message)
        })?;
        let mut ctx = Ctx::new(text);
        let parsed = S::parse(&doc, &mut ctx);
        match ctx.errors.first() {
            None => match parsed.check() {
                Ok(()) => Ok(parsed),
                Err(err) => Err(at(offset_of(&doc, &err.key), err.message)),
            },
            Some(first) => {
                let more = ctx.errors.len() - 1;
                let message = match more {
                    0 => first.message.clone(),
                    1 => format!("{} (and 1 more problem)", first.message),
                    n => format!("{} (and {n} more problems)", first.message),
                };
                Err(at(first.offset, message))
            }
        }
    }
}

/// Where the node a key path names starts, or the file's start when it is not written.
fn offset_of(doc: &kdl::KdlDocument, key: &str) -> usize {
    let Ok(path) = parse_path(key) else {
        return 0;
    };
    let mut nodes = doc.nodes();
    let mut offset = 0;
    for seg in &path {
        let found = nodes.iter().find(|n| {
            n.name().value() == seg.name
                && seg.key.as_ref().is_none_or(|k| {
                    n.entries().first().and_then(|e| e.value().as_string()) == Some(k.as_str())
                })
        });
        match found {
            Some(node) => {
                offset = node.span().offset();
                nodes = node.children().map_or(&[], |c| c.nodes());
            }
            None => break,
        }
    }
    offset
}

fn read(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn changed_keys<S: Section>(old: &S, new: &S) -> Vec<String> {
    let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
    old.flatten("", &mut a);
    new.flatten("", &mut b);
    let mut keys: Vec<String> = a
        .iter()
        .filter(|(k, v)| b.get(*k) != Some(*v))
        .map(|(k, _)| k.clone())
        .collect();
    keys.extend(b.keys().filter(|k| !a.contains_key(*k)).cloned());
    keys.sort();
    keys.dedup();
    keys
}

/// Writes through a symlink to its target, by temporary file, fsync and rename.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let dir = target.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "settings.kdl".into());
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    if let Ok(meta) = fs::metadata(&target) {
        let _ = fs::set_permissions(&temp, meta.permissions());
    }
    fs::rename(&temp, &target)?;
    fs::File::open(dir)?.sync_all()
}
