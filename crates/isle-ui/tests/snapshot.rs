//! An element drawn to pixels off any window: its size at the scale, its colours, a clear
//! ground around it, and a glyph drawn by the first frame.

use std::{cell::RefCell, rc::Rc};

use freya::prelude::*;
use freya_testing::TestingRunner;
use isle_tokens::{Appearance, Source};
use isle_ui::{Glyph, ThemeContext, snapshot::snapshot};

/// The pixel at `(x, y)` as B, G, R, A.
fn at(shot: &isle_ui::snapshot::Snapshot, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * shot.width + x) * 4) as usize;
    shot.pixels[i..i + 4].try_into().unwrap()
}

fn shot_in_window(
    size: (f32, f32),
    scale: f64,
    element: fn() -> Element,
) -> isle_ui::snapshot::Snapshot {
    let taken = Rc::new(RefCell::new(None));
    let out = taken.clone();
    let (mut test, _) = TestingRunner::new(
        move || {
            use_provide_context(|| {
                ThemeContext(State::create(Rc::new(
                    Source::shipped().theme(&Appearance::default()).unwrap(),
                )))
            });
            let out = out.clone();
            use_hook(move || *out.borrow_mut() = snapshot(size, scale, element));
            rect().into_element()
        },
        (100., 100.).into(),
        |_| {},
        1.,
    );
    test.sync_and_update();
    taken.borrow_mut().take().expect("a snapshot")
}

#[test]
fn a_block_fills_its_size_at_the_scale_on_a_clear_ground() {
    let shot = shot_in_window((40., 20.), 2., || {
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .padding(5.)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background((255, 0, 0)),
            )
            .into_element()
    });
    assert_eq!((shot.width, shot.height), (80, 40));
    assert_eq!(
        at(&shot, 40, 20),
        [0, 0, 255, 255],
        "the red block, as B, G, R, A"
    );
    assert_eq!(at(&shot, 2, 2)[3], 0, "clear outside the block");
}

#[test]
fn a_glyph_is_drawn_by_the_first_frame() {
    let shot = shot_in_window((32., 32.), 1., || {
        Glyph::new("folder-simple", 32., Color::BLACK).into_element()
    });
    let inked = shot.pixels.chunks(4).filter(|p| p[3] > 0).count();
    assert!(inked > 20, "{inked} pixels inked");
}
