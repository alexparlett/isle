//! The window drawn headless, scanning a temporary home.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use freya::prelude::*;
use freya_testing::TestingRunner;

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn settle_until(test: &mut TestingRunner, wanted: &str) {
    for _ in 0..300 {
        test.poll(
            std::time::Duration::from_millis(16),
            std::time::Duration::from_millis(20),
        );
        if labels(test).iter().any(|l| l.contains(wanted)) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("never read {wanted:?}: {:?}", labels(test));
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}"));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
}

fn put(root: &Path, rel: &str, bytes: usize) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::File::create(&path)
        .unwrap()
        .write_all(&vec![3u8; bytes])
        .unwrap();
}

/// A home with caches, a build folder and some of the user's own files.
pub fn home(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp")
        .join(format!("cleaner-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    put(&d, ".cache/spotify/data.bin", 3_000_000);
    put(&d, ".cache/thumbnails/large/a.png", 400_000);
    put(&d, ".npm/_cacache/index", 900_000);
    put(&d, "code/app/target/debug/app", 5_000_000);
    put(&d, "code/app/Cargo.toml", 200);
    put(&d, "Videos/trip.mkv", 8_000_000);
    put(&d, "Pictures/a.jpg", 1_500_000);
    put(&d, "Documents/notes.txt", 20_000);
    put(&d, "Backup/a.jpg", 1_500_000);
    put(&d, "Documents/old-report.pdf", 2_000_000);
    let then = std::time::SystemTime::now() - std::time::Duration::from_secs(400 * 24 * 3600);
    fs::File::options()
        .write(true)
        .open(d.join("Documents/old-report.pdf"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_accessed(then).set_modified(then))
        .unwrap();
    d
}

/// The window's tests set `HOME` and the XDG folders for the process, so they take turns.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn runner(folder: PathBuf, width: f32) -> TestingRunner {
    let state = folder.join(".state");
    // SAFETY: set before any thread of this test reads the environment.
    unsafe {
        std::env::set_var("XDG_STATE_HOME", &state);
        std::env::set_var("XDG_DATA_HOME", folder.join(".local/share"));
        std::env::set_var("HOME", &folder);
    }
    let (test, _) = TestingRunner::new(
        move || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::window::Cleaner {
                folder: Some(folder.clone()),
            }
            .render()
            .into_element()
        },
        (width, 760.).into(),
        |_| {},
        1.,
    );
    test
}

/// Draws the window to `$CLEANER_SHOT` once the scan has ended, for a look.
#[test]
#[ignore]
fn shot() {
    let _turn = serial();
    let Some(path) = std::env::var_os("CLEANER_SHOT") else {
        return;
    };
    let home = home("shot");
    let mut test = runner(
        home,
        std::env::var("CLEANER_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(1200.),
    );
    settle_until(&mut test, "scanned");
    for caption in std::env::var("CLEANER_CLICK")
        .unwrap_or_default()
        .split('|')
        .filter(|c| !c.is_empty())
    {
        click(&mut test, caption);
        for _ in 0..10 {
            test.poll(
                std::time::Duration::from_millis(16),
                std::time::Duration::from_millis(20),
            );
        }
    }
    for _ in 0..60 {
        test.poll(
            std::time::Duration::from_millis(16),
            std::time::Duration::from_millis(20),
        );
    }
    test.render_to_file(PathBuf::from(path));
}

#[test]
fn a_scan_fills_overview_and_cleanup_and_the_basket() {
    let _turn = serial();
    let home = home("fill");
    let mut test = runner(home.clone(), 1200.);
    settle_until(&mut test, "scanned");
    let seen = labels(&test);
    assert!(
        seen.iter().any(|l| l.ends_with("safe to clear")),
        "{seen:?}"
    );
    assert!(seen.iter().any(|l| l == "Largest folders"), "{seen:?}");
    assert!(
        seen.iter().any(|l| l == "Basket"),
        "the safe finds go in after the scan: {seen:?}"
    );
    click(&mut test, "Review");
    settle_until(&mut test, "Worth a look");
    let seen = labels(&test);
    assert!(seen.iter().any(|l| l == "Spotify"), "{seen:?}");
    assert!(
        seen.iter().any(|l| l == "app"),
        "the build folder, named for its project: {seen:?}"
    );
    assert!(seen.iter().any(|l| l == "These can't be put back."));
    let _ = fs::remove_dir_all(&home);
}

/// Every file under `root` with its inode and content, the Trash left out.
fn fingerprint(root: &Path) -> Vec<(PathBuf, u64, Vec<u8>)> {
    use std::os::unix::fs::MetadataExt;
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.starts_with(root.join(".local/share/Trash")) || p.starts_with(root.join(".state"))
            {
                continue;
            }
            let m = fs::symlink_metadata(&p).unwrap();
            if m.is_dir() {
                stack.push(p);
            } else {
                out.push((p.clone(), m.ino(), fs::read(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn nothing_moves_until_move_to_trash_and_undo_puts_it_all_back() {
    let _turn = serial();
    let home = home("unmoved");
    // SAFETY: set before any thread of this test reads the environment.
    unsafe { std::env::set_var("XDG_DATA_HOME", home.join(".local/share")) };
    let before = fingerprint(&home);
    let (tree, _) = isle_scan::Scan::start(vec![home.clone()]).finish();
    let ctx = isle_scan::Context {
        home: home.clone(),
        libraries: Vec::new(),
        now: isle_scan::Context::here(Vec::new()).now,
    };
    let finds = isle_scan::Rules::shipped().find(&tree, &ctx);
    let groups = isle_scan::duplicates::compare(
        isle_scan::duplicates::candidates(&tree, &finds),
        &std::sync::atomic::AtomicBool::new(false),
    );
    assert!(!finds.is_empty() && !groups.is_empty());
    let mut basket = crate::basket::Basket::default();
    for f in &finds {
        let m = fs::symlink_metadata(&f.path).unwrap();
        use std::os::unix::fs::MetadataExt;
        basket.add(crate::basket::Item {
            path: f.path.clone(),
            name: f.name.clone(),
            bytes: f.bytes,
            kind: f.kind,
            dev: m.dev(),
            ino: m.ino(),
        });
    }
    for c in groups.iter().flat_map(|g| g.copies.iter().skip(1)) {
        basket.add(crate::basket::Item {
            path: c.path.clone(),
            name: String::new(),
            bytes: c.bytes,
            kind: isle_scan::Kind::Other,
            dev: 0,
            ino: 0,
        });
    }
    assert_eq!(
        fingerprint(&home),
        before,
        "scanning, rules, comparing and filling the basket move nothing"
    );

    let (ok, refused) = crate::mover::check(basket.items());
    assert!(refused.is_empty(), "{refused:?}");
    struct Quiet;
    impl isle_fs::jobs::Control for Quiet {
        fn progress(&self, _: isle_fs::jobs::Progress) {}
        fn ask(&self, _: isle_fs::jobs::Conflict) -> isle_fs::jobs::Answer {
            isle_fs::jobs::Answer {
                choice: isle_fs::jobs::Choice::KeepBoth,
                all: true,
            }
        }
        fn stopped(&self) -> bool {
            false
        }
    }
    let report = isle_fs::jobs::run(isle_fs::jobs::Work::Trash { items: ok.clone() }, &Quiet);
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert!(ok.iter().all(|p| !p.exists()), "moved");
    assert_ne!(fingerprint(&home), before);
    let Some(isle_fs::jobs::Undo::Steps(steps)) = report.undo else {
        panic!("no undo")
    };
    let back = isle_fs::jobs::run(isle_fs::jobs::Work::Undo { steps }, &Quiet);
    assert!(back.failed.is_empty(), "{:?}", back.failed);
    assert_eq!(
        fingerprint(&home),
        before,
        "one undo puts every file back as it was, inode and all"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn an_item_changed_since_the_scan_is_not_moved() {
    let home = home("changed");
    let path = home.join("Videos/trip.mkv");
    use std::os::unix::fs::MetadataExt;
    let m = fs::symlink_metadata(&path).unwrap();
    let item = crate::basket::Item {
        path: path.clone(),
        name: "trip.mkv".into(),
        bytes: 1,
        kind: isle_scan::Kind::Video,
        dev: m.dev(),
        ino: m.ino(),
    };
    fs::remove_file(&path).unwrap();
    put(&home, "Videos/trip.mkv", 10);
    assert_eq!(
        crate::mover::refused(&item),
        Some("It changed since the scan.")
    );
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("/etc/hostname", &path).unwrap();
    assert_eq!(crate::mover::refused(&item), Some("It's now a link."));
    fs::remove_file(&path).unwrap();
    assert_eq!(crate::mover::refused(&item), Some("It's no longer there."));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn the_basket_moves_to_the_trash_and_undo_brings_it_back() {
    let _turn = serial();
    let home = home("move");
    let mut test = runner(home.clone(), 1200.);
    settle_until(&mut test, "scanned");
    click(&mut test, "Move to Trash");
    settle_until(&mut test, "Moved 3 items to the Trash");
    assert!(!home.join(".cache/spotify").exists());
    assert!(
        labels(&test)
            .iter()
            .any(|l| l == "Nothing is freed until the Trash is emptied")
    );
    click(&mut test, "Undo");
    settle_until(&mut test, "Basket");
    for _ in 0..50 {
        test.poll(
            std::time::Duration::from_millis(16),
            std::time::Duration::from_millis(20),
        );
        if home.join(".cache/spotify").exists() {
            break;
        }
    }
    assert!(home.join(".cache/spotify/data.bin").exists(), "put back");
    let _ = fs::remove_dir_all(&home);
}

/// Times a frame of Overview and of Space's map, in software, while `$CLEANER_BENCH` (a large
/// folder) scans and once it has: `cargo test --release -p isle-cleaner bench -- --ignored`.
#[test]
#[ignore]
fn bench() {
    let Some(folder) = std::env::var_os("CLEANER_BENCH").map(PathBuf::from) else {
        return;
    };
    let _turn = serial();
    let state =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/cleaner-bench-state");
    // SAFETY: set before any thread of this test reads the environment.
    unsafe { std::env::set_var("XDG_STATE_HOME", &state) };
    let (mut test, _) = TestingRunner::new(
        move || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::window::Cleaner {
                folder: Some(folder.clone()),
            }
            .render()
            .into_element()
        },
        (1200., 760.).into(),
        |_| {},
        1.,
    );
    // The test runner encodes each frame as a PNG; a plain window's render at the same size
    // stands for that and the surface, and is taken off.
    let (mut plain, _) = TestingRunner::new(
        || {
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .background((30, 30, 34))
                .into_element()
        },
        (1200., 760.).into(),
        |_| {},
        1.,
    );
    let mut base: Vec<f64> = (0..20)
        .map(|_| {
            let t = std::time::Instant::now();
            let _ = plain.render();
            t.elapsed().as_secs_f64() * 1000.
        })
        .collect();
    base.sort_by(f64::total_cmp);
    let base = base[10];
    let mut time = |test: &mut TestingRunner, n: usize| {
        let mut frames: Vec<(f64, f64)> = (0..n)
            .map(|_| {
                let t = std::time::Instant::now();
                test.sync_and_update();
                let update = t.elapsed().as_secs_f64() * 1000.;
                let _ = test.render();
                (update, t.elapsed().as_secs_f64() * 1000. - update)
            })
            .collect();
        frames.sort_by(|a, b| (a.0 + a.1).total_cmp(&(b.0 + b.1)));
        let at = |i: usize| (frames[i].0, frames[i].0 + (frames[i].1 - base).max(0.));
        (at(n / 2), at(n * 95 / 100))
    };
    eprintln!("a plain frame's render and encode: {base:.1} ms");
    let scanning = time(&mut test, 40);
    settle_until(&mut test, "scanned");
    let overview = time(&mut test, 40);
    click(&mut test, "Space");
    for _ in 0..20 {
        test.poll(
            std::time::Duration::from_millis(16),
            std::time::Duration::from_millis(20),
        );
    }
    let map = time(&mut test, 40);
    eprintln!(
        "(update, update and draw) ms at p50 and p95: scanning {scanning:.1?}, overview {overview:.1?}, map {map:.1?}"
    );
    let _ = fs::remove_dir_all(&state);
}
