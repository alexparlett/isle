//! New login's sheet and the delete alert's words (`p1/surfaces/keychain.md` Sheets and
//! alerts).

use freya::prelude::*;
use isle_secrets::generator::Recipe;
use isle_tokens::Theme;
use isle_ui::{TextField, colour, form, text};

use crate::{
    edit::{self, Fields},
    model::{Detail, Item, Source},
};

fn row(theme: &Theme, label_text: &str, control: Element) -> Element {
    edit::row_inset(theme, label_text, control, 0.)
}

pub struct NewLogin<'a> {
    pub fields: Fields,
    /// The unlocked collections, the default first.
    pub sources: &'a [Source],
    pub save_in: usize,
    pub generator: Option<(Recipe, String)>,
    pub on_save_in: EventHandler<usize>,
    pub on_generator: EventHandler<edit::Act>,
    pub on_cancel: EventHandler<()>,
    pub on_save: EventHandler<()>,
}

pub fn new_login(theme: &Theme, s: NewLogin<'_>) -> Element {
    let t = &theme.tokens;
    let body = vec![
        text::line(theme, text::Style::Headline, colour(t.colour_text), "New Login").into_element(),
        rect()
            .width(Size::fill())
            .spacing(t.space_xs)
            .child(row(theme, "Title", TextField::new(s.fields.title, "Title").into_element()))
            .child(row(theme, "User name", TextField::new(s.fields.user, "User name").into_element()))
            .child(edit::password_row(theme, 0., s.fields.password, s.generator, s.on_generator))
            .child(row(theme, "Website", TextField::new(s.fields.website, "Website or server").placeholder("example.com").into_element()))
            .child(row(theme, "Notes", TextField::new(s.fields.notes, "Notes").into_element()))
            .child(row(
                theme,
                "Save in",
                form::PopUp::new(s.sources.iter().map(|c| c.label.clone()).collect(), s.save_in, "Save in", s.on_save_in)
                    .into_element(),
            ))
            .into_element(),
    ];
    isle_ui::sheet(
        t.size_form * 0.8,
        body,
        vec![("Cancel".into(), s.on_cancel), ("Save".into(), s.on_save)],
    )
}

/// The delete alert's informative text: what stops working, or that it can't be undone.
pub fn delete_words(item: &Item) -> String {
    match &item.detail {
        Detail::Secret { stored_by: Some(app), storage_key: true, .. } => {
            format!("{app} will lose the passwords and sign-ins it keeps.")
        }
        Detail::Secret { stored_by: Some(app), .. } if item.kind == isle_secrets::kinds::Kind::App => {
            format!("{app} will ask you to sign in again.")
        }
        _ => "You can't undo this.".into(),
    }
}
