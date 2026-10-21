//! One item in a folder, as one `statx` finds it.

use std::{
    os::fd::AsFd,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustix::fs::{AtFlags, FileType, Statx, StatxFlags, statx};

use crate::natural_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    /// A symbolic link; `to_dir` when it leads to a folder, `broken` when it leads nowhere.
    Link { to_dir: bool, broken: bool },
    /// A device, socket or pipe.
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The name as shown; a name that is not UTF-8 is shown with replacement characters.
    pub name: String,
    pub path: PathBuf,
    pub kind: Kind,
    pub size: u64,
    /// Seconds since the epoch.
    pub modified: i64,
    pub created: Option<i64>,
    pub accessed: i64,
    pub mode: u32,
    pub uid: u32,
    /// The type its name says, `inode/directory` for a folder; `None` until its content is
    /// read when the name says nothing.
    pub mime: Option<Arc<str>>,
    pub key: Box<[u8]>,
    /// For an item in the Trash: where it came from and when.
    pub trashed: Option<Arc<crate::trash::Trashed>>,
}

/// What `statx` is asked for: the type and the columns.
const WANTED: StatxFlags = StatxFlags::TYPE
    .union(StatxFlags::MODE)
    .union(StatxFlags::UID)
    .union(StatxFlags::SIZE)
    .union(StatxFlags::MTIME)
    .union(StatxFlags::ATIME)
    .union(StatxFlags::BTIME);

/// Never mount an automount point and never ask a network server for fresh attributes.
const QUIET: AtFlags = AtFlags::NO_AUTOMOUNT.union(AtFlags::STATX_DONT_SYNC);

impl Entry {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, Kind::Dir | Kind::Link { to_dir: true, .. })
    }

    pub fn hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    /// The type to sort and group by: the folder or file type, empty while unknown.
    pub fn kind_name(&self) -> &str {
        self.mime.as_deref().unwrap_or("")
    }

    /// Reads `name` in the open folder `dir`, whose path is `parent`.
    pub fn read(dir: impl AsFd, parent: &Path, name: &std::ffi::OsStr) -> rustix::io::Result<Self> {
        let st = statx(&dir, name, AtFlags::SYMLINK_NOFOLLOW | QUIET, WANTED)?;
        let mut kind = kind_of(&st);
        if let Kind::Link { .. } = kind {
            kind = match statx(&dir, name, QUIET, StatxFlags::TYPE) {
                Ok(target) => Kind::Link {
                    to_dir: FileType::from_raw_mode(target.stx_mode.into()) == FileType::Directory,
                    broken: false,
                },
                Err(_) => Kind::Link { to_dir: false, broken: true },
            };
        }
        Ok(Self::from_statx(parent.join(name), kind, &st))
    }

    /// Reads one path, for a watched change or a job.
    pub fn at(path: &Path) -> rustix::io::Result<Self> {
        let parent = path.parent().unwrap_or(Path::new("/"));
        let name = path.file_name().unwrap_or(path.as_os_str());
        let dir = rustix::fs::open(
            parent,
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )?;
        Self::read(&dir, parent, name)
    }

    fn from_statx(path: PathBuf, kind: Kind, st: &Statx) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let mime = match kind {
            Kind::Dir | Kind::Link { to_dir: true, .. } => Some(Arc::from(isle_apps::mime::DIRECTORY)),
            _ => isle_apps::mime::types().by_name(&name).map(Arc::from),
        };
        let born = st.stx_mask & StatxFlags::BTIME.bits() != 0;
        Self {
            key: natural_key(&name),
            name,
            path,
            kind,
            size: st.stx_size,
            modified: st.stx_mtime.tv_sec,
            created: born.then_some(st.stx_btime.tv_sec),
            accessed: st.stx_atime.tv_sec,
            mode: u32::from(st.stx_mode) & 0o7777,
            uid: st.stx_uid,
            mime,
            trashed: None,
        }
    }
}

fn kind_of(st: &Statx) -> Kind {
    match FileType::from_raw_mode(st.stx_mode.into()) {
        FileType::Directory => Kind::Dir,
        FileType::RegularFile => Kind::File,
        FileType::Symlink => Kind::Link { to_dir: false, broken: false },
        _ => Kind::Other,
    }
}
