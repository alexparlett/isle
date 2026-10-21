//! Files' settings window: General, Sidebar and Advanced, each row bound to its key in Files'
//! own section (`conventions.md`, An app's own settings).

use freya::prelude::*;
use isle_app::{Desktop, ToolbarExt};
use isle_settings::Value;
use isle_ui::{Glyph, colour, form, text, use_theme};

use crate::prefs::{self, Control, PANES};

/// The pane shown and the row a link asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct At {
    pub pane: String,
    pub row: Option<String>,
}

impl At {
    /// `isle-settings://<pane>[/<row>]`, as Spotlight's rows link.
    pub fn from_uri(uri: &str) -> Option<Self> {
        let rest = uri.strip_prefix("isle-settings://")?;
        let (pane, row) = match rest.split_once('/') {
            Some((pane, row)) => (pane, Some(row.to_owned()).filter(|r| !r.is_empty())),
            None => (rest, None),
        };
        PANES.iter().any(|p| p.id == pane).then(|| At { pane: pane.to_owned(), row })
    }
}

#[derive(Clone)]
pub struct SettingsWindow {
    pub at: State<At>,
}

impl App for SettingsWindow {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();
        let t = &theme.tokens;
        let desktop = Desktop::get();
        let mut at = self.at;
        let shown = at.read().clone();
        let pane = PANES.iter().find(|p| p.id == shown.pane).unwrap_or(&PANES[0]);
        let tabs = PANES.iter().map(|p| {
            let chosen = p.id == pane.id;
            let id = p.id;
            rect()
                .width(Size::px(76.))
                .height(Size::px(t.size_toolbar - 2. * t.space_xs))
                .center()
                .spacing(t.space_xxs)
                .corner_radius(t.radius_control)
                .maybe(chosen, |el| el.background(colour(t.colour_control_pressed)))
                .a11y_role(AccessibilityRole::Tab)
                .a11y_alt(p.name)
                .a11y_builder(move |node| node.set_selected(chosen))
                .on_press(move |e: Event<PressEventData>| {
                    e.stop_propagation();
                    at.set(At { pane: id.to_owned(), row: None });
                })
                .child(Glyph::new(p.glyph, t.size_glyph_large, colour(if chosen { t.colour_accent } else { t.colour_text_2 })))
                .child(text::line(&theme, text::Style::Callout, colour(if chosen { t.colour_text } else { t.colour_text_2 }), p.name))
                .into_element()
        });
        let toolbar = rect()
            .width(Size::fill())
            .height(Size::px(t.size_toolbar))
            .horizontal()
            .center()
            .spacing(t.space_xs)
            .background(colour(t.colour_window))
            .border(
                Border::new()
                    .fill(colour(t.colour_hairline))
                    .width(BorderWidth { top: 0., right: 0., bottom: 1., left: 0. })
                    .alignment(BorderAlignment::Inner),
            )
            .toolbar()
            .a11y_role(AccessibilityRole::TabList)
            .children(tabs);

        let refused = desktop.refused.read().clone();
        let rows: Vec<Element> = pane
            .rows
            .iter()
            .map(|row| {
                let key = prefs::key(row.key);
                let found = shown.row.as_deref() == Some(row.id);
                let why = refused.as_ref().filter(|(k, _)| *k == key).map(|(_, w)| w.clone());
                let control = match row.control {
                    Control::Switch(default) => {
                        let on = match desktop.value(&key) {
                            Some(Value::Bool(b)) => b,
                            _ => default,
                        };
                        let key = key.clone();
                        isle_ui::Switch::new(on, row.label, move |next: bool| desktop.set(key.clone(), Value::Bool(next)))
                            .into_element()
                    }
                    Control::Choice(options) => {
                        let now = match desktop.value(&key) {
                            Some(Value::Str(s)) => s,
                            _ => String::new(),
                        };
                        let at = options.iter().position(|(v, _)| *v == now).unwrap_or(if row.key == "search-scope" { 1 } else { 0 });
                        let key = key.clone();
                        form::PopUp::new(options.iter().map(|(_, l)| l.to_string()).collect(), at, row.label, move |i: usize| {
                            desktop.set(key.clone(), Value::Str(options[i].0.to_owned()))
                        })
                        .into_element()
                    }
                };
                form::Row::new(row.label).control(control).found(found).refused(why).into_element()
            })
            .collect();
        let broken = desktop.status.read().clone().filter(|s| !s.ok);
        rect()
            .expanded()
            .content(Content::flex())
            .background(colour(t.colour_window))
            .child(toolbar)
            .child(
                isle_ui::scroll_area(&theme).width(Size::fill()).height(Size::flex(1.)).child(
                    rect()
                        .width(Size::fill())
                        .padding(t.space_l)
                        .spacing(t.space_m)
                        .maybe_child(broken.map(|s| form::Notice {
                            message: "settings.kdl has an error, so changes here are not saved.".into(),
                            detail: format!("Line {}: {}", s.line, s.message),
                            actions: Vec::new(),
                        }))
                        .child(form::group(None, rows)),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_open_a_known_pane_at_a_row() {
        assert_eq!(
            At::from_uri("isle-settings://advanced/show-extensions"),
            Some(At { pane: "advanced".into(), row: Some("show-extensions".into()) })
        );
        assert_eq!(At::from_uri("isle-settings://sidebar"), Some(At { pane: "sidebar".into(), row: None }));
        assert_eq!(At::from_uri("isle-settings://wi-fi"), None);
        assert_eq!(At::from_uri("file:///home"), None);
    }
}
