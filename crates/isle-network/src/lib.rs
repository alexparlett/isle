//! NetworkManager: the wired link, Wi-Fi with its networks in range and those it knows, and
//! VPN and WireGuard connections; and the secret agent that asks for their passwords.

pub mod agent;

use std::{collections::HashMap, path::Path};

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
const SYS_NET: &str = "/sys/class/net";
/// NetworkManager's `wake-on-lan` and `wake-on-wlan` value that defers to its global default.
const WAKE_DEFAULT: u32 = 0x1;

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

/// A wired or Wi-Fi profile as Wake for network access sets it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WakeProfile {
    /// Its settings path.
    pub path: String,
    /// It wakes the computer on a magic packet.
    pub wakes: bool,
}

/// What Wake for network access works on.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Wake {
    /// A wired or Wi-Fi device reports it can wake the computer.
    pub capable: bool,
    /// Every saved wired and Wi-Fi profile; a path not seen before is a profile added.
    pub profiles: Vec<WakeProfile>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct State {
    pub wifi_enabled: bool,
    pub wired: Option<Wired>,
    pub wifi: Option<Wifi>,
    pub vpns: Vec<Vpn>,
    /// Every Wi-Fi network saved, by name.
    pub known: Vec<Known>,
    pub wake: Wake,
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

/// A saved connection as the state reads it.
struct SavedConn {
    path: String,
    kind: String,
    id: String,
    /// Wi-Fi's SSID.
    ssid: Option<String>,
    /// It joins by itself.
    auto: bool,
    /// A wired or Wi-Fi profile's wake flags.
    wake: Option<u32>,
}

/// The setting and key that hold a profile type's wake flags, and the flag for a magic packet.
fn wake_key(kind: &str) -> Option<(&'static str, &'static str, u32)> {
    match kind {
        "802-3-ethernet" => Some(("802-3-ethernet", "wake-on-lan", 0x40)),
        "802-11-wireless" => Some(("802-11-wireless", "wake-on-wlan", 0x8)),
        _ => None,
    }
}

/// Whether an interface's device can wake the computer, under sysfs's `net` class at `net`.
fn can_wake(net: &Path, interface: &str) -> bool {
    !interface.is_empty()
        && !interface.contains('/')
        && net.join(interface).join("device/power/wakeup").exists()
}

async fn saved(conn: &Connection) -> Result<Vec<SavedConn>, Error> {
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
        // GetSettings leaves out a value at its default.
        let wake = wake_key(&kind).map(|(setting, key, _)| {
            section(setting).i64(key).map_or(WAKE_DEFAULT, |v| v as u32)
        });
        out.push(SavedConn {
            path: path.to_string(),
            id: connection.str("id").unwrap_or_default(),
            ssid: section("802-11-wireless")
                .bytes("ssid")
                .map(|b| String::from_utf8_lossy(&b).into_owned()),
            auto: connection.bool("autoconnect").unwrap_or(true),
            kind,
            wake,
        });
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
        let kind = device.i64("DeviceType");
        if matches!(kind, Some(1 | 2))
            && can_wake(Path::new(SYS_NET), &device.str("Interface").unwrap_or_default())
        {
            state.wake.capable = true;
        }
        match kind {
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
                        .find(|s| s.kind == "802-11-wireless" && s.ssid.as_deref() == Some(&ssid))
                        .map(|s| s.path.clone());
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
    for SavedConn { path, kind, ssid, auto, .. } in &saved {
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
        .filter(|s| s.kind == "vpn" || s.kind == "wireguard")
        .map(|SavedConn { path, id, .. }| {
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
    state.wake.profiles = saved
        .iter()
        .filter_map(|s| {
            let (_, _, magic) = wake_key(&s.kind)?;
            Some(WakeProfile {
                path: s.path.clone(),
                wakes: s.wake? & magic != 0,
            })
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

type Sections = HashMap<String, HashMap<String, OwnedValue>>;

/// Changes a saved connection through `edit`, its secrets kept.
async fn update(
    conn: &Connection,
    saved: &str,
    edit: impl FnOnce(&mut Sections),
) -> Result<(), Error> {
    let mut settings: Sections = conn
        .call_method(Some(NM), saved, Some(SAVED), "GetSettings", &())
        .await?
        .body()
        .deserialize()?;
    // An update replaces the whole connection; its secrets come back with it or are lost.
    for setting in ["802-11-wireless-security", "802-1x"] {
        if !settings.contains_key(setting) {
            continue;
        }
        let secrets: Sections = conn
            .call_method(Some(NM), saved, Some(SAVED), "GetSecrets", &(setting,))
            .await?
            .body()
            .deserialize()?;
        for (section, values) in secrets {
            settings.entry(section).or_default().extend(values);
        }
    }
    edit(&mut settings);
    conn.call_method(Some(NM), saved, Some(SAVED), "Update", &(settings,))
        .await?;
    Ok(())
}

/// Whether a saved Wi-Fi network is joined by itself, its password kept.
pub async fn set_auto_join(saved: &str, on: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    update(&conn, saved, |settings| {
        settings
            .entry("connection".into())
            .or_default()
            .insert("autoconnect".into(), OwnedValue::from(on));
    })
    .await
}

/// Wake for network access: every saved wired and Wi-Fi profile wakes the computer on a
/// magic packet, or goes back to NetworkManager's default. A profile already so is not
/// updated. Each profile is tried; the last refusal is returned.
pub async fn set_wake(on: bool) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let mut refused = None;
    for profile in saved(&conn).await? {
        let (Some((setting, key, magic)), Some(flags)) = (wake_key(&profile.kind), profile.wake)
        else {
            continue;
        };
        let set = if on { flags & magic != 0 } else { flags == WAKE_DEFAULT };
        if set {
            continue;
        }
        let want = if on { magic } else { WAKE_DEFAULT };
        let result = update(&conn, &profile.path, |settings| {
            settings
                .entry(setting.into())
                .or_default()
                .insert(key.into(), OwnedValue::from(want));
        })
        .await;
        if let Err(e) = result {
            tracing::warn!(profile = %profile.path, "wake for network access: {e}");
            refused = Some(e);
        }
    }
    refused.map_or(Ok(()), Err)
}

/// Where a saved network keeps its password, by its secret's flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Keeps {
    /// NetworkManager keeps it.
    System,
    /// The user's secret agent keeps it, in the keyring.
    Agent,
    /// Asked for every time.
    NotSaved,
    /// The network has none.
    Nothing,
}

/// A saved Wi-Fi profile as Keychain shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedWifi {
    /// Its settings path.
    pub path: String,
    pub id: String,
    pub uuid: String,
    pub ssid: String,
    /// `key-mgmt`; empty for an open network.
    pub security: String,
    pub keeps: Keeps,
    /// The setting and key that hold the password: `802-11-wireless-security` and `psk`, say.
    pub setting: String,
    pub key: String,
    /// WPA-Enterprise's identity.
    pub identity: Option<String>,
}

fn wifi_profile(path: String, settings: &HashMap<String, HashMap<String, OwnedValue>>) -> Option<SavedWifi> {
    let section = |name: &str| settings.get(name).cloned().map(Props).unwrap_or_default();
    let connection = section("connection");
    if connection.str("type").as_deref() != Some("802-11-wireless") {
        return None;
    }
    let ssid = section("802-11-wireless")
        .bytes("ssid")
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let security = section("802-11-wireless-security");
    let key_mgmt = security.str("key-mgmt").unwrap_or_default();
    let x = section("802-1x");
    let (setting, key, flags, identity) = match key_mgmt.as_str() {
        "" | "owe" => ("", "", None, None),
        "wpa-eap" | "wpa-eap-suite-b-192" | "ieee8021x" => (
            "802-1x",
            "password",
            Some(x.i64("password-flags").unwrap_or(0)),
            x.str("identity"),
        ),
        "none" => (
            "802-11-wireless-security",
            "wep-key0",
            Some(security.i64("wep-key-flags").unwrap_or(0)),
            None,
        ),
        _ => (
            "802-11-wireless-security",
            "psk",
            Some(security.i64("psk-flags").unwrap_or(0)),
            None,
        ),
    };
    let keeps = match flags {
        None => Keeps::Nothing,
        Some(f) if f & 2 != 0 => Keeps::NotSaved,
        Some(f) if f & 4 != 0 => Keeps::Nothing,
        Some(f) if f & 1 != 0 => Keeps::Agent,
        Some(_) => Keeps::System,
    };
    Some(SavedWifi {
        path,
        id: connection.str("id").unwrap_or_default(),
        uuid: connection.str("uuid").unwrap_or_default(),
        ssid,
        security: key_mgmt,
        keeps,
        setting: setting.to_owned(),
        key: key.to_owned(),
        identity,
    })
}

/// Every saved Wi-Fi profile, by name, without a password.
pub async fn saved_wifi() -> Result<Vec<SavedWifi>, Error> {
    saved_wifi_on(&isle_bus::system().await?).await
}

pub async fn saved_wifi_on(conn: &Connection) -> Result<Vec<SavedWifi>, Error> {
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
            .call_method(Some(NM), path.as_str(), Some(SAVED), "GetSettings", &())
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
        out.extend(wifi_profile(path.to_string(), &settings));
    }
    out.sort_by_key(|w| w.id.to_lowercase());
    Ok(out)
}

/// The password NetworkManager keeps for a system-owned profile, through its own polkit
/// check; `None` when it holds none.
pub async fn wifi_password(
    conn: &Connection,
    wifi: &SavedWifi,
) -> Result<Option<zeroize::Zeroizing<String>>, Error> {
    type Sections = HashMap<String, HashMap<String, OwnedValue>>;
    let secrets: Sections = conn
        .call_method(Some(NM), wifi.path.as_str(), Some(SAVED), "GetSecrets", &(wifi.setting.as_str(),))
        .await?
        .body()
        .deserialize()?;
    Ok(secrets
        .get(&wifi.setting)
        .and_then(|s| s.get(&wifi.key))
        .and_then(|v| match &**v {
            Value::Str(s) => Some(zeroize::Zeroizing::new(s.as_str().to_owned())),
            _ => None,
        })
        .filter(|s| !s.is_empty()))
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
    fn a_device_wakes_with_a_wakeup_file() {
        let net = std::env::temp_dir().join(format!("isle-network-wake-{}", std::process::id()));
        std::fs::create_dir_all(net.join("eth0/device/power")).unwrap();
        std::fs::write(net.join("eth0/device/power/wakeup"), "disabled\n").unwrap();
        std::fs::create_dir_all(net.join("wlan0/device")).unwrap();
        assert!(can_wake(&net, "eth0"));
        assert!(!can_wake(&net, "wlan0"));
        assert!(!can_wake(&net, "missing0"));
        assert!(!can_wake(&net, ""));
        std::fs::remove_dir_all(&net).unwrap();
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
