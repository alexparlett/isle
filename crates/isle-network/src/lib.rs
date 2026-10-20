//! NetworkManager: the wired link, Wi-Fi with its networks in range and those it knows, and
//! VPN and WireGuard connections; and the secret agent that asks for their passwords.

pub mod agent;

use std::collections::HashMap;

use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const AP: &str = "org.freedesktop.NetworkManager.AccessPoint";
const ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";

/// A device's state, as NetworkManager's numbers group it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Link {
    /// Unplugged or off.
    Unavailable,
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

impl Link {
    fn of(state: i64) -> Self {
        match state {
            0..=20 => Link::Unavailable,
            30 => Link::Disconnected,
            40..=90 => Link::Connecting,
            100 => Link::Connected,
            120 => Link::Failed,
            _ => Link::Disconnected,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wired {
    pub path: String,
    pub link: Link,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Network {
    pub path: String,
    pub ssid: String,
    /// 0 to 100.
    pub strength: u8,
    pub secured: bool,
    /// A saved connection for it, by its settings path.
    pub known: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wifi {
    pub path: String,
    pub link: Link,
    /// The network joined or being joined.
    pub current: Option<Network>,
    /// The rest in range, strongest first, one per name.
    pub networks: Vec<Network>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Vpn {
    /// Its saved connection's settings path.
    pub path: String,
    pub name: String,
    pub link: Link,
    /// Its active connection, while it has one.
    pub active: Option<String>,
}

/// A Wi-Fi network saved, in range or not, once however many connections NetworkManager
/// keeps for its name.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Known {
    /// The saved connection used to join it: the active one, else the first.
    pub path: String,
    /// Every saved connection for its name, `path` first, for Forget and Auto-Join.
    pub paths: Vec<String>,
    pub ssid: String,
    /// It is joined by itself when in range.
    pub auto_join: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct State {
    pub wifi_enabled: bool,
    pub wired: Option<Wired>,
    pub wifi: Option<Wifi>,
    pub vpns: Vec<Vpn>,
    /// Every Wi-Fi network saved, by name.
    pub known: Vec<Known>,
}

/// How a device is connected, as a network's details show it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Details {
    pub address: Option<String>,
    /// The subnet mask, dotted.
    pub subnet: Option<String>,
    pub router: Option<String>,
    pub dns: Vec<String>,
    pub hardware: Option<String>,
    /// Megabits a second, for a wire.
    pub speed: Option<u32>,
}

impl State {
    /// Nothing connects the machine: no wire, no Wi-Fi.
    pub fn offline(&self) -> bool {
        self.wired
            .as_ref()
            .is_none_or(|w| w.link != Link::Connected)
            && self.wifi.as_ref().is_none_or(|w| w.link != Link::Connected)
    }
}

async fn props(conn: &Connection, path: &str, interface: &str) -> Result<Props, Error> {
    Ok(Props::read(conn, NM, path, interface).await?)
}

/// Each saved connection: its settings path, type, id, for Wi-Fi its SSID, and whether it
/// joins by itself.
async fn saved(conn: &Connection) -> Result<Vec<(String, String, String, Option<String>, bool)>, Error> {
    let paths: Vec<OwnedObjectPath> = conn
        .call_method(
            Some(NM),
            SETTINGS_PATH,
            Some("org.freedesktop.NetworkManager.Settings"),
            "ListConnections",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    let mut out = Vec::new();
    for path in paths {
        let Ok(reply) = conn
            .call_method(
                Some(NM),
                path.as_str(),
                Some("org.freedesktop.NetworkManager.Settings.Connection"),
                "GetSettings",
                &(),
            )
            .await
        else {
            continue;
        };
        let Ok(settings) = reply
            .body()
            .deserialize::<HashMap<String, HashMap<String, OwnedValue>>>()
        else {
            continue;
        };
        let section = |name: &str| settings.get(name).cloned().map(Props).unwrap_or_default();
        let connection = section("connection");
        let kind = connection.str("type").unwrap_or_default();
        let id = connection.str("id").unwrap_or_default();
        let ssid = section("802-11-wireless")
            .bytes("ssid")
            .map(|b| String::from_utf8_lossy(&b).into_owned());
        let auto = connection.bool("autoconnect").unwrap_or(true);
        out.push((path.to_string(), kind, id, ssid, auto));
    }
    Ok(out)
}

async fn read(conn: &Connection) -> Result<State, Error> {
    let manager = props(conn, NM_PATH, NM).await?;
    let saved = saved(conn).await?;
    let mut state = State {
        wifi_enabled: manager.bool("WirelessEnabled").unwrap_or(false),
        ..Default::default()
    };
    // Active connections by the saved connection they came from.
    let mut active: HashMap<String, (String, Link)> = HashMap::new();
    for path in manager.paths("ActiveConnections") {
        if let Ok(a) = props(conn, &path, ACTIVE).await {
            let link = match a.i64("State") {
                Some(1) => Link::Connecting,
                Some(2) => Link::Connected,
                _ => Link::Disconnected,
            };
            active.insert(a.str("Connection").unwrap_or_default(), (path, link));
        }
    }
    for path in manager.paths("Devices") {
        let Ok(device) = props(conn, &path, DEVICE).await else {
            continue;
        };
        let link = Link::of(device.i64("State").unwrap_or(0));
        match device.i64("DeviceType") {
            Some(1) if state.wired.is_none() => {
                state.wired = Some(Wired { path, link });
            }
            Some(2) if state.wifi.is_none() => {
                let wireless = props(conn, &path, WIRELESS).await.unwrap_or_default();
                let current_path = wireless.str("ActiveAccessPoint").filter(|p| p != "/");
                let mut networks: Vec<Network> = Vec::new();
                for ap in wireless.paths("AccessPoints") {
                    let Ok(p) = props(conn, &ap, AP).await else {
                        continue;
                    };
                    let ssid = p
                        .bytes("Ssid")
                        .map(|b| String::from_utf8_lossy(&b).into_owned())
                        .unwrap_or_default();
                    if ssid.is_empty() {
                        continue;
                    }
                    let secured = p.i64("WpaFlags").unwrap_or(0) != 0
                        || p.i64("RsnFlags").unwrap_or(0) != 0
                        || p.i64("Flags").unwrap_or(0) & 1 != 0;
                    let known = saved
                        .iter()
                        .find(|(_, kind, _, s, _)| {
                            kind == "802-11-wireless" && s.as_deref() == Some(&ssid)
                        })
                        .map(|(path, ..)| path.clone());
                    networks.push(Network {
                        path: ap,
                        ssid,
                        strength: p.i64("Strength").unwrap_or(0).clamp(0, 100) as u8,
                        secured,
                        known,
                    });
                }
                // The access point in use, before access points are collapsed by name: the
                // strongest of a name need not be the one joined.
                let current = current_path
                    .as_ref()
                    .and_then(|p| networks.iter().position(|n| &n.path == p))
                    .map(|i| networks.remove(i));
                networks.sort_by(|a, b| b.strength.cmp(&a.strength));
                let mut seen: Vec<String> = current.iter().map(|c| c.ssid.clone()).collect();
                networks.retain(|n| {
                    let first = !seen.contains(&n.ssid);
                    seen.push(n.ssid.clone());
                    first
                });
                state.wifi = Some(Wifi {
                    path,
                    link,
                    current,
                    networks,
                });
            }
            _ => {}
        }
    }
    for (path, kind, _, ssid, auto) in &saved {
        let (Some(ssid), true) = (ssid, kind == "802-11-wireless") else {
            continue;
        };
        let in_use = active.contains_key(path);
        match state.known.iter_mut().find(|k| &k.ssid == ssid) {
            Some(known) if in_use => {
                known.paths.insert(0, path.clone());
                known.path = path.clone();
                known.auto_join = *auto;
            }
            Some(known) => known.paths.push(path.clone()),
            None => state.known.push(Known {
                path: path.clone(),
                paths: vec![path.clone()],
                ssid: ssid.clone(),
                auto_join: *auto,
            }),
        }
    }
    state.known.sort_by_key(|k| k.ssid.to_lowercase());
    state.vpns = saved
        .iter()
        .filter(|(_, kind, ..)| kind == "vpn" || kind == "wireguard")
        .map(|(path, _, id, _, _)| {
            let (active, link) = active
                .get(path)
                .map_or((None, Link::Disconnected), |(a, l)| (Some(a.clone()), *l));
            Vpn {
                path: path.clone(),
                name: id.clone(),
                link,
                active,
            }
        })
        .collect();
    Ok(state)
}

/// Follows NetworkManager.
pub async fn watch(publisher: Publisher<State>) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    isle_bus::follow(&conn, NM, NM_PATH, &publisher, || read(&conn)).await
}

pub async fn set_wifi(enabled: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(NM),
        NM_PATH,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(NM, "WirelessEnabled", Value::from(enabled)),
    )
    .await?;
    Ok(())
}

/// Asks the Wi-Fi device to look for networks.
pub async fn scan(device: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let options: HashMap<&str, Value> = HashMap::new();
    conn.call_method(Some(NM), device, Some(WIRELESS), "RequestScan", &(options,))
        .await?;
    Ok(())
}

fn path(p: &str) -> Result<OwnedObjectPath, Error> {
    Ok(OwnedObjectPath::try_from(p.to_owned())?)
}

/// An access point's path, or `/` for NetworkManager to choose one.
fn ap_or_any(ap: &str) -> &str {
    if ap.is_empty() { "/" } else { ap }
}

/// Joins a network in range: its saved connection if known, else a new one NetworkManager
/// completes from the access point.
pub async fn join(device: &str, network: &Network) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    match &network.known {
        Some(saved) => {
            conn.call_method(
                Some(NM),
                NM_PATH,
                Some(NM),
                "ActivateConnection",
                &(path(saved)?, path(device)?, path(ap_or_any(&network.path))?),
            )
            .await?;
        }
        None => {
            let settings: HashMap<&str, HashMap<&str, Value>> = HashMap::new();
            let options: HashMap<&str, Value> = HashMap::from([("persist", Value::from("disk"))]);
            conn.call_method(
                Some(NM),
                NM_PATH,
                Some(NM),
                "AddAndActivateConnection2",
                &(settings, path(device)?, path(ap_or_any(&network.path))?, options),
            )
            .await?;
        }
    }
    Ok(())
}

/// Starts a saved connection on whatever device suits it, as a VPN is.
pub async fn activate(saved: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(NM),
        NM_PATH,
        Some(NM),
        "ActivateConnection",
        &(path(saved)?, path("/")?, path("/")?),
    )
    .await?;
    Ok(())
}

pub async fn deactivate(active: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(NM),
        NM_PATH,
        Some(NM),
        "DeactivateConnection",
        &(path(active)?,),
    )
    .await?;
    Ok(())
}

/// Disconnects a device, as the Wi-Fi network's Disconnect does.
pub async fn disconnect(device: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(Some(NM), device, Some(DEVICE), "Disconnect", &())
        .await?;
    Ok(())
}

const SAVED: &str = "org.freedesktop.NetworkManager.Settings.Connection";

/// Forgets a saved connection: its settings and its secrets.
pub async fn forget(saved: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(Some(NM), saved, Some(SAVED), "Delete", &()).await?;
    Ok(())
}

/// Whether a saved Wi-Fi network is joined by itself, its password kept.
pub async fn set_auto_join(saved: &str, on: bool) -> Result<(), Error> {
    type Sections = HashMap<String, HashMap<String, OwnedValue>>;
    let conn = isle_bus::system().await?;
    let mut settings: Sections = conn
        .call_method(Some(NM), saved, Some(SAVED), "GetSettings", &())
        .await?
        .body()
        .deserialize()?;
    // An update replaces the whole connection; its secrets come back with it or are lost.
    if settings.contains_key("802-11-wireless-security") {
        let secrets: Sections = conn
            .call_method(Some(NM), saved, Some(SAVED), "GetSecrets", &("802-11-wireless-security",))
            .await?
            .body()
            .deserialize()?;
        for (section, values) in secrets {
            settings.entry(section).or_default().extend(values);
        }
    }
    settings
        .entry("connection".into())
        .or_default()
        .insert("autoconnect".into(), OwnedValue::from(on));
    conn.call_method(Some(NM), saved, Some(SAVED), "Update", &(settings,))
        .await?;
    Ok(())
}

/// Joins a network that does not broadcast its name; its password, if it has one, comes
/// from the secret agent.
pub async fn join_hidden(device: &str, ssid: &str, secured: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let mut settings: HashMap<&str, HashMap<&str, Value>> = HashMap::from([
        ("connection", HashMap::from([("type", Value::from("802-11-wireless")), ("id", Value::from(ssid))])),
        (
            "802-11-wireless",
            HashMap::from([("ssid", Value::from(ssid.as_bytes().to_vec())), ("hidden", Value::from(true))]),
        ),
    ]);
    if secured {
        settings.insert("802-11-wireless-security", HashMap::from([("key-mgmt", Value::from("wpa-psk"))]));
    }
    let options: HashMap<&str, Value> = HashMap::from([("persist", Value::from("disk"))]);
    conn.call_method(
        Some(NM),
        NM_PATH,
        Some(NM),
        "AddAndActivateConnection2",
        &(settings, path(device)?, path("/")?, options),
    )
    .await?;
    Ok(())
}

/// A prefix length as a dotted mask: 24 is 255.255.255.0.
fn mask(prefix: u32) -> String {
    let bits = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix.min(32)) };
    std::net::Ipv4Addr::from(bits).to_string()
}

/// How `device` is connected: its address, subnet, router, DNS, hardware address and speed.
pub async fn details(device: &str) -> Result<Details, Error> {
    let conn = isle_bus::system().await?;
    let dev = props(&conn, device, DEVICE).await?;
    let mut out = Details {
        hardware: dev.str("HwAddress").filter(|a| !a.is_empty()),
        ..Default::default()
    };
    if dev.i64("DeviceType") == Some(1) {
        let wired = props(&conn, device, "org.freedesktop.NetworkManager.Device.Wired")
            .await
            .unwrap_or_default();
        out.speed = wired.i64("Speed").filter(|s| *s > 0).map(|s| s as u32);
    }
    if let Some(config) = dev.str("Ip4Config").filter(|p| p != "/") {
        let ip = props(&conn, &config, "org.freedesktop.NetworkManager.IP4Config")
            .await
            .unwrap_or_default();
        if let Some(first) = ip.maps("AddressData").into_iter().next() {
            out.address = first.str("address");
            out.subnet = first.i64("prefix").map(|p| mask(p as u32));
        }
        out.router = ip.str("Gateway").filter(|g| !g.is_empty());
        out.dns = ip.maps("NameserverData").iter().filter_map(|n| n.str("address")).collect();
    }
    Ok(out)
}

/// The bars a strength shows as: Phosphor's Wi-Fi level glyph.
pub fn signal_glyph(strength: u8) -> &'static str {
    match strength {
        0..=34 => "wifi-low",
        35..=69 => "wifi-medium",
        _ => "wifi-high",
    }
}

/// The word a strength is read as.
pub fn signal_word(strength: u8) -> &'static str {
    match strength {
        0..=34 => "low",
        35..=69 => "medium",
        _ => "high",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_states_group_as_links() {
        assert_eq!(Link::of(20), Link::Unavailable);
        assert_eq!(Link::of(30), Link::Disconnected);
        assert_eq!(Link::of(70), Link::Connecting);
        assert_eq!(Link::of(100), Link::Connected);
        assert_eq!(Link::of(120), Link::Failed);
    }

    #[test]
    fn prefixes_are_masks() {
        assert_eq!(mask(24), "255.255.255.0");
        assert_eq!(mask(20), "255.255.240.0");
        assert_eq!(mask(0), "0.0.0.0");
    }

    #[test]
    fn offline_is_neither_wire_nor_wifi() {
        let mut state = State::default();
        assert!(state.offline());
        state.wired = Some(Wired {
            path: "/d/1".into(),
            link: Link::Connected,
        });
        assert!(!state.offline());
    }
}

#[cfg(test)]
mod host {
    /// Prints what the host's NetworkManager reports; run by hand.
    #[tokio::test]
    #[ignore]
    async fn reads_the_hosts_network_manager() {
        let conn = zbus::Connection::system().await.unwrap();
        println!("{:#?}", super::read(&conn).await.unwrap());
    }
}
