//! A field opened for editing takes every key typed, not only the first.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{GlassShapes, TextField, ThemeContext};

#[test]
fn an_editing_field_takes_every_key() {
    let typed = Rc::new(std::cell::RefCell::new(String::new()));
    let seen = typed.clone();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap()))));
            use_provide_context(GlassShapes::default);
            let text = use_state(|| "notes.md".to_owned());
            *seen.borrow_mut() = text.read().clone();
            rect().expanded().child(TextField::new(text, "Name").editing())
        },
        (400., 200.).into(),
        |_| {},
        1.,
    );
    for _ in 0..4 {
        test.sync_and_update();
    }
    for c in "ideas.md".chars() {
        test.press_key(Key::Character(c.to_string()));
        test.sync_and_update();
    }
    for _ in 0..4 {
        test.sync_and_update();
    }
    assert_eq!(*typed.borrow(), "ideas.md");
}

#[test]
fn an_editing_field_in_a_virtual_list_takes_every_key() {
    let typed = Rc::new(std::cell::RefCell::new(String::new()));
    let seen = typed.clone();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap()))));
            use_provide_context(GlassShapes::default);
            let text = use_state(|| "notes.md".to_owned());
            // The parent reads the text, so each key redraws the list around the field.
            *seen.borrow_mut() = text.read().clone();
            let controller = use_scroll_controller(ScrollConfig::default);
            VirtualScrollView::new_with_data_controlled(
                text,
                |item: VirtualItem, text: &State<String>| {
                    if item.index == 1 {
                        rect().key(1).child(TextField::new(*text, "Name").editing()).into_element()
                    } else {
                        rect().key(item.index).height(Size::px(28.)).into_element()
                    }
                },
                controller,
            )
            .length(3usize)
            .item_size(28.)
            .expanded()
            .into_element()
        },
        (400., 200.).into(),
        |_| {},
        1.,
    );
    for _ in 0..4 {
        test.sync_and_update();
    }
    for c in "ideas.md".chars() {
        test.press_key(Key::Character(c.to_string()));
        test.sync_and_update();
    }
    for _ in 0..4 {
        test.sync_and_update();
    }
    assert_eq!(*typed.borrow(), "ideas.md");
}
