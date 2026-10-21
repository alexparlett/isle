//! Settings' rows from the index installed with Settings, and apps' own settings rows in the
//! same form (`apps.md` Settings 4; `conventions.md`, An app's own settings).

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

#[derive(Clone, Debug, PartialEq)]
pub struct SettingRow {
    pub pane: String,
    pub row: String,
    pub label: String,
    pub pane_label: String,
    pub also: Vec<String>,
    /// The app whose own settings the row is in, by id and name; none for System Settings.
    pub app: Option<(String, String)>,
}

/// The index's rows: `row pane=… id=… label=… pane-label=… also=…`, with `app=… app-name=…`
/// for an app's own settings.
pub fn parse(text: &str) -> Vec<SettingRow> {
    let Ok(doc) = text.parse::<kdl::KdlDocument>() else {
        return Vec::new();
    };
    doc.nodes()
        .iter()
        .filter(|n| n.name().value() == "row")
        .filter_map(|n| {
            let get = |key: &str| n.get(key).and_then(|v| v.as_string()).map(str::to_owned);
            Some(SettingRow {
                pane: get("pane")?,
                row: get("id")?,
                label: get("label")?,
                pane_label: get("pane-label").unwrap_or_default(),
                also: super::words(&get("also").unwrap_or_default()),
                app: get("app").map(|id| (id, get("app-name").unwrap_or_default())),
            })
        })
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct SettingsSource {
    pub rows: Vec<SettingRow>,
}

impl Source for SettingsSource {
    fn group(&self) -> Group {
        Group::Settings
    }

    fn answers(&self, scope: &Scope) -> bool {
        *scope == Scope::General
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        if query.text.trim().is_empty() {
            return Vec::new();
        }
        let rows = self
            .rows
            .iter()
            .filter_map(|r| {
                let also: Vec<&str> = r.also.iter().map(String::as_str).collect();
                let owner = r.app.as_ref().map_or("", |(id, _)| id.as_str());
                let key = format!("setting:{owner}:{}/{}", r.pane, r.row);
                let settings = match &r.app {
                    Some((_, name)) => format!("{name} Settings"),
                    None => "Settings".to_owned(),
                };
                super::row(
                    cx,
                    query,
                    key,
                    Group::Settings,
                    &r.label,
                    &also,
                    false,
                    || {
                        (
                            Some(if r.row.is_empty() {
                                settings.clone()
                            } else {
                                format!("{settings} › {}", r.pane_label)
                            }),
                            None,
                            Icon::Glyph("gear-six"),
                            Item::Setting {
                                app: r.app.as_ref().map(|(id, _)| id.clone()),
                                pane: r.pane.clone(),
                                row: r.row.clone(),
                            },
                        )
                    },
                )
            })
            .collect();
        super::sorted(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_parses_and_the_shipped_one_names_every_pane() {
        let rows = parse(
            r#"row pane="displays" id="scale" label="Scale" pane-label="Displays" also="zoom;size""#,
        );
        assert_eq!(rows[0].label, "Scale");
        assert_eq!(rows[0].also, ["zoom", "size"]);
        assert_eq!(rows[0].app, None);
        let own = parse(r#"row app="a.Files" app-name="Files" pane="general" id="" label="General" pane-label="General""#);
        assert_eq!(own[0].app, Some(("a.Files".into(), "Files".into())));
        let shipped = parse(include_str!("../../../../data/settings/index.kdl"));
        assert!(shipped.iter().any(|r| r.pane == "displays" && r.row.is_empty() && r.label == "Displays"));
    }
}
