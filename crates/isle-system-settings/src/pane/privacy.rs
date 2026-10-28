//! Privacy & Security: the permissions portals remember, and every provider's grants.

use std::collections::BTreeSet;

use freya::prelude::*;
use isle_ui::form;

use super::Nav;
use crate::{
    nav::Location,
    services::{Act, Follow, Permission, use_follow},
    ui::{self, Rows},
};

/// Each kind's table, the row and page names, and the page's sentence.
const KINDS: [(&str, &str, &str); 2] = [
    (
        "screencast",
        "Screen & System Audio Recording",
        "Apps that may share your screen again without asking.",
    ),
    (
        "screenshot",
        "Screenshots",
        "Apps that may take screenshots without asking.",
    ),
];

fn count(n: usize) -> String {
    match n {
        0 => "None".into(),
        1 => "1 app".into(),
        n => format!("{n} apps"),
    }
}

#[derive(PartialEq)]
pub struct Privacy {
    pub at: Location,
}

/// "Widget", for a provider's kind on the list.
fn kind_name(_: &isle_providers::installed::Installed) -> &'static str {
    "Widget"
}

/// The source's own name: its repository's.
fn source_name(url: &str) -> String {
    url.trim_end_matches('/').trim_end_matches(".git").rsplit('/').next().unwrap_or(url).to_owned()
}

/// One provider's page: what it runs, its source and that source's switch, and each
/// permission it declares as a switch (`structure/widgets.md` Grants).
fn provider_page(r: &Rows, p: &isle_providers::installed::Installed) -> Element {
    use isle_settings::Value;
    let id = p.manifest.id.clone();
    let key = format!("provider[{id}].granted");
    let granted = r.list(&key);
    let source = match &p.source {
        Some(url) => format!("{} · from {}", kind_name(p), source_name(url)),
        None => format!("{} · your own", kind_name(p)),
    };
    let mut about = vec![r.info("runs", "What it runs", p.manifest.command()).into_element()];
    if let Some(url) = &p.source {
        about.push(r.info("source", "Source", url.clone()).into_element());
        about.push(
            r.switch_key(
                "source-allowed",
                "Allow widgets from this source to run",
                format!("widget-source[{url}].allowed"),
                false,
                true,
            )
            .into_element(),
        );
    }
    let desktop = r.desktop;
    let rows: Vec<Element> = p
        .manifest
        .declared()
        .into_iter()
        .map(|g| {
            let on = granted.contains(&g.key());
            let (key, list, grant) = (key.clone(), granted.clone(), g.key());
            let row_id = grant.clone();
            r.switch_with(&row_id, &g.words(), on, move |on| {
                let mut list: Vec<String> = list.iter().filter(|k| **k != grant).cloned().collect();
                if on {
                    list.push(grant.clone());
                }
                desktop.set(key.clone(), Value::List(list));
            })
            .leading(isle_ui::Glyph::new(g.glyph(), 16., isle_ui::colour(isle_ui::theme().tokens.colour_text_2)).into_element())
            .into_element()
        })
        .collect();
    rect()
        .width(Size::fill())
        .spacing(20.)
        .child(ui::sub_header("plug", &p.manifest.name, &source))
        .child(form::group(None, about))
        .child(if rows.is_empty() {
            ui::missing("It asks for nothing")
        } else {
            form::group(Some("Permissions".into()), rows).into_element()
        })
        .into_element()
}

impl Component for Privacy {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Permissions, Follow::Apps]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let providers = isle_providers::installed::scan();
        if let Some(id) = self.at.sub.as_deref().and_then(|s| s.strip_prefix("provider:")) {
            return match providers.get(id) {
                Some(p) => provider_page(&r, p),
                None => ui::missing("That provider isn't installed"),
            };
        }
        if self.at.sub.as_deref() == Some("providers") {
            let rows: Vec<Element> = providers
                .values()
                .map(|p| {
                    let (at, sub) = (self.at.clone(), format!("provider:{}", p.manifest.id));
                    r.row(&sub, p.manifest.name.clone(), None)
                        .opens(Some(kind_name(p).to_owned()), move |_| super::open(nav, &at, &sub))
                        .into_element()
                })
                .collect();
            return rect()
                .width(Size::fill())
                .spacing(20.)
                .child(ui::sub_header("plug", "Providers", "What each provider may reach, one switch for each."))
                .child(if rows.is_empty() {
                    ui::missing("No providers")
                } else {
                    form::group(None, rows).into_element()
                })
                .into_element();
        }
        let revoked = use_state(BTreeSet::<(String, String)>::new);
        let all = services.permissions.read().state.clone();
        if let Some(kind) = KINDS
            .iter()
            .find(|(table, ..)| self.at.sub.as_deref() == Some(*table))
        {
            let (table, name, sentence) = *kind;
            let mut apps: Vec<String> = all
                .iter()
                .filter(|p| p.table == table)
                .map(|p| p.app.clone())
                .collect();
            for (t, app) in revoked.read().iter() {
                if t == table && !apps.contains(app) {
                    apps.push(app.clone());
                }
            }
            let rows: Vec<Element> = apps
                .iter()
                .map(|app| {
                    let known = services.app(app);
                    let label = known.as_ref().map_or(app.clone(), |a| a.name.clone());
                    let on = !revoked.read().contains(&(table.to_owned(), app.clone()));
                    let id = app.clone();
                    r.switch_with(app, &label, on, move |on| {
                        let app = id.clone();
                        if !on {
                            let mut revoked = revoked;
                            revoked.write().insert((table.to_owned(), app.clone()));
                            services.act(Act::Revoke(Permission {
                                table: table.into(),
                                app: app.clone(),
                            }));
                        }
                    })
                    .leading(ui::app_icon(known.and_then(|a| a.icon), 24.))
                    .into_element()
                })
                .collect();
            return rect()
                .width(Size::fill())
                .spacing(20.)
                .child(ui::sub_header("hand-palm", name, sentence))
                .child(if rows.is_empty() {
                    ui::missing("No apps")
                } else {
                    form::group(None, rows).into_element()
                })
                .into_element();
        }
        let rows: Vec<Element> = KINDS
            .iter()
            .map(|(table, name, _)| {
                let n = all.iter().filter(|p| p.table == *table).count();
                let at = self.at.clone();
                r.row(table, *name, None)
                    .opens(Some(count(n)), move |_| super::open(nav, &at, table))
                    .into_element()
            })
            .collect();
        let n = providers.len();
        let at = self.at.clone();
        let providers_row = r
            .row("providers", "Providers", None)
            .opens(Some(count(n).replace("app", "provider")), move |_| super::open(nav, &at, "providers"))
            .into_element();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(Some("Privacy".into()), rows))
            .child(form::group(None, vec![providers_row]))
            .into_element()
    }
}
