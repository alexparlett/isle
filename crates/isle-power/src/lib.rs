//! UPower's batteries and keyboard backlight, and power-profiles-daemon's profiles and holds.

use isle_bus::{Error, Props, Publisher};
use serde::Serialize;
use zbus::Connection;

const UPOWER: &str = "org.freedesktop.UPower";
const UPOWER_PATH: &str = "/org/freedesktop/UPower";
const DEVICE: &str = "org.freedesktop.UPower.Device";
const PROFILES: &str = "org.freedesktop.UPower.PowerProfiles";
const PROFILES_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";
const KBD: &str = "org.freedesktop.UPower.KbdBacklight";
const KBD_PATH: &str = "/org/freedesktop/UPower/KbdBacklight";

/// What a battery powers, from UPower's device type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// The machine's own.
    System,
    Mouse,
    Keyboard,
    Headset,
    Pad,
    Pen,
    Touchpad,
    Phone,
    Other,
}

impl Kind {
    fn of(device_type: i64, power_supply: bool) -> Self {
        match device_type {
            2 if power_supply => Kind::System,
            5 => Kind::Mouse,
            6 => Kind::Keyboard,
            17 | 19 | 21 => Kind::Headset,
            12 => Kind::Pad,
            13 => Kind::Pen,
            14 => Kind::Touchpad,
            8 => Kind::Phone,
            _ => Kind::Other,
        }
    }

    /// The Phosphor glyph for it.
    pub fn glyph(self) -> &'static str {
        match self {
            Kind::System => "laptop",
            Kind::Mouse | Kind::Touchpad => "mouse",
            Kind::Keyboard => "keyboard",
            Kind::Headset => "headphones",
            Kind::Pad => "game-controller",
            Kind::Phone => "device-mobile",
            Kind::Pen | Kind::Other => "battery-full",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Battery {
    pub path: String,
    pub kind: Kind,
    pub name: String,
    /// Percent, from `Percentage` or a coarse `BatteryLevel`; None while unknown.
    pub level: Option<u8>,
    pub charging: bool,
    /// Seconds left while discharging, when UPower knows.
    pub time_to_empty: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Power {
    /// The machine runs on its battery.
    pub on_battery: bool,
    /// The machine's batteries first, then the rest by kind and name.
    pub batteries: Vec<Battery>,
}

impl Power {
    /// The machine's own battery, if it has one.
    pub fn system(&self) -> Option<&Battery> {
        self.batteries.iter().find(|b| b.kind == Kind::System)
    }
}

/// At or below this a battery is low (macOS's first warning).
pub const LOW: u8 = 20;

fn coarse(level: i64) -> Option<u8> {
    // UPower's BatteryLevel: 3 low, 4 critical, 6 normal, 7 high, 8 full.
    match level {
        3 => Some(15),
        4 => Some(5),
        6 => Some(55),
        7 => Some(80),
        8 => Some(100),
        _ => None,
    }
}

async fn read_power(conn: &Connection) -> Result<Power, Error> {
    let manager = Props::read(conn, UPOWER, UPOWER_PATH, UPOWER).await?;
    let paths: Vec<zbus::zvariant::OwnedObjectPath> = conn
        .call_method(
            Some(UPOWER),
            UPOWER_PATH,
            Some(UPOWER),
            "EnumerateDevices",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    let mut batteries = Vec::new();
    for path in paths {
        let Ok(p) = Props::read(conn, UPOWER, path.as_str(), DEVICE).await else {
            continue;
        };
        let device_type = p.i64("Type").unwrap_or(0);
        // Line power and UPS units carry no battery to show.
        if matches!(device_type, 0 | 1 | 3) || p.bool("IsPresent") == Some(false) {
            continue;
        }
        let power_supply = p.bool("PowerSupply").unwrap_or(false);
        let kind = Kind::of(device_type, power_supply);
        let level = match p.i64("BatteryLevel").and_then(coarse) {
            Some(level) => Some(level),
            None => p
                .f64("Percentage")
                .filter(|&n| n > 0.0 || p.i64("State").unwrap_or(0) != 0)
                .map(|n| n.round().clamp(0.0, 100.0) as u8),
        };
        let state = p.i64("State").unwrap_or(0);
        let name = p
            .str("Model")
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| default_name(kind).to_owned());
        batteries.push(Battery {
            path: path.to_string(),
            kind,
            name,
            level,
            // 1 charging, 4 fully charged, 5 pending charge.
            charging: matches!(state, 1 | 5),
            time_to_empty: p
                .i64("TimeToEmpty")
                .filter(|&s| s > 0 && state == 2)
                .map(|s| s as u64),
        });
    }
    batteries.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
    Ok(Power {
        on_battery: manager.bool("OnBattery").unwrap_or(false),
        batteries,
    })
}

fn default_name(kind: Kind) -> &'static str {
    match kind {
        Kind::System => "Battery",
        Kind::Mouse => "Mouse",
        Kind::Keyboard => "Keyboard",
        Kind::Headset => "Headset",
        Kind::Pad => "Controller",
        Kind::Pen => "Pen",
        Kind::Touchpad => "Trackpad",
        Kind::Phone => "Phone",
        Kind::Other => "Device",
    }
}

/// Follows UPower's batteries.
pub async fn watch_batteries(publisher: Publisher<Power>) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    isle_bus::follow(&conn, UPOWER, UPOWER_PATH, &publisher, || read_power(&conn)).await
}

/// A profile held on someone's behalf.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Hold {
    pub profile: String,
    pub reason: String,
    pub app: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Profiles {
    /// `power-saver`, `balanced` or `performance`.
    pub active: String,
    pub choices: Vec<String>,
    pub holds: Vec<Hold>,
}

async fn read_profiles(conn: &Connection) -> Result<Profiles, Error> {
    let p = Props::read(conn, PROFILES, PROFILES_PATH, PROFILES).await?;
    Ok(Profiles {
        active: p.str("ActiveProfile").unwrap_or_default(),
        choices: p
            .maps("Profiles")
            .iter()
            .filter_map(|m| m.str("Profile"))
            .collect(),
        holds: p
            .maps("ActiveProfileHolds")
            .iter()
            .map(|m| Hold {
                profile: m.str("Profile").unwrap_or_default(),
                reason: m.str("Reason").unwrap_or_default(),
                app: m.str("ApplicationId").unwrap_or_default(),
            })
            .collect(),
    })
}

/// Follows power-profiles-daemon.
pub async fn watch_profiles(publisher: Publisher<Profiles>) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    isle_bus::follow(&conn, PROFILES, PROFILES_PATH, &publisher, || {
        read_profiles(&conn)
    })
    .await
}

/// Makes `profile` the active one, as the user's own choice.
pub async fn set_profile(profile: &str) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    let value = zbus::zvariant::Value::from(profile);
    conn.call_method(
        Some(PROFILES),
        PROFILES_PATH,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(PROFILES, "ActiveProfile", value),
    )
    .await?;
    Ok(())
}

/// Holds `profile` until released, or until the user picks another; returns the cookie.
pub async fn hold_profile(profile: &str, reason: &str, app: &str) -> Result<u32, Error> {
    let conn = isle_bus::system().await?;
    Ok(conn
        .call_method(
            Some(PROFILES),
            PROFILES_PATH,
            Some(PROFILES),
            "HoldProfile",
            &(profile, reason, app),
        )
        .await?
        .body()
        .deserialize()?)
}

pub async fn release_profile(cookie: u32) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(PROFILES),
        PROFILES_PATH,
        Some(PROFILES),
        "ReleaseProfile",
        &(cookie,),
    )
    .await?;
    Ok(())
}

/// The keyboard backlight's level and its maximum, when there is one.
pub async fn keyboard_backlight() -> Result<Option<(u32, u32)>, Error> {
    let conn = isle_bus::system().await?;
    let call = |method: &'static str| {
        let conn = conn.clone();
        async move {
            conn.call_method(Some(UPOWER), KBD_PATH, Some(KBD), method, &())
                .await
                .and_then(|m| m.body().deserialize::<i32>())
        }
    };
    match (call("GetMaxBrightness").await, call("GetBrightness").await) {
        (Ok(max), Ok(level)) if max > 0 => Ok(Some((level.max(0) as u32, max as u32))),
        _ => Ok(None),
    }
}

pub async fn set_keyboard_backlight(level: u32) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some(UPOWER),
        KBD_PATH,
        Some(KBD),
        "SetBrightness",
        &(level as i32,),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_take_their_kind_from_upowers_type() {
        assert_eq!(Kind::of(2, true), Kind::System);
        assert_eq!(Kind::of(2, false), Kind::Other);
        assert_eq!(Kind::of(5, false), Kind::Mouse);
        assert_eq!(Kind::of(17, false), Kind::Headset);
        assert_eq!(Kind::of(12, false), Kind::Pad);
    }

    #[test]
    fn a_coarse_level_reads_as_a_percentage() {
        assert_eq!(coarse(8), Some(100));
        assert_eq!(coarse(3), Some(15));
        assert_eq!(coarse(1), None);
    }
}
