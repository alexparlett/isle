//! The front app's menu items, from the menu bar's index (`menu-bar.md` Menu items in
//! Spotlight).

use crate::{Context, Group, Icon, Item, Query, Row, Scope, Source};

#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    /// The labels from the title down, the item's last.
    pub path: Vec<String>,
    /// Its chord as the menu shows it.
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct MenusSource {
    /// The app whose menus these are.
    pub app: String,
    /// Its enabled items.
    pub items: Vec<MenuItem>,
}

impl Source for MenusSource {
    fn group(&self) -> Group {
        Group::MenuItems
    }

    fn answers(&self, scope: &Scope) -> bool {
        matches!(scope, Scope::General | Scope::Actions)
    }

    fn search(&self, query: &Query, cx: &mut Context<'_>) -> Vec<Row> {
        if query.text.trim().is_empty() {
            return Vec::new();
        }
        let rows = self
            .items
            .iter()
            .filter_map(|m| {
                let (label, above) = m.path.split_last()?;
                let key = format!("menu:{}", m.path.join("/"));
                super::row(cx, query, key, Group::MenuItems, label, &[], false, || {
                    (
                        Some(above.join(" › ")),
                        m.hint.clone(),
                        Icon::App(self.app.clone()),
                        Item::MenuItem(m.path.clone()),
                    )
                })
            })
            .collect();
        super::sorted(rows)
    }
}
