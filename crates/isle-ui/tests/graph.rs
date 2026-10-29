//! The graph drawn headless: a rate's scale shows, and the hover reads the nearest sample.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Colour, Source};
use isle_ui::{
    GlassShapes, ThemeContext,
    graph::{Graph, GraphData, Series, Shape, Unit},
};

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
}

#[test]
fn a_rate_shows_its_scale_and_the_hover_reads_the_nearest_sample() {
    let (mut test, _) = TestingRunner::new(
        || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(Source::shipped().theme(&Appearance::default()).unwrap())))
            });
            use_provide_context(GlassShapes::default);
            let data = GraphData {
                series: vec![Series {
                    values: vec![Some(1_000.0), Some(30_000_000.0), Some(2_000_000.0)],
                    colour: Colour::parse("#1ca2c3").unwrap(),
                }],
                shape: Shape::Lines,
                max: None,
                unit: Unit::BytesPerSecond,
                times: vec!["10:00:00".into(), "10:00:02".into(), "10:00:04".into()],
            };
            rect()
                .padding(40.)
                .child(Graph { data: Rc::new(data), width: 240., name: "Network".into(), summary: "Network".into() })
                .into_element()
        },
        (400., 200.).into(),
        |_| {},
        1.,
    );
    for _ in 0..4 {
        test.sync_and_update();
    }
    assert!(labels(&test).iter().any(|l| l == "50 MB/s"), "{:?}", labels(&test));
    // The well runs from x 40 to 280; the newest sample is at its trailing edge, the one before
    // a step of 4 points back.
    test.move_cursor((40. + 240. - 4., 80.));
    for _ in 0..4 {
        test.sync_and_update();
    }
    assert!(labels(&test).iter().any(|l| l.contains("30 MB/s") && l.contains("10:00:02")), "{:?}", labels(&test));
}
