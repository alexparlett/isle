//! The walk (D87; `p19b-cleaner.md` choices 5–8): a pool of workers each reading one folder at a
//! time with `getdents64` and `statx`, never following a link, crossing only into mounts of the
//! filesystem it started on, and never into `.snapshots`.

use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

use isle_fs::mounts::{Class, Mount};
use rustix::fs::{Dir, FileType, Mode, OFlags, StatxFlags};

use crate::{
    kinds::{self, Kind},
    tree::{File, GIT, HIDDEN, Id, READ, TOP, Tree, UNREADABLE},
};

/// A file's last use older than this is untouched (`structure/cleaner.md`'s shortest period).
const UNTOUCHED_SECS: i64 = 182 * 24 * 3600;
/// Duplicates are looked for among files at least this long (D90).
pub const CANDIDATE_BYTES: u64 = 1 << 20;

/// A disk-backed filesystem All disks walks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disk {
    pub point: PathBuf,
    pub source: String,
    pub fstype: String,
}

fn disk_backed(m: &Mount) -> bool {
    m.class == Class::Local && m.source.starts_with("/dev/")
}

/// All disks' roots: each mounted disk-backed filesystem, less those another root's walk reaches
/// (D90), as the host's `/` reaches `/home`.
pub fn all_disks(mounts: &[Mount]) -> Vec<Disk> {
    let disks: Vec<&Mount> = mounts.iter().filter(|m| disk_backed(m)).collect();
    let mut out: Vec<Disk> = Vec::new();
    for m in &disks {
        let reached = disks.iter().any(|o| {
            !std::ptr::eq(*o, *m)
                && o.source == m.source
                && o.fstype == m.fstype
                && m.point.starts_with(&o.point)
                && m.point != o.point
        });
        if !reached && !out.iter().any(|d| d.point == m.point) {
            out.push(Disk {
                point: m.point.clone(),
                source: m.source.clone(),
                fstype: m.fstype.clone(),
            });
        }
    }
    out
}

/// How far a scan has got.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    /// Allocated bytes counted.
    pub scanned: u64,
    /// The used bytes of the disks the roots are on, where every root is a whole disk.
    pub used: Option<u64>,
    pub done: bool,
    /// Stopped before it ended.
    pub partial: bool,
}

impl Progress {
    /// The share scanned, 0 to 1. A compressed btrfs counts more than its used bytes, so the
    /// share waits at 99% until the walk ends.
    pub fn share(&self) -> Option<f64> {
        if self.done {
            return Some(1.0);
        }
        self.used
            .filter(|u| *u > 0)
            .map(|u| (self.scanned as f64 / u as f64).min(0.99))
    }
}

struct Job {
    id: Id,
    path: PathBuf,
    mount: u64,
    kind: Option<Kind>,
    flags: u8,
}

#[derive(Default)]
struct Queue {
    jobs: Vec<Job>,
    busy: usize,
}

struct Shared {
    tree: Mutex<Tree>,
    queue: Mutex<Queue>,
    ready: Condvar,
    stop: AtomicBool,
    scanned: AtomicU64,
    used: Option<u64>,
    done: AtomicBool,
    /// Workers not yet returned.
    live: AtomicUsize,
    /// The disk-backed mounts by id.
    mounts: HashMap<u64, Mount>,
    /// Mount points the walk stays out of, so a stalled network share is never asked.
    fences: HashSet<PathBuf>,
    fenced_in: HashSet<PathBuf>,
    /// Inodes with several names already counted, by device and inode.
    links: Mutex<HashSet<(u64, u64)>>,
    now: i64,
    /// Bytes on disk where the helper read them (`ondisk`).
    sizes: Option<Arc<crate::ondisk::OnDisk>>,
}

impl Shared {
    fn crosses(&self, from: u64, into: u64) -> bool {
        match (self.mounts.get(&from), self.mounts.get(&into)) {
            (Some(a), Some(b)) => a.source == b.source && a.fstype == b.fstype,
            _ => false,
        }
    }
}

/// A scan in flight. Dropping it cancels the walk.
pub struct Scan {
    shared: Arc<Shared>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

fn makedev(major: u32, minor: u32) -> u64 {
    rustix::fs::makedev(major, minor)
}

const MASK: StatxFlags = StatxFlags::TYPE
    .union(StatxFlags::MODE)
    .union(StatxFlags::NLINK)
    .union(StatxFlags::INO)
    .union(StatxFlags::SIZE)
    .union(StatxFlags::BLOCKS)
    .union(StatxFlags::ATIME)
    .union(StatxFlags::MTIME)
    .union(StatxFlags::MNT_ID);

/// libc's `statx`, for the subvolume (`STATX_SUBVOL`), which rustix's leaves out.
fn statx(dir: impl rustix::fd::AsFd, name: &std::ffi::CStr) -> std::io::Result<libc::statx> {
    use rustix::fd::AsRawFd;
    let mut st: libc::statx = unsafe { std::mem::zeroed() };
    // SAFETY: `name` is a C string and `st` a statx buffer the call fills.
    let r = unsafe {
        libc::statx(
            dir.as_fd().as_raw_fd(),
            name.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW | libc::AT_NO_AUTOMOUNT,
            MASK.bits() | libc::STATX_SUBVOL,
            &mut st,
        )
    };
    if r == 0 { Ok(st) } else { Err(std::io::Error::last_os_error()) }
}

impl Scan {
    /// Starts walking `roots` with one worker per core, up to eight.
    pub fn start(roots: Vec<PathBuf>) -> Self {
        let workers = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(8);
        Self::start_with(roots, isle_fs::mounts::read(), workers, None)
    }

    /// Starts walking `roots`, counting files at their bytes on disk where `sizes` has them.
    pub fn start_sized(roots: Vec<PathBuf>, sizes: Arc<crate::ondisk::OnDisk>) -> Self {
        let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
        Self::start_with(roots, isle_fs::mounts::read(), workers, Some(sizes))
    }

    pub fn start_with(roots: Vec<PathBuf>, mounts: Vec<Mount>, workers: usize, sizes: Option<Arc<crate::ondisk::OnDisk>>) -> Self {
        let mut tree = Tree::new();
        let mut jobs = Vec::new();
        let mut scanned = 0;
        for root in &roots {
            let Ok(c) = CString::new(root.as_os_str().as_bytes()) else {
                continue;
            };
            let Ok(st) = statx(rustix::fs::CWD, &c) else {
                continue;
            };
            if FileType::from_raw_mode(st.stx_mode.into()) != FileType::Directory {
                continue;
            }
            let at_root = root.parent() == Some(Path::new("/"));
            let name = root.file_name().map_or(&b""[..], |n| n.as_bytes());
            let kind = kinds::of_folder(name, at_root, None);
            let id = tree.push(TOP, root.as_os_str().as_bytes(), kind, 0);
            let bytes = st.stx_blocks * 512;
            let node = tree.node_mut(id);
            node.add_own(kinds::of(kind, b""), bytes);
            node.dev = makedev(st.stx_dev_major, st.stx_dev_minor);
            node.ino = st.stx_ino;
            scanned += bytes;
            jobs.push(Job {
                id,
                path: root.clone(),
                mount: st.stx_mnt_id,
                kind,
                flags: 0,
            });
        }

        let of_root = |root: &Path| isle_fs::mounts::of(&mounts, root).cloned();
        let root_mounts: Vec<Option<Mount>> = roots.iter().map(|r| of_root(r)).collect();
        // A whole disk's progress is a share of its used space; a folder's has nothing to go by.
        let whole = roots
            .iter()
            .zip(&root_mounts)
            .all(|(r, m)| m.as_ref().is_some_and(|m| &m.point == r));
        let used = whole.then(|| {
            let mut seen = HashSet::new();
            root_mounts
                .iter()
                .flatten()
                .filter(|m| seen.insert((m.source.clone(), m.fstype.clone())))
                .filter_map(|m| rustix::fs::statvfs(&m.point).ok())
                .map(|v| (v.f_blocks - v.f_bfree) * v.f_frsize)
                .sum()
        });

        // A walk crosses into a mount of the filesystem it is on, as a btrfs subvolume appears,
        // and into nothing else.
        let same_fs =
            |a: &Mount, b: &Mount| disk_backed(a) && a.source == b.source && a.fstype == b.fstype;
        let fences: HashSet<PathBuf> = mounts
            .iter()
            .filter(|m| {
                roots.iter().zip(&root_mounts).any(|(r, rm)| {
                    m.point.starts_with(r)
                        && &m.point != r
                        && !rm.as_ref().is_some_and(|rm| same_fs(m, rm))
                })
            })
            .map(|m| m.point.clone())
            .collect();
        let fenced_in = fences
            .iter()
            .filter_map(|f| f.parent().map(Path::to_path_buf))
            .collect();
        let by_id = mounts
            .iter()
            .filter(|m| disk_backed(m))
            .map(|m| (u64::from(m.id), m.clone()))
            .collect();

        let shared = Arc::new(Shared {
            tree: Mutex::new(tree),
            queue: Mutex::new(Queue { jobs, busy: 0 }),
            ready: Condvar::new(),
            stop: AtomicBool::new(false),
            scanned: AtomicU64::new(scanned),
            used,
            done: AtomicBool::new(false),
            live: AtomicUsize::new(workers.max(1)),
            mounts: by_id,
            fences,
            fenced_in,
            links: Mutex::new(HashSet::new()),
            sizes,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        });
        let workers = (0..workers.max(1))
            .map(|i| {
                let shared = shared.clone();
                std::thread::Builder::new()
                    .name(format!("isle-scan-{i}"))
                    .spawn(move || work(&shared))
                    .expect("scan worker")
            })
            .collect();
        Self { shared, workers }
    }

    pub fn progress(&self) -> Progress {
        let done = self.shared.done.load(Ordering::Acquire);
        let stopped = self.shared.stop.load(Ordering::Acquire);
        Progress {
            scanned: self.shared.scanned.load(Ordering::Relaxed),
            used: self.shared.used,
            done: done && !stopped,
            partial: stopped,
        }
    }

    /// Reads the tree with its totals brought up to date.
    pub fn read<R>(&self, f: impl FnOnce(&Tree) -> R) -> R {
        let mut tree = self.shared.tree.lock().unwrap();
        tree.total();
        f(&tree)
    }

    /// Ends the walk, keeping what it found.
    /// Whether any worker is still reading; after Stop, the tree stops changing once none is.
    pub fn running(&self) -> bool {
        self.shared.live.load(Ordering::Acquire) > 0
    }

    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.ready.notify_all();
    }

    /// Waits for the walk to end and gives its tree.
    pub fn finish(mut self) -> (Tree, Progress) {
        for w in std::mem::take(&mut self.workers) {
            let _ = w.join();
        }
        let progress = self.progress();
        let mut tree = std::mem::take(&mut *self.shared.tree.lock().unwrap());
        tree.total();
        (tree, progress)
    }
}

impl Drop for Scan {
    fn drop(&mut self) {
        if !self.workers.is_empty() {
            self.stop();
        }
    }
}

fn work(shared: &Shared) {
    read_until_done(shared);
    shared.live.fetch_sub(1, Ordering::AcqRel);
}

fn read_until_done(shared: &Shared) {
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap();
            loop {
                if shared.stop.load(Ordering::Acquire) {
                    return;
                }
                if let Some(job) = q.jobs.pop() {
                    q.busy += 1;
                    break job;
                }
                if q.busy == 0 {
                    shared.done.store(true, Ordering::Release);
                    shared.ready.notify_all();
                    return;
                }
                q = shared.ready.wait(q).unwrap();
            }
        };
        let children = read_folder(shared, &job);
        let mut q = shared.queue.lock().unwrap();
        q.busy -= 1;
        let woken = children.len();
        q.jobs.extend(children);
        drop(q);
        if woken > 0 {
            shared.ready.notify_all();
        } else {
            shared.ready.notify_one();
        }
    }
}

struct Sub {
    name: CString,
    bytes: u64,
    dev: u64,
    ino: u64,
    mount: u64,
    kind: Option<Kind>,
    flags: u8,
}

/// Reads one folder: its files' bytes into its node, its folders as new nodes and jobs.
fn read_folder(shared: &Shared, job: &Job) -> Vec<Job> {
    let opened = rustix::fs::open(
        &job.path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    );
    let Ok(fd) = opened else {
        let mut tree = shared.tree.lock().unwrap();
        tree.node_mut(job.id).flags |= UNREADABLE | READ;
        return Vec::new();
    };
    let Ok(dir) = Dir::read_from(&fd) else {
        let mut tree = shared.tree.lock().unwrap();
        tree.node_mut(job.id).flags |= UNREADABLE | READ;
        return Vec::new();
    };
    let names: Vec<(CString, FileType)> = dir
        .filter_map(Result::ok)
        .filter(|e| !matches!(e.file_name().to_bytes(), b"." | b".."))
        .map(|e| (e.file_name().to_owned(), e.file_type()))
        .collect();
    let here_git = names.iter().any(|(n, _)| n.to_bytes() == b".git");
    let flags = job.flags | if here_git { GIT } else { 0 };
    let at_root = job.path == Path::new("/");
    let fenced_in = shared.fenced_in.contains(&job.path);

    let (largest_floor, untouched_floor) = {
        let tree = shared.tree.lock().unwrap();
        (tree.largest.floor(), tree.untouched.floor())
    };
    let mut own: Vec<(Kind, u64)> = Vec::new();
    let mut own_files = 0u32;
    let mut subs = Vec::new();
    let mut largest = Vec::new();
    let mut untouched = Vec::new();
    let mut candidates = Vec::new();
    for (name, d_type) in names {
        let bytes_name = name.to_bytes();
        if d_type == FileType::Directory
            && (bytes_name == b".snapshots"
                || (fenced_in
                    && shared
                        .fences
                        .contains(&job.path.join(std::ffi::OsStr::from_bytes(bytes_name)))))
        {
            continue;
        }
        let Ok(st) = statx(&fd, &name) else { continue };
        let ftype = FileType::from_raw_mode(st.stx_mode.into());
        let dev = makedev(st.stx_dev_major, st.stx_dev_minor);
        let measured = shared.sizes.as_ref().filter(|_| st.stx_mask & libc::STATX_SUBVOL != 0).and_then(|sizes| {
            let source = &shared.mounts.get(&st.stx_mnt_id)?.source;
            sizes.bytes(source, st.stx_subvol, st.stx_ino)
        });
        let bytes = measured.unwrap_or(st.stx_blocks * 512);
        if ftype == FileType::Directory {
            if bytes_name == b".snapshots" {
                continue;
            }
            if st.stx_mnt_id != job.mount && !shared.crosses(job.mount, st.stx_mnt_id) {
                continue;
            }
            let hidden = if bytes_name.first() == Some(&b'.') {
                HIDDEN
            } else {
                0
            };
            subs.push(Sub {
                kind: kinds::of_folder(bytes_name, at_root, job.kind),
                name,
                bytes,
                dev,
                ino: st.stx_ino,
                mount: st.stx_mnt_id,
                flags: flags | hidden,
            });
            continue;
        }
        if st.stx_nlink > 1 && !shared.links.lock().unwrap().insert((dev, st.stx_ino)) {
            continue;
        }
        let kind = kinds::of(job.kind, bytes_name);
        match own.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, b)) => *b += bytes,
            None => own.push((kind, bytes)),
        }
        own_files += 1;
        if ftype != FileType::RegularFile {
            continue;
        }
        let modified = st.stx_mtime.tv_sec;
        let used = modified.max(st.stx_atime.tv_sec);
        let file = || File {
            folder: job.id,
            name: 0,
            bytes,
            size: st.stx_size,
            kind,
            used,
            modified,
            dev,
            ino: st.stx_ino,
        };
        let named = |f: File| (f, bytes_name.to_vec());
        if bytes > largest_floor {
            largest.push(named(file()));
        }
        if bytes > untouched_floor && used < shared.now - UNTOUCHED_SECS && kind != Kind::System {
            untouched.push(named(file()));
        }
        let executable = u32::from(st.stx_mode) & 0o111 != 0;
        let apps_own = matches!(
            kind,
            Kind::Games | Kind::System | Kind::Developer | Kind::Caches
        );
        if st.stx_size >= CANDIDATE_BYTES && !executable && flags & (HIDDEN | GIT) == 0 && !apps_own
        {
            candidates.push(named(file()));
        }
    }

    let mut folder_bytes: u64 = own.iter().map(|(_, b)| b).sum();
    let mut jobs = Vec::with_capacity(subs.len());
    let mut tree = shared.tree.lock().unwrap();
    {
        let node = tree.node_mut(job.id);
        node.flags |= READ | flags;
        node.own_files += own_files;
        for (k, b) in own {
            node.add_own(k, b);
        }
    }
    for sub in subs {
        let id = tree.push(job.id, sub.name.to_bytes(), sub.kind, sub.flags);
        let node = tree.node_mut(id);
        node.add_own(kinds::of(sub.kind, b""), sub.bytes);
        node.dev = sub.dev;
        node.ino = sub.ino;
        folder_bytes += sub.bytes;
        jobs.push(Job {
            id,
            path: job
                .path
                .join(std::ffi::OsStr::from_bytes(sub.name.to_bytes())),
            mount: sub.mount,
            kind: sub.kind,
            flags: sub.flags,
        });
    }
    for (mut f, name) in largest {
        f.name = tree.names.intern(&name);
        tree.largest.offer(f);
    }
    for (mut f, name) in untouched {
        f.name = tree.names.intern(&name);
        tree.untouched.offer(f);
    }
    for (mut f, name) in candidates {
        f.name = tree.names.intern(&name);
        tree.candidates.push(f);
    }
    tree.scanned += folder_bytes;
    drop(tree);
    shared.scanned.fetch_add(folder_bytes, Ordering::Relaxed);
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mount(id: u32, point: &str, source: &str, fstype: &str) -> Mount {
        Mount {
            id,
            parent: 1,
            root: "/".into(),
            point: point.into(),
            fstype: fstype.into(),
            source: source.into(),
            options: "rw".into(),
            class: isle_fs::mounts::class_of(fstype),
        }
    }

    #[test]
    fn all_disks_drops_a_root_another_reaches() {
        let mounts = vec![
            mount(1, "/", "/dev/nvme0n1p2", "btrfs"),
            mount(2, "/proc", "proc", "proc"),
            mount(3, "/home", "/dev/nvme0n1p2", "btrfs"),
            mount(4, "/boot", "/dev/nvme0n1p1", "vfat"),
            mount(5, "/media/d", "/dev/nvme1n1p1", "btrfs"),
            mount(6, "/run/user/1000", "tmpfs", "tmpfs"),
            mount(7, "/mnt/share", "//nas/share", "cifs"),
        ];
        let points: Vec<PathBuf> = all_disks(&mounts).into_iter().map(|d| d.point).collect();
        assert_eq!(
            points,
            [PathBuf::from("/"), "/boot".into(), "/media/d".into()]
        );
    }

    #[test]
    fn the_share_waits_at_ninety_nine_until_the_end() {
        let p = Progress {
            scanned: 120,
            used: Some(100),
            done: false,
            partial: false,
        };
        assert_eq!(p.share(), Some(0.99));
        assert_eq!(Progress { done: true, ..p }.share(), Some(1.0));
        assert_eq!(Progress { used: None, ..p }.share(), None);
    }
}
