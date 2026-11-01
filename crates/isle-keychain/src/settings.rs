//! Keychain's two settings, in its own section of `settings.kdl` (choice 24), and the
//! settings window of one pane (`conventions.md`, An app's own settings).

use std::time::Duration;

use freya::prelude::*;
use isle_app::Desktop;
use isle_settings::Value;
use isle_ui::{colour, form, use_theme};

pub const APP: &str = "io.github.alexparlett.Isle.Keychain";

pub fn key(name: &str) -> String {
    format!("app[{APP}].{name}")
}

const CLEAR: [(i64, &str); 4] = [
    (30, "30 seconds"),
    (60, "1 minute"),
    (120, "2 minutes"),
    (300, "5 minutes"),
];
const HIDE: [(i64, &str); 3] = [(30, "30 seconds"), (60, "1 minute"), (300, "5 minutes")];

fn seconds(desktop: &Desktop, name: &str) -> i64 {
    match desktop.value(&key(name)) {
        Some(Value::Int(s)) if s > 0 => s,
        _ => 60,
    }
}

/// How long a copied password stays on the clipboard.
pub fn clear_after(desktop: &Desktop) -> Duration {
    Duration::from_secs(seconds(desktop, "clear-copied-after") as u64)
}

/// How long a shown password stays shown.
pub fn hide_after(desktop: &Desktop) -> Duration {
    Duration::from_secs(seconds(desktop, "hide-shown-after") as u64)
}

fn row(
    desktop: Desktop,
    name: &'static str,
    label: &'static str,
    options: &'static [(i64, &'static str)],
) -> Element {
    let now = seconds(&desktop, name);
    let at = options.iter().position(|(s, _)| *s == now).unwrap_or(1);
    let refused = desktop
        .refused
        .read()
        .clone()
        .filter(|(k, _)| *k == key(name))
        .map(|(_, why)| why);
    form::Row::new(label)
        .control(form::PopUp::new(
            options.iter().map(|(_, l)| (*l).to_owned()).collect(),
            at,
            label,
            move |i: usize| desktop.set(key(name), Value::Int(options[i].0)),
        ))
        .refused(refused)
        .into_element()
}

#[derive(Clone, PartialEq)]
pub struct SettingsWindow;

impl App for SettingsWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        rect()
            .expanded()
            .background(colour(t.colour_window))
            .padding((t.size_toolbar, t.space_l, t.space_l, t.space_l))
            .child(form::group(
                None,
                vec![
                    row(
                        desktop,
                        "clear-copied-after",
                        "Clear copied passwords after",
                        &CLEAR,
                    ),
                    row(
                        desktop,
                        "hide-shown-after",
                        "Hide shown passwords after",
                        &HIDE,
                    ),
                ],
            ))
    }
}
