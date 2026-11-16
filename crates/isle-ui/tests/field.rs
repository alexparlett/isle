//! A wiped field takes the keys typed into it, focused by a click or as it appears, and edits
//! as a macOS text field with Cmd as an app gets it and as the shell gets it translated.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{Field, FieldText, GlassShapes, ThemeContext};

fn run(auto: bool) -> (TestingRunner, FieldText) {
    run_on(auto, TargetPlatform::Linux)
}

fn run_on(auto: bool, platform: TargetPlatform) -> (TestingRunner, FieldText) {
    let text: FieldText = FieldText::default();
    let inner = text.clone();
    let (test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| platform);
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

fn chord(test: &mut TestingRunner, key: Key, modifiers: Modifiers) {
    test.press_key_with_modifiers(key, modifiers);
    test.sync_and_update();
}

fn letter(c: &str) -> Key {
    Key::Character(c.into())
}

#[test]
fn the_shell_takes_cmd_as_ctrl_select_all_word_and_line_keys() {
    let (mut test, text) = run_on(true, TargetPlatform::Linux);
    typed(&mut test);
    chord(&mut test, letter("a"), Modifiers::CONTROL);
    assert_eq!(text.borrow().selection(), Some(0..3));
    chord(&mut test, letter("x"), Modifiers::empty());
    assert_eq!(text.borrow().as_str(), "x");
    for c in "yz".chars() {
        chord(&mut test, letter(&c.to_string()), Modifiers::empty());
    }
    chord(&mut test, Key::Named(NamedKey::Home), Modifiers::empty());
    chord(&mut test, letter("w"), Modifiers::empty());
    assert_eq!(text.borrow().as_str(), "wxyz");
    chord(&mut test, Key::Named(NamedKey::End), Modifiers::SHIFT);
    chord(
        &mut test,
        Key::Named(NamedKey::Backspace),
        Modifiers::empty(),
    );
    assert_eq!(text.borrow().as_str(), "w");
    chord(
        &mut test,
        Key::Named(NamedKey::Backspace),
        Modifiers::CONTROL,
    );
    assert_eq!(
        text.borrow().as_str(),
        "",
        "a secure field deletes its whole text by word"
    );
}

#[test]
fn an_app_takes_cmd_option_and_the_emacs_keys() {
    let (mut test, text) = run_on(true, TargetPlatform::MacOs);
    typed(&mut test);
    chord(&mut test, Key::Named(NamedKey::ArrowLeft), Modifiers::META);
    assert_eq!(text.borrow().caret(), 0);
    chord(&mut test, letter("e"), Modifiers::CONTROL);
    assert_eq!(text.borrow().caret(), 3);
    chord(&mut test, Key::Named(NamedKey::ArrowLeft), Modifiers::SHIFT);
    assert_eq!(text.borrow().selection(), Some(2..3));
    chord(&mut test, letter("a"), Modifiers::META);
    assert_eq!(text.borrow().selection(), Some(0..3));
    chord(&mut test, letter("c"), Modifiers::META);
    chord(&mut test, letter("a"), Modifiers::CONTROL);
    chord(&mut test, letter("k"), Modifiers::CONTROL);
    assert_eq!(text.borrow().as_str(), "");
}

#[test]
fn paste_goes_to_the_owner_where_the_tree_has_no_clipboard() {
    let text: FieldText = FieldText::default();
    let asked = Rc::new(std::cell::Cell::new(0));
    let (inner, seen) = (text.clone(), asked.clone());
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| TargetPlatform::Linux);
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            let seen = seen.clone();
            rect().expanded().padding(20.).child(
                Field::new(inner.clone(), "Passphrase")
                    .secret(true)
                    .auto_focus(true)
                    .on_paste(move |_| seen.set(seen.get() + 1)),
            )
        },
        (400., 200.).into(),
        |_| {},
        1.,
    );
    typed(&mut test);
    chord(&mut test, letter("v"), Modifiers::CONTROL);
    assert_eq!(asked.get(), 1);
    assert!(isle_ui::field::paste_into(&text, "d\ne"));
    assert_eq!(text.borrow().as_str(), "abcde");
}
