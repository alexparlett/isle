//! A wiped field takes the keys typed into it, focused by a click or as it appears.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{Field, FieldText, GlassShapes, ThemeContext};

fn run(auto: bool) -> (TestingRunner, FieldText) {
    let text: FieldText = FieldText::default();
    let inner = text.clone();
    let (test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            rect().expanded().padding(20.).child(
                Field::new(inner.clone(), "Passphrase")
                    .secret(true)
                    .auto_focus(auto),
            )
        },
        (400., 200.).into(),
        |_| {},
        1.,
    );
    (test, text)
}

fn typed(test: &mut TestingRunner) {
    for _ in 0..4 {
        test.sync_and_update();
    }
    for c in "abc".chars() {
        test.press_key(Key::Character(c.to_string()));
        test.sync_and_update();
    }
}

#[test]
fn a_field_focused_as_it_appears_takes_keys() {
    let (mut test, text) = run(true);
    typed(&mut test);
    assert_eq!(text.borrow().as_str(), "abc");
}

#[test]
fn a_field_clicked_takes_keys() {
    let (mut test, text) = run(false);
    for _ in 0..4 {
        test.sync_and_update();
    }
    test.click_cursor((100., 35.));
    typed(&mut test);
    assert_eq!(text.borrow().as_str(), "abc");
}
