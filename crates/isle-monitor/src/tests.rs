//! The window drawn headless on this machine's own `/proc`: samples arrive, the table and the
//! footer fill, and the resource switch swaps them.

use std::collections::BTreeMap;

use freya::prelude::*;
use freya_testing::TestingRunner;

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn settle_until(test: &mut TestingRunner, wanted: &str) {
    for _ in 0..200 {
        test.sync_and_update();
        if labels(test).iter().any(|l| l == wanted) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("never read {wanted:?}: {:?}", labels(test));
}

/// Starts a copy of `sleep` under `bin`'s name, retrying while another test's fork still
/// holds the copy open for writing (ETXTBSY).
fn sleeper(bin: &std::path::Path) -> std::process::Child {
    std::fs::copy("/usr/bin/sleep", bin).unwrap();
    for _ in 0..50 {
        match std::process::Command::new(bin).arg("60").spawn() {
            Ok(child) => return child,
            Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(e) => panic!("{e}"),
        }
    }
    panic!("{} stayed busy", bin.display());
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| Label::try_downcast(element).filter(|l| l.text == caption).map(|_| node.layout().area.center()))
        .unwrap_or_else(|| panic!("nothing reads {caption:?}"));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
}

#[test]
fn samples_fill_the_table_and_footer_and_resources_switch() {
    let state = std::env::temp_dir().join(format!("isle-monitor-test-{}", std::process::id()));
    // SAFETY: set before any thread of this test reads the environment.
    unsafe { std::env::set_var("XDG_STATE_HOME", &state) };
    let (mut test, _) = TestingRunner::new(
        || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::window::Monitor.render().into_element()
        },
        (1040., 680.).into(),
        |_| {},
        1.,
    );
    settle_until(&mut test, "CPU load");
    let seen = labels(&test);
    assert!(seen.iter().any(|l| l == "All Processes"));
    assert!(seen.iter().any(|l| l == "% CPU"));
    assert!(seen.iter().any(|l| l == "Processes"));
    click(&mut test, "Memory");
    settle_until(&mut test, "Memory pressure");
    assert!(labels(&test).iter().any(|l| l == "Physical memory"));
    click(&mut test, "Network");
    settle_until(&mut test, "UDP traffic isn't counted per process.");
    let _ = std::fs::remove_dir_all(&state);
}

/// Draws the window to `$MONITOR_SHOT` once samples have come, for a look.
#[test]
#[ignore]
fn shot() {
    let Some(path) = std::env::var_os("MONITOR_SHOT") else { return };
    let (mut test, _) = TestingRunner::new(
        || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::window::Monitor.render().into_element()
        },
        (1040., 680.).into(),
        |_| {},
        1.,
    );
    settle_until(&mut test, "CPU load");
    if let Ok(caption) = std::env::var("MONITOR_CLICK") {
        click(&mut test, &caption);
    }
    for _ in 0..150 {
        test.sync_and_update();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    test.render_to_file(std::path::PathBuf::from(path));
}

#[test]
fn a_process_found_by_search_is_quit_through_the_alert() {
    let state = std::env::temp_dir().join(format!("isle-monitor-quit-{}", std::process::id()));
    // SAFETY: set before any thread of this test reads the environment.
    unsafe { std::env::set_var("XDG_STATE_HOME", &state) };
    let bin = state.join("isle-test-sleeper");
    std::fs::create_dir_all(&state).unwrap();
    let mut child = sleeper(&bin);
    let (mut test, _) = TestingRunner::new(
        || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::window::Monitor.render().into_element()
        },
        (1040., 680.).into(),
        |_| {},
        1.,
    );
    settle_until(&mut test, "CPU load");
    // The search field sits at the toolbar's trailing end.
    test.click_cursor((950., 26.));
    test.write_text("isle-test-sleeper");
    settle_until(&mut test, "isle-test-sleeper");
    click(&mut test, "isle-test-sleeper");
    for _ in 0..4 {
        test.sync_and_update();
    }
    click(&mut test, "Quit Process");
    settle_until(&mut test, "Quit “isle-test-sleeper”?");
    click(&mut test, "Quit");
    for _ in 0..100 {
        test.sync_and_update();
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    use std::os::unix::process::ExitStatusExt;
    let status = child.try_wait().unwrap();
    if status.is_none() {
        let _ = child.kill();
    }
    assert_eq!(status.and_then(|s| s.signal()), Some(libc::SIGTERM));
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn inspect_reads_a_process_and_says_when_it_has_ended() {
    let dir = std::env::temp_dir().join(format!("isle-monitor-inspect-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("isle-test-inspected");
    let mut child = sleeper(&bin);
    let pid = child.id() as i32;
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let start = stat.rsplit_once(')').unwrap().1.split_whitespace().nth(19).unwrap().parse().unwrap();
    let key = isle_sysinfo::procs::Key { pid, start };
    let (mut test, _) = TestingRunner::new(
        move || {
            isle_app::provide_for_tests(BTreeMap::new());
            crate::inspect::Inspect { key, name: "isle-test-inspected".into() }.render().into_element()
        },
        (520., 600.).into(),
        |_| {},
        1.,
    );
    settle_until(&mut test, "Parent process");
    let cmdline = format!("{} 60", bin.display());
    settle_until(&mut test, &cmdline);
    assert!(labels(&test).iter().any(|l| l == "Sleeping"), "{:?}", labels(&test));
    let _ = child.kill();
    let _ = child.wait();
    settle_until(&mut test, "This process has ended.");
    let _ = std::fs::remove_dir_all(&dir);
}
