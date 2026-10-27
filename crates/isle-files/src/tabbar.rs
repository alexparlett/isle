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
}

impl Component for TabBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut hover = use_state(|| None::<usize>);
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
                // Equal widths down to the minimum; past it the tabs are cut at the bar's end.
                rect()
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .horizontal()
                    .content(Content::flex())
                    .overflow(Overflow::Clip)
                    .children(self.titles.iter().enumerate().map(|(i, title)| {
                        let selected = i == self.current;
                        let (on_select, on_close) = (self.on_select.clone(), self.on_close.clone());
                        let closing = self.on_close.clone();
                        rect()
                            .key(i)
                            .width(Size::flex(1.))
                            .min_width(Size::px(t.size_tab_min))
                            .height(Size::fill())
                            .center()
                            .background(colour(if selected { t.colour_view } else { t.colour_window_sunken }))
                            .a11y_role(AccessibilityRole::Tab)
                            .a11y_alt(title.clone())
                            .a11y_builder(move |node| node.set_selected(selected))
                            .on_pointer_enter(move |_| hover.set(Some(i)))
                            .on_pointer_leave(move |_| hover.set_if_modified(None))
                            .on_press(move |_| on_select.call(i))
                            .on_mouse_down(move |e: Event<MouseEventData>| {
                                if e.button == Some(MouseButton::Middle) {
                                    on_close.call(i);
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
                                title.clone(),
                            ))
                            .into_element()
                    })),
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
