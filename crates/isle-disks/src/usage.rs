//! Each volume's used and free space from the filesystem, for the dashboard's Storage widget
//! and Cleaner's disks.

/// One filesystem's figures, in bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub name: String,
    /// Where it is mounted, which Files opens.
    pub path: String,
    pub total: u64,
    /// What an unprivileged user can still write.
    pub free: u64,
}

impl Usage {
    /// The used share, 0 to 1.
    pub fn used(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            1.0 - self.free as f32 / self.total as f32
        }
    }

    pub fn nearly_full(&self) -> bool {
        self.used() >= 0.9
    }
}

/// The figures for each of `places`, a path and its name; the same filesystem mounted twice,
/// or as btrfs subvolumes, is one volume, named by its first place.
pub fn usage(places: impl IntoIterator<Item = (String, String)>) -> Vec<Usage> {
    let mut seen: Vec<(u64, u64, u64)> = Vec::new();
    let mut out = Vec::new();
    for (path, name) in places {
        let Ok(stat) = rustix::fs::statvfs(path.as_str()) else {
            continue;
        };
        let id = (stat.f_blocks, stat.f_bfree, stat.f_frsize);
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let total = stat.f_blocks * stat.f_frsize;
        if total == 0 {
            continue;
        }
        out.push(Usage {
            name,
            path,
            total,
            free: stat.f_bavail * stat.f_frsize,
        });
    }
    out
}

/// The system's own filesystems, then `mounted`.
pub fn with_system(mounted: Vec<(String, String)>) -> Vec<Usage> {
    usage(
        [
            ("/".to_owned(), "System".to_owned()),
            ("/home".to_owned(), "Home".to_owned()),
        ]
        .into_iter()
        .chain(mounted),
    )
}
