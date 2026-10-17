//! Settings' frame headless: the sidebar, search, links, history and the notice.

use std::{collections::BTreeMap, path::PathBuf};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_app::{Desktop, Request};

use crate::frame::{Frame, MemoryDir};

fn settle(test: &mut TestingRunner) {
    for _ in 0..6 {
        test.sync_and_update();
    }
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

fn click(test: &mut TestingRunner, caption: &str) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}: {:?}", labels(test)));
    test.click_cursor((f64::from(at.x), f64::from(at.y)));
    settle(test);
}

fn memory(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("isle-settings-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The frame with a desktop to drive; `pending` holds what a test hands it.
fn mount(memory: PathBuf, pending: std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn(Desktop)>>>>) -> TestingRunner {
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| MemoryDir(memory.clone()));
            use_provide_context(crate::services::Services::still);
            let desktop = isle_app::provide_for_tests(BTreeMap::new());
            if let Some(act) = pending.borrow_mut().take() {
                act(desktop);
            }
            rect().expanded().child(Frame)
        },
        (900., 760.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    test
}

type Pending = std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn(Desktop)>>>>;

#[test]
fn it_opens_on_general_then_where_it_was_left() {
    let dir = memory("first");
    let mut test = mount(dir.clone(), Pending::default());
    assert!(labels(&test).contains(&"About this computer and the system it runs.".to_owned()));
    click(&mut test, "Sound");
    assert!(labels(&test).contains(&"Choose alert sounds, speakers and microphones.".to_owned()));

    let test = mount(dir, Pending::default());
    assert!(labels(&test).contains(&"Choose alert sounds, speakers and microphones.".to_owned()));
}

#[test]
fn search_lists_panes_and_keywords_and_says_when_nothing_matches() {
    let mut test = mount(memory("search"), Pending::default());
    let field = test
        .find(|node, element| Paragraph::try_downcast(element).map(|_| node.layout().area.center()))
        .unwrap();
    test.click_cursor((f64::from(field.x), f64::from(field.y)));
    test.write_text("resolution");
    settle(&mut test);
    let shown = labels(&test);
    assert!(!shown.contains(&"Bluetooth".to_owned()), "the pane list gives way: {shown:?}");
    click(&mut test, "Displays");
    assert!(labels(&test).contains(&"Arrange and set up the screens you use.".to_owned()));

    test.click_cursor((f64::from(field.x), f64::from(field.y)));
    test.write_text("zzz");
    settle(&mut test);
    assert!(labels(&test).contains(&"No results for “resolutionzzz”".to_owned()), "{:?}", labels(&test));
}

#[test]
fn a_link_opens_its_pane_and_back_returns() {
    let pending = Pending::default();
    *pending.borrow_mut() = Some(Box::new(|desktop: Desktop| {
        let mut requests = desktop.requests;
        requests.write().push(Request::Open {
            uris: vec!["isle-settings://focus".into()],
            token: None,
        });
    }));
    let mut test = mount(memory("link"), pending);
    assert!(labels(&test).contains(&"Choose what interrupts you, and when.".to_owned()), "{:?}", labels(&test));
    // The toolbar's Back, after the sidebar and its padding.
    test.click_cursor((220. + 20. + 12., 26.));
    settle(&mut test);
    assert!(labels(&test).contains(&"About this computer and the system it runs.".to_owned()), "{:?}", labels(&test));
}

#[test]
fn the_notice_names_the_line_when_settings_kdl_is_rejected() {
    let pending = Pending::default();
    *pending.borrow_mut() = Some(Box::new(|desktop: Desktop| {
        let mut status = desktop.status;
        status.set(Some(isle_ipc::SettingsStatus {
            ok: false,
            file: "settings.kdl".into(),
            line: 12,
            column: 4,
            message: "`repeat-rate` is a whole number".into(),
        }));
    }));
    let test = mount(memory("notice"), pending);
    let shown = labels(&test);
    assert!(
        shown.contains(&"settings.kdl has an error on line 12, column 4. Changes won't be saved until it's fixed.".to_owned()),
        "{shown:?}"
    );
    assert!(shown.contains(&"`repeat-rate` is a whole number".to_owned()));
}
