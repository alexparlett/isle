//! Locations' volumes and shares, followed live: UDisks2's volumes through `isle-disks`, the
//! shares the system has mounted from the mount table, and each mounted one's free space
//! (`p1/structure/files.md` § Places, Volumes: eject and mount).

use std::{collections::HashMap, path::PathBuf};

use freya::prelude::*;
use isle_app::AppHandle;
use isle_disks::Volume;
use isle_fs::mounts::{self, Class};

/// What Locations lists beside the computer and the Trash.
#[derive(Clone, Copy, PartialEq)]
pub struct Locations {
    pub volumes: State<Vec<Volume>>,
    pub shares: State<Vec<PathBuf>>,
    /// Free and total bytes by mount point.
    pub space: State<HashMap<PathBuf, (u64, u64)>>,
}

impl Locations {
    pub fn new() -> Self {
        Self {
            volumes: State::create_global(Vec::new()),
            shares: State::create_global(Vec::new()),
            space: State::create_global(HashMap::new()),
        }
    }

    /// Follows UDisks2 and the mount table for the app's life.
    pub fn follow(self, app: &AppHandle) {
        let (publisher, mut rx) = isle_bus::snapshot::<isle_disks::Disks>();
        app.worker().spawn(async move {
            if let Err(err) = isle_disks::watch(publisher).await {
                tracing::warn!("no volumes: {err}");
            }
        });
        let mut locations = self;
        app.spawn(async move {
            while rx.changed().await.is_ok() {
                let volumes = rx.borrow().state.volumes.clone();
                locations.volumes.set(volumes);
                locations.measure().await;
            }
        });
        let (tx, mut shares_rx) = futures_channel::mpsc::unbounded::<Vec<PathBuf>>();
        mounts::follow(move |table| {
            let shares = table.iter().filter(|m| is_share(m)).map(|m| m.point.clone()).collect();
            tx.unbounded_send(shares).is_ok()
        });
        app.spawn(async move {
            use futures_util::StreamExt;
            while let Some(shares) = shares_rx.next().await {
                if *locations.shares.peek() != shares {
                    locations.shares.set(shares);
                }
            }
        });
    }

    /// Reads each mounted volume's free space, off the window's thread and with a deadline.
    pub async fn measure(self) {
        let points: Vec<PathBuf> = self.volumes.peek().iter().filter_map(|v| v.mount_point.as_ref().map(PathBuf::from)).collect();
        let mut space = self.space;
        {
            let found = crate::window::off_thread(move || {
                let prober = mounts::Prober::default();
                points
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, point)| {
                        let probe = point.clone();
                        let stat = prober
                            .probe(i as u32, std::time::Duration::from_secs(2), move || rustix::fs::statvfs(&probe).ok())
                            .ok()
                            .flatten()?;
                        Some((point, (stat.f_bavail * stat.f_frsize, stat.f_blocks * stat.f_frsize)))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .await;
            if let Some(found) = found {
                space.set(found);
            }
        }
    }
}

/// A network share the system mounted: not a FUSE helper of the session's own, as the
/// document portal and gvfs mount under the runtime folder.
fn is_share(mount: &mounts::Mount) -> bool {
    const NETWORK: [&str; 10] = ["cifs", "smb3", "smbfs", "nfs", "nfs4", "davfs", "9p", "ceph", "fuse.sshfs", "fuse.rclone"];
    mount.class == Class::Remote && NETWORK.contains(&mount.fstype.as_str()) && !mount.point.starts_with("/run/user")
}

/// The share's name: its folder's.
pub fn share_name(point: &std::path::Path) -> String {
    point.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| point.display().to_string())
}

/// Past nine-tenths full, when a volume has something to say.
pub fn nearly_full(space: Option<&(u64, u64)>) -> Option<f32> {
    let (free, total) = *space?;
    if total == 0 {
        return None;
    }
    let used = 1. - free as f32 / total as f32;
    (used > 0.9).then_some(used)
}
