//! The image a drag of items carries (`p1/surfaces/files.md` § Drag and drop).

use freya::prelude::*;
use isle_fs::Entry;
use isle_ui::{colour, snapshot::snapshot, text};

use crate::{list_view, view::View};

thread_local! {
    /// The items a drag from any of Files' windows carries, while it lasts.
    static DRAGGED: std::cell::RefCell<Vec<std::path::PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn set_dragged(items: Vec<std::path::PathBuf>) {
    DRAGGED.set(items);
}

pub fn dragged() -> Vec<std::path::PathBuf> {
    DRAGGED.with_borrow(Clone::clone)
}

/// A row's title is cut to this width in List and Columns.
const TITLE: f32 = 200.;
/// Items drawn one by one up to this many; past it, the preview's stack.
const SHOWN: usize = 3;

/// How the image is laid out, in logical pixels.
struct Layout {
    view: View,
    icon: f32,
    /// One item's box: a row, or an Icons cell.
    item: (f32, f32),
    /// The badge's room above and after the items.
    room: f32,
}

impl Layout {
    fn new(view: View, icon_size: f32, t: &isle_tokens::Tokens) -> Self {
        let room = t.size_badge / 2.;
        match view {
            View::Icons => {
                let (_, line, _) = text::metrics(&isle_ui::theme(), text::Style::Body);
                Layout {
                    view,
                    icon: icon_size,
                    item: (icon_size + 2. * t.space_l, icon_size + t.space_xs + line),
                    room,
                }
            }
            View::List | View::Columns => Layout {
                view,
                icon: t.size_app_icon_small,
                item: (
                    t.size_app_icon_small + t.space_s + TITLE + 2. * t.space_s,
                    t.size_row,
                ),
                room,
            },
        }
    }

    /// The items' area for `count` items.
    fn items(&self, count: usize, t: &isle_tokens::Tokens) -> (f32, f32) {
        if count > SHOWN {
            let side = self.stack_icon(t) + 2. * t.space_s;
            return (side, side);
        }
        let n = count.max(1) as f32;
        match self.view {
            View::Icons => (self.item.0 * n, self.item.1),
            View::List | View::Columns => (self.item.0, self.item.1 * n),
        }
    }

    fn stack_icon(&self, t: &isle_tokens::Tokens) -> f32 {
        match self.view {
            View::Icons => self.icon,
            View::List | View::Columns => t.size_app_icon,
        }
    }
}

/// The image for dragging `entries`, the pressed one first, at the window's `scale`.
pub fn drag_image(
    entries: Vec<Entry>,
    view: View,
    icon_size: f32,
    dark: bool,
    scale: f64,
) -> Option<isle_app::data::DragImage> {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let layout = Layout::new(view, icon_size, t);
    let (items_w, items_h) = layout.items(entries.len(), t);
    let size = (items_w + layout.room, items_h + layout.room);
    // The pointer holds the first item's icon at its middle.
    let hold = match (entries.len() > SHOWN, view) {
        (true, _) => (
            t.space_s + layout.stack_icon(t) / 2.,
            layout.room + t.space_s + layout.stack_icon(t) / 2.,
        ),
        (false, View::Icons) => (layout.item.0 / 2., layout.room + layout.icon / 2.),
        (false, _) => (
            t.space_s + layout.icon / 2.,
            layout.room + layout.item.1 / 2.,
        ),
    };
    let buffer_scale = scale.ceil().max(1.);
    let shot = snapshot(size, buffer_scale, move || image(&entries, &layout, dark))?;
    Some(isle_app::data::DragImage {
        width: shot.width,
        height: shot.height,
        scale: buffer_scale as i32,
        hold: (hold.0.round() as i32, hold.1.round() as i32),
        pixels: shot.pixels,
    })
}

fn image(entries: &[Entry], layout: &Layout, dark: bool) -> Element {
    let theme = isle_ui::theme();
    let t = &theme.tokens;
    let items: Element = if entries.len() > SHOWN {
        let icon = layout.stack_icon(t);
        let stack = entries.iter().take(SHOWN).enumerate().rev().map(|(i, e)| {
            rect()
                .position(
                    Position::new_absolute()
                        .left(i as f32 * t.space_s)
                        .top(i as f32 * t.space_s),
                )
                .opacity([1., 0.8, 0.6][i])
                .child(list_view::icon(e, dark, icon, t.colour_text_2))
                .into_element()
        });
        let side = icon + 2. * t.space_s;
        rect()
            .width(Size::px(side))
            .height(Size::px(side))
            .children(stack)
            .into_element()
    } else {
        let one = |e: &Entry| match layout.view {
            View::Icons => rect()
                .width(Size::px(layout.item.0))
                .height(Size::px(layout.item.1))
                .cross_align(Alignment::Center)
                .spacing(t.space_xs)
                .child(list_view::icon(e, dark, layout.icon, t.colour_text_2))
                .child(
                    text::MiddleCut::new(
                        crate::prefs::display_name(e),
                        text::Style::Body,
                        colour(t.colour_text),
                    )
                    .width(Size::px(layout.item.0)),
                )
                .into_element(),
            View::List | View::Columns => rect()
                .width(Size::px(layout.item.0))
                .height(Size::px(layout.item.1))
                .padding((0., t.space_s))
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t.space_s)
                .child(list_view::icon(e, dark, layout.icon, t.colour_text_2))
                .child(
                    text::MiddleCut::new(
                        crate::prefs::display_name(e),
                        text::Style::Body,
                        colour(t.colour_text),
                    )
                    .width(Size::px(TITLE)),
                )
                .into_element(),
        };
        let row = rect().maybe(layout.view == View::Icons, |el| el.horizontal());
        row.children(entries.iter().map(one)).into_element()
    };
    let badge = (entries.len() >= 2).then(|| {
        let (_, line, _) = text::metrics(&theme, text::Style::Subheadline);
        // On the top trailing corner of the stack or the cells; on the first row's icon in a list.
        let at = match (entries.len() > SHOWN, layout.view) {
            (false, View::List | View::Columns) => Position::new_absolute()
                .left(t.space_s + layout.icon - t.size_badge / 2.)
                .top(0.),
            _ => Position::new_absolute().right(0.).top(0.),
        };
        rect()
            .position(at)
            .min_width(Size::px(t.size_badge))
            .height(Size::px(t.size_badge.max(line)))
            .padding((0., 5.))
            .center()
            .corner_radius(t.size_badge)
            .background(colour(t.colour_accent))
            .child(
                text::line(
                    &theme,
                    text::Style::Subheadline,
                    colour(t.colour_on_accent),
                    entries.len().to_string(),
                )
                .font_weight(FontWeight::SEMI_BOLD),
            )
    });
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(
            rect()
                .padding((layout.room, layout.room, 0., 0.))
                .opacity(0.7)
                .child(items),
        )
        .maybe_child(badge)
        .into_element()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use freya_testing::TestingRunner;
    use isle_tokens::{Appearance, Source};
    use isle_ui::ThemeContext;

    use super::*;

    /// The image for `n` files in `view`, made inside a window as a drag start makes it.
    fn image_for(n: usize, view: View) -> isle_app::data::DragImage {
        let dir = tempfile::tempdir().unwrap();
        let entries: Vec<Entry> = (0..n)
            .map(|i| {
                let path = dir.path().join(format!("file {i}.txt"));
                std::fs::write(&path, "x").unwrap();
                Entry::at(&path).unwrap()
            })
            .collect();
        let made = Rc::new(RefCell::new(None));
        let out = made.clone();
        let (mut test, _) = TestingRunner::new(
            move || {
                use_provide_context(|| {
                    ThemeContext(State::create(Rc::new(
                        Source::shipped().theme(&Appearance::default()).unwrap(),
                    )))
                });
                let (out, entries) = (out.clone(), entries.clone());
                use_hook(move || *out.borrow_mut() = drag_image(entries, view, 64., false, 2.));
                rect().into_element()
            },
            (100., 100.).into(),
            |_| {},
            1.,
        );
        test.sync_and_update();
        made.borrow_mut().take().expect("an image")
    }

    fn alpha(image: &isle_app::data::DragImage, x: u32, y: u32) -> u8 {
        image.pixels[((y * image.width + x) * 4 + 3) as usize]
    }

    #[test]
    fn one_row_is_its_icon_and_title_at_seventy_percent_without_a_badge() {
        let image = image_for(1, View::List);
        assert_eq!(image.scale, 2);
        assert_eq!(
            image.pixels.len(),
            (image.width * image.height * 4) as usize
        );
        let most = image.pixels.chunks(4).map(|p| p[3]).max().unwrap();
        assert!(
            most > 0 && most <= 180,
            "the row drawn at 70%: alpha at most {most}"
        );
        assert_eq!(
            alpha(&image, image.width - 2, 1),
            0,
            "no badge for one item"
        );
        let (hx, hy) = image.hold;
        assert!(
            hx > 0 && hy > 0 && (hx as u32) < image.width / 2 && (hy as u32) < image.height / 2
        );
    }

    #[test]
    fn several_rows_carry_the_count_badge_in_full_on_the_first_icon() {
        let image = image_for(2, View::List);
        let opaque: Vec<u32> = (0..image.width)
            .filter(|&x| (0..20).any(|y| alpha(&image, x, y) == 255))
            .collect();
        assert!(
            !opaque.is_empty(),
            "the badge is drawn opaque along the top"
        );
        assert!(
            opaque.iter().all(|&x| x < image.width / 3),
            "the badge sits by the icon, not the title's end: {opaque:?}"
        );
    }

    #[test]
    fn past_three_the_items_are_a_stack() {
        let three = image_for(3, View::Icons);
        let five = image_for(5, View::Icons);
        assert!(
            five.width < three.width,
            "a stack is narrower than three cells"
        );
    }
}
