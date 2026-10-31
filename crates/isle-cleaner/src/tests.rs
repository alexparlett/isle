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

fn runner(folder: PathBuf, width: f32) -> TestingRunner {
    let state = folder.join(".state");
    // SAFETY: set before any thread of this test reads the environment.
    unsafe {
        std::env::set_var("XDG_STATE_HOME", &state);
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
