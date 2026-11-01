//! A press on a control stays with it: the toolbar around it never sees it, so a double click
//! on a button or in a field is not the title bar's.

use std::{cell::Cell, rc::Rc};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{Button, GlassShapes, SearchField, TextField, ThemeContext, form::PopUp};

#[test]
fn a_controls_press_does_not_reach_its_toolbar() {
    let seen = Rc::new(Cell::new(0u32));
    let count = seen.clone();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            let query = use_state(|| "alpha beta".to_string());
            let name = use_state(|| "gamma delta".to_string());
            let count = count.clone();
            rect()
                .expanded()
                .horizontal()
                .padding(20.)
                .spacing(20.)
                .on_pointer_down(move |_| count.set(count.get() + 1))
                .child(Button::new("Edit"))
                .child(PopUp::new(
                    vec!["One".into(), "Two".into()],
                    0,
                    "Choice",
                    |_: usize| {},
                ))
                .child(SearchField {
                    value: query.into_writable(),
                    placeholder: "Search".into(),
                    width: 120.,
                    focus: false,
                })
                .child(TextField::new(name, "Name").width(Size::px(120.)))
        },
        (600., 100.).into(),
        |_| {},
        1.,
    );
    for _ in 0..3 {
        test.sync_and_update();
    }
    let at = |test: &TestingRunner, caption: &str| {
        test.find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text == caption)
                .map(|_| node.layout().area.center())
        })
        .unwrap_or_else(|| panic!("no {caption}"))
    };
    for caption in ["Edit", "One"] {
        let p = at(&test, caption);
        test.click_cursor((f64::from(p.x), f64::from(p.y)));
        test.click_cursor((f64::from(p.x), f64::from(p.y)));
        test.sync_and_update();
    }
    assert_eq!(
        seen.get(),
        0,
        "no press on a button or a pop-up reached the toolbar"
    );
    let double_click = |test: &mut TestingRunner, x: f32, y: f32| {
        test.click_cursor((f64::from(x), f64::from(y)));
        test.click_cursor((f64::from(x), f64::from(y)));
        test.sync_and_update();
    };
    // Each field's text, then the search field's glyph, left of its text.
    let texts = test
        .find_many(|node, element| Paragraph::try_downcast(element).map(|_| node.layout().area));
    assert_eq!(texts.len(), 2, "a search field and a text field");
    for text in &texts {
        double_click(&mut test, text.min_x() + 12., text.center().y);
    }
    assert_eq!(
        seen.get(),
        0,
        "no press on a field's text reached the toolbar"
    );
    let field = test
        .find_many(|node, element| {
            let area = node.layout().area;
            Rect::try_downcast(element)
                .filter(|_| area.width() == 120. && area.contains(texts[0].center()))
                .map(|_| area)
        })
        .into_iter()
        .next()
        .expect("the search field");
    double_click(&mut test, field.min_x() + 14., field.center().y);
    assert_eq!(
        seen.get(),
        0,
        "no press on the search field's glyph reached the toolbar"
    );
    test.click_cursor((590., 95.));
    test.sync_and_update();
    assert_eq!(seen.get(), 1, "a press on the toolbar itself does");
}
