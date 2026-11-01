//! Edit in place: the detail's values as fields, Cancel and Done, the generator's popover from
//! the password field, and Delete… under the groups (`p1/surfaces/keychain.md` Edit).

use std::rc::Rc;

use freya::prelude::*;
use isle_secrets::{generator::Recipe, kinds::Kind};
use isle_tokens::Theme;
use isle_ui::{Button, Glyph, TextField, colour, scroll_area, text};
use zeroize::Zeroizing;

use crate::{list::item_tile, model::Item, store::LoginFields};

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Cancel,
    Done,
    Delete,
    Generator(bool),
    Recipe(Recipe),
    Again,
    Use,
}

/// What Edit started from, to tell whether anything changed.
#[derive(Clone)]
pub struct Editing {
    pub id: crate::model::ItemId,
    /// A login's fields change; an Apps item's title and secret only.
    pub login: bool,
    pub original: LoginFields,
    pub password: Rc<Zeroizing<String>>,
}

impl PartialEq for Editing {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// The fields' states, owned by the window.
#[derive(Clone, Copy, PartialEq)]
pub struct Fields {
    pub title: State<String>,
    pub user: State<String>,
    pub website: State<String>,
    pub notes: State<String>,
    pub password: State<String>,
}

impl Fields {
    pub fn read(&self) -> LoginFields {
        LoginFields {
            title: self.title.peek().clone(),
            user: self.user.peek().clone(),
            website: self.website.peek().clone(),
            notes: self.notes.peek().clone(),
        }
    }

    pub fn fill(&mut self, from: &LoginFields, password: &str) {
        self.title.set(from.title.clone());
        self.user.set(from.user.clone());
        self.website.set(from.website.clone());
        self.notes.set(from.notes.clone());
        self.password.set(password.to_owned());
    }

    pub fn password(&self) -> Zeroizing<String> {
        Zeroizing::new(self.password.peek().clone())
    }

    /// Whether anything differs from where Edit started.
    pub fn changed(&self, editing: &Editing) -> bool {
        self.read() != editing.original || self.password().as_str() != editing.password.as_str()
    }
}

fn row(theme: &Theme, label_text: &str, control: Element) -> Element {
    row_inset(theme, label_text, control, theme.tokens.space_m)
}

/// A row with its label column; `inset` at the sides, none in a sheet.
pub fn row_inset(theme: &Theme, label_text: &str, control: Element, inset: f32) -> Element {
    let t = &theme.tokens;
    rect()
        .width(Size::fill())
        .height(Size::px(t.size_row_large))
        .padding((0., inset))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(
            rect()
                .width(Size::px(t.size_detail_label))
                .child(text::line(theme, text::Style::Body, colour(t.colour_text_2), label_text.to_owned())),
        )
        .child(rect().width(Size::flex(1.)).child(control))
        .into_element()
}

/// The password field with its generator button, and the generator's popover under it.
pub fn password_row(
    theme: &Theme,
    inset: f32,
    password: State<String>,
    generator: Option<(Recipe, String)>,
    on_act: EventHandler<Act>,
) -> Element {
    let t = &theme.tokens;
    let open = generator.is_some();
    let toggle = on_act.clone();
    let dice = rect()
        .width(Size::px(20.))
        .height(Size::px(20.))
        .center()
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt("Generator")
        .on_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            toggle.call(Act::Generator(!open));
        })
        .child(Glyph::new("dice-five", 16., colour(t.colour_text_2)));
    let close = on_act.clone();
    let mut popover = isle_ui::Popover::new(dice, open, move |_| close.call(Act::Generator(false))).alt("Generator");
    if let Some((recipe, password)) = generator {
        let (r, again, used) = (on_act.clone(), on_act.clone(), on_act.clone());
        popover = popover.child(crate::generator_view::popover(
            theme,
            recipe,
            &password,
            EventHandler::new(move |next| r.call(Act::Recipe(next))),
            EventHandler::new(move |_| again.call(Act::Again)),
            EventHandler::new(move |_| used.call(Act::Use)),
        ));
    }
    row_inset(
        theme,
        "Password",
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_s)
            .child(rect().width(Size::flex(1.)).child(TextField::new(password, "Password")))
            .child(popover)
            .into_element(),
        inset,
    )
}

pub struct View<'a> {
    pub item: &'a Item,
    pub fields: Fields,
    pub login: bool,
    pub generator: Option<(Recipe, String)>,
    pub on_act: EventHandler<Act>,
}

pub fn view(theme: &Theme, v: View<'_>) -> Element {
    let t = &theme.tokens;
    let on = v.on_act.clone();
    let mut rows = vec![row(theme, "Title", TextField::new(v.fields.title, "Title").into_element())];
    if v.login {
        rows.push(row(theme, "User name", TextField::new(v.fields.user, "User name").into_element()));
    }
    rows.push(password_row(theme, t.space_m, v.fields.password, v.generator.clone(), on.clone()));
    if v.login {
        rows.push(row(
            theme,
            if v.item.kind == Kind::Login { "Website" } else { "Server" },
            TextField::new(v.fields.website, "Website").into_element(),
        ));
        rows.push(row(theme, "Notes", TextField::new(v.fields.notes, "Notes").into_element()));
    }
    let (cancel, done, delete) = (on.clone(), on.clone(), on.clone());
    let header = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(item_tile(theme, v.item, t.size_app_icon_medium, 12.))
        .child(
            rect()
                .width(Size::flex(1.))
                .child(text::line(theme, text::Style::Title2, colour(t.colour_text), v.fields.title.read().clone())),
        )
        .child(Button::new("Cancel").on_press(move |_| cancel.call(Act::Cancel)))
        .child(
            Button::new("Done")
                .kind(isle_ui::button::Kind::Default)
                .on_press(move |_| done.call(Act::Done)),
        );
    scroll_area(theme)
        .width(Size::fill())
        .height(Size::fill())
        .child(
            rect()
                .width(Size::fill())
                .padding((t.space_xl, t.space_xxl))
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .width(Size::fill())
                        .max_width(Size::px(t.size_form))
                        .spacing(t.space_xl)
                        .child(header)
                        .child(isle_ui::form::group(None, rows))
                        .child(
                            rect().child(
                                Button::new("Delete…")
                                    .kind(isle_ui::button::Kind::Destructive)
                                    .on_press(move |_| delete.call(Act::Delete)),
                            ),
                        ),
                ),
        )
        .into_element()
}
