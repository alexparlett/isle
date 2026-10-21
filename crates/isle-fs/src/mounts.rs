//! The mount table, each mount's class, and probes that give up on a mount that stops
//! answering (`p15-files.md` choice 13; `files-2026.md` §5): a call into a stalled network
//! mount cannot be cancelled, so it costs one thread of its own and a mark, never the
//! caller.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// A disk on this machine.
    Local,
    /// A share or a FUSE filesystem: no content reads, thumbnails or counts by default, no
    /// watch, read again when shown.
    Remote,
    /// An automount point, touched only when the user opens it.
    Automount,
    /// The kernel's own: proc, sys, cgroups, tmpfs and the like.
    Virtual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    pub id: u32,
    pub parent: u32,
    /// The path within its filesystem that is mounted: `/@home` for a btrfs subvolume.
    pub root: String,
    pub point: PathBuf,
    pub fstype: String,
    pub source: String,
    pub options: String,
    pub class: Class,
}

/// A filesystem type's class.
pub fn class_of(fstype: &str) -> Class {
    match fstype {
        "autofs" => Class::Automount,
        "nfs" | "nfs4" | "cifs" | "smb3" | "smbfs" | "ncpfs" | "afs" | "9p" | "ceph" | "glusterfs"
        | "davfs" => Class::Remote,
        // A FUSE filesystem answers through a process that may be waiting on a network;
        // `fuseblk` is a local disk through FUSE.
        t if t.starts_with("fuse.") || t == "fuse" => Class::Remote,
        "proc" | "sysfs" | "devtmpfs" | "devpts" | "tmpfs" | "cgroup" | "cgroup2" | "securityfs"
        | "pstore" | "efivarfs" | "bpf" | "debugfs" | "tracefs" | "configfs" | "fusectl" | "mqueue"
        | "hugetlbfs" | "binfmt_misc" | "ramfs" | "rpc_pipefs" | "nsfs" | "overlay" | "squashfs" => {
            Class::Virtual
        }
        _ => Class::Local,
    }
}

/// `/proc/self/mountinfo`'s escapes: a space is `\040`.
fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b)) {
            let n = (bytes[i + 1] - b'0') * 64 + (bytes[i + 2] - b'0') * 8 + (bytes[i + 3] - b'0');
            out.push(n);
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reads mountinfo's lines.
pub fn parse(text: &str) -> Vec<Mount> {
    text.lines()
        .filter_map(|line| {
            let (before, after) = line.split_once(" - ")?;
            let mut f = before.split(' ');
            let id = f.next()?.parse().ok()?;
            let parent = f.next()?.parse().ok()?;
            let _dev = f.next()?;
            let root = unescape(f.next()?);
            let point = PathBuf::from(unescape(f.next()?));
            let options = f.next()?.to_owned();
            let mut a = after.split(' ');
            let fstype = a.next()?.to_owned();
            let source = unescape(a.next()?);
            Some(Mount {
                id,
                parent,
                root,
                point,
                class: class_of(&fstype),
                fstype,
                source,
                options,
            })
        })
        .collect()
}

/// The process's mount table now.
pub fn read() -> Vec<Mount> {
    std::fs::read_to_string("/proc/self/mountinfo")
        .map(|t| parse(&t))
        .unwrap_or_default()
}

/// The mount a path is on: the longest mount point it starts with, the last mounted there.
pub fn of<'a>(mounts: &'a [Mount], path: &Path) -> Option<&'a Mount> {
    mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| path.starts_with(&m.point))
        .max_by_key(|(i, m)| (m.point.components().count(), *i))
        .map(|(_, m)| m)
}

/// Calls `on` with the table each time it changes, from a thread of its own, until `on`
/// says to stop. Reading mountinfo mounts nothing.
pub fn follow(on: impl Fn(Vec<Mount>) -> bool + Send + 'static) {
    std::thread::Builder::new()
        .name("isle-fs-mounts".into())
        .spawn(move || {
            use rustix::event::{PollFd, PollFlags, poll};
            let Ok(file) = std::fs::File::open("/proc/self/mountinfo") else { return };
            if !on(read()) {
                return;
            }
            loop {
                let mut fds = [PollFd::new(&file, PollFlags::PRI | PollFlags::ERR)];
                if poll(&mut fds, None).is_err() {
                    return;
                }
                // The change is acknowledged by reading the file again from its start.
                let _ = std::io::Read::read_to_end(&mut &file, &mut Vec::new());
                let _ = std::io::Seek::rewind(&mut &file);
                if !on(read()) {
                    return;
                }
            }
        })
        .ok();
}

/// Why a probe gave no answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slow {
    /// The mount has not answered the last probe yet.
    Busy,
    /// It did not answer by the deadline; the call carries on alone.
    Late,
}

/// Probes, at most one outstanding per mount.
#[derive(Clone, Default)]
pub struct Prober {
    busy: Arc<Mutex<HashSet<u32>>>,
}

impl Prober {
    /// Runs `call` against `mount` on a thread of its own and waits until `deadline` for it.
    /// Blocks the caller for at most `deadline`: call it from a worker.
    pub fn probe<T: Send + 'static>(
        &self,
        mount: u32,
        deadline: Duration,
        call: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, Slow> {
        if !self.busy.lock().unwrap().insert(mount) {
            return Err(Slow::Busy);
        }
        let (tx, rx) = mpsc::channel();
        let busy = self.busy.clone();
        let spawned = std::thread::Builder::new().name("isle-fs-probe".into()).spawn(move || {
            let result = call();
            busy.lock().unwrap().remove(&mount);
            let _ = tx.send(result);
        });
        if spawned.is_err() {
            self.busy.lock().unwrap().remove(&mount);
            return Err(Slow::Busy);
        }
        rx.recv_timeout(deadline).map_err(|_| Slow::Late)
    }

    /// Whether a probe of `mount` is still out.
    pub fn is_busy(&self, mount: u32) -> bool {
        self.busy.lock().unwrap().contains(&mount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: &str = "\
22 1 0:21 / / rw,relatime shared:1 - btrfs /dev/nvme0n1p2 rw,subvol=/@
23 22 0:21 /@home /home rw,relatime shared:2 - btrfs /dev/nvme0n1p2 rw,subvol=/@home
40 22 0:44 / /proc rw - proc proc rw
51 23 0:60 / /home/me/My\\040Share rw - cifs //nas/share rw
52 23 0:61 / /home/me/drive rw - fuse.rclone gdrive: rw
60 22 0:70 / /net rw - autofs systemd-1 rw
";

    #[test]
    fn mountinfo_reads_with_its_escapes_and_classes() {
        let mounts = parse(INFO);
        assert_eq!(mounts.len(), 6);
        assert_eq!(mounts[1].root, "/@home");
        assert_eq!(mounts[3].point, PathBuf::from("/home/me/My Share"));
        let classes: Vec<Class> = mounts.iter().map(|m| m.class).collect();
        assert_eq!(
            classes,
            [Class::Local, Class::Local, Class::Virtual, Class::Remote, Class::Remote, Class::Automount]
        );
    }

    #[test]
    fn a_path_is_on_the_deepest_mount() {
        let mounts = parse(INFO);
        assert_eq!(of(&mounts, Path::new("/home/me/My Share/x")).unwrap().fstype, "cifs");
        assert_eq!(of(&mounts, Path::new("/home/me/notes")).unwrap().root, "/@home");
        assert_eq!(of(&mounts, Path::new("/etc")).unwrap().id, 22);
        assert_eq!(of(&mounts, Path::new("/homes")).unwrap().id, 22);
    }

    #[test]
    fn a_probe_that_stalls_is_late_and_the_mount_busy_until_it_ends() {
        let prober = Prober::default();
        let (release, held) = mpsc::channel::<()>();
        let late = prober.probe(7, Duration::from_millis(50), move || held.recv().ok());
        assert_eq!(late, Err(Slow::Late));
        assert_eq!(prober.probe(7, Duration::from_millis(50), || 1), Err(Slow::Busy));
        assert_eq!(prober.probe(8, Duration::from_millis(500), || 2), Ok(2), "other mounts answer");
        release.send(()).unwrap();
        for _ in 0..50 {
            if !prober.is_busy(7) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(prober.probe(7, Duration::from_millis(500), || 3), Ok(3));
    }
}
