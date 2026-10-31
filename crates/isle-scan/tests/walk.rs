//! The walk on real trees in the target's temporary folder: its totals equal `du`'s, a file with
//! two names counts once, links are never followed, `.snapshots` is never entered, and a folder
//! it cannot read is counted apart.

use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
};

use isle_scan::{Scan, TOP, Tree};

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = Command::new("chmod")
            .args(["-R", "u+rwx"])
            .arg(&self.0)
            .status();
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn dir(name: &str) -> Dir {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("isle-scan-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    Dir(d)
}

fn write(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = fs::File::create(path).unwrap();
    let block = vec![0x5au8; 64 * 1024];
    let mut left = bytes;
    while left > 0 {
        let n = left.min(block.len());
        f.write_all(&block[..n]).unwrap();
        left -= n;
    }
    f.sync_all().unwrap();
}

fn du(path: &Path) -> u64 {
    let out = Command::new("du")
        .args(["-s", "-B1", "--exclude=.snapshots"])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

fn scan(root: &Path) -> Tree {
    Scan::start(vec![root.to_path_buf()]).finish().0
}

fn names(tree: &Tree) -> Vec<String> {
    (1..tree.nodes.len() as u32)
        .map(|i| tree.path(i).to_string_lossy().into_owned())
        .collect()
}

#[test]
fn totals_equal_du_with_links_once_and_nothing_followed() {
    let d = dir("du");
    let r = &d.0;
    write(&r.join("a/one.bin"), 300_000);
    write(&r.join("a/b/two.bin"), 2_000_000);
    write(&r.join("a/b/c/three.txt"), 5_000);
    fs::hard_link(r.join("a/b/two.bin"), r.join("a/two-again.bin")).unwrap();
    // Sparse: ten megabytes long, one block written.
    let mut sparse = fs::File::create(r.join("sparse.img")).unwrap();
    sparse.write_all(&[1; 4096]).unwrap();
    sparse.set_len(10 << 20).unwrap();
    sparse.sync_all().unwrap();
    drop(sparse);
    write(&r.join("data/x.bin"), 4096);
    symlink(r, r.join("data/loop")).unwrap();
    symlink("nowhere", r.join("data/dangling")).unwrap();
    write(&r.join(".snapshots/1/snapshot/huge.bin"), 3_000_000);

    let tree = scan(r);
    let root = tree.children(TOP).next().unwrap();
    assert_eq!(
        tree.node(root).bytes,
        du(r),
        "allocated bytes as du counts them"
    );
    assert_eq!(tree.node(TOP).bytes, tree.node(root).bytes);
    let paths = names(&tree);
    assert!(!paths.iter().any(|p| p.contains(".snapshots")), "{paths:?}");
    assert!(
        !paths.iter().any(|p| p.ends_with("loop")),
        "a link to a folder is a file, not a folder: {paths:?}"
    );
    let largest = tree.largest.sorted();
    assert!(
        tree.file_name(&largest[0])
            .to_string_lossy()
            .starts_with("two"),
        "either name, whichever is met first"
    );
    assert_eq!(
        largest.iter().filter(|f| f.ino == largest[0].ino).count(),
        1,
        "two names, counted once"
    );
    let sparse = largest
        .iter()
        .find(|f| tree.file_name(f) == "sparse.img")
        .expect("kept");
    assert_eq!(sparse.size, 10 << 20);
    assert!(
        sparse.bytes < 1 << 20,
        "a sparse file is its allocated blocks: {}",
        sparse.bytes
    );
}

#[test]
fn a_folder_that_cannot_be_read_is_counted_apart() {
    let d = dir("unreadable");
    let r = &d.0;
    write(&r.join("open/a.bin"), 100_000);
    write(&r.join("closed/secret.bin"), 1_000_000);
    fs::set_permissions(r.join("closed"), fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(r.join("closed")).is_ok() {
        eprintln!("skipped: running as root");
        return;
    }
    let tree = scan(r);
    let unreadable: Vec<_> = tree.unreadable().map(|i| tree.path(i)).collect();
    assert_eq!(unreadable, [r.join("closed")]);
    let root = tree.children(TOP).next().unwrap();
    assert_eq!(tree.node(root).bytes, du(r));
}

#[test]
fn duplicates_are_looked_for_only_among_the_users_own_files() {
    let d = dir("candidates");
    let r = &d.0;
    write(&r.join("Photos/a.jpg"), 2 << 20);
    write(&r.join("Photos/small.jpg"), 100_000);
    write(&r.join(".hidden/b.jpg"), 2 << 20);
    write(&r.join("project/sub/c.jpg"), 2 << 20);
    fs::create_dir_all(r.join("project/.git")).unwrap();
    write(&r.join("node_modules/d.jpg"), 2 << 20);
    write(&r.join("tool"), 2 << 20);
    fs::set_permissions(r.join("tool"), fs::Permissions::from_mode(0o755)).unwrap();
    let tree = scan(r);
    let found: Vec<_> = tree.candidates.iter().map(|f| tree.file_path(f)).collect();
    assert_eq!(found, [r.join("Photos/a.jpg")]);
}

#[test]
fn stopping_keeps_what_was_found_marked_partial() {
    let d = dir("stop");
    for i in 0..400 {
        write(&d.0.join(format!("{}/{}/f", i % 20, i)), 10);
    }
    let scan = Scan::start(vec![d.0.clone()]);
    scan.stop();
    let (tree, progress) = scan.finish();
    assert!(progress.partial && !progress.done, "{progress:?}");
    assert!(!tree.nodes.is_empty());

    let (_, progress) = Scan::start(vec![d.0.clone()]).finish();
    assert!(progress.done && !progress.partial, "{progress:?}");
    assert_eq!(progress.share(), Some(1.0));
}
