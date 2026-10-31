//! Duplicates on real files: equal content grouped, equal sizes or equal ends alone not, and a
//! reflinked copy said to share its storage.

use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

use isle_scan::{Scan, duplicates};

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn dir(name: &str) -> Dir {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("isle-dups-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    Dir(d)
}

/// Two megabytes of a pattern, with `poke` written at `at` when given.
fn write(path: &Path, seed: u8, poke: Option<(u64, u8)>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = fs::File::create(path).unwrap();
    let block: Vec<u8> = (0..65536u32)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect();
    for _ in 0..32 {
        f.write_all(&block).unwrap();
    }
    if let Some((at, b)) = poke {
        f.seek(SeekFrom::Start(at)).unwrap();
        f.write_all(&[b]).unwrap();
    }
    f.sync_all().unwrap();
}

fn groups(root: &Path) -> Vec<duplicates::Group> {
    let (tree, _) = Scan::start(vec![root.to_path_buf()]).finish();
    duplicates::compare(duplicates::candidates(&tree, &[]), &AtomicBool::new(false))
}

#[test]
fn equal_content_groups_and_nothing_else_does() {
    let d = dir("content");
    let r = &d.0;
    write(&r.join("Photos/a.raw"), 1, None);
    write(&r.join("Backup/a copy.raw"), 1, None);
    write(&r.join("Backup/old/a again.raw"), 1, None);
    write(&r.join("Photos/other.raw"), 2, None);
    // Same size and ends as a.raw, a byte changed in the middle.
    write(&r.join("Photos/nearly.raw"), 1, Some((1 << 20, 0xff)));
    let found = groups(r);
    assert_eq!(found.len(), 1, "{found:?}");
    let mut names: Vec<_> = found[0]
        .copies
        .iter()
        .map(|c| c.path.strip_prefix(r).unwrap().to_path_buf())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            PathBuf::from("Backup/a copy.raw"),
            "Backup/old/a again.raw".into(),
            "Photos/a.raw".into()
        ]
    );
    assert_eq!(found[0].size, 2 << 20);
}

#[test]
fn a_reflinked_copy_shares_its_storage() {
    let d = dir("reflink");
    let r = &d.0;
    write(&r.join("Videos/clip.raw"), 3, None);
    write(&r.join("Videos/plain.raw"), 4, None);
    let cloned = Command::new("cp")
        .arg("--reflink=always")
        .arg(r.join("Videos/clip.raw"))
        .arg(r.join("Videos/clip copy.raw"))
        .status()
        .is_ok_and(|s| s.success());
    if !cloned {
        eprintln!("skipped: this filesystem does not reflink");
        return;
    }
    let plain = duplicates::sharing(&r.join("Videos/plain.raw")).expect("an extent map");
    assert_eq!((plain.bytes, plain.shared), (2 << 20, 0));
    let shared = duplicates::sharing(&r.join("Videos/clip.raw")).unwrap();
    assert_eq!(shared.shared, shared.bytes);
    let found = groups(r);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].copies.iter().all(|c| c.shares_storage),
        "{found:?}"
    );
}

#[test]
fn a_rules_find_is_no_duplicate() {
    let d = dir("claimed");
    let r = &d.0;
    write(&r.join("Photos/a.raw"), 5, None);
    write(&r.join("Videos/a.raw"), 5, None);
    let (tree, _) = Scan::start(vec![r.clone()]).finish();
    let find = isle_scan::Find {
        rule: "test".into(),
        name: "Videos".into(),
        path: r.join("Videos"),
        node: None,
        bytes: 1,
        kind: isle_scan::Kind::Video,
        safe: false,
        why: String::new(),
    };
    assert_eq!(duplicates::candidates(&tree, &[]).len(), 2);
    assert_eq!(duplicates::candidates(&tree, &[find]).len(), 1);
}
