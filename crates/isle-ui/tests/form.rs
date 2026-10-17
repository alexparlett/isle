//! The form's controls headless: what a click and the keys choose, and what each says.

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{GlassShapes, ThemeContext, form};

fn settle(test: &mut TestingRunner) {
    for _ in 0..4 {
        test.sync_and_update();
    }
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

/// Clicks the `nth` label reading `caption`, counting from the top.
fn click(test: &mut TestingRunner, caption: &str, nth: usize) {
    let mut found: Vec<(f32, f32)> = test.find_many(|node, element| {
        Label::try_downcast(element)
            .filter(|label| label.text == caption)
            .map(|_| {
                let c = node.layout().area.center();
                (c.x, c.y)
            })
    });
    found.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (x, y) = *found
        .get(nth)
        .unwrap_or_else(|| panic!("no {caption:?} #{nth}: {:?}", labels(test)));
    test.click_cursor((f64::from(x), f64::from(y)));
    settle(test);
}

fn mount(app: impl Fn() -> Element + 'static) -> TestingRunner {
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            rect().expanded().child(app())
        },
        (900., 700.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    test
}

#[test]
fn a_segment_clicked_is_chosen() {
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let heard = chosen.clone();
    let mut test = mount(move || {
        let mut at = use_state(|| 0usize);
        let heard = heard.clone();
        form::Segmented::new(
            vec!["Auto".into(), "Light".into(), "Dark".into()],
            *at.read(),
            "Appearance",
            move |i| {
                heard.borrow_mut().push(i);
                at.set(i);
            },
        )
        .into_element()
    });
    click(&mut test, "Dark", 0);
    click(&mut test, "Dark", 0);
    assert_eq!(*chosen.borrow(), vec![2], "choosing the chosen one again says nothing");
}

#[test]
fn a_pop_up_opens_its_menu_and_takes_the_choice() {
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let heard = chosen.clone();
    let mut test = mount(move || {
        let mut at = use_state(|| 0usize);
        let heard = heard.clone();
        form::PopUp::new(
            vec!["Fill".into(), "Minimise".into(), "Do nothing".into()],
            *at.read(),
            "Double-click a title bar to",
            move |i| {
                heard.borrow_mut().push(i);
                at.set(i);
            },
        )
        .into_element()
    });
    assert!(!labels(&test).contains(&"Minimise".to_owned()));
    click(&mut test, "Fill", 0);
    assert!(labels(&test).contains(&"Do nothing".to_owned()), "{:?}", labels(&test));
    click(&mut test, "Minimise", 0);
    assert_eq!(*chosen.borrow(), vec![1]);
    let shown = labels(&test);
    assert!(shown.contains(&"Minimise".to_owned()) && !shown.contains(&"Do nothing".to_owned()));
}

#[test]
fn a_navigation_row_opens_and_a_disabled_one_does_not() {
    let opened = Rc::new(RefCell::new(0));
    let heard = opened.clone();
    let mut test = mount(move || {
        let heard = heard.clone();
        let again = heard.clone();
        form::group(
            None,
            vec![
                form::Row::new("Hot Corners")
                    .opens(Some("2 set".into()), move |_| *heard.borrow_mut() += 1)
                    .into_element(),
                form::Row::new("Widgets")
                    .opens(None, move |_| *again.borrow_mut() += 10)
                    .enabled(false)
                    .into_element(),
            ],
        )
        .into_element()
    });
    click(&mut test, "Hot Corners", 0);
    click(&mut test, "Widgets", 0);
    assert_eq!(*opened.borrow(), 1);
}

#[test]
fn a_text_field_edits_with_the_macs_keys() {
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            let value = use_state(|| "night light".to_owned());
            let shown = value.read().clone();
            rect()
                .expanded()
                .child(isle_ui::TextField::new(value, "Search"))
                .child(label().text(format!("value={shown}")))
        },
        (600., 300.).into(),
        |runner| {
            runner.provide_root_context(|| TargetPlatform::MacOs);
        },
        1.,
    );
    settle(&mut test);
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element).map(|_| node.layout().area.center())
        })
        .expect("the field's text");
    test.click_cursor((f64::from(field.x), f64::from(field.y)));
    settle(&mut test);

    // Cmd+Right to the line's end, Option+Backspace takes the word before.
    test.press_key_with_modifiers(Key::Named(NamedKey::ArrowRight), Modifiers::META);
    test.press_key_with_modifiers(Key::Named(NamedKey::Backspace), Modifiers::ALT);
    settle(&mut test);
    assert!(labels(&test).contains(&"value=night ".to_owned()), "{:?}", labels(&test));

    // Cmd+A selects it all; typing replaces it.
    test.press_key_with_modifiers(Key::Character("a".into()), Modifiers::META);
    test.write_text("light");
    settle(&mut test);
    assert!(labels(&test).contains(&"value=light".to_owned()), "{:?}", labels(&test));
}
