//! The Trash by the freedesktop spec 1.0, each item to its own volume's trash (`apps.md`
//! Files 4; v1's D70): the home trash on home's filesystem, `$top/.Trash/$uid` when it is a
//! sticky folder and not a link, else `$top/.Trash-$uid`. The info file is written first,
//! with `O_EXCL`; a volume's paths are kept relative to its top so Put Back works after it
//! is mounted somewhere else. Only local mounts are looked at (`files-2026.md` §4).

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use crate::mounts::{self, Class, Mount};

/// One trash: its `files` and `info` folders, and the volume top its paths are relative to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashDir {
    pub root: PathBuf,
    pub top: Option<PathBuf>,
}

/// An item in a trash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trashed {
    /// Where it is now, under the trash's `files`.
    pub file: PathBuf,
    pub info: PathBuf,
    /// Where Put Back returns it.
    pub original: PathBuf,
    /// `YYYY-MM-DDThh:mm:ss`, local time.
    pub deleted: String,
}

fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn home_trash() -> TrashDir {
    TrashDir {
        root: data_home().join("Trash"),
        top: None,
    }
}

fn uid() -> u32 {
    rustix::process::getuid().as_raw()
}

/// The device of `path`, or of its nearest folder that exists: a home trash not made yet
/// is still on home's filesystem.
fn device(path: &Path) -> Option<u64> {
    path.ancestors().find_map(|p| fs::symlink_metadata(p).ok()).map(|m| m.dev())
}

/// The trash `path` goes to.
pub fn trash_for(path: &Path, table: &[Mount]) -> io::Result<TrashDir> {
    trash_for_in(path, table, &data_home())
}

fn trash_for_in(path: &Path, table: &[Mount], data_home: &Path) -> io::Result<TrashDir> {
    let home = TrashDir {
        root: data_home.join("Trash"),
        top: None,
    };
    let home_dev = device(data_home);
    let dev = device(path.parent().unwrap_or(path));
    if dev.is_some() && dev == home_dev {
        return Ok(home);
    }
    let top = mounts::of(table, path).map(|m| m.point.clone()).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let shared = top.join(".Trash");
    if let Ok(meta) = fs::symlink_metadata(&shared)
        && meta.is_dir()
        && meta.permissions().mode() & 0o1000 != 0
    {
        let root = shared.join(uid().to_string());
        if fs::DirBuilder::new().recursive(true).mode(0o700).create(&root).is_ok() {
            return Ok(TrashDir { root, top: Some(top) });
        }
    }
    let root = top.join(format!(".Trash-{}", uid()));
    fs::DirBuilder::new().recursive(true).mode(0o700).create(&root)?;
    Ok(TrashDir { root, top: Some(top) })
}

/// The spec's escaping of a `Path=` value: every byte outside the unreserved set and `/`.
fn escape(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::new();
    for &b in path.as_os_str().as_bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn unescape(s: &str) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(b) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(std::ffi::OsString::from_vec(out))
}

fn now_local() -> String {
    jiff::Zoned::now().strftime("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Moves `path` to its trash: the info file claimed first under a free name, then the item
/// renamed into `files`; if the rename fails the claim is taken back.
pub fn trash(path: &Path) -> io::Result<Trashed> {
    let table = mounts::read();
    let dir = trash_for(path, &table)?;
    trash_into(path, &dir)
}

pub fn trash_into(path: &Path, dir: &TrashDir) -> io::Result<Trashed> {
    let files = dir.root.join("files");
    let infos = dir.root.join("info");
    fs::DirBuilder::new().recursive(true).mode(0o700).create(&files)?;
    fs::DirBuilder::new().recursive(true).mode(0o700).create(&infos)?;
    let name = path.file_name().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?.to_string_lossy().into_owned();
    let original = match &dir.top {
        Some(top) => path.strip_prefix(top).map(Path::to_path_buf).unwrap_or_else(|_| path.to_owned()),
        None => path.to_owned(),
    };
    let deleted = now_local();
    for n in 1.. {
        let candidate = if n == 1 { name.clone() } else { format!("{name}.{n}") };
        let info = infos.join(format!("{candidate}.trashinfo"));
        let mut claim = match OpenOptions::new().write(true).create_new(true).mode(0o600).open(&info) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let file = files.join(&candidate);
        if file.symlink_metadata().is_ok() {
            drop(claim);
            let _ = fs::remove_file(&info);
            continue;
        }
        writeln!(claim, "[Trash Info]\nPath={}\nDeletionDate={deleted}", escape(&original))?;
        claim.sync_all().ok();
        drop(claim);
        if let Err(e) = fs::rename(path, &file) {
            let _ = fs::remove_file(&info);
            return Err(e);
        }
        return Ok(Trashed {
            file,
            info,
            original: dir.top.as_ref().map(|t| t.join(&original)).unwrap_or(original),
            deleted,
        });
    }
    unreachable!()
}

/// Every trash on a local mount that has one, the home trash first.
pub fn all_trashes() -> Vec<TrashDir> {
    let mut found = vec![home_trash()];
    let home_dev = device(&data_home());
    let mut seen = std::collections::HashSet::new();
    for m in mounts::read().into_iter().filter(|m| m.class == Class::Local) {
        if device(&m.point).is_none_or(|d| Some(d) == home_dev) || !seen.insert(m.source.clone()) {
            continue;
        }
        for root in [m.point.join(".Trash").join(uid().to_string()), m.point.join(format!(".Trash-{}", uid()))] {
            if root.join("files").is_dir() {
                found.push(TrashDir { root, top: Some(m.point.clone()) });
            }
        }
    }
    found
}

/// Reads one trash's items.
pub fn list(dir: &TrashDir) -> Vec<Trashed> {
    let Ok(read) = fs::read_dir(dir.root.join("info")) else { return Vec::new() };
    read.flatten()
        .filter_map(|e| {
            let info = e.path();
            let name = info.file_name()?.to_string_lossy().strip_suffix(".trashinfo")?.to_owned();
            let text = fs::read_to_string(&info).ok()?;
            let field = |key: &str| text.lines().find_map(|l| l.strip_prefix(key)).map(str::to_owned);
            let path = unescape(&field("Path=")?);
            let original = match (&dir.top, path.is_absolute()) {
                (Some(top), false) => top.join(path),
                _ => path,
            };
            let file = dir.root.join("files").join(&name);
            file.symlink_metadata().ok()?;
            Some(Trashed {
                file,
                info,
                original,
                deleted: field("DeletionDate=").unwrap_or_default(),
            })
        })
        .collect()
}

/// Returns an item to where it was; refuses to overwrite what is there now.
pub fn put_back(item: &Trashed) -> io::Result<PathBuf> {
    if let Some(parent) = item.original.parent() {
        fs::create_dir_all(parent)?;
    }
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        &item.file,
        rustix::fs::CWD,
        &item.original,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(io::Error::from)
    .or_else(|e| {
        // Another filesystem: copied back, then the trashed copy removed.
        if e.raw_os_error() == Some(libc_exdev()) {
            crate::jobs::copy_tree(&item.file, &item.original)?;
            remove_tree(&item.file)
        } else {
            Err(e)
        }
    })?;
    let _ = fs::remove_file(&item.info);
    Ok(item.original.clone())
}

fn libc_exdev() -> i32 {
    rustix::io::Errno::XDEV.raw_os_error()
}

/// Erases an item for good.
pub fn erase(item: &Trashed) -> io::Result<()> {
    remove_tree(&item.file)?;
    fs::remove_file(&item.info)
}

pub fn remove_tree(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Items deleted more than `days` ago, for Remove items from the Trash after 30 days.
pub fn older_than(items: &[Trashed], days: i64) -> Vec<Trashed> {
    let cutoff = jiff::Zoned::now().checked_sub(jiff::Span::new().days(days)).ok();
    let Some(cutoff) = cutoff.map(|c| c.strftime("%Y-%m-%dT%H:%M:%S").to_string()) else { return Vec::new() };
    items.iter().filter(|t| !t.deleted.is_empty() && t.deleted < cutoff).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_in(base: &Path, top: Option<&Path>) -> TrashDir {
        TrashDir {
            root: base.join("Trash"),
            top: top.map(Path::to_owned),
        }
    }

    #[test]
    fn an_item_goes_with_its_info_and_comes_back() {
        let base = tempfile::tempdir().unwrap();
        let docs = base.path().join("docs");
        fs::create_dir(&docs).unwrap();
        let a = docs.join("my notes.md");
        fs::write(&a, "x").unwrap();
        let trash = dir_in(base.path(), None);
        let item = trash_into(&a, &trash).unwrap();
        assert!(!a.exists());
        assert!(item.file.exists());
        let info = fs::read_to_string(&item.info).unwrap();
        assert!(info.contains("Path=") && info.contains("my%20notes.md"), "{info}");
        assert_eq!(list(&trash), vec![item.clone()]);

        // A second item of the name takes the next.
        fs::write(&a, "y").unwrap();
        let second = trash_into(&a, &trash).unwrap();
        assert!(second.file.ends_with("my notes.md.2"));

        // Put Back refuses to overwrite, and returns once the way is clear.
        fs::write(&a, "new").unwrap();
        assert!(put_back(&item).is_err());
        fs::remove_file(&a).unwrap();
        assert_eq!(put_back(&item).unwrap(), a);
        assert_eq!(fs::read_to_string(&a).unwrap(), "x");
        assert!(!item.info.exists());

        erase(&second).unwrap();
        assert!(list(&trash).is_empty());
    }

    #[test]
    fn a_home_trash_not_made_yet_is_still_homes() {
        let base = tempfile::tempdir().unwrap();
        let data = base.path().join("not-yet/share");
        let file = base.path().join("x");
        fs::write(&file, "").unwrap();
        let table = crate::mounts::read();
        assert_eq!(trash_for_in(&file, &table, &data).unwrap(), TrashDir { root: data.join("Trash"), top: None });
    }

    #[test]
    fn a_volumes_paths_are_relative_to_its_top() {
        let base = tempfile::tempdir().unwrap();
        let top = base.path().join("volume");
        fs::create_dir_all(top.join("photos")).unwrap();
        let a = top.join("photos/a.jpg");
        fs::write(&a, "").unwrap();
        let trash = TrashDir {
            root: top.join(".Trash-1000"),
            top: Some(top.clone()),
        };
        let item = trash_into(&a, &trash).unwrap();
        assert!(fs::read_to_string(&item.info).unwrap().contains("Path=photos/a.jpg"));
        // Mounted elsewhere, Put Back follows the volume.
        let moved = base.path().join("elsewhere");
        fs::rename(&top, &moved).unwrap();
        let again = TrashDir {
            root: moved.join(".Trash-1000"),
            top: Some(moved.clone()),
        };
        let listed = list(&again);
        assert_eq!(listed[0].original, moved.join("photos/a.jpg"));
        put_back(&listed[0]).unwrap();
        assert!(moved.join("photos/a.jpg").exists());
    }
}
