//! Notifications on the loop: each arrival decided once, live ones with their actions, records
//! of those missed, the user's close, action and reply sent back with an activation token
//! (`p8-services.md` choices 7, 9, 10).

use isle_notifications::{Incoming, Notification, Outgoing, Reason};
use isle_state::{Domain, Loaded, StateDir};
use serde::{Deserialize, Serialize};
use smithay::wayland::xdg_activation::XdgActivationTokenData;
use tracing::{info, warn};

use crate::{
    state::{Isle, State},
    state_writer::Saver,
};

/// How many records are kept.
const RECORDS: usize = 300;

/// What became of an arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fate {
    Banner,
    /// Kept for the centre and counted for the summary.
    Held,
    /// Kept for the centre, never a banner or a sound, not counted.
    Quiet,
    /// A banner if allowed, never kept.
    Transient,
}

/// What holds banners now, each with its own reason (`p8-services.md` choice 14).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Holds {
    /// A Focus: its name, the apps it lets through, whether time-sensitive ones pass.
    pub focus: Option<(String, Vec<String>, bool)>,
    /// Game Mode is the Focus that holds.
    pub game: bool,
    pub full_screen: bool,
    pub away: bool,
    /// A screen being shared or recorded, which holds even critical ones.
    pub capture: bool,
}

impl Holds {
    pub fn any(&self) -> bool {
        self.focus.is_some() || self.full_screen || self.away || self.capture
    }
}

/// Decides an arrival once, in `structure/notifications.md`'s order.
pub fn decide(
    n: &Notification,
    app: &str,
    muted: bool,
    over_full_screen: bool,
    holds: &Holds,
) -> Fate {
    let critical = n.urgency == 2;
    let banner = |holds: &Holds| {
        if holds.capture || holds.away {
            return false;
        }
        if let Some((_, allowed, critical_passes)) = &holds.focus
            && !(allowed.iter().any(|a| a == app) || (critical && *critical_passes))
        {
            return false;
        }
        !holds.full_screen || critical || over_full_screen
    };
    if n.transient {
        return if banner(holds) {
            Fate::Transient
        } else {
            Fate::Quiet
        };
    }
    if muted || n.urgency == 0 {
        return Fate::Quiet;
    }
    if banner(holds) {
        Fate::Banner
    } else {
        Fate::Held
    }
}

/// One notification the desktop holds: live while its sender keeps it, else a record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub n: Notification,
    pub live: bool,
    #[serde(skip, default = "held_fate")]
    pub fate: Fate,
}

fn held_fate() -> Fate {
    Fate::Quiet
}

/// The records kept across restarts, and the next id, so ids are never reused.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Saved {
    pub next_id: u32,
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub last_crash: Option<String>,
}

impl Domain for Saved {
    const NAME: &'static str = "notifications";
    const VERSION: u32 = 1;
}

#[derive(Default)]
pub struct NotificationsState {
    /// Newest last.
    pub entries: Vec<Entry>,
    pub next_id: u32,
    pub server: Option<isle_notifications::Handle>,
    pub saver: Option<Saver<Saved>>,
    /// Arrivals held since silence began, for the summary.
    pub held: Vec<u32>,
    /// The desktop's own notifications' subjects, by id.
    pub desktop: std::collections::HashMap<u32, crate::desktop_notes::Note>,
    pub desktop_next: u32,
    /// The broken state of `settings.kdl` already said, as file, line and column.
    pub settings_noted: Option<String>,
    /// The crash record last said.
    pub last_crash: Option<String>,
    /// Apps whose stack is open in the centre.
    pub expanded: std::collections::HashSet<String>,
    /// Clear All asked once.
    pub clear_armed: bool,
}

impl Isle {
    /// Loads the records, every entry of a past session one.
    pub fn load_notifications(&mut self, runtime: &tokio::runtime::Handle) -> u32 {
        let dir = StateDir::new(StateDir::default_dir());
        let (mut saved, loaded) = dir.load::<Saved>();
        if !matches!(loaded, Loaded::Read | Loaded::Missing) {
            warn!(?loaded, "notification records");
        }
        for entry in &mut saved.entries {
            entry.live = false;
        }
        let next = saved
            .entries
            .iter()
            .map(|e| e.n.id + 1)
            .max()
            .unwrap_or(1)
            .max(saved.next_id);
        self.notifications.entries = saved.entries;
        self.notifications.next_id = next;
        self.notifications.last_crash = saved.last_crash;
        self.notifications.saver = Some(Saver::spawn(runtime, dir));
        next
    }

    fn save_notifications(&self) {
        if let Some(saver) = &self.notifications.saver {
            saver.save(Saved {
                next_id: self.notifications.next_id,
                entries: self
                    .notifications
                    .entries
                    .iter()
                    .filter(|e| e.fate != Fate::Transient)
                    .cloned()
                    .collect(),
                last_crash: self.notifications.last_crash.clone(),
            });
        }
    }

    fn send(&self, outgoing: Outgoing) {
        if let Some(server) = &self.notifications.server {
            let _ = server.outgoing.send(outgoing);
        }
    }

    /// The app a notification is from: its desktop entry, else its sender's window's app,
    /// else its name.
    pub fn notification_app(&self, n: &Notification) -> String {
        if let Some(entry) = &n.desktop_entry {
            return entry.clone();
        }
        let (name, _) = self.stream_app(n.pid, &n.app_name);
        self.wm
            .records
            .iter()
            .find(|r| self.app_name(&r.app) == name)
            .map(|r| r.app.clone())
            .unwrap_or(name)
    }

    /// The Focus that holds banners, its allowed apps and whether time-sensitive ones pass.
    pub fn focus_hold(&self) -> Option<(String, Vec<String>, bool)> {
        self.focus_hold_now()
    }

    pub fn game_mode_on(&self) -> bool {
        self.game_mode_active()
    }

    /// An app's rules: muted, and whether it shows over full screen.
    pub fn notification_rules(&self, app: &str) -> (bool, bool) {
        self.settings
            .notification_app
            .get(app)
            .map_or((false, false), |a| (!a.allow, a.over_full_screen))
    }

    /// What holds banners at this moment.
    pub fn holds(&self) -> Holds {
        let main = self.main_output();
        let full_screen = main.as_ref().is_some_and(|o| self.output_full_screen(o))
            && !self.space.outputs().any(|o| !self.output_full_screen(o));
        Holds {
            focus: self.focus_hold(),
            game: self.game_mode_on(),
            full_screen: full_screen && self.settings.notifications.hold_full_screen,
            away: self.services.login.state.preparing_for_sleep || self.lock.locked,
            capture: false,
        }
    }

    /// Whether `output` shows a full-screen window now.
    pub fn output_full_screen(&self, output: &smithay::output::Output) -> bool {
        let identity = Self::output_identity(output);
        self.desktops.rows.get(&identity).is_some_and(|row| {
            row.desktops
                .get(row.current)
                .is_some_and(|d| matches!(d.kind, crate::wm::desktops::Kind::FullScreen { .. }))
        })
    }

    pub fn notifications_status(&self) -> String {
        let entries: Vec<serde_json::Value> = self
            .notifications
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.n.id,
                    "app": self.notification_app(&e.n),
                    "summary": e.n.summary,
                    "body": e.n.body,
                    "live": e.live,
                    "fate": e.fate,
                    "image": e.n.image,
                    "actions": e.n.actions,
                })
            })
            .collect();
        serde_json::json!({ "entries": entries, "held": self.notifications.held.len() }).to_string()
    }
}

impl State {
    pub fn notification_incoming(&mut self, incoming: Incoming) {
        match incoming {
            Incoming::Notify(n) => self.notification_arrived(*n),
            Incoming::Close(id) => {
                // An app's close leaves a record of what was missed (`notifications.md`).
                if let Some(entry) = self
                    .isle
                    .notifications
                    .entries
                    .iter_mut()
                    .find(|e| e.n.id == id && e.live)
                {
                    entry.live = false;
                    self.isle.send(Outgoing::Closed(id, Reason::ClosedByCall));
                    self.forget_transient();
                    self.notifications_changed();
                }
            }
            Incoming::SenderGone(sender) => {
                let mut changed = false;
                for entry in &mut self.isle.notifications.entries {
                    if entry.live && entry.n.sender == sender {
                        entry.live = false;
                        changed = true;
                    }
                }
                if changed {
                    self.forget_transient();
                    self.notifications_changed();
                }
            }
        }
    }

    fn notification_arrived(&mut self, n: Notification) {
        let app = self.isle.notification_app(&n);
        let holds = self.isle.holds();
        let (muted, over_full_screen) = self.isle.notification_rules(&app);
        let fate = decide(&n, &app, muted, over_full_screen, &holds);
        info!(id = n.id, app, ?fate, "notification");
        let id = n.id;
        let isle = &mut self.isle;
        isle.notifications.next_id = isle.notifications.next_id.max(id + 1);
        if fate == Fate::Held && !isle.notifications.held.contains(&id) {
            isle.notifications.held.push(id);
        }
        let entry = Entry {
            n,
            live: true,
            fate,
        };
        match isle.notifications.entries.iter_mut().find(|e| e.n.id == id) {
            // A replacement updates the same notification in place.
            Some(old) => *old = entry,
            None => isle.notifications.entries.push(entry),
        }
        let records = isle.notifications.entries.len();
        if records > RECORDS {
            isle.notifications.entries.drain(..records - RECORDS);
        }
        self.notification_shown(id, fate);
        self.notifications_changed();
    }

    /// A transient notification is never kept once it is no longer live.
    fn forget_transient(&mut self) {
        let gone: Vec<u32> = self
            .isle
            .notifications
            .entries
            .iter()
            .filter(|e| !e.live && e.fate == Fate::Transient)
            .map(|e| e.n.id)
            .collect();
        self.isle
            .notifications
            .entries
            .retain(|e| e.live || e.fate != Fate::Transient);
        for id in gone {
            self.notification_hidden(id);
        }
    }

    pub fn notifications_changed(&mut self) {
        self.isle.save_notifications();
        self.isle.notify(crate::worker::Event::NotificationsChanged);
        self.isle.refresh_island();
    }

    /// The user closed a notification: it and its record go.
    pub fn notification_close(&mut self, id: u32) -> bool {
        let before = self.isle.notifications.entries.len();
        let live = self
            .isle
            .notifications
            .entries
            .iter()
            .any(|e| e.n.id == id && e.live);
        self.isle.notifications.entries.retain(|e| e.n.id != id);
        if live {
            self.isle.send(Outgoing::Closed(id, Reason::Dismissed));
        }
        self.notification_hidden(id);
        let changed = before != self.isle.notifications.entries.len();
        if changed {
            self.notifications_changed();
        }
        changed
    }

    /// Clears every notification, live and recorded.
    pub fn notifications_clear(&mut self) {
        let ids: Vec<u32> = self
            .isle
            .notifications
            .entries
            .iter()
            .map(|e| e.n.id)
            .collect();
        for id in ids {
            self.notification_close(id);
        }
    }

    /// Runs an action, `default` being the click; the app gets a token to come forward.
    /// A record's click opens its app instead.
    pub fn notification_invoke(&mut self, id: u32, key: &str) -> bool {
        let Some(entry) = self
            .isle
            .notifications
            .entries
            .iter()
            .find(|e| e.n.id == id)
            .cloned()
        else {
            return false;
        };
        if self.isle.notifications.desktop.contains_key(&id) {
            self.desktop_action(id, key);
            self.notification_close(id);
            return true;
        }
        if !entry.live {
            if let Some(app) = entry.n.desktop_entry.clone() {
                self.activate_or_launch(&app);
            }
            self.notification_close(id);
            return true;
        }
        let known = key == "default" || entry.n.actions.iter().any(|(k, _)| k == key);
        if known {
            let token = self
                .isle
                .xdg_activation_state
                .create_external_token(XdgActivationTokenData::default())
                .0
                .to_string();
            self.isle
                .send(Outgoing::Action(id, key.to_owned(), Some(token)));
        } else if let Some(app) = entry.n.desktop_entry.clone() {
            self.activate_or_launch(&app);
        }
        if !entry.n.resident {
            self.notification_close(id);
        }
        true
    }

    pub fn notification_reply(&mut self, id: u32, text: &str) -> bool {
        let Some(entry) = self
            .isle
            .notifications
            .entries
            .iter()
            .find(|e| e.n.id == id && e.live)
            .cloned()
        else {
            return false;
        };
        self.isle.send(Outgoing::Replied(id, text.to_owned()));
        if !entry.n.resident {
            self.notification_close(id);
        }
        true
    }

    /// An arrival decided: a banner, when it is one.
    fn notification_shown(&mut self, id: u32, fate: Fate) {
        self.banner_for(id, fate);
    }

    /// A notification gone: its banner goes with it.
    fn notification_hidden(&mut self, id: u32) {
        self.banner_gone(id);
    }

    /// Brings an app forward, or starts it from its entry.
    pub fn activate_or_launch(&mut self, app: &str) {
        if !self.activate_app(app) {
            self.launch(crate::launch::Launch::App {
                app: app.to_owned(),
                action: None,
                uris: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(urgency: u8) -> Notification {
        Notification {
            urgency,
            ..Default::default()
        }
    }

    #[test]
    fn each_arrival_is_decided_in_the_sheets_order() {
        let none = Holds::default();
        assert_eq!(decide(&n(1), "mail", false, false, &none), Fate::Banner);
        assert_eq!(decide(&n(0), "mail", false, false, &none), Fate::Quiet);
        assert_eq!(decide(&n(1), "mail", true, false, &none), Fate::Quiet);
        let transient = Notification {
            transient: true,
            ..n(1)
        };
        assert_eq!(
            decide(&transient, "mail", false, false, &none),
            Fate::Transient
        );

        let focus = Holds {
            focus: Some(("Do Not Disturb".into(), vec!["chat".into()], true)),
            ..Default::default()
        };
        assert_eq!(decide(&n(1), "mail", false, false, &focus), Fate::Held);
        assert_eq!(decide(&n(1), "chat", false, false, &focus), Fate::Banner);
        assert_eq!(decide(&n(2), "mail", false, false, &focus), Fate::Banner);
        assert_eq!(
            decide(&transient, "mail", false, false, &focus),
            Fate::Quiet
        );

        let screen = Holds {
            full_screen: true,
            ..Default::default()
        };
        assert_eq!(decide(&n(1), "mail", false, false, &screen), Fate::Held);
        assert_eq!(decide(&n(1), "mail", false, true, &screen), Fate::Banner);
        assert_eq!(decide(&n(2), "mail", false, false, &screen), Fate::Banner);

        // A screen capture holds even critical ones.
        let capture = Holds {
            capture: true,
            ..Default::default()
        };
        assert_eq!(decide(&n(2), "mail", false, false, &capture), Fate::Held);
    }
}
