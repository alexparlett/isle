//! Privacy & Security: the screen-recording and screenshot permissions portals remember.

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

impl Component for Privacy {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::Permissions, Follow::Apps]);
        let r = Rows::new(&self.at);
        let nav = Nav::get();
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
        form::group(Some("Privacy".into()), rows).into_element()
    }
}
