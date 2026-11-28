//! Keychron keyboards and mice through their receivers or cable: battery, the Mac/Windows switch, DPI and report rate.

mod hidraw;
pub mod protocol;

use std::collections::BTreeMap;

use isle_bus::Publisher;
use protocol::{Charging, Kind, Report, Slot};
use serde::Serialize;

/// A Keychron keyboard or mouse that a receiver has paired or that is on its cable.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct Peripheral {
    /// Its USB product id, which its receiver reports.
    pub product: u16,
    pub kind: Kind,
    pub name: String,
    /// The sysfs directories of the USB devices it is reached through: its receiver, its cable.
    pub usb: Vec<String>,
    /// Its receiver has it connected, or it is on its cable.
    pub connected: bool,
    pub battery: Option<u8>,
    pub charging: bool,
    /// The keyboard's Mac/Windows switch, true on Mac; None until the keyboard says.
    pub mac: Option<bool>,
    /// The mouse's DPI per stage, and the stage in use.
    pub dpi: Vec<u16>,
    pub stage: Option<u8>,
    pub rate_hz: Option<u32>,
    /// When it last reported anything, in seconds since the Unix epoch.
    pub seen: Option<u64>,
}

/// Follows every Keychron receiver and keyboard the user can open.
pub async fn watch(publisher: Publisher<Vec<Peripheral>>) -> Result<(), isle_bus::Error> {
    tokio::task::spawn_blocking(move || hidraw::run(&publisher)).await?
}

/// What is known of each USB device and the products behind it.
#[derive(Default)]
struct Model {
    /// By USB device directory: its own product id and, for a receiver, its paired devices.
    devices: BTreeMap<String, (u16, Vec<Slot>)>,
    products: BTreeMap<u16, Known>,
}

#[derive(Default, Clone)]
struct Known {
    battery: Option<u8>,
    charging: bool,
    mac: Option<bool>,
    dpi: Vec<u16>,
    stage: Option<u8>,
    rate: Option<u8>,
    seen: Option<u64>,
}

impl Model {
    /// The product a report from `usb` is about: the device itself, or its receiver's paired one.
    fn product(&self, usb: &str, kind: Kind) -> Option<u16> {
        let (own, slots) = self.devices.get(usb)?;
        if !protocol::is_receiver(*own) {
            return Some(*own);
        }
        slots
            .iter()
            .find(|s| Kind::of(s.product) == kind)
            .map(|s| s.product)
    }

    /// Takes in a report from `usb` read at `now`, in seconds since the Unix epoch.
    fn apply(&mut self, usb: &str, report: Report, now: u64) {
        let kind = match &report {
            Report::Slots(slots) => {
                if let Some(device) = self.devices.get_mut(usb) {
                    device.1 = slots.clone();
                }
                return;
            }
            Report::Keyboard(_) | Report::Layer(_) => Kind::Keyboard,
            Report::Mouse(_) | Report::MouseEvent { .. } => Kind::Mouse,
        };
        let Some(product) = self.product(usb, kind) else {
            return;
        };
        let known = self.products.entry(product).or_default();
        known.seen = Some(now);
        match report {
            Report::Keyboard(b) => {
                known.battery = Some(b.percent);
                known.charging = b.charging == Charging::Charging;
            }
            // Keychron's boards put Mac on layer 0 and Windows on layer 2.
            Report::Layer(0) => known.mac = Some(true),
            Report::Layer(2) => known.mac = Some(false),
            Report::Mouse(s) => {
                known.battery = Some(s.battery);
                known.charging = s.charging;
                known.dpi = s.dpi;
                known.stage = Some(s.stage);
                known.rate = Some(s.rate);
            }
            Report::MouseEvent {
                battery,
                stage,
                rate,
            } => {
                known.battery = Some(battery);
                known.stage = stage.or(known.stage);
                known.rate = rate.or(known.rate);
            }
            Report::Layer(_) | Report::Slots(_) => {}
        }
    }

    fn peripherals(&self) -> Vec<Peripheral> {
        let mut out: BTreeMap<u16, Peripheral> = BTreeMap::new();
        for (usb, (own, slots)) in &self.devices {
            let reached: Vec<(u16, bool)> = if protocol::is_receiver(*own) {
                slots.iter().map(|s| (s.product, s.connected)).collect()
            } else {
                vec![(*own, true)]
            };
            for (product, connected) in reached {
                let known = self.products.get(&product).cloned().unwrap_or_default();
                let p = out.entry(product).or_insert_with(|| Peripheral {
                    product,
                    kind: Kind::of(product),
                    name: protocol::name(product).to_owned(),
                    usb: Vec::new(),
                    connected: false,
                    battery: known.battery,
                    charging: known.charging,
                    mac: known.mac,
                    dpi: known.dpi.clone(),
                    stage: known.stage,
                    rate_hz: known.rate.and_then(protocol::rate_hz),
                    seen: known.seen,
                });
                p.usb.push(usb.clone());
                p.connected |= connected;
            }
        }
        let mut list: Vec<Peripheral> = out.into_values().collect();
        list.sort_by(|a, b| (a.kind as u8, &a.name).cmp(&(b.kind as u8, &b.name)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{KeyboardBattery, MouseStatus};

    const LINK: &str = "/sys/devices/usb5/5-1/5-1.4";
    const ULTRA: &str = "/sys/devices/usb5/5-1/5-1.3";
    const CABLE: &str = "/sys/devices/usb8/8-1";

    fn model() -> Model {
        let mut m = Model::default();
        m.devices.insert(LINK.into(), (0xD030, vec![]));
        m.devices.insert(ULTRA.into(), (0xD028, vec![]));
        m.apply(
            LINK,
            Report::Slots(vec![Slot {
                product: 0x0861,
                connected: true,
            }]),
            0,
        );
        m.apply(
            ULTRA,
            Report::Slots(vec![Slot {
                product: 0xD056,
                connected: true,
            }]),
            0,
        );
        m
    }

    #[test]
    fn receivers_stand_for_what_they_paired() {
        let m = model();
        let list = m.peripherals();
        assert_eq!(list.len(), 2);
        assert_eq!(
            (list[0].name.as_str(), list[0].kind),
            ("Keychron Q6 Max", Kind::Keyboard)
        );
        assert_eq!(
            (list[1].name.as_str(), list[1].kind),
            ("Keychron M7 8K", Kind::Mouse)
        );
        assert_eq!(list[0].usb, vec![LINK.to_owned()]);
        assert_eq!(list[0].battery, None);
    }

    #[test]
    fn reports_reach_the_paired_device() {
        let mut m = model();
        m.apply(
            LINK,
            Report::Keyboard(KeyboardBattery {
                percent: 83,
                millivolts: 3974,
                charging: Charging::Charging,
                connected: true,
            }),
            10,
        );
        m.apply(LINK, Report::Layer(2), 11);
        m.apply(
            ULTRA,
            Report::Mouse(MouseStatus {
                battery: 17,
                charging: false,
                dpi: vec![400, 800],
                stage: 1,
                rate: 2,
            }),
            12,
        );
        m.apply(
            ULTRA,
            Report::MouseEvent {
                battery: 16,
                stage: Some(0),
                rate: None,
            },
            13,
        );
        let list = m.peripherals();
        assert_eq!(
            (list[0].battery, list[0].charging, list[0].mac),
            (Some(83), true, Some(false))
        );
        assert_eq!(
            (list[1].battery, list[1].stage, list[1].rate_hz),
            (Some(16), Some(0), Some(1000))
        );
        assert_eq!(list[1].dpi, vec![400, 800]);
        assert_eq!((list[0].seen, list[1].seen), (Some(11), Some(13)));
    }

    #[test]
    fn a_keyboard_on_its_cable_and_its_receiver_is_one() {
        let mut m = model();
        m.devices.insert(CABLE.into(), (0x0861, vec![]));
        m.apply(CABLE, Report::Layer(0), 20);
        let list = m.peripherals();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].usb, vec![LINK.to_owned(), CABLE.to_owned()]);
        assert_eq!(list[0].mac, Some(true));
    }
}
