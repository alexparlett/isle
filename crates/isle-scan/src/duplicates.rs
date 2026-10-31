//! Duplicates (D90; `p19b-cleaner.md` choices 14 and 15): the walk's candidates grouped by
//! size, then by both ends, then by their whole content, read at idle I/O priority; and which
//! of a file's extents are shared, so a reflinked copy is said to free nothing.

use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek, SeekFrom},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    rules::Find,
    tree::{File, Tree},
};

const ENDS: u64 = 16 * 1024;

/// One file of a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copy {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified: i64,
    /// Every extent is shared with another file: removing it frees nothing.
    pub shares_storage: bool,
}

/// Files with the same content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub size: u64,
    pub copies: Vec<Copy>,
}

/// The walk's candidates less those a rule has found, each with its path.
pub fn candidates(tree: &Tree, finds: &[Find]) -> Vec<(PathBuf, File)> {
    let claimed: std::collections::HashSet<&Path> =
        finds.iter().map(|f| f.path.as_path()).collect();
    tree.candidates
        .iter()
        .map(|f| (tree.file_path(f), f.clone()))
        .filter(|(p, _)| !p.ancestors().any(|a| claimed.contains(a)))
        .collect()
}

/// Puts the calling thread in the idle I/O class, so a game loading wins the disk.
pub fn idle_io() {
    const IOPRIO_WHO_PROCESS: libc::c_long = 1;
    const IOPRIO_CLASS_IDLE: libc::c_long = 3;
    // SAFETY: ioprio_set takes plain integers; 0 is the calling thread.
    unsafe {
        libc::syscall(
            libc::SYS_ioprio_set,
            IOPRIO_WHO_PROCESS,
            0,
            IOPRIO_CLASS_IDLE << 13,
        );
    }
}

/// Opens for reading without touching the access time, where the file is the user's own.
fn open(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOATIME)
        .open(path)
        .or_else(|_| fs::File::open(path))
}

fn ends(path: &Path, size: u64) -> Option<u64> {
    let mut f = open(path).ok()?;
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut buf = vec![0u8; ENDS as usize];
    let head = size.min(ENDS) as usize;
    f.read_exact(&mut buf[..head]).ok()?;
    hasher.update(&buf[..head]);
    if size > ENDS {
        let tail = (size - ENDS).max(ENDS);
        f.seek(SeekFrom::Start(tail)).ok()?;
        let n = (size - tail) as usize;
        f.read_exact(&mut buf[..n]).ok()?;
        hasher.update(&buf[..n]);
    }
    Some(hasher.digest())
}

fn whole(path: &Path, stop: &AtomicBool) -> Option<[u8; 32]> {
    let mut f = open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if stop.load(Ordering::Relaxed) {
            return None;
        }
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(*hasher.finalize().as_bytes())
}

/// Groups `candidates` by content: by size, then both ends, then the whole file, the last
/// only where the ends agree. Run it on a thread of its own; it gives up when `stop` is set.
pub fn compare(candidates: Vec<(PathBuf, File)>, stop: &AtomicBool) -> Vec<Group> {
    let mut by_size: HashMap<u64, Vec<(PathBuf, File)>> = HashMap::new();
    for c in candidates {
        by_size.entry(c.1.size).or_default().push(c);
    }
    let mut groups = Vec::new();
    for (size, same) in by_size.into_iter().filter(|(_, v)| v.len() > 1) {
        let mut by_ends: HashMap<u64, Vec<(PathBuf, File)>> = HashMap::new();
        for c in same {
            if stop.load(Ordering::Relaxed) {
                return Vec::new();
            }
            if let Some(h) = ends(&c.0, size) {
                by_ends.entry(h).or_default().push(c);
            }
        }
        for (_, same) in by_ends.into_iter().filter(|(_, v)| v.len() > 1) {
            let mut by_whole: HashMap<[u8; 32], Vec<(PathBuf, File)>> = HashMap::new();
            for c in same {
                match whole(&c.0, stop) {
                    Some(h) => by_whole.entry(h).or_default().push(c),
                    None if stop.load(Ordering::Relaxed) => return Vec::new(),
                    None => {}
                }
            }
            for (_, copies) in by_whole.into_iter().filter(|(_, v)| v.len() > 1) {
                let mut copies: Vec<Copy> = copies
                    .into_iter()
                    .map(|(path, f)| Copy {
                        shares_storage: sharing(&path)
                            .is_some_and(|s| s.bytes > 0 && s.shared == s.bytes),
                        path,
                        bytes: f.bytes,
                        modified: f.modified,
                    })
                    .collect();
                copies.sort_by_key(|c| std::cmp::Reverse(c.modified));
                groups.push(Group { size, copies });
            }
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.size * (g.copies.len() as u64 - 1)));
    groups
}

/// A file's extents: their bytes, and how many of them another file shares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sharing {
    pub bytes: u64,
    pub shared: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Extent {
    logical: u64,
    physical: u64,
    length: u64,
    reserved64: [u64; 2],
    flags: u32,
    reserved: [u32; 3],
}

const BATCH: usize = 256;

#[repr(C)]
struct Fiemap {
    start: u64,
    length: u64,
    flags: u32,
    mapped: u32,
    count: u32,
    reserved: u32,
    extents: [Extent; BATCH],
}

const FS_IOC_FIEMAP: libc::c_ulong = 0xC020_660B;
const EXTENT_LAST: u32 = 0x1;
const EXTENT_SHARED: u32 = 0x2000;

/// Reads `path`'s extent map with FIEMAP; none where the filesystem cannot say.
pub fn sharing(path: &Path) -> Option<Sharing> {
    let f = open(path).ok()?;
    let mut out = Sharing {
        bytes: 0,
        shared: 0,
    };
    let mut start = 0u64;
    loop {
        let mut map = Box::new(Fiemap {
            start,
            length: u64::MAX - start,
            flags: 0,
            mapped: 0,
            count: BATCH as u32,
            reserved: 0,
            extents: [Extent::default(); BATCH],
        });
        // SAFETY: `map` is a fiemap header followed by room for `count` extents, as the ioctl wants.
        let r = unsafe { libc::ioctl(f.as_raw_fd(), FS_IOC_FIEMAP, &mut *map as *mut Fiemap) };
        if r != 0 {
            return None;
        }
        let mapped = &map.extents[..map.mapped as usize];
        for e in mapped {
            out.bytes += e.length;
            if e.flags & EXTENT_SHARED != 0 {
                out.shared += e.length;
            }
        }
        match mapped.last() {
            Some(e) if e.flags & EXTENT_LAST == 0 => start = e.logical + e.length,
            _ => return Some(out),
        }
    }
}
