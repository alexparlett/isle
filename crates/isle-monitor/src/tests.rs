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
    for _ in 0..150 {
        test.sync_and_update();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    test.render_to_file(std::path::PathBuf::from(path));
}
