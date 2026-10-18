//! The slider drawn headless: its handle over its fill at any value.

use std::rc::Rc;

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{GlassShapes, Slider, ThemeContext};

/// The pixel at `(x, y)` of the slider drawn at `value` on a track 218 points wide.
fn pixel(value: f32, x: u32, y: u32) -> [u8; 3] {
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            use_provide_context(GlassShapes::default);
            rect()
                .width(Size::px(218.))
                .child(Slider::new(value, "Level", |_: f32| {}))
                .into_element()
        },
        (218., 40.).into(),
        |_| {},
        1.,
    );
    for _ in 0..4 {
        test.sync_and_update();
    }
    let png = test.render();
    let decoder = png::Decoder::new(std::io::Cursor::new(png.as_bytes().to_vec()));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).unwrap();
    let channels = info.color_type.samples();
    let at = (y as usize * info.width as usize + x as usize) * channels;
    [buffer[at], buffer[at + 1], buffer[at + 2]]
}

#[test]
fn the_handle_is_over_the_fill() {
    let theme = Source::shipped().theme(&Appearance::default()).unwrap();
    let middle = (theme.tokens.size_control / 2.0) as u32;
    // The knob's centre runs from 9 to 209 points along the track.
    for value in [0.25, 0.5, 0.75, 1.0] {
        let x = (9.0 + value * 200.0) as u32;
        // Left of the knob's centre, where the fill would be.
        assert_eq!(
            pixel(value, x - 4, middle),
            [255, 255, 255],
            "value {value}"
        );
    }
}
