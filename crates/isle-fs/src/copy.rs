//! Copying one file or folder as the filesystem allows: a reflink first, then the kernel's
//! copy in steps for progress, then reading and writing; mode, times and `user.*`
//! attributes kept (`p15-files.md` choice 16).

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

use rustix::fs::{Timespec, Timestamps};

/// The kernel's copy moves this much between progress reports.
const STEP: usize = 8 * 1024 * 1024;

/// Copies `from` to the new file `to`, reporting each step's bytes; `stop` asks between
/// steps. `to` must not exist.
pub fn file(from: &Path, to: &Path, step: &mut dyn FnMut(u64) -> bool) -> io::Result<()> {
    let mut src = File::open(from)?;
    let meta = src.metadata()?;
    let mut dst = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(meta.permissions().mode() & 0o7777)
        .open(to)?;
    let len = meta.len();
    let copied = if rustix::fs::ioctl_ficlone(&dst, &src).is_ok() {
        if !step(len) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        true
    } else {
        kernel_copy(&src, &dst, len, step)?
    };
    if !copied {
        stream(&mut src, &mut dst, step)?;
    }
    dst.sync_all().ok();
    keep_attributes(from, to, &meta)?;
    Ok(())
}

/// `copy_file_range` in steps; false when the filesystems refuse it before any byte moved.
fn kernel_copy(src: &File, dst: &File, len: u64, step: &mut dyn FnMut(u64) -> bool) -> io::Result<bool> {
    let mut done = 0u64;
    while done < len {
        match rustix::fs::copy_file_range(src, None, dst, None, STEP) {
            Ok(0) => break,
            Ok(n) => {
                done += n as u64;
                if !step(n as u64) {
                    return Err(io::Error::from(io::ErrorKind::Interrupted));
                }
            }
            Err(e) if done == 0 && matches!(e, rustix::io::Errno::XDEV | rustix::io::Errno::NOSYS | rustix::io::Errno::INVAL | rustix::io::Errno::OPNOTSUPP) => {
                return Ok(false);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

fn stream(src: &mut File, dst: &mut File, step: &mut dyn FnMut(u64) -> bool) -> io::Result<()> {
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        dst.write_all(&buf[..n])?;
        if !step(n as u64) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
    }
}

/// The modification and access times and `user.*` extended attributes of `from` on `to`.
pub fn keep_attributes(from: &Path, to: &Path, meta: &fs::Metadata) -> io::Result<()> {
    let times = Timestamps {
        last_access: Timespec { tv_sec: meta.atime(), tv_nsec: meta.atime_nsec() as _ },
        last_modification: Timespec { tv_sec: meta.mtime(), tv_nsec: meta.mtime_nsec() as _ },
    };
    rustix::fs::utimensat(rustix::fs::CWD, to, &times, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)?;
    let mut names = vec![0u8; 4096];
    if let Ok(n) = rustix::fs::llistxattr(from, &mut names[..]) {
        for name in names[..n].split(|b| *b == 0).filter(|n| n.starts_with(b"user.")) {
            let name = <std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(name);
            let mut value = vec![0u8; 64 * 1024];
            if let Ok(len) = rustix::fs::lgetxattr(from, name, &mut value[..]) {
                let _ = rustix::fs::lsetxattr(to, name, &value[..len], rustix::fs::XattrFlags::empty());
            }
        }
    }
    Ok(())
}

/// The bytes and items under `path`, links not followed.
pub fn measure(path: &Path) -> (u64, u64) {
    let Ok(meta) = fs::symlink_metadata(path) else { return (0, 0) };
    if !meta.is_dir() {
        return (meta.len(), 1);
    }
    let (mut bytes, mut items) = (0, 1);
    if let Ok(read) = fs::read_dir(path) {
        for entry in read.flatten() {
            let (b, i) = measure(&entry.path());
            bytes += b;
            items += i;
        }
    }
    (bytes, items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_keeps_its_bytes_mode_time_and_attributes() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("a");
        fs::write(&from, vec![7u8; 3_000_000]).unwrap();
        fs::set_permissions(&from, fs::Permissions::from_mode(0o750)).unwrap();
        let _ = rustix::fs::setxattr(&from, "user.xdg.comment", b"kept", rustix::fs::XattrFlags::empty());
        let old = Timespec { tv_sec: 1_000_000, tv_nsec: 0 };
        rustix::fs::utimensat(rustix::fs::CWD, &from, &Timestamps { last_access: old, last_modification: old }, rustix::fs::AtFlags::empty()).unwrap();
        let to = dir.path().join("b");
        let mut seen = 0;
        file(&from, &to, &mut |n| {
            seen += n;
            true
        })
        .unwrap();
        assert_eq!(seen, 3_000_000);
        assert_eq!(fs::read(&to).unwrap(), fs::read(&from).unwrap());
        let meta = fs::metadata(&to).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o750);
        assert_eq!(meta.mtime(), 1_000_000);
        let mut buf = [0u8; 16];
        if rustix::fs::getxattr(&from, "user.xdg.comment", &mut buf[..]).is_ok() {
            let n = rustix::fs::getxattr(&to, "user.xdg.comment", &mut buf[..]).unwrap();
            assert_eq!(&buf[..n], b"kept");
        }
        assert!(file(&from, &to, &mut |_| true).is_err(), "never over an existing file");
    }
}
