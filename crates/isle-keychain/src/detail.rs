//! The detail: the item's header, its fields with Show and Copy, and what Keychain knows about
//! it (`p1/surfaces/keychain.md` The detail). A secret shows only as the window was given it.

use freya::prelude::*;
use isle_network::Keeps;
use isle_secrets::kinds::Kind;
use isle_tokens::Theme;
use isle_ui::{Button, colour, scroll_area, text};

use crate::{
    list::item_tile,
    model::{Detail, Item},
    store::security_words,
};

/// What the detail asks of the window.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Show,
    Hide,
    CopySecret,
    CopyUser,
    CopyPublicKey,
    OpenSite(String),
    WifiSettings,
    AddToAgent,
    RemoveFromAgent,
    Edit,
}

pub fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Login => "Login",
        Kind::WiFi => "Wi-Fi network",
        Kind::SshKey => "SSH key",
        Kind::App => "App secret",
    }
}

fn date(seconds: u64) -> String {
    if seconds == 0 {
        return "—".into();
    }
    jiff::Timestamp::from_second(seconds as i64)
        .map(|ts| {
            ts.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%-d %B %Y at %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

/// A field row: the label column, the value, and the row's buttons trailing.
fn field(
    theme: &Theme,
    label_text: &str,
    value: Element,
    buttons: Vec<Element>,
    compact: bool,
) -> Element {
    field_of(
        theme,
        label_text,
        value,
        buttons,
        compact,
        theme.appearance.text_scale > 1.3,
    )
}

fn field_of(
    theme: &Theme,
    label_text: &str,
    value: Element,
    buttons: Vec<Element>,
    compact: bool,
    wraps: bool,
) -> Element {
    let t = &theme.tokens;
    let large = theme.appearance.text_scale > 1.3;
    let label_el = text::line(
        theme,
        text::Style::Body,
        colour(t.colour_text_2),
        label_text.to_owned(),
    );
    let value_and_buttons = rect()
        .width(Size::flex(1.))
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_s)
        .child(rect().width(Size::flex(1.)).child(value))
        .children(buttons);
    let body = if large {
        rect()
            .width(Size::fill())
            .spacing(t.space_xxs)
            .child(label_el)
            .child(value_and_buttons)
    } else {
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::flex())
            .cross_align(Alignment::Center)
            .spacing(t.space_m)
            .child(rect().width(Size::px(t.size_detail_label)).child(label_el))
            .child(value_and_buttons)
    };
    let height = if compact {
        t.size_row
    } else {
        t.size_row_large
    };
    // A wrapping value sets its row's height; any other row is the token's.
    rect()
        .width(Size::fill())
        .map((!wraps).then_some(height), |el, h| el.height(Size::px(h)))
        .padding(if wraps {
            (t.space_s, t.space_m)
        } else {
            (0., t.space_m)
        })
        .main_align(Alignment::Center)
        .child(body)
        .into_element()
}

fn value(theme: &Theme, words: impl Into<String>) -> Element {
    let t = &theme.tokens;
    label()
        .width(Size::fill())
        .text(words.into())
        .font_size(t.type_style_body_size)
        .color(colour(t.colour_text))
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis)
        .into_element()
}

fn wrapping(theme: &Theme, words: impl Into<String>, mono: bool, ink: Color) -> Element {
    let t = &theme.tokens;
    label()
        .width(Size::fill())
        .text(words.into())
        .font_size(t.type_style_body_size)
        .maybe(mono, |l| l.font_family("JetBrains Mono"))
        .color(ink)
        .into_element()
}

/// Ten dots whatever the length, so the length never shows.
fn dots(theme: &Theme) -> Element {
    let t = &theme.tokens;
    label()
        .text("••••••••••")
        .font_size(t.type_style_body_size)
        .letter_spacing(t.type_style_body_size * 0.12)
        .color(colour(t.colour_text))
        .into_element()
}

fn small(label_text: &str, on: EventHandler<Act>, act: Act) -> Element {
    Button::new(label_text)
        .on_press(move |_| on.call(act.clone()))
        .into_element()
}

pub struct View<'a> {
    pub item: Option<&'a Item>,
    /// The item's secret, while shown.
    pub shown: Option<&'a str>,
    pub can_edit: bool,
    pub on_act: EventHandler<Act>,
}

pub fn view(theme: &Theme, v: View<'_>) -> Element {
    let t = &theme.tokens;
    let Some(item) = v.item else {
        return rect()
            .expanded()
            .background(colour(t.colour_window))
            .center()
            .child(text::line(
                theme,
                text::Style::Body,
                colour(t.colour_text_2),
                "No item selected",
            ))
            .into_element();
    };
    let on = v.on_act.clone();
    let secret_row = |label_text: &str, enabled: bool, note: Option<&str>| -> Element {
        let shown = v.shown.filter(|_| enabled);
        let value_el = match (note, shown) {
            (Some(n), _) => wrapping(theme, n, false, colour(t.colour_text_2)),
            (None, Some(s)) => wrapping(theme, s, true, colour(t.colour_text)),
            (None, None) => dots(theme),
        };
        let buttons = if note.is_some() {
            Vec::new()
        } else {
            vec![
                if shown.is_some() {
                    small("Hide", on.clone(), Act::Hide)
                } else {
                    small("Show", on.clone(), Act::Show)
                },
                small("Copy", on.clone(), Act::CopySecret),
            ]
        };
        let wraps = shown.is_some() || note.is_some();
        field_of(theme, label_text, value_el, buttons, false, wraps)
    };

    let mut fields: Vec<Element> = Vec::new();
    let mut about: Vec<(&str, String)> = Vec::new();
    let mut foot: Option<Element> = None;
    let mut well: Option<Element> = None;
    match &item.detail {
        Detail::Secret {
            user,
            website,
            notes,
            stored_by,
            created,
            modified,
            ..
        } => {
            if let Some(u) = user.as_ref().filter(|u| !u.is_empty()) {
                fields.push(field(
                    theme,
                    "User name",
                    value(theme, u.clone()),
                    vec![small("Copy", on.clone(), Act::CopyUser)],
                    false,
                ));
            }
            fields.push(secret_row("Password", true, None));
            if let Some(site) = website.as_ref().filter(|w| !w.is_empty()) {
                let url = if site.contains("://") {
                    site.clone()
                } else {
                    format!("https://{site}")
                };
                let open = on.clone();
                let site_label = if item.kind == Kind::Login {
                    "Website"
                } else {
                    "Server"
                };
                fields.push(field(
                    theme,
                    site_label,
                    rect()
                        .on_press(move |_| open.call(Act::OpenSite(url.clone())))
                        .a11y_role(AccessibilityRole::Link)
                        .child(text::line(
                            theme,
                            text::Style::Body,
                            colour(t.colour_accent_text),
                            site.clone(),
                        ))
                        .into_element(),
                    Vec::new(),
                    false,
                ));
            }
            if let Some(n) = notes.as_ref().filter(|n| !n.is_empty()) {
                fields.push(field_of(
                    theme,
                    "Notes",
                    wrapping(theme, n.clone(), false, colour(t.colour_text)),
                    Vec::new(),
                    false,
                    true,
                ));
            }
            about.push(("Kind", kind_name(item.kind).into()));
            if let Some(by) = stored_by {
                about.push(("Stored by", by.clone()));
            }
            about.push(("Source", item.source.clone()));
            about.push(("Created", date(*created)));
            about.push(("Modified", date(*modified)));
        }
        Detail::Wifi { saved, .. } => {
            fields.push(field(
                theme,
                "Network",
                value(theme, saved.ssid.clone()),
                Vec::new(),
                false,
            ));
            fields.push(field(
                theme,
                "Security",
                value(theme, security_words(&saved.security)),
                Vec::new(),
                false,
            ));
            if let Some(identity) = &saved.identity {
                fields.push(field(
                    theme,
                    "User name",
                    value(theme, identity.clone()),
                    vec![small("Copy", on.clone(), Act::CopyUser)],
                    false,
                ));
            }
            match saved.keeps {
                Keeps::System | Keeps::Agent => fields.push(secret_row("Password", true, None)),
                Keeps::NotSaved => fields.push(secret_row(
                    "Password",
                    false,
                    Some("Asked for each time; nothing is saved"),
                )),
                Keeps::Nothing => {}
            }
            about.push(("Kind", kind_name(item.kind).into()));
            about.push((
                "Source",
                match saved.keeps {
                    Keeps::Agent => "Wi-Fi · Keyring".into(),
                    _ => "Wi-Fi".into(),
                },
            ));
            let open = on.clone();
            foot = Some(
                Button::new("Wi-Fi Settings…")
                    .on_press(move |_| open.call(Act::WifiSettings))
                    .into_element(),
            );
        }
        Detail::Ssh { key, in_agent } => {
            fields.push(field(
                theme,
                "Type",
                value(theme, key.kind.clone()),
                Vec::new(),
                false,
            ));
            fields.push(field_of(
                theme,
                "Fingerprint",
                wrapping(theme, key.fingerprint.clone(), true, colour(t.colour_text)),
                Vec::new(),
                false,
                true,
            ));
            if !key.comment.is_empty() {
                fields.push(field(
                    theme,
                    "Comment",
                    value(theme, key.comment.clone()),
                    Vec::new(),
                    false,
                ));
            }
            let agent_button = match (in_agent, key.private.is_some() && key.encrypted.is_some()) {
                (true, _) => vec![small("Remove from agent", on.clone(), Act::RemoveFromAgent)],
                (false, true) => vec![small("Add to agent", on.clone(), Act::AddToAgent)],
                (false, false) => Vec::new(),
            };
            fields.push(field(
                theme,
                "In agent",
                value(theme, if *in_agent { "Yes" } else { "No" }),
                agent_button,
                false,
            ));
            let copy = on.clone();
            well = Some(
                rect()
                    .width(Size::fill())
                    .spacing(t.space_s)
                    .child(text::line(
                        theme,
                        text::Style::Body,
                        colour(t.colour_text_2),
                        "Public key",
                    ))
                    .child(
                        rect()
                            .width(Size::fill())
                            .background(colour(t.colour_window_sunken))
                            .corner_radius(t.radius_control)
                            .padding(t.space_s)
                            .child(
                                label()
                                    .width(Size::fill())
                                    .text(key.public_key.clone())
                                    .font_family("JetBrains Mono")
                                    .font_size(t.type_style_subheadline_size)
                                    .color(colour(t.colour_text_2))
                                    .max_lines(3)
                                    .text_overflow(TextOverflow::Ellipsis),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .main_align(Alignment::End)
                            .horizontal()
                            .child(
                                Button::new("Copy public key")
                                    .on_press(move |_| copy.call(Act::CopyPublicKey)),
                            ),
                    )
                    .into_element(),
            );
            about.push(("Kind", kind_name(item.kind).into()));
            about.push(("Source", "~/.ssh".into()));
        }
    }

    // Wi-Fi and SSH keys are their own sources; the kind says it.
    let subtitle = match item.kind {
        Kind::WiFi | Kind::SshKey => kind_name(item.kind).to_owned(),
        _ => format!("{} · {}", kind_name(item.kind), item.source),
    };
    let edit = v.can_edit.then(|| {
        let on = on.clone();
        Button::new("Edit")
            .on_press(move |_| on.call(Act::Edit))
            .into_element()
    });
    let header = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::flex())
        .cross_align(Alignment::Center)
        .spacing(t.space_m)
        .child(item_tile(theme, item, t.size_app_icon_medium, 12.))
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t.space_xxs)
                .child(text::line(
                    theme,
                    text::Style::Title2,
                    colour(t.colour_text),
                    item.title.clone(),
                ))
                .child(text::line(
                    theme,
                    text::Style::Body,
                    colour(t.colour_text_2),
                    subtitle,
                )),
        )
        .maybe_child(edit);
    let about_rows: Vec<Element> = about
        .into_iter()
        .map(|(l, v)| field(theme, l, value(theme, v), Vec::new(), true))
        .collect();
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
                        .child(isle_ui::form::group(None, fields))
                        .maybe_child(well)
                        .child(isle_ui::form::group(None, about_rows))
                        .maybe_child(foot),
                ),
        )
        .into_element()
}
