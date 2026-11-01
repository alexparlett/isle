//! The SSH key sheets: the passphrase to add a key to the agent, and New SSH key with its
//! indeterminate bar while generating (`p1/surfaces/keychain.md` Sheets and alerts).

use freya::{animation::*, prelude::*};
use isle_tokens::Theme;
use isle_ui::{Field, FieldText, TextField, colour, form, text, use_theme};

use crate::edit;

/// `conventions.md`'s indeterminate bar: a segment sweeping a track, for work with no progress
/// to show.
#[derive(Clone, PartialEq)]
pub struct IndeterminateBar;

impl Component for IndeterminateBar {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let mut width = use_state(|| 0f32);
        let sweep = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0.0, 1.0).time(1200).function(Function::Sine).ease(Ease::InOut)
        });
        let w = *width.read();
        let segment = w * 0.3;
        let at = sweep.get().value() * (w + segment) - segment;
        rect()
            .width(Size::fill())
            .height(Size::px(6.))
            .corner_radius(3.)
            .background(colour(t.colour_fill_strong))
            .overflow(Overflow::Clip)
            .a11y_role(AccessibilityRole::ProgressIndicator)
            .a11y_alt("Generating")
            .on_sized(move |e: Event<SizedEventData>| width.set_if_modified(e.area.width()))
            .child(
                rect()
                    .position(Position::new_absolute().left(at).top(0.))
                    .width(Size::px(segment))
                    .height(Size::fill())
                    .corner_radius(3.)
                    .background(colour(t.colour_accent)),
            )
    }
}

fn row(theme: &Theme, label_text: &str, control: Element) -> Element {
    edit::row_inset(theme, label_text, control, 0.)
}

pub fn passphrase(
    theme: &Theme,
    key: &str,
    text_field: FieldText,
    wrong: bool,
    on_cancel: EventHandler<()>,
    on_add: EventHandler<()>,
) -> Element {
    let t = &theme.tokens;
    let submit = on_add.clone();
    isle_ui::sheet(
        400.,
        vec![
            text::line(theme, text::Style::Headline, colour(t.colour_text), format!("Enter the passphrase for “{key}”")).into_element(),
            Field::new(text_field, "Passphrase")
                .secret(true)
                .error(wrong)
                .auto_focus(true)
                .on_submit(move |_| submit.call(()))
                .into_element(),
        ],
        vec![("Cancel".into(), on_cancel), ("Add".into(), on_add)],
    )
}

pub struct NewKey {
    pub name: State<String>,
    pub kind: usize,
    pub comment: State<String>,
    pub passphrase: FieldText,
    pub confirm: FieldText,
    pub generating: bool,
    pub error: Option<String>,
    pub on_kind: EventHandler<usize>,
    pub on_cancel: EventHandler<()>,
    pub on_save: EventHandler<()>,
}

pub fn new_key(theme: &Theme, k: NewKey) -> Element {
    let t = &theme.tokens;
    let mut body = vec![
        text::line(theme, text::Style::Headline, colour(t.colour_text), "New SSH Key").into_element(),
        rect()
            .width(Size::fill())
            .spacing(t.space_xs)
            .child(row(theme, "Name", TextField::new(k.name, "Name").enabled(!k.generating).into_element()))
            .child(row(
                theme,
                "Type",
                form::PopUp::new(vec!["Ed25519".into(), "RSA".into()], k.kind, "Type", k.on_kind)
                    .enabled(!k.generating)
                    .into_element(),
            ))
            .child(row(theme, "Comment", TextField::new(k.comment, "Comment").enabled(!k.generating).into_element()))
            .child(row(theme, "Passphrase", Field::new(k.passphrase, "Passphrase").secret(true).enabled(!k.generating).into_element()))
            .child(row(theme, "Confirm", Field::new(k.confirm, "Confirm passphrase").secret(true).enabled(!k.generating).into_element()))
            .into_element(),
    ];
    if k.generating {
        body.push(IndeterminateBar.into_element());
    }
    if let Some(error) = k.error {
        body.push(text::line(theme, text::Style::Subheadline, colour(t.colour_danger_text), error).into_element());
    }
    let save = if k.generating {
        EventHandler::new(|_| {})
    } else {
        k.on_save
    };
    isle_ui::sheet(t.size_form * 0.8, body, vec![("Cancel".into(), k.on_cancel), ("Save".into(), save)])
}
