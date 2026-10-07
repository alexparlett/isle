//! Screen brightness: a monitor's over DDC/CI on its i2c bus, written by Isle, and a laptop
//! panel's backlight through logind (`p8-services.md` choice 5). Every call blocks, so callers
//! run it off the main loop.

use std::{
    fs,
    io::{Read, Write},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use isle_bus::Error;

/// The DDC/CI device's i2c address, and the EDID's.
const DDC: u16 = 0x37;
const EDID: u16 = 0x50;
/// VCP feature 0x10, luminance.
pub const BRIGHTNESS: u8 = 0x10;
const I2C_SLAVE: libc::c_ulong = 0x0703;

/// What a screen's brightness goes through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Control {
    /// A monitor on `/dev/i2c-N`.
    Ddc(PathBuf),
    /// A panel's backlight, by its name under `/sys/class/backlight`.
    Backlight(String),
}

/// A brightness, raw and the most it can be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub current: u32,
    pub max: u32,
}

impl Level {
    pub fn share(self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

/// The bytes a host writes to ask for a VCP feature's value.
pub fn get_request(feature: u8) -> [u8; 5] {
    let body = [0x51, 0x82, 0x01, feature];
    let mut out = [0; 5];
    out[..4].copy_from_slice(&body);
    out[4] = checksum(0x6e, &body);
    out
}

/// The bytes a host writes to set a VCP feature.
pub fn set_request(feature: u8, value: u16) -> [u8; 7] {
    let [hi, lo] = value.to_be_bytes();
    let body = [0x51, 0x84, 0x03, feature, hi, lo];
    let mut out = [0; 7];
    out[..6].copy_from_slice(&body);
    out[6] = checksum(0x6e, &body);
    out
}

fn checksum(seed: u8, bytes: &[u8]) -> u8 {
    bytes.iter().fold(seed, |acc, b| acc ^ b)
}

/// A VCP reply's current and maximum values, if it is whole, for `feature`, and supported.
pub fn parse_reply(feature: u8, reply: &[u8]) -> Option<Level> {
    // Source, length, opcode 0x02, result, feature, type, max hi, max lo, cur hi, cur lo, sum.
    if reply.len() < 11 || reply[2] != 0x02 || reply[3] != 0 || reply[4] != feature {
        return None;
    }
    // The checksum covers the reply from the host's virtual address 0x50.
    if checksum(0x50, &reply[..10]) != reply[10] {
        return None;
    }
    Some(Level {
        max: u16::from_be_bytes([reply[6], reply[7]]).into(),
        current: u16::from_be_bytes([reply[8], reply[9]]).into(),
    })
}

/// An open i2c bus aimed at one address.
struct Bus(fs::File);

impl Bus {
    fn open(path: &Path, address: u16) -> Result<Self, Error> {
        let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
        // SAFETY: the fd is open for the call, and I2C_SLAVE takes the address by value.
        let ret = unsafe { libc::ioctl(file.as_raw_fd(), I2C_SLAVE, libc::c_ulong::from(address)) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(Self(file))
    }
}

/// The monitor's brightness, asked as ddcutil does: a request, a pause, the reply, tried
/// again when the reply is garbled.
pub fn ddc_get(path: &Path) -> Result<Level, Error> {
    let mut bus = Bus::open(path, DDC)?;
    for _ in 0..3 {
        bus.0.write_all(&get_request(BRIGHTNESS))?;
        sleep(Duration::from_millis(50));
        let mut reply = [0; 12];
        if bus.0.read_exact(&mut reply[..11]).is_ok()
            && let Some(level) = parse_reply(BRIGHTNESS, &reply[..11])
        {
            return Ok(level);
        }
        sleep(Duration::from_millis(100));
    }
    Err("the monitor gave no brightness over DDC".into())
}

pub fn ddc_set(path: &Path, value: u32) -> Result<(), Error> {
    let mut bus = Bus::open(path, DDC)?;
    bus.0
        .write_all(&set_request(BRIGHTNESS, value.min(u16::MAX as u32) as u16))?;
    // A monitor needs a pause after a set before it hears the next request.
    sleep(Duration::from_millis(50));
    Ok(())
}

/// The first 128 bytes of the EDID a bus answers with, if any.
fn bus_edid(path: &Path) -> Option<Vec<u8>> {
    let mut bus = Bus::open(path, EDID).ok()?;
    bus.0.write_all(&[0]).ok()?;
    let mut edid = vec![0; 128];
    bus.0.read_exact(&mut edid).ok()?;
    (edid[..8] == [0, 255, 255, 255, 255, 255, 255, 0]).then_some(edid)
}

/// Whether a bus by its adapter's name can carry a display: never an SMBus, which may hold
/// memory's SPD EEPROM at the EDID's address.
pub fn display_bus(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && !["SMBus", "smbus", "DesignWare", "SPD"]
            .iter()
            .any(|n| name.contains(n))
}

/// The i2c buses that may carry displays, as device paths.
fn candidate_buses(sys: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(sys.join("bus/i2c/devices")) else {
        return Vec::new();
    };
    let mut buses: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let name = fs::read_to_string(e.path().join("name")).unwrap_or_default();
            (file.starts_with("i2c-") && display_bus(&name))
                .then(|| PathBuf::from("/dev").join(file))
        })
        .collect();
    buses.sort();
    buses
}

/// The control for each connector, by its name (`DP-4`): a backlight for a laptop panel, else
/// the bus whose EDID is the connector's. Connectors with neither are left out.
pub fn discover(connectors: &[String]) -> Vec<(String, Control)> {
    let sys = Path::new("/sys");
    let mut found = Vec::new();
    let mut edids: Option<Vec<(PathBuf, Vec<u8>)>> = None;
    for connector in connectors {
        let Some(dir) = connector_dir(sys, connector) else {
            continue;
        };
        if let Some(name) = backlight_for(sys, connector) {
            found.push((connector.clone(), Control::Backlight(name)));
            continue;
        }
        // The connector's own DDC link, where the driver gives one.
        if let Ok(link) = fs::read_link(dir.join("ddc"))
            && let Some(bus) = link.file_name()
        {
            found.push((
                connector.clone(),
                Control::Ddc(PathBuf::from("/dev").join(bus)),
            ));
            continue;
        }
        let Ok(edid) = fs::read(dir.join("edid")) else {
            continue;
        };
        if edid.len() < 128 {
            continue;
        }
        let edids = edids.get_or_insert_with(|| {
            candidate_buses(sys)
                .into_iter()
                .filter_map(|bus| bus_edid(&bus).map(|e| (bus, e)))
                .collect()
        });
        if let Some((bus, _)) = edids.iter().find(|(_, e)| e[..] == edid[..128]) {
            found.push((connector.clone(), Control::Ddc(bus.clone())));
        }
    }
    found
}

fn connector_dir(sys: &Path, connector: &str) -> Option<PathBuf> {
    fs::read_dir(sys.join("class/drm"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.split_once('-'))
                .is_some_and(|(card, rest)| card.starts_with("card") && rest == connector)
        })
}

/// A backlight belonging to a laptop panel's connector.
fn backlight_for(sys: &Path, connector: &str) -> Option<String> {
    if !["eDP", "LVDS", "DSI"]
        .iter()
        .any(|p| connector.starts_with(p))
    {
        return None;
    }
    let entries = fs::read_dir(sys.join("class/backlight")).ok()?;
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    // Firmware and platform backlights come before raw ones.
    names.sort_by_key(|n| {
        match fs::read_to_string(sys.join("class/backlight").join(n).join("type")).as_deref() {
            Ok(t) if t.trim() == "firmware" => 0,
            Ok(t) if t.trim() == "platform" => 1,
            _ => 2,
        }
    });
    names.into_iter().next()
}

pub fn backlight_get(name: &str) -> Result<Level, Error> {
    let dir = Path::new("/sys/class/backlight").join(name);
    let read =
        |file| -> Result<u32, Error> { Ok(fs::read_to_string(dir.join(file))?.trim().parse()?) };
    Ok(Level {
        current: read("brightness")?,
        max: read("max_brightness")?,
    })
}

/// Sets a backlight through logind, which lets the session's own user do it.
pub async fn backlight_set(name: &str, value: u32) -> Result<(), Error> {
    let conn = isle_bus::system().await?;
    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1/session/auto",
        Some("org.freedesktop.login1.Session"),
        "SetBrightness",
        &("backlight", name, value),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_carry_ddcutils_bytes() {
        // As ddcutil sends them for `getvcp 10` and `setvcp 10 50`.
        assert_eq!(get_request(0x10), [0x51, 0x82, 0x01, 0x10, 0xac]);
        assert_eq!(
            set_request(0x10, 50),
            [0x51, 0x84, 0x03, 0x10, 0x00, 0x32, 0x9a]
        );
    }

    #[test]
    fn a_whole_reply_reads_and_a_garbled_one_does_not() {
        let mut reply = [
            0x6e, 0x88, 0x02, 0x00, 0x10, 0x00, 0x00, 0x64, 0x00, 0x3c, 0x00,
        ];
        reply[10] = checksum(0x50, &reply[..10]);
        assert_eq!(
            parse_reply(0x10, &reply),
            Some(Level {
                current: 60,
                max: 100
            })
        );
        let mut garbled = reply;
        garbled[9] = 0x3d;
        assert_eq!(parse_reply(0x10, &garbled), None);
        let mut unsupported = reply;
        unsupported[3] = 0x01;
        unsupported[10] = checksum(0x50, &unsupported[..10]);
        assert_eq!(parse_reply(0x10, &unsupported), None);
    }

    #[test]
    fn smbus_adapters_are_never_probed() {
        assert!(display_bus("NVIDIA i2c adapter 4 at 1:00.0"));
        assert!(display_bus("AMDGPU DM i2c hw bus 0"));
        assert!(!display_bus("SMBus PIIX4 adapter port 0 at 0b00"));
        assert!(!display_bus("Synopsys DesignWare I2C adapter"));
        assert!(!display_bus(""));
    }
}

#[cfg(test)]
mod host {
    /// Reads the brightness of `ISLE_DDC_CONNECTOR`'s monitor; run by hand on hardware.
    #[test]
    #[ignore]
    fn reads_a_real_monitor() {
        let connector = std::env::var("ISLE_DDC_CONNECTOR").unwrap();
        let found = super::discover(&[connector]);
        println!("{found:?}");
        if let Some((_, super::Control::Ddc(bus))) = found.first() {
            println!("{:?}", super::ddc_get(bus));
        }
    }
}
