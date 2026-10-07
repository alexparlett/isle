//! Wi-Fi and VPN: their glyphs, modules and pages, scanning while the Wi-Fi page shows, and
//! joining a network. A password is the Wi-Fi prompt's, which P9 builds.

use std::time::Duration;

use isle_bus::Snapshot;
use isle_network::{Link, Network, State as Net, signal_glyph, signal_word};
use isle_shell::island::{
    grid::TileSize,
    model::Glyph,
    panel::{Module, PanelAction},
    rows::{Choice, Lead, PageRow},
};

use crate::state::{Isle, State};

/// How often the Wi-Fi page asks for a fresh scan while it shows.
const SCAN_EVERY: Duration = Duration::from_secs(10);

impl State {
    pub fn network_changed(&mut self, snapshot: Snapshot<Net>) {
        self.isle.services.network = snapshot;
        // A join that finished, either way, is no longer pending.
        if let Some((ssid, _)) = self.isle.services.joining.clone() {
            let wifi = self.isle.services.network.state.wifi.clone();
            let joined = wifi
                .as_ref()
                .and_then(|w| w.current.as_ref())
                .is_some_and(|c| c.ssid == ssid && wifi.as_ref().unwrap().link == Link::Connected);
            let failed = wifi.as_ref().is_some_and(|w| w.link == Link::Failed);
            if joined {
                self.isle.services.joining = None;
            } else if failed {
                self.isle.services.joining = None;
                self.isle.services.errors.insert(
                    format!("wifi/{ssid}"),
                    format!("Couldn't connect to “{ssid}”. Check the password and try again."),
                );
            }
        }
        self.isle.services_changed();
    }

    pub fn wifi_action(&mut self, action: PanelAction) {
        let net = self.isle.services.network.state.clone();
        match action {
            PanelAction::Toggle(_) => {
                let on = !net.wifi_enabled;
                self.isle
                    .run_service(isle_network::set_wifi(on), |state, result| {
                        state
                            .isle
                            .service_error("wifi", result, "Couldn't turn Wi-Fi on or off");
                    });
            }
            PanelAction::Expand { row, .. } => {
                let key = format!("wifi/{row}");
                if !self.isle.services.expanded.remove(&key) {
                    self.isle.services.expanded.insert(key);
                }
            }
            PanelAction::Choose { row, .. } => {
                let Some(wifi) = net.wifi.clone() else { return };
                if row == "join-other" {
                    self.panel_action(PanelAction::Settings("Wi-Fi".into()));
                    return;
                }
                let Some(network) = wifi.networks.iter().find(|n| n.path == row).cloned() else {
                    return;
                };
                self.join(&wifi.path, network);
            }
            _ => {}
        }
    }

    fn join(&mut self, device: &str, network: Network) {
        let ssid = network.ssid.clone();
        self.isle.services.errors.remove(&format!("wifi/{ssid}"));
        self.isle.services.joining = Some((ssid.clone(), network.path.clone()));
        let device = device.to_owned();
        self.isle.run_service(
            async move { isle_network::join(&device, &network).await },
            move |state, result| {
                if let Err(err) = result {
                    state.isle.services.joining = None;
                    // With no secret agent until P9, a secured network NetworkManager has no
                    // password for cannot be joined from here.
                    let message = if err.contains("Secrets were required")
                        || err.contains("No agents")
                    {
                        format!("“{ssid}” needs a password")
                    } else {
                        format!("Couldn't connect to “{ssid}”. Check the password and try again.")
                    };
                    state
                        .isle
                        .services
                        .errors
                        .insert(format!("wifi/{ssid}"), message);
                    state.isle.refresh_island();
                }
            },
        );
        self.isle.refresh_island();
    }

    pub fn vpn_action(&mut self, action: PanelAction) {
        let net = self.isle.services.network.state.clone();
        let target = match action {
            PanelAction::Toggle(_) => net
                .vpns
                .iter()
                .find(|v| v.active.is_some())
                .or_else(|| {
                    let last = self.isle.services.last_vpn.clone();
                    net.vpns
                        .iter()
                        .find(|v| Some(&v.path) == last.as_ref())
                        .or(net.vpns.first())
                })
                .cloned(),
            PanelAction::Choose { row, .. } => net.vpns.iter().find(|v| v.path == row).cloned(),
            _ => None,
        };
        let Some(vpn) = target else { return };
        self.isle.services.errors.remove("vpn");
        let command = match vpn.active.clone() {
            Some(active) => Box::pin(async move { isle_network::deactivate(&active).await })
                as std::pin::Pin<Box<dyn Future<Output = Result<(), isle_bus::Error>> + Send>>,
            None => {
                self.isle.services.last_vpn = Some(vpn.path.clone());
                let path = vpn.path.clone();
                Box::pin(async move { isle_network::activate(&path).await })
            }
        };
        let name = vpn.name.clone();
        self.isle.run_service(command, move |state, result| {
            state
                .isle
                .service_error("vpn", result, &format!("Couldn't connect to {name}"));
        });
    }

    /// Scans for networks while the Wi-Fi page shows, and only then.
    pub fn follow_scan(&mut self) {
        let isle = &mut self.isle;
        let device = isle
            .services
            .network
            .state
            .wifi
            .as_ref()
            .map(|w| w.path.clone())
            .filter(|_| isle.services.network.state.wifi_enabled);
        let wanted = isle
            .island
            .panel
            .as_ref()
            .and_then(|p| p.page.as_deref())
            .is_some_and(|p| p.split('/').next() == Some("wifi"));
        match (
            wanted.then_some(device).flatten(),
            isle.services.scan.is_some(),
        ) {
            (Some(device), false) => {
                let Some(runtime) = &isle.worker else { return };
                isle.services.scan = Some(runtime.spawn(async move {
                    loop {
                        let _ = isle_network::scan(&device).await;
                        tokio::time::sleep(SCAN_EVERY).await;
                    }
                }));
            }
            (None, true) => {
                if let Some(task) = isle.services.scan.take() {
                    task.abort();
                }
            }
            _ => {}
        }
    }
}

impl Isle {
    /// A failed command's message in its module's place, or the place cleared.
    pub fn service_error(&mut self, key: &str, result: Result<(), String>, what: &str) {
        match result {
            Ok(()) => self.services.errors.remove(key),
            Err(err) => self
                .services
                .errors
                .insert(key.into(), format!("{what}: {err}")),
        };
        self.refresh_island();
    }

    /// The Wi-Fi glyph: on Wi-Fi, connecting, or with no connection at all.
    pub fn wifi_glyph(&self) -> Option<Glyph> {
        let net = &self.services.network;
        if !net.available {
            return None;
        }
        let state = &net.state;
        if let Some(wifi) = &state.wifi {
            match (&wifi.current, wifi.link) {
                (Some(current), Link::Connected) => {
                    return Some(Glyph {
                        active: true,
                        symbol: "wifi-high".into(),
                        level: Some(signal_glyph(current.strength).into()),
                        alt: format!(
                            "Wi-Fi: {}, signal {}",
                            current.ssid,
                            signal_word(current.strength)
                        ),
                        ..Default::default()
                    });
                }
                (_, Link::Connecting) => {
                    return Some(Glyph {
                        active: true,
                        symbol: "wifi-high".into(),
                        level: Some("wifi-none".into()),
                        alt: "Connecting to Wi-Fi".into(),
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
        state.offline().then(|| Glyph {
            active: true,
            symbol: "wifi-slash".into(),
            alt: "Not connected".into(),
            ..Default::default()
        })
    }

    /// The VPN glyph: a VPN connected or connecting.
    pub fn vpn_glyph(&self) -> Option<Glyph> {
        let net = &self.services.network;
        let vpn = net
            .state
            .vpns
            .iter()
            .find(|v| matches!(v.link, Link::Connected | Link::Connecting))
            .filter(|_| net.available)?;
        Some(Glyph {
            active: true,
            symbol: "isle-vpn".into(),
            alt: if vpn.link == Link::Connected {
                format!("VPN: {}", vpn.name)
            } else {
                "VPN: connecting".into()
            },
            ..Default::default()
        })
    }

    fn network_row(&self, n: &Network, current: bool) -> PageRow {
        let joining = self
            .services
            .joining
            .as_ref()
            .is_some_and(|(ssid, _)| *ssid == n.ssid);
        PageRow::Choice(Choice {
            id: n.path.clone(),
            label: n.ssid.clone(),
            lead: Lead::Glyph(if n.secured { "lock-simple" } else { "globe" }.into()),
            checked: current,
            busy: joining,
            detail: joining.then(|| "Connecting".into()),
            error: self
                .services
                .errors
                .get(&format!("wifi/{}", n.ssid))
                .cloned(),
            trailing: Some(signal_glyph(n.strength).into()),
            enabled: self.services.network.available,
            ..Default::default()
        })
    }

    pub fn wifi_module(&self, size: TileSize) -> Option<Module> {
        let net = &self.services.network;
        let wifi = net.state.wifi.as_ref()?;
        let on = net.state.wifi_enabled;
        let state = if !net.available {
            "Not available".to_owned()
        } else if !on {
            "Off".to_owned()
        } else if let (Some(current), Link::Connected) = (&wifi.current, wifi.link) {
            current.ssid.clone()
        } else if wifi.link == Link::Connecting || self.services.joining.is_some() {
            "Connecting".to_owned()
        } else {
            "Not connected".to_owned()
        };
        let mut rows = Vec::new();
        if let Some(wired) = &net.state.wired {
            rows.push(PageRow::Choice(Choice {
                id: "wired".into(),
                label: "Ethernet".into(),
                lead: Lead::Glyph("plugs-connected".into()),
                detail: Some(
                    match wired.link {
                        Link::Connected => "Connected",
                        Link::Unavailable => "Cable unplugged",
                        Link::Connecting => "Connecting",
                        _ => "Not connected",
                    }
                    .into(),
                ),
                enabled: false,
                ..Default::default()
            }));
        }
        if on {
            if let Some(current) = &wifi.current {
                rows.push(self.network_row(current, wifi.link == Link::Connected));
            }
            let (known, other): (Vec<&Network>, Vec<&Network>) =
                wifi.networks.iter().partition(|n| n.known.is_some());
            if !known.is_empty() {
                rows.push(PageRow::Heading("Known networks".into()));
                rows.extend(known.iter().map(|n| self.network_row(n, false)));
            }
            if wifi.current.is_none() && wifi.networks.is_empty() {
                rows.push(PageRow::Status("No networks found".into()));
            }
            if !other.is_empty() {
                rows.push(PageRow::Group {
                    id: "other".into(),
                    label: "Other networks".into(),
                    open: self.services.expanded.contains("wifi/other"),
                    rows: other.iter().map(|n| self.network_row(n, false)).collect(),
                });
            }
            rows.push(PageRow::Choice(Choice {
                id: "join-other".into(),
                label: "Join other network…".into(),
                enabled: true,
                ..Default::default()
            }));
        }
        if let Some(error) = self.services.errors.get("wifi") {
            rows.push(PageRow::Status(error.clone()));
        }
        Some(Module {
            id: "wifi".into(),
            size,
            name: "Wi-Fi".into(),
            state,
            glyph: if on { "wifi-high" } else { "wifi-slash" }.into(),
            on: on && net.available,
            toggles: net.available,
            page: Some(rows),
            pane: "Wi-Fi".into(),
            ..Default::default()
        })
    }

    pub fn vpn_module(&self, size: TileSize) -> Option<Module> {
        let net = &self.services.network;
        if net.state.vpns.is_empty() {
            return None;
        }
        let live = net
            .state
            .vpns
            .iter()
            .find(|v| matches!(v.link, Link::Connected | Link::Connecting));
        let state = match live {
            _ if !net.available => "Not available".to_owned(),
            Some(v) if v.link == Link::Connected => v.name.clone(),
            Some(_) => "Connecting".to_owned(),
            None => "Not connected".to_owned(),
        };
        let mut rows = vec![PageRow::Heading("Connections".into())];
        rows.extend(net.state.vpns.iter().map(|v| {
            PageRow::Choice(Choice {
                id: v.path.clone(),
                label: v.name.clone(),
                lead: Lead::Glyph("isle-vpn".into()),
                checked: v.link == Link::Connected,
                busy: v.link == Link::Connecting,
                enabled: net.available,
                ..Default::default()
            })
        }));
        if let Some(error) = self.services.errors.get("vpn") {
            rows.push(PageRow::Status(error.clone()));
        }
        Some(Module {
            id: "vpn".into(),
            size,
            name: "VPN".into(),
            state,
            glyph: "isle-vpn".into(),
            on: live.is_some_and(|v| v.link == Link::Connected),
            toggles: net.available,
            page: Some(rows),
            pane: "VPN".into(),
            ..Default::default()
        })
    }
}
