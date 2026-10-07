//! LauncherEntry's Update signals, kept per app for the switcher's cards (`p7-switcher.md`
//! choice 12).

use std::{collections::HashMap, error::Error};

use futures_util::StreamExt;
use isle_apps::badges::{LauncherEntry, Value};
use tracing::debug;
use zvariant::OwnedValue;

use crate::{state::State, worker::ToLoop};

/// Listens for Update from every sender, and for senders leaving the bus.
pub async fn watch(to_loop: ToLoop) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = zbus::Connection::session().await?;
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("com.canonical.Unity.LauncherEntry")?
        .member("Update")?
        .build();
    let mut updates = zbus::MessageStream::for_match_rule(rule, &bus, None).await?;
    let dbus = zbus::fdo::DBusProxy::new(&bus).await?;
    let mut owners = dbus.receive_name_owner_changed().await?;
    loop {
        tokio::select! {
            Some(message) = updates.next() => {
                let Ok(message) = message else { continue };
                let header = message.header();
                let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
                let Ok((uri, properties)) =
                    message.body().deserialize::<(String, HashMap<String, OwnedValue>)>()
                else {
                    continue;
                };
                let properties: Vec<(String, Value)> = properties
                    .into_iter()
                    .filter_map(|(name, value)| Some((name, value_of(&value)?)))
                    .collect();
                to_loop.post(move |state| state.launcher_entry(&uri, sender, &properties));
            }
            Some(change) = owners.next() => {
                let Ok(args) = change.args() else { continue };
                if args.new_owner().is_none() {
                    let name = args.name().to_string();
                    to_loop.post(move |state| state.launcher_entry_gone(&name));
                }
            }
            else => return Ok(()),
        }
    }
}

fn value_of(value: &OwnedValue) -> Option<Value> {
    match &**value {
        zvariant::Value::Bool(b) => Some(Value::Bool(*b)),
        zvariant::Value::I64(n) => Some(Value::Int(*n)),
        zvariant::Value::I32(n) => Some(Value::Int(i64::from(*n))),
        zvariant::Value::U32(n) => Some(Value::Int(i64::from(*n))),
        zvariant::Value::U64(n) => Some(Value::Int(*n as i64)),
        zvariant::Value::F64(n) => Some(Value::Float(*n)),
        _ => None,
    }
}

impl State {
    pub fn launcher_entry(&mut self, uri: &str, sender: String, properties: &[(String, Value)]) {
        let Some(id) = isle_apps::badges::desktop_id(uri) else {
            return;
        };
        let app = self.isle.apps.resolve(id);
        let entry = self.isle.badges.entry(app.clone()).or_default();
        let was_urgent = entry.urgent;
        entry.merge(properties);
        entry.sender = sender;
        debug!(app, ?properties, "launcher entry");
        if entry.urgent && !was_urgent && Some(&app) != self.isle.front_app().as_ref() {
            self.island_attention(app, true);
        }
        self.isle.notify(crate::worker::Event::WindowsChanged);
    }

    pub fn launcher_entry_gone(&mut self, sender: &str) {
        let before = self.isle.badges.len();
        self.isle.badges.retain(|_, e| e.sender != sender);
        if self.isle.badges.len() != before {
            self.isle.notify(crate::worker::Event::WindowsChanged);
        }
    }
}

/// An app activated answers its LauncherEntry's urgency, as it answers its windows'.
pub fn calm(badges: &mut HashMap<String, LauncherEntry>, app: &str) {
    if let Some(entry) = badges.get_mut(app) {
        entry.urgent = false;
    }
}
