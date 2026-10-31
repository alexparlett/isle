//! What a scan keeps (D87): every folder as a node with its own and total bytes, files and bytes
//! by kind, and the few files the views list.

use std::{
    collections::HashMap,
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use crate::kinds::{COUNT, Kind};

pub type Id = u32;
const NONE: Id = Id::MAX;

/// The node above the roots.
pub const TOP: Id = 0;

/// Names interned: many folders share one (`src`, `.git`'s `00` to `ff`).
#[derive(Default)]
pub struct Names {
    all: Vec<Box<[u8]>>,
    index: HashMap<Box<[u8]>, u32>,
}

impl Names {
    pub fn intern(&mut self, name: &[u8]) -> u32 {
        if let Some(i) = self.index.get(name) {
            return *i;
        }
        let i = self.all.len() as u32;
        self.all.push(name.into());
        self.index.insert(name.into(), i);
        i
    }

    pub fn get(&self, i: u32) -> &[u8] {
        &self.all[i as usize]
    }
}

pub(crate) const HIDDEN: u8 = 1;
pub(crate) const GIT: u8 = 2;
pub(crate) const UNREADABLE: u8 = 4;
pub(crate) const READ: u8 = 8;

pub struct Node {
    name: u32,
    pub parent: Id,
    first_child: Id,
    next: Id,
    /// The kind its folders give it, if any did.
    pub kind: Option<Kind>,
    pub(crate) flags: u8,
    /// Allocated bytes of the folder itself and the files directly in it.
    pub own_bytes: u64,
    pub own_files: u32,
    own_kinds: Vec<(Kind, u64)>,
    /// With everything below.
    pub bytes: u64,
    pub files: u64,
    pub dev: u64,
    pub ino: u64,
}

impl Node {
    pub fn unreadable(&self) -> bool {
        self.flags & UNREADABLE != 0
    }

    /// Whether the walk has listed it yet.
    pub fn read(&self) -> bool {
        self.flags & READ != 0
    }

    pub(crate) fn add_own(&mut self, kind: Kind, bytes: u64) {
        self.own_bytes += bytes;
        match self.own_kinds.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, b)) => *b += bytes,
            None => self.own_kinds.push((kind, bytes)),
        }
    }
}

/// A file a view lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub folder: Id,
    pub name: u32,
    /// Allocated bytes.
    pub bytes: u64,
    /// Its length.
    pub size: u64,
    pub kind: Kind,
    /// Seconds since the epoch: the later of access and modification, and modification.
    pub used: i64,
    pub modified: i64,
    pub dev: u64,
    pub ino: u64,
}

/// The largest `limit` files offered, smallest first once full.
pub struct Kept {
    limit: usize,
    files: Vec<File>,
    floor: u64,
}

impl Kept {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            files: Vec::new(),
            floor: 0,
        }
    }

    /// The size a file must beat to be kept.
    pub fn floor(&self) -> u64 {
        self.floor
    }

    pub fn offer(&mut self, file: File) {
        if self.files.len() < self.limit {
            self.files.push(file);
        } else if file.bytes > self.floor
            && let Some(i) = self.files.iter().position(|f| f.bytes == self.floor)
        {
            self.files[i] = file;
        } else {
            return;
        }
        if self.files.len() == self.limit {
            self.floor = self.files.iter().map(|f| f.bytes).min().unwrap_or(0);
        }
    }

    /// Largest first.
    pub fn sorted(&self) -> Vec<File> {
        let mut out = self.files.clone();
        out.sort_by_key(|f| std::cmp::Reverse(f.bytes));
        out
    }
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub names: Names,
    pub largest: Kept,
    pub untouched: Kept,
    /// Duplicate candidates (D90): a megabyte or more, not executable, outside hidden folders,
    /// git working trees, and the kinds that are an app's own.
    pub candidates: Vec<File>,
    /// Allocated bytes counted so far.
    pub scanned: u64,
    dirty: bool,
}

pub const KEPT: usize = 500;

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    pub fn new() -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            names: Names::default(),
            largest: Kept::new(KEPT),
            untouched: Kept::new(KEPT),
            candidates: Vec::new(),
            scanned: 0,
            dirty: false,
        };
        tree.push(NONE, b"", None, 0);
        tree.nodes[0].flags |= READ;
        tree
    }

    pub(crate) fn push(&mut self, parent: Id, name: &[u8], kind: Option<Kind>, flags: u8) -> Id {
        let id = self.nodes.len() as Id;
        let name = self.names.intern(name);
        let next = if parent == NONE {
            NONE
        } else {
            self.nodes[parent as usize].first_child
        };
        self.nodes.push(Node {
            name,
            parent,
            first_child: NONE,
            next,
            kind,
            flags,
            own_bytes: 0,
            own_files: 0,
            own_kinds: Vec::new(),
            bytes: 0,
            files: 0,
            dev: 0,
            ino: 0,
        });
        if parent != NONE {
            self.nodes[parent as usize].first_child = id;
        }
        self.dirty = true;
        id
    }

    pub(crate) fn node_mut(&mut self, id: Id) -> &mut Node {
        self.dirty = true;
        &mut self.nodes[id as usize]
    }

    pub fn node(&self, id: Id) -> &Node {
        &self.nodes[id as usize]
    }

    pub fn name(&self, id: Id) -> &OsStr {
        OsStr::from_bytes(self.names.get(self.nodes[id as usize].name))
    }

    pub fn file_name(&self, file: &File) -> &OsStr {
        OsStr::from_bytes(self.names.get(file.name))
    }

    pub fn children(&self, id: Id) -> impl Iterator<Item = Id> + '_ {
        let mut at = self.nodes[id as usize].first_child;
        std::iter::from_fn(move || {
            (at != NONE).then(|| {
                let this = at;
                at = self.nodes[at as usize].next;
                this
            })
        })
    }

    /// A node's path; a root's name is its whole path, and the top has none.
    pub fn path(&self, id: Id) -> PathBuf {
        let mut parts = Vec::new();
        let mut at = id;
        while at != TOP && at != NONE {
            parts.push(self.name(at));
            at = self.nodes[at as usize].parent;
        }
        parts.iter().rev().collect()
    }

    pub fn file_path(&self, file: &File) -> PathBuf {
        self.path(file.folder).join(self.file_name(file))
    }

    /// The node at `path`, if the scan reached it.
    pub fn find(&self, path: &Path) -> Option<Id> {
        let root = self
            .children(TOP)
            .find(|r| path.starts_with(self.path(*r)))?;
        let rest = path.strip_prefix(self.path(root)).ok()?;
        rest.iter().try_fold(root, |at, part| {
            self.children(at).find(|c| self.name(*c) == part)
        })
    }

    /// The folders that could not be read.
    pub fn unreadable(&self) -> impl Iterator<Item = Id> + '_ {
        (0..self.nodes.len() as Id).filter(|i| self.nodes[*i as usize].unreadable())
    }

    /// Brings every folder's totals up to date with what has been read; children always come
    /// after their parents, so one pass from the end carries them up.
    pub fn total(&mut self) {
        if !self.dirty {
            return;
        }
        for n in &mut self.nodes {
            n.bytes = n.own_bytes;
            n.files = u64::from(n.own_files);
        }
        for i in (1..self.nodes.len()).rev() {
            let (bytes, files, parent) = {
                let n = &self.nodes[i];
                (n.bytes, n.files, n.parent)
            };
            let p = &mut self.nodes[parent as usize];
            p.bytes += bytes;
            p.files += files;
        }
        self.dirty = false;
    }

    /// A folder's bytes by kind with everything below, added up as asked: a view asks for the
    /// few it shows, and keeping them for every folder would double the tree.
    pub fn kinds(&self, id: Id) -> [u64; COUNT] {
        let mut out = [0; COUNT];
        let mut stack = vec![id];
        while let Some(at) = stack.pop() {
            for (k, b) in &self.nodes[at as usize].own_kinds {
                out[k.index()] += b;
            }
            stack.extend(self.children(at));
        }
        out
    }

    /// The kind most of a folder is.
    pub fn main_kind(&self, id: Id) -> Kind {
        let kinds = self.kinds(id);
        Kind::ALL
            .into_iter()
            .max_by_key(|k| (kinds[k.index()], std::cmp::Reverse(k.index())))
            .unwrap_or(Kind::Other)
    }
}
