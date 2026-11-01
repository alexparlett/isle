//! Every btrfs filesystem's file extents read through `BTRFS_IOC_TREE_SEARCH_V2`, as compsize
//! does, into each inode's bytes on disk (`isle_scan::ondisk`).

use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    os::fd::AsRawFd,
};

const ROOT_TREE: u64 = 1;
const FS_TREE: u64 = 5;
const FIRST_FREE: u64 = 256;
const LAST_FREE: u64 = u64::MAX - 255;
const ROOT_ITEM: u32 = 132;
const EXTENT_DATA: u32 = 108;
const KEY: usize = 104;
const HEADER: usize = 32;
const BUF: usize = 1 << 20;
/// `_IOWR(0x94, 17, struct btrfs_ioctl_search_args_v2)`, its fixed part 112 bytes.
const TREE_SEARCH_V2: libc::c_ulong = 0xC070_9411;

struct Item<'a> {
    objectid: u64,
    data: &'a [u8],
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// Calls `each` for every item of `kind` in `tree` with an object id in `objects`.
fn search(
    fd: i32,
    tree: u64,
    objects: (u64, u64),
    kind: u32,
    mut each: impl FnMut(Item),
) -> io::Result<()> {
    let mut args = vec![0u8; KEY + 8 + BUF];
    let (mut objectid, mut kind_at, mut offset) = (objects.0, kind, 0u64);
    loop {
        let key = &mut args[..KEY];
        key.fill(0);
        let put =
            |key: &mut [u8], at: usize, v: u64| key[at..at + 8].copy_from_slice(&v.to_le_bytes());
        put(key, 0, tree);
        put(key, 8, objectid);
        put(key, 16, objects.1);
        put(key, 24, offset);
        put(key, 32, u64::MAX);
        put(key, 40, 0);
        put(key, 48, u64::MAX);
        key[56..60].copy_from_slice(&kind_at.to_le_bytes());
        key[60..64].copy_from_slice(&kind.to_le_bytes());
        key[64..68].copy_from_slice(&4096u32.to_le_bytes());
        args[KEY..KEY + 8].copy_from_slice(&(BUF as u64).to_le_bytes());
        // SAFETY: `args` is a search key, the buffer's size, then that many bytes, as the ioctl wants.
        if unsafe { libc::ioctl(fd, TREE_SEARCH_V2, args.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let found = u32_at(&args, 64) as usize;
        if found == 0 {
            return Ok(());
        }
        let buf = &args[KEY + 8..];
        let mut at = 0;
        let mut last = (0, 0, 0);
        for _ in 0..found {
            let header = &buf[at..at + HEADER];
            let (oid, off, ty, len) = (
                u64_at(header, 8),
                u64_at(header, 16),
                u32_at(header, 24),
                u32_at(header, 28) as usize,
            );
            // The key range is ordered by (object, type, offset), so other types come through.
            if ty == kind {
                each(Item {
                    objectid: oid,
                    data: &buf[at + HEADER..at + HEADER + len],
                });
            }
            last = (oid, ty, off);
            at += HEADER + len;
        }
        (objectid, kind_at, offset) = match last {
            (o, t, u64::MAX) if t >= 255 => (o.saturating_add(1), 0, 0),
            (o, t, u64::MAX) => (o, t + 1, 0),
            (o, t, off) => (o, t, off + 1),
        };
        if objectid > objects.1 {
            return Ok(());
        }
    }
}

/// The disk-backed btrfs filesystems: each source once, with a mount point and the subvolume
/// ids mounted from it, in mount order.
fn filesystems() -> Vec<(String, String, Vec<u64>)> {
    let table = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    let mut out: Vec<(String, String, Vec<u64>)> = Vec::new();
    for line in table.lines() {
        let Some((before, after)) = line.split_once(" - ") else {
            continue;
        };
        let mut a = after.split(' ');
        let (Some("btrfs"), Some(source), Some(options)) = (a.next(), a.next(), a.next()) else {
            continue;
        };
        if !source.starts_with("/dev/") {
            continue;
        }
        let Some(point) = before.split(' ').nth(4) else {
            continue;
        };
        let subvolid = options
            .split(',')
            .find_map(|o| o.strip_prefix("subvolid="))
            .and_then(|v| v.parse().ok());
        match out.iter_mut().find(|(s, _, _)| s == source) {
            Some((_, _, ids)) => ids.extend(subvolid),
            None => out.push((
                source.to_owned(),
                point.replace("\\040", " "),
                subvolid.into_iter().collect(),
            )),
        }
    }
    out
}

/// One filesystem's subvolumes, mounted ones first, each inode with its bytes on disk; an
/// extent counts once, for the first file met.
fn measure(point: &str, mounted: &[u64]) -> io::Result<Vec<isle_scan::ondisk::Subvolume>> {
    let dir = std::fs::File::open(point)?;
    let fd = dir.as_raw_fd();
    let mut trees = vec![FS_TREE];
    search(fd, ROOT_TREE, (FIRST_FREE, LAST_FREE), ROOT_ITEM, |item| {
        trees.push(item.objectid)
    })?;
    trees.dedup();
    trees.sort_by_key(|t| {
        (
            mounted.iter().position(|m| m == t).unwrap_or(usize::MAX),
            *t,
        )
    });
    let mut seen: HashSet<u64> = HashSet::new();
    let mut out = Vec::new();
    for tree in trees {
        let mut inodes: HashMap<u64, u64> = HashMap::new();
        let read = search(fd, tree, (0, u64::MAX), EXTENT_DATA, |item| {
            let d = item.data;
            if d.len() < 21 {
                return;
            }
            let bytes = match d[20] {
                // Inline: stored in the metadata, as long as its data.
                0 => (d.len() - 21) as u64,
                _ if d.len() >= 53 => {
                    let (bytenr, disk) = (u64_at(d, 21), u64_at(d, 29));
                    if bytenr == 0 || !seen.insert(bytenr) {
                        0
                    } else {
                        disk
                    }
                }
                _ => 0,
            };
            *inodes.entry(item.objectid).or_default() += bytes;
        });
        // A tree being deleted, or one the search cannot open, has nothing to give.
        if read.is_err() {
            continue;
        }
        let mut list: Vec<(u64, u64)> = inodes.into_iter().collect();
        list.sort_unstable();
        out.push((tree, list));
    }
    Ok(out)
}

pub fn run() -> Result<(), String> {
    let stdout = io::stdout();
    let mut w = io::BufWriter::with_capacity(1 << 20, stdout.lock());
    for (source, point, mounted) in filesystems() {
        match measure(&point, &mounted) {
            Ok(subvolumes) => isle_scan::ondisk::write_filesystem(&mut w, &source, &subvolumes)
                .map_err(|e| e.to_string())?,
            Err(e) => eprintln!("isle-clean-helper: {source}: {e}"),
        }
    }
    w.flush().map_err(|e| e.to_string())
}
