//! Markup in Quick Look: the tool row under its toolbar and the marks drawn over the image
//! (`p1/surfaces/capture.md` § Markup).

use std::{path::PathBuf, rc::Rc};

use bytes::Bytes;
use freya::prelude::*;
use image::DynamicImage;
use isle_ui::{Glyph, Popover, colour, text, use_theme};

use crate::{
    markup::{self, Edits, Mark, Point, Rgb, TOOLS, Tool},
    window::off_thread,
};

/// The image being marked up: decoded, as drawn, and its size after its turns.
#[derive(Clone)]
struct Base {
    image: Rc<DynamicImage>,
    shown: Bytes,
    size: (f32, f32),
}

impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.image, &other.image)
    }
}

fn png(image: &DynamicImage) -> Option<Bytes> {
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(Bytes::from(out.into_inner()))
}

#[derive(PartialEq)]
pub struct MarkupView {
    pub path: PathBuf,
    /// Markup ended: true once Done wrote the file.
    pub on_end: EventHandler<bool>,
}

impl Component for MarkupView {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let palette: [(Rgb, &str); 8] = {
            let rgb = |c: isle_tokens::Colour| {
                let c = colour(c);
                (c.r(), c.g(), c.b())
            };
            [
                (rgb(t.annotation_red), "Red"),
                (rgb(t.annotation_orange), "Orange"),
                (rgb(t.annotation_yellow), "Yellow"),
                (rgb(t.annotation_green), "Green"),
                (rgb(t.annotation_blue), "Blue"),
                (rgb(t.annotation_purple), "Purple"),
                (rgb(t.annotation_black), "Black"),
                (rgb(t.annotation_white), "White"),
            ]
        };
        let mut base = use_state(|| None::<Base>);
        let mut edits = use_state(Edits::default);
        let mut tool = use_state(|| Tool::Arrow);
        let mut ink = use_state(|| palette[0].0);
        let mut picking = use_state(|| false);
        let mut drawing = use_state(|| None::<Mark>);
        let mut writing = use_state(|| None::<Point>);
        let typed = use_state(String::new);
        let mut area = use_state(|| (0f32, 0f32, 0f32, 0f32));
        let saving = use_state(|| false);
        use_hook({
            let path = self.path.clone();
            move || {
                spawn(async move {
                    let read = off_thread(move || {
                        let image = image::open(&path).ok()?;
                        let shown = Bytes::from(std::fs::read(&path).ok()?);
                        Some((image, shown))
                    });
                    if let Some(Some((image, shown))) = read.await {
                        let size = (image.width() as f32, image.height() as f32);
                        base.set(Some(Base { image: Rc::new(image), shown, size }));
                    }
                });
            }
        });

        // Where a pointer at `global` falls in the image's pixels.
        let to_image = move |gx: f64, gy: f64| -> Option<Point> {
            let (ax, ay, aw, ah) = *area.peek();
            let (w, h) = base.peek().as_ref()?.size;
            let scale = (aw / w).min(ah / h);
            let (ox, oy) = (ax + (aw - w * scale) / 2., ay + (ah - h * scale) / 2.);
            Some((((gx as f32) - ox) / scale, ((gy as f32) - oy) / scale))
        };
        let begin = move |e: Event<MouseEventData>| {
            if e.button != Some(MouseButton::Left) {
                return;
            }
            let Some(at) = to_image(e.global_location.x, e.global_location.y) else { return };
            let colour = *ink.peek();
            let started = match *tool.peek() {
                Tool::Pen | Tool::Highlighter => Some(Mark::Stroke { tool: *tool.peek(), colour, points: vec![at] }),
                Tool::Text => {
                    writing.set(Some(at));
                    None
                }
                Tool::Redact | Tool::Crop => Some(Mark::Redact { from: at, to: at }),
                shape => Some(Mark::Shape { tool: shape, colour, from: at, to: at }),
            };
            drawing.set(started);
        };
        let extend = move |e: Event<PointerEventData>| {
            if drawing.peek().is_none() {
                return;
            }
            let at = e.global_location();
            let Some(at) = to_image(at.x, at.y) else { return };
            let mut current = drawing.write();
            match current.as_mut() {
                Some(Mark::Stroke { points, .. }) => points.push(at),
                Some(Mark::Shape { to, .. } | Mark::Redact { to, .. }) => *to = at,
                _ => {}
            }
        };
        let finish = move |_: Event<PointerEventData>| {
            let Some(done) = drawing.write().take() else { return };
            let mut next = edits.peek().clone();
            match (done, *tool.peek()) {
                (Mark::Redact { from, to }, Tool::Crop) => next.crop = Some((from, to)),
                (mark, _) => next.marks.push(mark),
            }
            edits.set(next);
        };

        let preview = {
            let mut shown = edits.read().clone();
            match drawing.read().clone() {
                Some(Mark::Redact { from, to }) if *tool.read() == Tool::Crop => shown.crop = Some((from, to)),
                Some(mark) => shown.marks.push(mark),
                None => {}
            }
            shown
        };
        let rotate = move || {
            let Some(current) = base.peek().clone() else { return };
            let (image, width) = (current.image.as_ref().clone(), current.size.0);
            spawn(async move {
                let turned = off_thread(move || {
                    let turned = image.rotate270();
                    png(&turned).map(|shown| (turned, shown))
                })
                .await
                .flatten();
                if let Some((turned, shown)) = turned {
                    let size = (turned.width() as f32, turned.height() as f32);
                    base.set(Some(Base { image: Rc::new(turned), shown, size }));
                    let mut next = edits.peek().clone();
                    next.rotate_left(width);
                    edits.set(next);
                }
            });
        };
        let done = {
            let (path, on_end) = (self.path.clone(), self.on_end.clone());
            move || {
                let mut saving = saving;
                if edits.peek().is_empty() {
                    on_end.call(false);
                    return;
                }
                // Done renders from the file, the turns with the marks.
                let (all, original, on_end) = (edits.peek().clone(), path.clone(), on_end.clone());
                saving.set(true);
                spawn(async move {
                    let wrote = off_thread(move || {
                        let image = image::open(&original).map_err(std::io::Error::other)?;
                        let drawn = markup::render(&image, &all).ok_or_else(|| std::io::Error::other("not drawn"))?;
                        markup::replace(&original, &drawn)
                    })
                    .await;
                    saving.set(false);
                    match wrote {
                        Some(Ok(())) => on_end.call(true),
                        Some(Err(err)) => tracing::warn!("Markup was not saved: {err}"),
                        None => {}
                    }
                });
            }
        };

        let tool_button = |(which, glyph, name): (Tool, &'static str, &'static str)| {
            let chosen = *tool.read() == which;
            rect()
                .key(name)
                .width(Size::px(t.size_control_large))
                .height(Size::px(t.size_control_large))
                .center()
                .corner_radius(t.radius_control)
                .maybe(chosen, |el| el.background(colour(t.colour_control_pressed)))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(name)
                .a11y_builder(move |node| node.set_selected(chosen))
                .on_press(move |_| tool.set(which))
                .child(Glyph::new(glyph, t.size_glyph_large, colour(if chosen { t.colour_text } else { t.colour_text_2 })))
                .into_element()
        };
        let (r, g, b) = *ink.read();
        let swatch = rect()
            .width(Size::px(t.size_control_large))
            .height(Size::px(t.size_control_large))
            .center()
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt("Colour")
            .on_press(move |_| picking.toggle())
            .child(
                rect()
                    .width(Size::px(16.))
                    .height(Size::px(16.))
                    .corner_radius(8.)
                    .background(Color::from_rgb(r, g, b))
                    .border(Border::new().fill(colour(t.colour_hairline_strong)).width(1.).alignment(BorderAlignment::Outer)),
            );
        let swatches = palette.iter().map(|&(rgb, name)| {
            let current = *ink.read() == rgb;
            rect()
                .key(name)
                .width(Size::px(20.))
                .height(Size::px(20.))
                .corner_radius(10.)
                .background(Color::from_rgb(rgb.0, rgb.1, rgb.2))
                .maybe(current, |el| el.border(Border::new().fill(colour(t.colour_text)).width(2.).alignment(BorderAlignment::Outer)))
                .a11y_role(AccessibilityRole::Button)
                .a11y_alt(name)
                .on_press(move |_| {
                    ink.set(rgb);
                    picking.set(false);
                })
                .into_element()
        });
        let colour_picker = Popover::new(swatch, *picking.read(), move |_| picking.set(false))
            .alt("Colour")
            .child(rect().horizontal().spacing(t.space_xs).children(swatches));
        let row = rect()
            .width(Size::fill())
            .height(Size::px(t.size_row_large))
            .padding((0., t.space_l))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_xs)
            .background(colour(t.colour_titlebar))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .children(TOOLS.into_iter().map(tool_button))
            .child(rect().width(Size::px(t.space_l)))
            .child(crate::toolbar::icon_button(&theme, "arrow-counter-clockwise", "Undo", !edits.read().is_empty(), false, move || {
                let mut edits = edits;
                let mut next = edits.peek().clone();
                if next.marks.pop().is_none() {
                    next.crop = None;
                }
                edits.set(next);
            }))
            .child(crate::toolbar::icon_button(&theme, "arrow-arc-left", "Rotate Left", true, false, rotate))
            .child(rect().width(Size::px(t.space_l)))
            .child(colour_picker)
            .child(rect().width(Size::flex(1.)))
            .child(isle_ui::Button::new("Revert").enabled(!edits.read().is_empty()).on_press(move |_| edits.set(Edits::default())))
            .child(
                isle_ui::Button::new(if *saving.read() { "Saving…" } else { "Done" })
                    .kind(isle_ui::button::Kind::Default)
                    .enabled(!*saving.read())
                    .on_press(move |_| done()),
            );

        let canvas = base.read().clone().map(|current| {
            let layer = markup::svg(&preview, current.size.0, current.size.1, true);
            let key = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                layer.hash(&mut h);
                h.finish().to_string()
            };
            let fill = || rect().position(Position::new_absolute().top(0.).left(0.)).width(Size::fill()).height(Size::fill());
            rect()
                .expanded()
                .on_sized(move |e: Event<SizedEventData>| {
                    let a = e.area;
                    area.set((a.origin.x, a.origin.y, a.size.width, a.size.height));
                })
                .on_mouse_down(begin)
                .on_global_pointer_move(extend)
                .on_global_pointer_up(finish)
                .child(fill().child(ImageViewer::new(current.shown.clone()).expanded().aspect_ratio(AspectRatio::Min).image_cover(ImageCover::Center)))
                .child(fill().layer(Layer::Overlay).child(SvgViewer::new((key, Bytes::from(layer))).show_loader(false).expanded()))
                .into_element()
        });
        let text_entry = (*writing.read()).map(|at| {
            let submit = move |text: String| {
                let text = text.trim().to_owned();
                if !text.is_empty() {
                    let mut next = edits.peek().clone();
                    next.marks.push(Mark::Text { colour: *ink.peek(), at, text });
                    edits.set(next);
                }
                writing.set(None);
                let mut typed = typed;
                typed.set(String::new());
            };
            rect()
                .position(Position::new_absolute().top(t.space_l).left(t.space_l))
                .layer(Layer::Overlay)
                .padding(t.space_s)
                .corner_radius(t.radius_card)
                .background(colour(t.colour_window_raised))
                .child(text::line(&theme, text::Style::Callout, colour(t.colour_text_2), "Text"))
                .child(
                    isle_ui::TextField::new(typed, "Text")
                        .editing()
                        .width(Size::px(260.))
                        .on_submit(submit)
                        .on_escape(move |_| writing.set(None)),
                )
                .into_element()
        });
        rect()
            .expanded()
            .content(Content::flex())
            .child(row)
            .child(rect().width(Size::fill()).height(Size::flex(1.)).padding(t.space_l).child(rect().expanded().maybe_child(canvas).maybe_child(text_entry)))
    }
}
