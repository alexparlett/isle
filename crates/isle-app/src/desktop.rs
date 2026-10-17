//! The desktop state an app draws with, read from the compositor and followed live: the
//! theme, every setting's value and whether `settings.kdl` is in force (`apps.md`
//! Conventions 19, 21, 26).

use std::{collections::BTreeMap, time::Duration};

use futures_channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
use isle_ipc::{SettingsStatus, ThemeInfo, settings::SettingsProxy, shell::ShellProxy};
use isle_settings::Value;
use tracing::{info, warn};
use zvariant::OwnedValue;

/// What the compositor told the app.
#[derive(Clone, Debug)]
pub enum Update {
    Theme(ThemeInfo),
    TokensStatus(SettingsStatus),
    Values(BTreeMap<String, Value>),
    Status(SettingsStatus),
}

/// A value from the bus as the settings store holds it.
pub fn from_owned(value: &OwnedValue) -> Option<Value> {
    use zvariant::Value as V;
    Some(match &**value {
        V::Bool(b) => Value::Bool(*b),
        V::I32(i) => Value::Int((*i).into()),
        V::U32(i) => Value::Int((*i).into()),
        V::I64(i) => Value::Int(*i),
        V::F64(f) => Value::Float(*f),
        V::Str(s) => Value::Str(s.to_string()),
        V::Array(a) => Value::List(
            a.iter()
                .filter_map(|v| v.downcast_ref::<zvariant::Str>().ok().map(|s| s.to_string()))
                .collect(),
        ),
        V::Value(inner) => return from_owned(&OwnedValue::try_from(&**inner).ok()?),
        _ => return None,
    })
}

/// Follows the compositor until the bus goes; while Isle is not on the bus, waits for it.
pub async fn watch(bus: zbus::Connection, updates: UnboundedSender<Update>) {
    loop {
        if let Err(err) = follow(&bus, &updates).await {
            warn!("the compositor's state is not reachable: {err}");
        }
        if updates.is_closed() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn follow(bus: &zbus::Connection, updates: &UnboundedSender<Update>) -> zbus::Result<()> {
    let shell = ShellProxy::new(bus).await?;
    let settings = SettingsProxy::new(bus).await?;
    let mut theme_changed = shell.receive_theme_changed().await?;
    let mut changed = settings.receive_changed().await?;
    let mut status_changed = settings.receive_status_changed().await?;
    let mut owner = shell.inner().receive_owner_changed().await?;

    let send = |update| updates.unbounded_send(update).is_ok();
    let theme = async || -> zbus::Result<()> {
        send(Update::Theme(shell.theme().await?));
        send(Update::TokensStatus(shell.tokens_status().await?));
        Ok(())
    };
    let values = async || -> zbus::Result<()> {
        let values = settings
            .values()
            .await?
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), from_owned(v)?)))
            .collect();
        send(Update::Values(values));
        Ok(())
    };
    theme().await?;
    values().await?;
    send(Update::Status(settings.status().await?));
    info!("following the compositor");

    loop {
        tokio::select! {
            Some(_) = theme_changed.next() => theme().await?,
            Some(_) = changed.next() => values().await?,
            Some(_) = status_changed.next() => {
                send(Update::Status(settings.status().await?));
            }
            Some(name) = owner.next() => {
                if name.is_none() {
                    return Err(zbus::Error::Failure("Isle left the bus".into()));
                }
                theme().await?;
                values().await?;
            }
            else => return Ok(()),
        }
        if updates.is_closed() {
            return Ok(());
        }
    }
}
