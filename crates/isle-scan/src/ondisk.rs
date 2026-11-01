//! Each file's bytes on a compressed btrfs disk, as `isle-clean-helper extents` reads them as
//! root (B139): compressed, and an extent several files share counted once. `st_blocks` counts
//! the bytes before compression, so the walk takes these where it has them.
//!
//! The helper writes, little-endian: `F`, a `u16` length and a filesystem's source device;
//! then per subvolume `S`, its id, a count, and that many inode and byte pairs, inodes rising.

use std::io::{self, Read, Write};

/// One subvolume's inodes and their bytes, inodes rising.
pub type Subvolume = (u64, Vec<(u64, u64)>);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OnDisk {
    filesystems: Vec<(String, Vec<Subvolume>)>,
}

fn u64_of(r: &mut impl Read) -> io::Result<u64> {
    let mut b = [0; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

impl OnDisk {
    pub fn read(mut r: impl Read) -> io::Result<Self> {
        let mut out = OnDisk::default();
        let mut tag = [0u8; 1];
        loop {
            match r.read(&mut tag)? {
                0 => return Ok(out),
                _ => match tag[0] {
                    b'F' => {
                        let mut len = [0; 2];
                        r.read_exact(&mut len)?;
                        let mut source = vec![0; u16::from_le_bytes(len) as usize];
                        r.read_exact(&mut source)?;
                        out.filesystems
                            .push((String::from_utf8_lossy(&source).into_owned(), Vec::new()));
                    }
                    b'S' => {
                        let id = u64_of(&mut r)?;
                        let count = u64_of(&mut r)? as usize;
                        let mut inodes = Vec::with_capacity(count.min(1 << 24));
                        for _ in 0..count {
                            inodes.push((u64_of(&mut r)?, u64_of(&mut r)?));
                        }
                        let fs = out
                            .filesystems
                            .last_mut()
                            .ok_or_else(|| io::Error::other("a subvolume before its filesystem"))?;
                        fs.1.push((id, inodes));
                    }
                    other => return Err(io::Error::other(format!("unknown record {other}"))),
                },
            }
        }
    }

    /// The file's bytes on disk, where the helper read its filesystem.
    pub fn bytes(&self, source: &str, subvolume: u64, inode: u64) -> Option<u64> {
        let (_, subvolumes) = self.filesystems.iter().find(|(s, _)| s == source)?;
        let (_, inodes) = subvolumes.iter().find(|(id, _)| *id == subvolume)?;
        inodes
            .binary_search_by_key(&inode, |(i, _)| *i)
            .ok()
            .map(|at| inodes[at].1)
    }

    pub fn is_empty(&self) -> bool {
        self.filesystems.is_empty()
    }
}

/// Writes one filesystem's records.
pub fn write_filesystem(
    w: &mut impl Write,
    source: &str,
    subvolumes: &[Subvolume],
) -> io::Result<()> {
    let source = &source.as_bytes()[..source.len().min(u16::MAX as usize)];
    w.write_all(b"F")?;
    w.write_all(&(source.len() as u16).to_le_bytes())?;
    w.write_all(source)?;
    for (id, inodes) in subvolumes {
        w.write_all(b"S")?;
        w.write_all(&id.to_le_bytes())?;
        w.write_all(&(inodes.len() as u64).to_le_bytes())?;
        for (inode, bytes) in inodes {
            w.write_all(&inode.to_le_bytes())?;
            w.write_all(&bytes.to_le_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_read_back() {
        let mut buf = Vec::new();
        write_filesystem(
            &mut buf,
            "/dev/nvme0n1p2",
            &[(256, vec![(257, 4096), (300, 12)]), (257, vec![(260, 99)])],
        )
        .unwrap();
        write_filesystem(&mut buf, "/dev/nvme1n1p1", &[(5, vec![(257, 1)])]).unwrap();
        let on = OnDisk::read(&buf[..]).unwrap();
        assert_eq!(on.bytes("/dev/nvme0n1p2", 256, 300), Some(12));
        assert_eq!(on.bytes("/dev/nvme0n1p2", 257, 260), Some(99));
        assert_eq!(on.bytes("/dev/nvme1n1p1", 5, 257), Some(1));
        assert_eq!(on.bytes("/dev/nvme0n1p2", 256, 299), None);
        assert_eq!(on.bytes("/dev/sda1", 5, 257), None);
        assert!(OnDisk::read(&b"Q"[..]).is_err());
    }
}
