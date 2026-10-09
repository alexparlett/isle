//! Open windows by title.

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub id: u64,
    pub title: String,
    pub app: String,
    pub app_name: String,
    /// "Desktop 2", or the full-screen app's name.
    pub desktop: String,
}

#[derive(Clone, Debug, Default)]
pub struct WindowsSource {
    pub windows: Vec<Window>,
}

impl Source for WindowsSource {
    fn group(&self) -> Group {
        Group::Windows
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::General | Scope::Windows | Scope::App(_))
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        if query.scope == Scope::General && query.text.trim().is_empty() {
            return Vec::new();
        }
        let rows = self
            .windows
            .iter()
            .filter(|w| match &query.scope {
                Scope::App(app) => &w.app == app,
                _ => true,
            })
            .filter_map(|w| {
                let title = if w.title.is_empty() {
                    w.app_name.as_str()
                } else {
                    w.title.as_str()
                };
                super::row(
                    cx,
                    query,
                    format!("window:{}", w.id),
                    Group::Windows,
                    title,
                    &[&w.app_name],
                    false,
                    || {
                        (
                            Some(format!("{} · {}", w.app_name, w.desktop)),
                            None,
                            Icon::App(w.app.clone()),
                            Item::Window(w.id),
                        )
                    },
                )
            })
            .collect();
        super::sorted(rows)
    }
}
