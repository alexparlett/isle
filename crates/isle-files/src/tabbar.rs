//! The tab bar (`p1/surfaces/files.md` § Tab bar).

use freya::prelude::*;
use isle_ui::{Glyph, colour, text, use_theme};

#[derive(PartialEq)]
pub(crate) struct TabBar {
    pub(crate) titles: Vec<String>,
    pub(crate) current: usize,
    pub(crate) on_select: EventHandler<usize>,
    pub(crate) on_close: EventHandler<usize>,
    pub(crate) on_new: EventHandler<()>,
    /// A secondary click on a tab, at a point in the window.
    pub(crate) on_menu: EventHandler<(usize, (f32, f32))>,
    /// A drag is over the window: a tab under it is a drop target and springs open.
    pub(crate) dragging: bool,
    /// The tab under the drag, ringed.
    pub(crate) drag_over: Option<usize>,
    /// The drag entered a tab, or left them, with none.
    pub(crate) on_drag: EventHandler<Option<usize>>,
}

impl Component for TabBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut hover = use_state(|| None::<usize>);
        let mut room = use_state(|| 0f32);
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        // Where the bar was last scrolled to, read without following the scroll.
        let mut start = use_state(|| 0f32);
        // Equal widths down to the minimum; past it the bar scrolls.
        let count = self.titles.len().max(1);
        let width = (*room.read() / count as f32).max(t.size_tab_min);
        let current = self.current;
        let shown_room = *room.read();
        use_side_effect_with_deps(&(current, width, shown_room), move |&(current, width, room)| {
            if room <= 0. {
                return;
            }
            let at = *start.peek();
            let (left, right) = (current as f32 * width, (current + 1) as f32 * width);
            let to = if left < at {
                left
            } else if right > at + room {
                right - room
            } else {
                return;
            };
            start.set(to);
            scroll.scroll_to_x(-to as i32);
        });
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_tab_bar))
            .horizontal()
            .content(Content::flex())
            .background(colour(t.colour_window_sunken))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .a11y_role(AccessibilityRole::TabList)
            .child(
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .on_sized(move |e: Event<SizedEventData>| room.set_if_modified(e.area.width()))
                    .child(
                        ScrollView::new_controlled(scroll)
                            .direction(Direction::Horizontal)
                            .show_scrollbar(false)
                            .expanded()
                            .children(self.titles.iter().enumerate().map(|(i, title)| {
                                let selected = i == self.current;
                                let (on_select, on_close) = (self.on_select.clone(), self.on_close.clone());
                                let closing = self.on_close.clone();
                            let on_menu = self.on_menu.clone();
                            let (drag_enter, drag_leave, dragging) = (self.on_drag.clone(), self.on_drag.clone(), self.dragging);
                            let ringed = self.drag_over == Some(i);
                                rect()
                                    .key(i)
                                    .width(Size::px(width))
                                    .height(Size::fill())
                                    .center()
                                    .background(colour(if selected { t.colour_view } else { t.colour_window_sunken }))
                                    .a11y_role(AccessibilityRole::Tab)
                                    .a11y_alt(title.clone())
                                    .a11y_builder(move |node| node.set_selected(selected))
                                    .maybe(ringed, |el| el.border(crate::interact::drop_ring(&theme)))
                                    .on_pointer_enter(move |_| {
                                        hover.set(Some(i));
                                        if dragging {
                                            drag_enter.call(Some(i));
                                        }
                                    })
                                    .on_pointer_leave(move |_| {
                                        hover.set_if_modified(None);
                                        if dragging {
                                            drag_leave.call(None);
                                        }
                                    })
                                    .on_press(move |_| on_select.call(i))
                                    .on_mouse_down(move |e: Event<MouseEventData>| {
                                        if e.button == Some(MouseButton::Middle) {
                                            on_close.call(i);
                                        } else if e.button == Some(MouseButton::Right) {
                                            on_menu.call((i, (e.global_location.x as f32, e.global_location.y as f32)));
                                        }
                                    })
                                    .maybe_child((*hover.read() == Some(i)).then(|| {
                                        rect()
                                            .position(Position::new_absolute().left(t.space_xs).top(7.))
                                            .width(Size::px(16.))
                                            .height(Size::px(16.))
                                            .center()
                                            .corner_radius(t.radius_small)
                                            .a11y_role(AccessibilityRole::Button)
                                            .a11y_alt("Close Tab")
                                            .on_press(move |e: Event<PressEventData>| {
                                                e.stop_propagation();
                                                closing.call(i);
                                            })
                                            .child(Glyph::new("x", 12., colour(t.colour_text_2)))
                                    }))
                                    .child(text::line(
                                        &theme,
                                        text::Style::Callout,
                                        colour(if selected { t.colour_text } else { t.colour_text_2 }),
                                        // Cut in the middle within the tab, clear of its close button.
                                        text::cut_middle(title, ((width - 40.) / (text::metrics(&theme, text::Style::Callout).0 * 0.56)).max(4.) as usize),
                                    ))
                                    .into_element()
                            })),
                    ),
            )
            .child({
                let on_new = self.on_new.clone();
                rect()
                    .width(Size::px(t.size_tab_bar))
                    .height(Size::fill())
                    .center()
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt("New Tab")
                    .on_press(move |_| on_new.call(()))
                    .child(Glyph::new("plus", 16., colour(t.colour_text_2)))
            })
    }
}
