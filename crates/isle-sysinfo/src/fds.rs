//! One pass over the user's processes' descriptors: their DRM nodes, for GPU use, and their
//! sockets, for network use. Another user's need ptrace access to read. `Scanner` rescans
//! every process only a few at a time once their scan is old, and a new one at once.

use std::{collections::HashMap, fs, path::Path};

#[derive(Clone, Debug, Default)]
pub struct Fds {
    /// Each DRM descriptor's process and number.
    pub drm: Vec<(i32, String)>,
    /// Socket inodes to the process holding them.
    pub sockets: HashMap<u32, i32>,
}

pub fn scan(root: &Path, own: &[i32]) -> Fds {
    let mut out = Fds::default();
    for pid in own {
        let Ok(dir) = fs::read_dir(root.join(format!("proc/{pid}/fd"))) else { continue };
        for entry in dir.flatten() {
            let Ok(target) = fs::read_link(entry.path()) else { continue };
            let target = target.to_string_lossy();
            if target.starts_with("/dev/dri/") {
                out.drm.push((*pid, entry.file_name().to_string_lossy().into_owned()));
            } else if let Some(inode) = target.strip_prefix("socket:[").and_then(|t| t.strip_suffix(']')).and_then(|t| t.parse().ok()) {
                out.sockets.entry(inode).or_insert(*pid);
            }
        }
    }
    out
}

/// How long a process's descriptors are trusted before they are read again.
const RESCAN: std::time::Duration = std::time::Duration::from_secs(5);

/// Keeps each process's descriptors between scans.
#[derive(Default)]
pub struct Scanner {
    by_pid: HashMap<i32, (std::time::Instant, Fds)>,
}

impl Scanner {
    /// The descriptors of `own`. A process not seen before is read at once; of those read
    /// longer than `RESCAN` ago, the oldest tenth are read again, so no sample pays for all.
    pub fn scan(&mut self, root: &Path, own: &[i32]) -> Fds {
        let now = std::time::Instant::now();
        self.by_pid.retain(|pid, _| own.contains(pid));
        for pid in own {
            self.by_pid.entry(*pid).or_insert_with(|| (now, scan(root, &[*pid])));
        }
        let mut stale: Vec<(std::time::Instant, i32)> = self
            .by_pid
            .iter()
            .filter(|(_, (at, _))| now.duration_since(*at) >= RESCAN)
            .map(|(pid, (at, _))| (*at, *pid))
            .collect();
        stale.sort();
        for (_, pid) in stale.into_iter().take(own.len() / 10 + 1) {
            self.by_pid.insert(pid, (now, scan(root, &[pid])));
        }
        let mut out = Fds::default();
        for (_, fds) in self.by_pid.values() {
            out.drm.extend(fds.drm.iter().cloned());
            for (inode, pid) in &fds.sockets {
                out.sockets.entry(*inode).or_insert(*pid);
            }
        }
        out
    }
}
