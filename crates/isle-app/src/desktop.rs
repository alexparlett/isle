//! The desktop state an app draws with, read from the compositor and followed live: the
//! theme, every setting's value and whether `settings.kdl` is in force (`apps.md`
//! Conventions 19, 21, 26).

use std::{collections::BTreeMap, time::Duration};

use futures_channel::mpsc::UnboundedSender;
use futures_util::{FutureExt, StreamExt};
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

/// A change to `settings.kdl`.
#[derive(Clone, Debug, PartialEq)]
pub enum Write {
    Set(String, Value),
    Reset(String),
    Remove(String),
}

impl Write {
    pub fn key(&self) -> &str {
        match self {
            Write::Set(key, _) | Write::Reset(key) | Write::Remove(key) => key,
        }
    }
}

/// A settings value as the bus carries it.
pub fn to_bus(value: Value) -> zvariant::Value<'static> {
    match value {
        Value::Bool(b) => b.into(),
        Value::Int(i) => i.into(),
        Value::Float(f) => f.into(),
        Value::Str(s) => s.into(),
        Value::List(l) => l.into(),
    }
}

/// Sends one change; the refusal's words when the compositor turns it down.
pub async fn send(bus: zbus::Connection, write: Write) -> Result<(), String> {
    let settings = SettingsProxy::new(&bus).await.map_err(|e| e.to_string())?;
    let result = match write {
        Write::Set(key, value) => settings.set(&key, &to_bus(value)).await,
        Write::Reset(key) => settings.reset(&key).await,
        Write::Remove(entry) => settings.remove(&entry).await,
    };
    result.map(|_| ()).map_err(|e| match e {
        zbus::Error::MethodError(_, Some(message), _) => message,
        other => other.to_string(),
    })
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
        // A burst of signals, as a dragged slider sends, is read once.
        tokio::select! {
            Some(_) = theme_changed.next() => {
                while let Some(Some(_)) = theme_changed.next().now_or_never() {}
                theme().await?
            }
            Some(_) = changed.next() => {
                while let Some(Some(_)) = changed.next().now_or_never() {}
                values().await?
            }
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
