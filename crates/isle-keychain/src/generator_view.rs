//! The generator's controls, as a sheet from the Item menu and as a popover from a password
//! field (`p1/surfaces/keychain.md` The generator).

use freya::prelude::*;
use isle_secrets::generator::{self, Recipe};
use isle_tokens::Theme;
use isle_ui::{Button, Switch, colour, form, text};

/// The well, the length and the four sets; `large` for the sheet.
pub fn controls(theme: &Theme, recipe: Recipe, password: &str, large: bool, on_recipe: EventHandler<Recipe>) -> Vec<Element> {
    let t = &theme.tokens;
    let size = if large { t.type_style_title_2_size } else { t.type_style_title_3_size };
    let well = rect()
        .width(Size::fill())
        .background(colour(t.colour_window_sunken))
        .corner_radius(t.radius_control)
        .padding(t.space_m)
        .center()
        .child(
            label()
                .width(Size::fill())
                .text(password.to_owned())
                .font_family("JetBrains Mono")
                .font_size(size)
                .color(colour(t.colour_text))
                .text_align(TextAlign::Center)
                .max_lines(if large { 3 } else { 2 })
                .text_overflow(TextOverflow::Ellipsis),
        );
    let span = (generator::MAX - generator::MIN) as f32;
    let share = (recipe.length.clamp(generator::MIN, generator::MAX) - generator::MIN) as f32 / span;
    let (slide, step) = (on_recipe.clone(), on_recipe.clone());
    let length = rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(text::line(theme, text::Style::Body, colour(t.colour_text), "Length"))
        .child(
            rect().width(Size::flex(1.)).child(
                isle_ui::Slider::new(share, "Length", move |v: f32| {
                    let length = generator::MIN + (v * span).round() as usize;
                    slide.call(Recipe { length, ..recipe });
                })
                .step(1. / span),
            ),
        )
        .child(form::Stepper::new(recipe.length as i64, (generator::MIN as i64, generator::MAX as i64), "Length", move |n: i64| {
            step.call(Recipe { length: n as usize, ..recipe })
        }));
    let on = [recipe.upper, recipe.lower, recipe.digits, recipe.symbols];
    let left_on = on.iter().filter(|o| **o).count();
    let sets = ["Upper case", "Lower case", "Digits", "Symbols"].into_iter().enumerate().map(|(i, name)| {
        let on_recipe = on_recipe.clone();
        // The last set left on cannot be turned off, so a password always has characters.
        let last = on[i] && left_on == 1;
        rect()
            .width(Size::fill())
            .height(Size::px(t.size_row))
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .child(rect().width(Size::flex(1.)).child(text::line(theme, text::Style::Body, colour(t.colour_text), name)))
            .child(
                Switch::new(on[i], name, move |next: bool| {
                    let mut r = recipe;
                    match i {
                        0 => r.upper = next,
                        1 => r.lower = next,
                        2 => r.digits = next,
                        _ => r.symbols = next,
                    }
                    on_recipe.call(r);
                })
                .enabled(!last),
            )
            .into_element()
    });
    let mut out = vec![well.into_element(), length.into_element()];
    out.extend(sets);
    out
}

/// The popover's body in a password field: the controls, Generate again and Use.
pub fn popover(theme: &Theme, recipe: Recipe, password: &str, on_recipe: EventHandler<Recipe>, on_again: EventHandler<()>, on_use: EventHandler<()>) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::px(t.size_generator))
        .spacing(t.space_s)
        .children(controls(theme, recipe, password, false, on_recipe))
        .child(
            rect()
                .width(Size::fill())
                .padding((t.space_s, 0., 0., 0.))
                .horizontal()
                .content(Content::flex())
                .child(Button::new("Generate again").on_press(move |_| on_again.call(())))
                .child(rect().width(Size::flex(1.)))
                .child(Button::new("Use").kind(isle_ui::button::Kind::Default).on_press(move |_| on_use.call(()))),
        )
        .into_element()
}
