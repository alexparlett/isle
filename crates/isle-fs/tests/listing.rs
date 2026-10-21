//! A large folder listed in batches: the first arrives long before the last, every entry
//! once, merged into order; failures say why.

use std::time::{Duration, Instant};

use isle_fs::{Folder, ListError, Listed, Order, spawn_listing};

#[test]
fn fifty_thousand_entries_arrive_in_batches_and_merge_in_order() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..50_000 {
        std::fs::File::create(dir.path().join(format!("file{i}.txt"))).unwrap();
    }
    std::fs::create_dir(dir.path().join("zz-folder")).unwrap();

    let start = Instant::now();
    let rx = spawn_listing(dir.path().to_owned(), 512);
    let mut folder = Folder::new(Order::default());
    let mut first = None;
    let mut batches = 0;
    loop {
        match rx.recv_timeout(Duration::from_secs(30)).expect("the listing ends") {
            Listed::Batch(batch) => {
                first.get_or_insert_with(|| start.elapsed());
                batches += 1;
                folder.merge(batch);
            }
            Listed::Done => break,
            Listed::Failed(e) => panic!("failed: {e:?}"),
        }
    }
    let all = start.elapsed();
    let first = first.unwrap();
    eprintln!("first batch {first:?}, all {all:?}, {batches} batches");
    assert_eq!(folder.len(), 50_001);
    assert!(batches >= 98);
    assert!(first * 10 < all, "the first batch, {first:?}, comes well before the end, {all:?}");
    let names: Vec<&str> = folder.entries().iter().take(4).map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["zz-folder", "file0.txt", "file1.txt", "file2.txt"]);
    assert_eq!(folder.position(&dir.path().join("file10.txt")), Some(11));
    assert_eq!(
        folder.get(&dir.path().join("file9.txt")).unwrap().mime.as_deref(),
        isle_apps::mime::types().by_name("a.txt").as_deref()
    );
}

#[test]
fn a_listing_says_why_it_failed() {
    let dir = tempfile::tempdir().unwrap();
    let rx = spawn_listing(dir.path().join("missing"), 512);
    assert!(matches!(rx.recv().unwrap(), Listed::Failed(ListError::Gone)));

    let file = dir.path().join("file");
    std::fs::write(&file, "").unwrap();
    let rx = spawn_listing(file, 512);
    assert!(matches!(rx.recv().unwrap(), Listed::Failed(ListError::NotAFolder)));
}

#[test]
fn a_link_to_a_folder_opens_as_one_and_a_broken_link_is_marked() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("real")).unwrap();
    std::os::unix::fs::symlink("real", dir.path().join("alias")).unwrap();
    std::os::unix::fs::symlink("nowhere", dir.path().join("broken")).unwrap();
    let rx = spawn_listing(dir.path().to_owned(), 512);
    let mut folder = Folder::new(Order::default());
    while let Ok(Listed::Batch(b)) = rx.recv() {
        folder.merge(b);
    }
    assert!(folder.get(&dir.path().join("alias")).unwrap().is_dir());
    assert_eq!(
        folder.get(&dir.path().join("broken")).unwrap().kind,
        isle_fs::Kind::Link { to_dir: false, broken: true }
    );
}
