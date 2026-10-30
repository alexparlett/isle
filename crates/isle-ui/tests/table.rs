//! The table drawn headless: only the rows that show are built, a click selects, arrows move
//! the selection, and the order holds while the pointer is over it.

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{
    GlassShapes, ThemeContext,
    table::{Cell, Column, Row, Sort, Table, TableEvent},
};

type Events = Rc<RefCell<Vec<TableEvent>>>;

fn rows(order: &[u64]) -> Rc<Vec<Row>> {
    Rc::new(
        order
            .iter()
            .map(|k| Row {
                key: *k,
                name: format!("proc-{k:03}"),
                cells: vec![Cell::from(format!("{k}.0"))],
                ..Default::default()
            })
            .collect(),
    )
}

/// A table of `order` that owns its selection, with the order given by `order_state`.
fn runner(order: Vec<u64>, events: Events) -> (TestingRunner, Rc<RefCell<Option<State<Vec<u64>>>>>) {
    let handle: Rc<RefCell<Option<State<Vec<u64>>>>> = Rc::default();
    let handle_in = handle.clone();
    let (test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap())))
            });
            use_provide_context(GlassShapes::default);
            let keys = use_state(|| order.clone());
            *handle_in.borrow_mut() = Some(keys);
            let mut selected = use_state(Vec::<u64>::new);
            let events = events.clone();
            let columns = Rc::new(vec![Column::text("name", "Process name", 240.), Column::figures("cpu", "% CPU", 64.)]);
            Table {
                columns,
                rows: rows(&keys.read()),
                sort: Sort { column: "cpu", descending: true },
                selected: Rc::new(selected.read().clone()),
                focused: true,
                hold: false,
                on_event: EventHandler::new(move |e: TableEvent| {
                    if let TableEvent::Selected(keys) = &e {
                        selected.set(keys.clone());
                    }
                    events.borrow_mut().push(e);
                }),
                controller: use_scroll_controller(ScrollConfig::default),
            }
            .into_element()
        },
        (600., 400.).into(),
        |_| {},
        1.,
    );
    (test, handle)
}

fn settle(test: &mut TestingRunner) {
    for _ in 0..6 {
        test.sync_and_update();
    }
}

fn names(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
        .into_iter()
        .filter(|t| t.starts_with("proc-"))
        .collect()
}

fn centre_of(test: &TestingRunner, caption: &str) -> (f64, f64) {
    let at = test
        .find(|node, element| {
            Label::try_downcast(element).filter(|l| l.text == caption).map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("nothing reads {caption:?}"));
    (f64::from(at.x), f64::from(at.y))
}

fn last_selection(events: &Events) -> Vec<u64> {
    events
        .borrow()
        .iter()
        .rev()
        .find_map(|e| match e {
            TableEvent::Selected(k) => Some(k.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn five_hundred_rows_build_only_those_that_show() {
    let events: Events = Rc::default();
    let (mut test, _) = runner((0..500).collect(), events);
    settle(&mut test);
    let shown = names(&test);
    assert!(!shown.is_empty() && shown.len() < 30, "{} rows built", shown.len());
    assert_eq!(shown[0], "proc-000");
}

#[test]
fn a_click_selects_and_arrows_move_the_selection() {
    let events: Events = Rc::default();
    let (mut test, _) = runner((0..20).collect(), events.clone());
    settle(&mut test);
    let at = centre_of(&test, "proc-003");
    test.click_cursor(at);
    settle(&mut test);
    assert_eq!(last_selection(&events), [3]);
    test.press_key(Key::Named(NamedKey::ArrowDown));
    settle(&mut test);
    assert_eq!(last_selection(&events), [4]);
    test.press_key_with_modifiers(Key::Named(NamedKey::ArrowDown), Modifiers::SHIFT);
    settle(&mut test);
    assert_eq!(last_selection(&events), [4, 5]);
}

#[test]
fn the_order_holds_while_the_pointer_is_over_the_table() {
    let events: Events = Rc::default();
    let (mut test, keys) = runner(vec![1, 2, 3], events);
    settle(&mut test);
    test.move_cursor((300., 60.));
    settle(&mut test);
    let mut keys_state = keys.borrow().unwrap();
    test.run_in(|| keys_state.set(vec![3, 2, 1]));
    settle(&mut test);
    assert_eq!(names(&test), ["proc-001", "proc-002", "proc-003"], "held");
    test.move_cursor((300., 900.));
    test.move_cursor((-10., -10.));
    settle(&mut test);
    test.run_in(|| keys_state.set(vec![3, 2, 1]));
    settle(&mut test);
    assert_eq!(names(&test), ["proc-003", "proc-002", "proc-001"], "caught up once the pointer left");
}

#[test]
fn a_title_dragged_onto_another_moves_its_column() {
    let events: Events = Rc::default();
    let sink = events.clone();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap())))
            });
            use_provide_context(GlassShapes::default);
            let sink = sink.clone();
            Table {
                columns: Rc::new(vec![
                    Column::text("name", "Process name", 240.),
                    Column::figures("cpu", "% CPU", 80.),
                    Column::figures("mem", "Memory", 80.),
                ]),
                rows: rows(&[1, 2]),
                sort: Sort { column: "cpu", descending: true },
                selected: Rc::new(Vec::new()),
                focused: true,
                hold: false,
                on_event: EventHandler::new(move |e: TableEvent| sink.borrow_mut().push(e)),
                controller: use_scroll_controller(ScrollConfig::default),
            }
            .into_element()
        },
        (600., 400.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    let from = centre_of(&test, "% CPU");
    let to = centre_of(&test, "Memory");
    test.press_cursor(from);
    test.move_cursor((from.0 + 20., from.1));
    test.move_cursor(to);
    test.release_cursor(to);
    settle(&mut test);
    let got = events.borrow().clone();
    assert!(got.contains(&TableEvent::Move("cpu", 1)), "{got:?}");
    assert!(!got.iter().any(|e| matches!(e, TableEvent::Sort(_))), "a drag is not a click: {got:?}");
}

#[test]
fn rows_with_icons_survive_a_switch_of_columns_and_back() {
    let handle: Rc<RefCell<Option<State<bool>>>> = Rc::default();
    let handle_in = handle.clone();
    let icons: Vec<std::path::PathBuf> = std::fs::read_dir("/usr/share/icons/hicolor/48x48/apps")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .take(8)
        .collect();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap())))
            });
            use_provide_context(GlassShapes::default);
            let memory = use_state(|| false);
            *handle_in.borrow_mut() = Some(memory);
            let memory_now = *memory.read();
            let mut keys: Vec<u64> = (1..40).collect();
            if memory_now {
                keys.sort_by_key(|k| (k * 7919) % 41);
            }
            let rows = keys
                .iter()
                .map(|k| Row {
                    key: *k,
                    name: format!("proc-{k:03}"),
                    icon: (k % 3 == 0).then(|| icons.get(*k as usize % icons.len().max(1)).cloned()).flatten(),
                    cells: if memory_now {
                        vec![Cell::from(format!("{k} MB")), Cell::from("4".to_owned())]
                    } else {
                        vec![Cell::from(format!("{k}.0")), Cell::from("0:01".to_owned()), Cell::from("4".to_owned())]
                    },
                    ..Default::default()
                })
                .collect();
            let columns = if memory_now {
                vec![Column::text("name", "Process name", 240.), Column::figures("memory", "Memory", 80.), Column::figures("threads", "Threads", 64.)]
            } else {
                vec![
                    Column::text("name", "Process name", 240.),
                    Column::figures("cpu", "% CPU", 64.),
                    Column::figures("cpu_time", "CPU time", 80.),
                    Column::figures("threads", "Threads", 64.),
                ]
            };
            Table {
                columns: Rc::new(columns),
                rows: Rc::new(rows),
                sort: Sort { column: if memory_now { "memory" } else { "cpu" }, descending: true },
                selected: Rc::new(vec![3]),
                focused: true,
                hold: false,
                on_event: EventHandler::new(|_| {}),
                controller: use_scroll_controller(ScrollConfig::default),
            }
            .into_element()
        },
        (600., 400.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    let mut memory = handle.borrow().unwrap();
    for on in [true, false, true, false] {
        test.run_in(|| memory.set(on));
        for _ in 0..20 {
            test.poll(std::time::Duration::from_millis(16), std::time::Duration::from_millis(20));
        }
    }
    assert!(!names(&test).is_empty());
}
