//! The bytes Keychron's receivers and keyboards send and answer, as read in `keychron-2026.md`.

/// Keychron's USB vendor id.
pub const VENDOR: u16 = 0x3434;

/// What a hidraw node of a Keychron device is for, from its descriptor's first usage page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// QMK raw HID (`0xFF60`), 32-byte unnumbered reports: the receiver's slot query, and the
    /// keyboard's own commands through the Link or on the cable.
    Raw,
    /// The Ultra-Link 8K's mouse channel (`0xFFC1`): reports `b3`–`b6`.
    Mouse,
    /// The Link's vendor page (`0x8C`): its `54` connection events.
    Link,
}

impl Channel {
    /// From a HID report descriptor.
    pub fn of(descriptor: &[u8]) -> Option<Self> {
        let page = match descriptor {
            [0x06, lo, hi, ..] => u16::from_le_bytes([*lo, *hi]),
            [0x05, page, ..] => *page as u16,
            _ => return None,
        };
        match page {
            0xFF60 => Some(Channel::Raw),
            0xFFC1 => Some(Channel::Mouse),
            0x8C => Some(Channel::Link),
            _ => None,
        }
    }
}

/// Keyboard or mouse, from a Keychron product id: their mice are `0xDxxx`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Keyboard,
    Mouse,
}

impl Kind {
    pub fn of(product: u16) -> Self {
        if product & 0xF000 == 0xD000 {
            Kind::Mouse
        } else {
            Kind::Keyboard
        }
    }
}

/// Whether a product id is one of Keychron's receivers rather than a keyboard or mouse.
pub fn is_receiver(product: u16) -> bool {
    matches!(product, 0xD028 | 0xD030 | 0xD031)
}

/// The product's name, from the id its receiver reports or it enumerates with.
pub fn name(product: u16) -> &'static str {
    match product {
        0x0861 => "Keychron Q6 Max",
        0xD056 => "Keychron M7 8K",
        0xD03F => "Keychron M6",
        0xD032 | 0xD033 => "Keychron M3",
        0xD035 => "Keychron M1",
        0xD03B => "Keychron M2",
        0xD03D => "Keychron M2 mini",
        0xD036 | 0xD03E => "Keychron M3 mini",
        0xD044 => "Keychron M7",
        p if Kind::of(p) == Kind::Mouse => "Keychron mouse",
        _ => "Keychron keyboard",
    }
}

/// Raw HID: the receiver's paired devices.
pub const SLOTS: [u8; 1] = [0xB2];
/// Raw HID: the keyboard's battery (community firmware's misc-group command).
pub const BATTERY: [u8; 2] = [0xA7, 0xF0];
/// Raw HID: the keyboard's default layer, which its Mac/Windows switch sets.
pub const LAYER: [u8; 1] = [0xA3];
/// The Ultra-Link 8K's mouse status request, report `b3`.
pub const MOUSE_STATUS: [u8; 2] = [0xB3, 0x06];
/// The length of a report `b3` with its id.
pub const MOUSE_STATUS_LEN: usize = 64;

/// A paired device as its receiver lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub product: u16,
    pub connected: bool,
}

/// What a report says, whichever channel it came on.
#[derive(Clone, Debug, PartialEq)]
pub enum Report {
    Slots(Vec<Slot>),
    Keyboard(KeyboardBattery),
    /// The default layer: 0 Mac, 2 Windows on Keychron's boards.
    Layer(u8),
    Mouse(MouseStatus),
    /// A mouse's pushed state on waking or a button: battery, and DPI stage and rate on the 8K.
    MouseEvent {
        battery: u8,
        stage: Option<u8>,
        rate: Option<u8>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyboardBattery {
    pub percent: u8,
    pub millivolts: u16,
    pub charging: Charging,
    /// The keyboard's radio is connected.
    pub connected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Charging {
    No,
    Charging,
    Full,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MouseStatus {
    pub battery: u8,
    pub charging: bool,
    /// The DPI of each stage, and which is in use.
    pub dpi: Vec<u16>,
    pub stage: u8,
    /// The report rate's index, see [`rate_hz`].
    pub rate: u8,
}

/// Report rates by the index the 8K receiver reports; index 2 measured 1000 Hz on the M7 8K, the
/// rest follow its six steps.
pub fn rate_hz(index: u8) -> Option<u32> {
    [125, 500, 1000, 2000, 4000, 8000]
        .get(index as usize)
        .copied()
}

/// Parses a report read from any channel; `r[0]` is the report id or, on raw HID, the command.
pub fn parse(channel: Channel, r: &[u8]) -> Option<Report> {
    match (channel, r) {
        (Channel::Raw, [0xB2, n, rest @ ..]) => Some(Report::Slots(slots(*n, rest))),
        (
            Channel::Raw,
            [
                0xA7,
                0xF0,
                0,
                percent,
                lo,
                hi,
                charging,
                _transport,
                state,
                ..,
            ],
        ) if *percent <= 100 => Some(Report::Keyboard(KeyboardBattery {
            percent: *percent,
            millivolts: u16::from_le_bytes([*lo, *hi]),
            charging: match charging {
                1 => Charging::Charging,
                2 => Charging::Full,
                _ => Charging::No,
            },
            connected: *state == 3,
        })),
        (Channel::Raw, [0xA3, layer, ..]) => Some(Report::Layer(*layer)),
        (Channel::Mouse, [0xB6, 0x03, n, rest @ ..]) => Some(Report::Slots(slots(*n, rest))),
        (Channel::Mouse, [0xB4, 0x06, ..]) if r.len() > 20 => mouse_status(r).map(Report::Mouse),
        (Channel::Mouse, [0xB6, 0xE2, 0x01, _, _, battery, stage, rate, ..]) if *battery <= 100 => {
            Some(Report::MouseEvent {
                battery: *battery,
                stage: Some(*stage),
                rate: Some(*rate),
            })
        }
        (Channel::Link, [0x54, 0xE2, 0x01, _, 0, battery, 2, 2, ..]) if *battery <= 100 => {
            Some(Report::MouseEvent {
                battery: *battery,
                stage: None,
                rate: None,
            })
        }
        _ => None,
    }
}

fn slots(n: u8, rest: &[u8]) -> Vec<Slot> {
    rest.chunks_exact(5)
        .take(n as usize)
        .filter(|s| u16::from_le_bytes([s[0], s[1]]) == VENDOR)
        .map(|s| Slot {
            product: u16::from_le_bytes([s[2], s[3]]),
            connected: s[4] != 0,
        })
        .collect()
}

fn mouse_status(r: &[u8]) -> Option<MouseStatus> {
    let battery = r[20] & 0x7F;
    if battery > 100 {
        return None;
    }
    let count = (r[17] as usize).min(5);
    Some(MouseStatus {
        battery,
        charging: r[20] & 0x80 != 0,
        dpi: (0..count)
            .map(|i| u16::from_le_bytes([r[6 + 2 * i], r[7 + 2 * i]]))
            .collect(),
        stage: r[5],
        rate: r[3] >> 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    #[test]
    fn channels_come_from_the_first_usage_page() {
        assert_eq!(Channel::of(&hex("06 60 ff 09 61")), Some(Channel::Raw));
        assert_eq!(Channel::of(&hex("06 c1 ff 09 01")), Some(Channel::Mouse));
        assert_eq!(Channel::of(&hex("05 8c 09 01")), Some(Channel::Link));
        assert_eq!(Channel::of(&hex("05 01 09 02")), None);
    }

    #[test]
    fn receivers_list_their_paired_devices() {
        let link = hex("b2 01 34 34 61 08 01 00 00 00");
        assert_eq!(
            parse(Channel::Raw, &link),
            Some(Report::Slots(vec![Slot {
                product: 0x0861,
                connected: true
            }]))
        );
        let ultra = hex("b6 03 01 34 34 56 d0 01 00 00");
        assert_eq!(
            parse(Channel::Mouse, &ultra),
            Some(Report::Slots(vec![Slot {
                product: 0xD056,
                connected: true
            }]))
        );
        assert_eq!(
            parse(Channel::Raw, &hex("b2 00 00 00")),
            Some(Report::Slots(vec![]))
        );
    }

    #[test]
    fn the_keyboard_answers_battery_and_layer() {
        let battery = hex("a7 f0 00 53 86 0f 01 04 03 01");
        assert_eq!(
            parse(Channel::Raw, &battery),
            Some(Report::Keyboard(KeyboardBattery {
                percent: 83,
                millivolts: 3974,
                charging: Charging::Charging,
                connected: true,
            }))
        );
        assert_eq!(parse(Channel::Raw, &hex("a7 f0 01 00")), None);
        assert_eq!(
            parse(Channel::Raw, &hex("a3 02 00")),
            Some(Report::Layer(2))
        );
    }

    #[test]
    fn the_8k_mouse_status_carries_battery_dpi_and_rate() {
        let status = hex(
            "b4 06 00 22 22 02 90 01 20 03 40 06 80 0c 88 13 15 05 04 0a 91 00 00 00 00 00 00 b7",
        );
        assert_eq!(
            parse(Channel::Mouse, &status),
            Some(Report::Mouse(MouseStatus {
                battery: 17,
                charging: true,
                dpi: vec![400, 800, 1600, 3200, 5000],
                stage: 2,
                rate: 2,
            }))
        );
        assert_eq!(rate_hz(2), Some(1000));
        assert_eq!(rate_hz(6), None);
    }

    #[test]
    fn mice_push_their_state() {
        assert_eq!(
            parse(Channel::Mouse, &hex("b6 e2 01 01 03 11 04 02 05")),
            Some(Report::MouseEvent {
                battery: 17,
                stage: Some(4),
                rate: Some(2)
            })
        );
        assert_eq!(
            parse(Channel::Link, &hex("54 e2 01 01 00 4c 02 02 00")),
            Some(Report::MouseEvent {
                battery: 76,
                stage: None,
                rate: None
            })
        );
        // A keyboard's connection event on the Link carries no battery.
        assert_eq!(
            parse(Channel::Link, &hex("54 e2 01 01 00 00 00 00 00")),
            None
        );
    }

    #[test]
    fn products_have_kinds_and_names() {
        assert_eq!(Kind::of(0x0861), Kind::Keyboard);
        assert_eq!(Kind::of(0xD056), Kind::Mouse);
        assert_eq!(name(0x0861), "Keychron Q6 Max");
        assert_eq!(name(0xD999), "Keychron mouse");
        assert!(is_receiver(0xD030) && !is_receiver(0x0861));
    }
}
