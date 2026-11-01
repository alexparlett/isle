//! What the window shows: the scope, the views, and a scan's snapshot as the views read it.

use std::{
    collections::BinaryHeap,
    path::{Path, PathBuf},
    sync::Arc,
};

use isle_scan::{Find, Kind, Progress, Scan, TOP, Tree, tree::Id};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum View {
    #[default]
    Overview,
    Cleanup,
    Space,
    Duplicates,
    LargeFiles,
    Untouched,
}

impl View {
    pub const ALL: [View; 6] = [
        View::Overview,
        View::Cleanup,
        View::Space,
        View::Duplicates,
        View::LargeFiles,
        View::Untouched,
    ];

    pub fn title(self) -> &'static str {
        match self {
            View::Overview => "Overview",
            View::Cleanup => "Cleanup",
            View::Space => "Space",
            View::Duplicates => "Duplicates",
            View::LargeFiles => "Large files",
            View::Untouched => "Untouched",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            View::Overview => "chart-pie-slice",
            View::Cleanup => "broom",
            View::Space => "layout",
            View::Duplicates => "copy",
            View::LargeFiles => "arrows-out-simple",
            View::Untouched => "clock-counter-clockwise",
        }
    }
}

/// What a scan covers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    #[default]
    All,
    /// A mounted disk, by its mount point.
    Disk(PathBuf),
    Folder(PathBuf),
}

/// A disk's name: the system's root is "System", another its mount point's last name.
pub fn disk_name(point: &Path) -> String {
    if point == Path::new("/") {
        return "System".into();
    }
    if point == Path::new("/home") {
        return "Home".into();
    }
    point.file_name().map_or_else(
        || point.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl Scope {
    pub fn roots(&self) -> Vec<PathBuf> {
        match self {
            Scope::All => isle_scan::all_disks(&isle_fs::mounts::read())
                .into_iter()
                .map(|d| d.point)
                .collect(),
            Scope::Disk(p) | Scope::Folder(p) => vec![p.clone()],
        }
    }

    pub fn title(&self) -> String {
        match self {
            Scope::All => "All disks".into(),
            Scope::Disk(p) => disk_name(p),
            Scope::Folder(p) => p.file_name().map_or_else(
                || p.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
        }
    }
}

/// A size in decimal units, as Files writes them.
pub fn size(n: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1000.0 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} bytes")
    } else if v >= 100.0 {
        format!("{v:.0} {}", UNITS[unit])
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// A path for a second line, `~` for the home.
pub fn shown_path(path: &Path) -> String {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    match path.strip_prefix(&home) {
        Ok(rest) if !home.as_os_str().is_empty() => {
            if rest.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", rest.display())
            }
        }
        _ => path.display().to_string(),
    }
}

/// A file a table lists.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRow {
    pub path: PathBuf,
    pub bytes: u64,
    pub kind: Kind,
    /// Seconds since the epoch: the later of access and modification.
    pub used: i64,
    pub modified: i64,
    pub dev: u64,
    pub ino: u64,
}

/// A folder Overview names.
#[derive(Clone, Debug, PartialEq)]
pub struct FolderRow {
    pub path: PathBuf,
    pub bytes: u64,
    pub kind: Kind,
    pub dev: u64,
    pub ino: u64,
}

/// One publication of a scan.
pub struct Snapshot {
    pub generation: u64,
    pub progress: Progress,
    pub roots: Vec<PathBuf>,
    pub bytes: u64,
    pub kinds: [u64; isle_scan::kinds::COUNT],
    pub unreadable: Vec<PathBuf>,
    pub folders: Vec<FolderRow>,
    pub largest: Vec<FileRow>,
    pub untouched: Vec<FileRow>,
    pub finds: Vec<Find>,
    /// The scan itself, for Space's folders.
    pub scan: Arc<Scan>,
    /// Counted at each file's bytes on disk, not before compression.
    pub measured: bool,
}

impl Snapshot {
    pub fn finished(&self) -> bool {
        self.progress.done || self.progress.partial
    }
}

fn file_rows(tree: &Tree, files: Vec<isle_scan::File>) -> Vec<FileRow> {
    files
        .into_iter()
        .map(|f| FileRow {
            path: tree.file_path(&f),
            bytes: f.bytes,
            kind: f.kind,
            used: f.used,
            modified: f.modified,
            dev: f.dev,
            ino: f.ino,
        })
        .collect()
}

/// Where the space is: from inside the roots down, a folder most of whose space is in one child
/// is only the way to that child, so the search goes on past it.
pub fn hotspots(tree: &Tree, limit: usize, trashes: &[PathBuf]) -> Vec<Id> {
    let mut heap: BinaryHeap<(u64, Id)> = tree
        .children(TOP)
        .flat_map(|root| tree.children(root))
        .map(|id| (tree.node(id).bytes, id))
        .collect();
    let mut out = Vec::new();
    while let Some((bytes, id)) = heap.pop() {
        if out.len() == limit || bytes == 0 {
            break;
        }
        if trashes.iter().any(|t| tree.path(id).starts_with(t)) {
            continue;
        }
        let biggest = tree
            .children(id)
            .map(|c| tree.node(c).bytes)
            .max()
            .unwrap_or(0);
        if biggest * 5 >= bytes * 3 {
            heap.extend(tree.children(id).map(|c| (tree.node(c).bytes, c)));
        } else {
            out.push(id);
        }
    }
    out
}

/// Reads a snapshot of `tree` for the views.
pub fn snapshot(
    generation: u64,
    scan: &Arc<Scan>,
    roots: &[PathBuf],
    rules: &isle_scan::Rules,
    ctx: &isle_scan::Context,
) -> Snapshot {
    let progress = scan.progress();
    // The Trash has its own row; its contents are not the user's files any more.
    let trashes: Vec<PathBuf> = isle_fs::trash::all_trashes()
        .into_iter()
        .map(|t| t.root)
        .collect();
    let kept = |f: &FileRow| !trashes.iter().any(|t| f.path.starts_with(t));
    scan.read(|tree| {
        let top = tree.node(TOP);
        let folders = hotspots(tree, 5, &trashes)
            .into_iter()
            .map(|id| FolderRow {
                path: tree.path(id),
                bytes: tree.node(id).bytes,
                kind: tree.main_kind(id),
                dev: tree.node(id).dev,
                ino: tree.node(id).ino,
            })
            .collect();
        Snapshot {
            generation,
            progress,
            roots: roots.to_vec(),
            bytes: top.bytes,
            kinds: tree.kinds(TOP),
            unreadable: tree.unreadable().map(|id| tree.path(id)).collect(),
            folders,
            largest: file_rows(tree, tree.largest.sorted())
                .into_iter()
                .filter(kept)
                .collect(),
            untouched: file_rows(tree, tree.untouched.sorted())
                .into_iter()
                .filter(kept)
                .collect(),
            finds: rules.find(tree, ctx),
            scan: scan.clone(),
            measured: false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_as_files_writes_them() {
        assert_eq!(size(0), "0 bytes");
        assert_eq!(size(999), "999 bytes");
        assert_eq!(size(1_500), "1.5 KB");
        assert_eq!(size(412_000_000_000), "412 GB");
        assert_eq!(size(18_200_000_000), "18.2 GB");
    }

    #[test]
    fn disks_are_named_for_their_place() {
        assert_eq!(disk_name(Path::new("/")), "System");
        assert_eq!(disk_name(Path::new("/media/d")), "d");
    }
}
