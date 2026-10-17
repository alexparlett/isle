//! BlueZ: the adapter, paired and nearby devices with their batteries, discovery, connecting
//! and pairing; and the pairing agent that asks for codes.

pub mod agent;

use std::collections::HashMap;

use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const BLUEZ: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Adapter {
    pub path: String,
    /// The name other devices see it by.
    pub name: String,
    pub powered: bool,
    pub discovering: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Device {
    pub path: String,
    pub name: String,
    /// BlueZ's icon name: `audio-headset`, `input-mouse` and the like.
    pub icon: String,
    pub paired: bool,
    pub connected: bool,
    pub battery: Option<u8>,
}

impl Device {
    /// The Phosphor glyph for its kind.
    pub fn glyph(&self) -> &'static str {
        match self.icon.as_str() {
            i if i.starts_with("audio-head") => "headphones",
            i if i.starts_with("audio") => "speaker-hifi",
            "input-mouse" | "input-tablet" => "mouse",
            "input-keyboard" => "keyboard",
            "input-gaming" => "game-controller",
            "phone" => "device-mobile",
            "computer" => "laptop",
            _ => "bluetooth",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Bluetooth {
    pub adapter: Option<Adapter>,
    /// Paired first, then by name.
    pub devices: Vec<Device>,
}

type Objects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

async fn read(conn: &Connection) -> Result<Bluetooth, Error> {
    let objects: Objects = conn
        .call_method(
            Some(BLUEZ),
            "/",
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    let mut state = Bluetooth::default();
    let mut paths: Vec<&OwnedObjectPath> = objects.keys().collect();
    paths.sort_by_key(|p| p.as_str());
    for path in paths {
        let interfaces = &objects[path];
        if let Some(a) = interfaces.get(ADAPTER)
            && state.adapter.is_none()
        {
            let a = Props(a.clone());
            state.adapter = Some(Adapter {
                path: path.to_string(),
                name: a.str("Alias").or_else(|| a.str("Name")).unwrap_or_default(),
                powered: a.bool("Powered").unwrap_or(false),
                discovering: a.bool("Discovering").unwrap_or(false),
            });
        }
        if let Some(d) = interfaces.get(DEVICE) {
            let d = Props(d.clone());
            let battery = interfaces
                .get(BATTERY)
                .and_then(|b| Props(b.clone()).i64("Percentage"))
                .map(|p| p.clamp(0, 100) as u8);
            let name = d
                .str("Alias")
                .or_else(|| d.str("Name"))
                .unwrap_or_else(|| d.str("Address").unwrap_or_default());
            state.devices.push(Device {
                path: path.to_string(),
                name,
                icon: d.str("Icon").unwrap_or_default(),
                paired: d.bool("Paired").unwrap_or(false),
                connected: d.bool("Connected").unwrap_or(false),
                battery,
            });
        }
    }
    state
        .devices
        .sort_by(|a, b| b.paired.cmp(&a.paired).then(a.name.cmp(&b.name)));
    Ok(state)
}

/// Follows BlueZ.
pub async fn watch(publisher: Publisher<Bluetooth>) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    isle_bus::follow(&conn, BLUEZ, "/", &publisher, || read(&conn)).await
}

async fn call(path: &str, interface: &str, method: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(Some(BLUEZ), path, Some(interface), method, &())
        .await?;
    Ok(())
}

pub async fn set_powered(adapter: &str, on: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(BLUEZ),
        adapter,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(ADAPTER, "Powered", Value::from(on)),
    )
    .await?;
    Ok(())
}

/// Starts looking for devices; it lasts until stopped or Isle leaves the bus.
pub async fn start_discovery(adapter: &str) -> Result<(), Error> {
    call(adapter, ADAPTER, "StartDiscovery").await
}

pub async fn stop_discovery(adapter: &str) -> Result<(), Error> {
    call(adapter, ADAPTER, "StopDiscovery").await
}

/// Forgets a device: unpaired and removed from the adapter.
pub async fn forget(adapter: &str, device: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let device = zbus::zvariant::ObjectPath::try_from(device)?;
    conn.call_method(Some(BLUEZ), adapter, Some(ADAPTER), "RemoveDevice", &(device,))
        .await?;
    Ok(())
}

/// The name a device is shown by; empty gives it back its own.
pub async fn rename(device: &str, name: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(BLUEZ),
        device,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(DEVICE, "Alias", Value::from(name)),
    )
    .await?;
    Ok(())
}

pub async fn connect(device: &str) -> Result<(), Error> {
    call(device, DEVICE, "Connect").await
}

pub async fn disconnect(device: &str) -> Result<(), Error> {
    call(device, DEVICE, "Disconnect").await
}

/// Pairs a nearby device, trusts it and connects it.
pub async fn pair(device: &str) -> Result<(), Error> {
    call(device, DEVICE, "Pair").await?;
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(BLUEZ),
        device,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(DEVICE, "Trusted", Value::from(true)),
    )
    .await?;
    connect(device).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_take_their_glyph_from_bluezs_icon() {
        let device = |icon: &str| Device {
            path: String::new(),
            name: String::new(),
            icon: icon.into(),
            paired: true,
            connected: false,
            battery: None,
        };
        assert_eq!(device("audio-headset").glyph(), "headphones");
        assert_eq!(device("input-gaming").glyph(), "game-controller");
        assert_eq!(device("").glyph(), "bluetooth");
    }
}
