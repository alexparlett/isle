//! Bluetooth: its glyph, module and page, discovery only while the page shows, connecting
//! and pairing.

use isle_bluetooth::{Bluetooth, Device};
use isle_bus::Snapshot;
use isle_shell::island::{
    grid::TileSize,
    model::Glyph,
    panel::{Module, PanelAction},
    rows::{Choice, Lead, PageRow},
};

use crate::state::{Isle, State};

impl State {
    pub fn bluetooth_changed(&mut self, snapshot: Snapshot<Bluetooth>) {
        self.isle.services.bluetooth = snapshot;
        // A device that finished connecting is no longer pending.
        let devices = self.isle.services.bluetooth.state.devices.clone();
        self.isle
            .services
            .connecting
            .retain(|p| !devices.iter().any(|d| &d.path == p && d.connected));
        self.isle.services_changed();
    }

    pub fn bluetooth_action(&mut self, action: PanelAction) {
        let bt = self.isle.services.bluetooth.state.clone();
        let Some(adapter) = bt.adapter.clone() else {
            return;
        };
        match action {
            PanelAction::Toggle(_) => {
                let on = !adapter.powered;
                let path = adapter.path.clone();
                self.isle.run_service(
                    async move { isle_bluetooth::set_powered(&path, on).await },
                    |state, result| {
                        state.isle.service_error(
                            "bluetooth",
                            result,
                            "Couldn't turn Bluetooth on or off",
                        )
                    },
                );
            }
            PanelAction::Choose { row, .. } => {
                let Some(device) = bt.devices.iter().find(|d| d.path == row).cloned() else {
                    return;
                };
                let key = format!("bluetooth/{}", device.path);
                self.isle.services.errors.remove(&key);
                if !device.connected {
                    self.isle.services.connecting.insert(device.path.clone());
                }
                let path = device.path.clone();
                let name = device.name.clone();
                let command = async move {
                    match (device.paired, device.connected) {
                        (_, true) => isle_bluetooth::disconnect(&path).await,
                        (true, false) => isle_bluetooth::connect(&path).await,
                        (false, _) => isle_bluetooth::pair(&path).await,
                    }
                };
                let pending = row.clone();
                self.isle.run_service(command, move |state, result| {
                    state.isle.services.connecting.remove(&pending);
                    if result.is_err() {
                        state.isle.services.errors.insert(
                            key,
                            format!("Couldn't connect to {name}. Make sure it's on and nearby."),
                        );
                    }
                    state.isle.refresh_island();
                });
            }
            _ => {}
        }
        self.isle.refresh_island();
    }

    /// Looks for nearby devices while the Bluetooth page shows, and only then.
    pub fn follow_discovery(&mut self) {
        let isle = &mut self.isle;
        let Some(adapter) = isle.services.bluetooth.state.adapter.clone() else {
            return;
        };
        let wanted = adapter.powered
            && isle
                .island
                .panel
                .as_ref()
                .and_then(|p| p.page.as_deref())
                .is_some_and(|p| p.split('/').next() == Some("bluetooth"));
        if wanted == isle.services.discovering {
            return;
        }
        isle.services.discovering = wanted;
        let path = adapter.path.clone();
        isle.run_service(
            async move {
                if wanted {
                    isle_bluetooth::start_discovery(&path).await
                } else {
                    isle_bluetooth::stop_discovery(&path).await
                }
            },
            |_, result| {
                if let Err(err) = result {
                    tracing::debug!("Bluetooth discovery: {err}");
                }
            },
        );
    }
}

impl Isle {
    /// The Bluetooth glyph: a device connected.
    pub fn bluetooth_glyph(&self) -> Option<Glyph> {
        let bt = &self.services.bluetooth;
        if !bt.available || !bt.state.adapter.as_ref().is_some_and(|a| a.powered) {
            return None;
        }
        let device = bt.state.devices.iter().find(|d| d.connected)?;
        Some(Glyph {
            active: true,
            symbol: "bluetooth-connected".into(),
            alt: format!("Bluetooth: {}", device.name),
            ..Default::default()
        })
    }

    fn device_row(&self, d: &Device) -> PageRow {
        let connecting = self.services.connecting.contains(&d.path);
        PageRow::Choice(Choice {
            id: d.path.clone(),
            label: d.name.clone(),
            lead: Lead::Glyph(d.glyph().into()),
            checked: d.connected,
            busy: connecting,
            detail: if connecting {
                Some(if d.paired { "Connecting" } else { "Pairing" }.into())
            } else {
                d.connected.then(|| "Connected".into())
            },
            error: self
                .services
                .errors
                .get(&format!("bluetooth/{}", d.path))
                .cloned(),
            battery: d.battery,
            enabled: self.services.bluetooth.available,
            ..Default::default()
        })
    }

    pub fn bluetooth_module(&self, size: TileSize) -> Option<Module> {
        let bt = &self.services.bluetooth;
        let adapter = bt.state.adapter.as_ref()?;
        let connected: Vec<&Device> = bt.state.devices.iter().filter(|d| d.connected).collect();
        let state = if !bt.available {
            "Not available".to_owned()
        } else if !adapter.powered {
            "Off".to_owned()
        } else {
            match connected.as_slice() {
                [] => "On".to_owned(),
                [one] => one.name.clone(),
                many => format!("{} devices", many.len()),
            }
        };
        let mut rows = Vec::new();
        if adapter.powered {
            let (paired, nearby): (Vec<&Device>, Vec<&Device>) =
                bt.state.devices.iter().partition(|d| d.paired);
            rows.push(PageRow::Heading("My devices".into()));
            if paired.is_empty() {
                rows.push(PageRow::Status("No devices".into()));
            }
            rows.extend(paired.iter().map(|d| self.device_row(d)));
            rows.push(PageRow::Heading("Nearby devices".into()));
            if nearby.is_empty() {
                rows.push(PageRow::Status(
                    if adapter.discovering {
                        "Looking…"
                    } else {
                        "No devices"
                    }
                    .into(),
                ));
            }
            rows.extend(nearby.iter().map(|d| self.device_row(d)));
        }
        if let Some(error) = self.services.errors.get("bluetooth") {
            rows.push(PageRow::Status(error.clone()));
        }
        Some(Module {
            id: "bluetooth".into(),
            size,
            name: "Bluetooth".into(),
            state,
            glyph: if connected.is_empty() {
                "bluetooth"
            } else {
                "bluetooth-connected"
            }
            .into(),
            on: adapter.powered && bt.available,
            toggles: bt.available,
            page: Some(rows),
            pane: "Bluetooth".into(),
            ..Default::default()
        })
    }
}
