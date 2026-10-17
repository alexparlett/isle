//! Notifications: the lock screen, holding banners, and each app that has posted.

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

use super::Nav;
use crate::{
    nav::Location,
    services::{Follow, Services, use_follow},
    ui::{self, Rows},
};

#[derive(PartialEq)]
pub struct Notifications {
    pub at: Location,
}

/// An app's rules in a few words: "Temporary banners, sound, badges" or "Off".
pub fn summary(r: &Rows, key: &str) -> String {
    let field = |f: &str| format!("notification-app[{key}].{f}");
    if !r.bool_or(&field("allow"), true) {
        return "Off".into();
    }
    let style = r
        .str(&field("banner-style"))
        .unwrap_or_else(|| "temporary".into());
    let mut words = vec![if style == "persistent" {
        "Persistent banners"
    } else {
        "Temporary banners"
    }];
    if r.bool_or(&field("sound"), true) {
        words.push("sound");
    }
    if r.bool_or(&field("badges"), true) {
        words.push("badges");
    }
    words.join(", ")
}

impl Component for Notifications {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[Follow::NotifyApps, Follow::Apps]);
        if let Some(key) = self.at.sub.as_deref().and_then(|s| s.strip_prefix("app/")) {
            return AppPage {
                at: self.at.clone(),
                key: key.to_owned(),
            }
            .into_element();
        }
        let r = Rows::new(&self.at);
        let nav = Nav::get();
        let mut apps = services.notify_apps.read().state.clone();
        apps.sort_by_key(|(_, name)| name.to_lowercase());
        let rows: Vec<Element> = apps
            .into_iter()
            .map(|(key, name)| {
                let icon = services.app(&key).and_then(|a| a.icon);
                let at = self.at.clone();
                let sub = format!("app/{key}");
                r.row(&key, name, None)
                    .leading(ui::app_icon(icon, 24.))
                    .description(summary(&r, &key))
                    .opens(None, move |_| super::open(nav, &at, &sub))
                    .into_element()
            })
            .collect();
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(
                None,
                vec![
                    r.choice(
                        "lock-screen",
                        "On the lock screen",
                        "notifications.lock-screen",
                        &[
                            ("none", "Don't show"),
                            ("count", "Show count"),
                            ("previews", "Show previews"),
                        ],
                    )
                    .into_element(),
                    r.switch(
                        "hold-full-screen",
                        "Hold banners while a window is full screen",
                        "notifications.hold-full-screen",
                    )
                    .description("Banners still show on a screen without one.")
                    .into_element(),
                    r.switch(
                        "hold-sharing",
                        "Hold banners while sharing your screen",
                        "notifications.hold-sharing",
                    )
                    .into_element(),
                ],
            ))
            .maybe(!rows.is_empty(), |el| {
                el.child(form::group(Some("Application notifications".into()), rows))
            })
            .into_element()
    }
}

#[derive(PartialEq)]
struct AppPage {
    at: Location,
    key: String,
}

impl Component for AppPage {
    fn render(&self) -> impl IntoElement {
        let services = Services::get();
        let r = Rows::new(&self.at);
        let key = self.key.clone();
        let field = |f: &str| format!("notification-app[{key}].{f}");
        let name = services
            .notify_apps
            .read()
            .state
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| key.clone());
        let icon = services.app(&key).and_then(|a| a.icon);
        let allowed = r.bool_or(&field("allow"), true);
        let style_key = field("banner-style");
        let style = r.str(&style_key).unwrap_or_else(|| "temporary".into());
        let desktop = r.desktop;
        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(ui::app_header(
                icon,
                &name,
                &format!("How {name}'s notifications reach you."),
            ))
            .child(form::group(
                None,
                vec![
                    r.switch_key("allow", "Allow notifications", field("allow"), true, true)
                        .into_element(),
                    r.row("banner-style", "Banner style", Some(&style_key))
                        .enabled(allowed)
                        .control(
                            form::Segmented::new(
                                vec!["Temporary".into(), "Persistent".into()],
                                usize::from(style == "persistent"),
                                "Banner style",
                                move |i: usize| {
                                    desktop.set(
                                        style_key.clone(),
                                        Value::Str(
                                            if i == 1 { "persistent" } else { "temporary" }.into(),
                                        ),
                                    )
                                },
                            )
                            .enabled(allowed),
                        )
                        .into_element(),
                    r.switch_key("sound", "Play sound", field("sound"), true, allowed)
                        .into_element(),
                    r.switch_key("badges", "Show badges", field("badges"), true, allowed)
                        .into_element(),
                    r.switch_key(
                        "over-full-screen",
                        "Show over full-screen windows",
                        field("over-full-screen"),
                        false,
                        allowed,
                    )
                    .into_element(),
                ],
            ))
    }
}
