//! A popover headless: it hangs below its anchor, keeps a press inside, and closes on one
//! outside or on Escape.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{GlassShapes, Popover, ThemeContext, text};

fn settle(test: &mut TestingRunner) {
    for _ in 0..4 {
        test.sync_and_update();
    }
}

fn centre(test: &TestingRunner, caption: &str) -> Option<(f64, f64)> {
    test.find(|node, element| {
        Label::try_downcast(element)
            .filter(|label| label.text == caption)
            .map(|_| {
                let c = node.layout().area.center();
                (f64::from(c.x), f64::from(c.y))
            })
    })
}

fn mount() -> TestingRunner {
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            let theme = isle_ui::use_theme();
            let mut open = use_state(|| false);
            let anchor = rect()
                .padding(8.)
                .on_press(move |_| open.toggle())
                .child(text::line(&theme, text::Style::Body, Color::BLACK, "Options"));
            rect()
                .expanded()
                .padding(100.)
                .child(
                    Popover::new(anchor, *open.read(), move |_| open.set(false))
                        .alt("View Options")
                        .child(text::line(&theme, text::Style::Body, Color::BLACK, "Sort By")),
                )
        },
        (900., 700.).into(),
        |_| {},
        1.,
    );
    settle(&mut test);
    test
}

#[test]
fn a_popover_opens_below_its_anchor_and_closes_outside_it() {
    let mut test = mount();
    assert!(centre(&test, "Sort By").is_none(), "closed at rest");

    let anchor = centre(&test, "Options").unwrap();
    test.click_cursor(anchor);
    settle(&mut test);
    let inside = centre(&test, "Sort By").expect("open after the anchor's press");
    assert!(inside.1 > anchor.1, "hangs below its anchor: {inside:?} under {anchor:?}");

    test.click_cursor(inside);
    settle(&mut test);
    assert!(centre(&test, "Sort By").is_some(), "a press inside keeps it");

    test.click_cursor((800., 600.));
    settle(&mut test);
    assert!(centre(&test, "Sort By").is_none(), "a press outside closes it");

    test.click_cursor(anchor);
    settle(&mut test);
    test.press_key(Key::Named(NamedKey::Escape));
    settle(&mut test);
    assert!(centre(&test, "Sort By").is_none(), "Escape closes it");
}
