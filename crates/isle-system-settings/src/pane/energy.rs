//! Energy: the power profile, sleep, wake and the power button, the apps that keep the
//! display on, and what keeps the computer awake now (`p20h-power.md` choices 15–19).

use freya::prelude::*;
use isle_settings::Value;
use isle_ui::form;

use crate::{
    nav::Location,
    services::{Act, Follow, use_follow},
    ui::{self, AppPicker, ListEditor, MoreMenu, Rows},
};

const PROFILES: [(&str, &str); 3] = [
    ("power-saver", "Low Power"),
    ("balanced", "Automatic"),
    ("performance", "High Power"),
];

const POWER_BUTTON: [(&str, &str); 3] = [
    ("session-dialog", "Show the Session Dialog"),
    ("sleep", "Sleep"),
    ("lock", "Lock Screen"),
];

const DISPLAY_APPS: &str = "energy.display-apps";

/// Holds Isle takes itself, which no app's list can change.
const OWN: [&str; 3] = ["Keep Awake", "logind", "Screen Sharing"];

/// One line of the holds group: an app, whether it holds the display, and its reason.
#[derive(Clone, Debug, PartialEq)]
struct Line {
    app: String,
    display: bool,
    why: String,
}

/// The holds as the group lists them: one line per app, holding the display if any of its
/// holds does, in the order they came.
fn lines(holds: &[isle_ipc::Hold]) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    for hold in holds {
        let display = hold.what.split(':').any(|w| w == "idle");
        match out.iter_mut().find(|l| l.app == hold.app) {
            Some(line) => {
                line.display |= display;
                if line.why.is_empty() {
                    line.why = hold.why.clone();
                }
            }
            None => out.push(Line {
                app: hold.app.clone(),
                display,
                why: hold.why.clone(),
            }),
        }
    }
    out
}

#[derive(PartialEq)]
pub struct Energy {
    pub at: Location,
}

impl Component for Energy {
    fn render(&self) -> impl IntoElement {
        let services = use_follow(&[
            Follow::Profiles,
            Follow::Network,
            Follow::Holds,
            Follow::Apps,
        ]);
        let r = Rows::new(&self.at);
        let desktop = r.desktop;
        let mut adding = use_state(|| false);

        let mut rows: Vec<Element> = Vec::new();
        let profiles = services.profiles.read().clone();
        if profiles.available {
            let profiles = profiles.state;
            let selected = PROFILES
                .iter()
                .position(|(p, _)| *p == profiles.active)
                .unwrap_or(1);
            let held = profiles.holds.iter().any(|h| h.profile == "performance");
            let mut row = r
                .row("energy-mode", "Energy Mode", None)
                .control(form::Segmented::new(
                    PROFILES.iter().map(|(_, l)| (*l).to_owned()).collect(),
                    selected,
                    "Energy Mode",
                    move |i: usize| services.act(Act::Profile(PROFILES[i].0.into())),
                ));
            if held {
                row = row.description("High Power while Game Mode is on.");
            }
            rows.push(row.into_element());
        }
        rows.push(
            r.switch(
                "prevent-sleep",
                "Prevent automatic sleeping when the display is off",
                "energy.prevent-sleep",
            )
            .into_element(),
        );
        let network = services.network.read().clone();
        let capable = network.available && network.state.wake.capable;
        let mut wake = r.switch_key(
            "wake-for-network",
            "Wake for network access",
            "energy.wake-for-network".into(),
            false,
            capable,
        );
        if network.available && !capable {
            wake = wake.description("This computer's network devices can't wake it.");
        }
        rows.push(wake.into_element());
        rows.push(
            r.choice(
                "power-button",
                "When the power button is pressed",
                "energy.power-button",
                &POWER_BUTTON,
            )
            .into_element(),
        );

        let listed = r.list(DISPLAY_APPS);
        let label = |id: &String| match services.app(id) {
            Some(app) => (app.name, Some(app.icon)),
            None => (id.clone(), Some(None)),
        };
        let remove = {
            let listed = listed.clone();
            EventHandler::new(move |i: usize| {
                let mut next = listed.clone();
                if i < next.len() {
                    next.remove(i);
                }
                desktop.set(DISPLAY_APPS, Value::List(next));
            })
        };

        let held = lines(&services.holds.read().state);
        let hold_rows: Vec<Element> = if held.is_empty() {
            vec![r.row("holds", "Nothing", None).into_element()]
        } else {
            held.into_iter()
                .map(|line| {
                    let app = services.app(&line.app);
                    let name = app.as_ref().map_or(line.app.clone(), |a| a.name.clone());
                    let icon = app.and_then(|a| a.icon);
                    let on_list = listed.iter().any(|id| id.eq_ignore_ascii_case(&line.app));
                    let mut status = if line.display || on_list {
                        "Keeps the display on".to_owned()
                    } else {
                        "Keeps the computer running".to_owned()
                    };
                    if !line.why.is_empty() {
                        status.push_str(" · ");
                        status.push_str(&line.why);
                    }
                    let mut row = r
                        .row(&format!("holds/{}", line.app), name.clone(), None)
                        .leading(ui::app_icon(icon, 20.))
                        .description(status);
                    if !OWN.contains(&line.app.as_str()) && (on_list || !line.display) {
                        let toggle = if on_list {
                            format!("Let the Display Turn Off for {name}")
                        } else {
                            format!("Keep the Display On for {name}")
                        };
                        let listed = listed.clone();
                        let id = line.app.clone();
                        row = row.control(MoreMenu {
                            alt: format!("{name} options"),
                            items: vec![(toggle, false), ("Show in Monitor".into(), false)],
                            on_choose: EventHandler::new(move |i: usize| {
                                if i == 1 {
                                    services.act(Act::ShowMonitor);
                                    return;
                                }
                                let mut next = listed.clone();
                                if on_list {
                                    next.retain(|a| !a.eq_ignore_ascii_case(&id));
                                } else {
                                    next.push(id.clone());
                                }
                                desktop.set(DISPLAY_APPS, Value::List(next));
                            }),
                        });
                    }
                    row.into_element()
                })
                .collect()
        };

        rect()
            .width(Size::fill())
            .spacing(20.)
            .child(form::group(None, rows))
            .child(ListEditor {
                on_open: None,
                on_move: None,
                title: Some("Apps that keep the display on".into()),
                items: listed.iter().map(label).collect(),
                empty: "No apps".into(),
                on_add: EventHandler::new(move |_| adding.set(true)),
                on_remove: remove,
            })
            .child(form::group(
                Some("Keeping this computer awake".into()),
                hold_rows,
            ))
            .maybe(*adding.read(), |el| {
                el.child(AppPicker {
                    title: "Keep the display on while this app keeps the computer awake".into(),
                    on_pick: EventHandler::new(move |id: String| {
                        let mut next = match desktop.value(DISPLAY_APPS) {
                            Some(Value::List(l)) => l,
                            _ => Vec::new(),
                        };
                        if !next.contains(&id) {
                            next.push(id);
                        }
                        desktop.set(DISPLAY_APPS, Value::List(next));
                        adding.set(false);
                    }),
                    on_cancel: EventHandler::new(move |_| adding.set(false)),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold(app: &str, what: &str, why: &str) -> isle_ipc::Hold {
        isle_ipc::Hold {
            pid: 0,
            app: app.into(),
            what: what.into(),
            why: why.into(),
        }
    }

    #[test]
    fn an_apps_holds_are_one_line() {
        let lines = lines(&[
            hold("a", "sleep", ""),
            hold("a", "idle", "Video"),
            hold("b", "sleep", "Audio"),
        ]);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].display && lines[0].why == "Video");
        assert!(!lines[1].display);
    }
}
