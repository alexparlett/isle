//! Batteries and energy modes: the Battery glyph, and the Energy Mode and Batteries modules.

use isle_bus::Snapshot;
use isle_power::{Kind, LOW, Power, Profiles};
use isle_shell::island::{
    grid::TileSize,
    model::Glyph,
    panel::Module,
    rows::{Choice, Lead, PageRow},
};

use crate::state::{Isle, State};

/// The application id Isle holds profiles under.
pub const APP_ID: &str = "io.github.alexparlett.Isle";

const MODES: [(&str, &str, &str); 3] = [
    ("power-saver", "Low Power", "leaf"),
    ("balanced", "Automatic", "gauge"),
    ("performance", "High Power", "lightning"),
];

fn level_symbol(level: u8, charging: bool) -> &'static str {
    match (charging, level) {
        (true, _) => "battery-charging",
        (_, 0..=LOW) => "battery-warning",
        (_, 21..=40) => "battery-low",
        (_, 41..=65) => "battery-medium",
        (_, 66..=90) => "battery-high",
        _ => "battery-full",
    }
}

fn time_left(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes >= 60 {
        format!("{}:{:02} left", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min left")
    }
}

impl State {
    pub fn power_changed(&mut self, snapshot: Snapshot<Power>) {
        let before = self.isle.services.power.state.keyboard;
        let was = self.isle.services.power.available;
        self.isle.services.power = snapshot;
        let now = self.isle.services.power.state.keyboard;
        if was && let (Some(_), Some((level, max))) = (before, now)
            && before != now
        {
            self.isle
                .osd_level("keyboard", "Keyboard", level as f32 / max as f32, false, None);
        }
        self.isle.services_changed();
    }

    pub fn profiles_changed(&mut self, snapshot: Snapshot<Profiles>) {
        self.isle.services.profiles = snapshot;
        self.isle.services_changed();
    }
}

impl Isle {
    /// Isle's own hold on the performance profile, while Game Mode keeps it.
    pub fn profile_held(&self) -> bool {
        let p = &self.services.profiles.state;
        p.active == "performance" && p.holds.iter().any(|h| h.app == APP_ID)
    }

    /// The Battery glyph: shown while the machine runs on its battery or any battery is low.
    pub fn battery_glyph(&self) -> Option<Glyph> {
        let power = &self.services.power;
        if !power.available {
            return None;
        }
        let low = power
            .state
            .batteries
            .iter()
            .filter(|b| !b.charging)
            .filter_map(|b| b.level.map(|l| (b, l)))
            .filter(|(_, l)| *l <= LOW)
            .min_by_key(|(_, l)| *l);
        let system = power.state.system().filter(|_| power.state.on_battery);
        let (battery, level) = match (low, system) {
            (Some(low), _) => low,
            (None, Some(b)) => (b, b.level?),
            (None, None) => return None,
        };
        let alt = match (battery.kind, battery.charging, level <= LOW) {
            (Kind::System, true, _) => format!("Battery {level}%, charging"),
            (Kind::System, false, true) => format!("Battery low, {level}%"),
            (Kind::System, false, false) => format!("Battery {level}%"),
            (_, _, _) => format!("{} battery low, {level}%", battery.name),
        };
        Some(Glyph {
            active: true,
            symbol: level_symbol(level, battery.charging).into(),
            alt,
            ..Default::default()
        })
    }

    pub fn energy_module(&self, size: TileSize) -> Option<Module> {
        let profiles = &self.services.profiles;
        if !profiles.available && profiles.state.choices.is_empty() {
            return None;
        }
        let p = &profiles.state;
        let held = self.profile_held();
        let (name, glyph) = MODES
            .iter()
            .find(|(id, ..)| *id == p.active)
            .map_or(("Automatic", "gauge"), |(_, name, glyph)| (*name, *glyph));
        let state = if !profiles.available {
            "Not available".to_owned()
        } else if held {
            "High Power while Game Mode is on".to_owned()
        } else {
            name.to_owned()
        };
        let error = self.services.errors.get("energy-mode").cloned();
        let rows = MODES
            .iter()
            .filter(|(id, ..)| p.choices.iter().any(|c| c == id))
            .map(|(id, label, glyph)| {
                PageRow::Choice(Choice {
                    id: (*id).into(),
                    label: (*label).into(),
                    lead: Lead::Glyph((*glyph).into()),
                    checked: p.active == *id,
                    detail: (held && *id == "performance").then(|| "While Game Mode is on".into()),
                    error: error.clone().filter(|_| p.active == *id),
                    enabled: profiles.available,
                    ..Default::default()
                })
            })
            .collect();
        Some(Module {
            id: "energy-mode".into(),
            size,
            name: "Energy Mode".into(),
            state,
            glyph: glyph.into(),
            on: false,
            toggles: false,
            page: Some(rows),
            page_title: None,
            pane: "Energy".into(),
            ..Default::default()
        })
    }

    pub fn batteries_module(&self, size: TileSize) -> Option<Module> {
        let power = &self.services.power;
        let batteries = &power.state.batteries;
        if batteries.is_empty() {
            return None;
        }
        let shown = power.state.system().or_else(|| {
            batteries
                .iter()
                .filter(|b| b.level.is_some())
                .min_by_key(|b| b.level)
        });
        let state = if !power.available {
            "Not available".to_owned()
        } else {
            match shown {
                Some(b) if b.kind == Kind::System => match (b.level, b.charging) {
                    (Some(l), true) => format!("{l}%, Charging"),
                    (Some(l), false) => format!("{l}%"),
                    (None, _) => String::new(),
                },
                Some(b) => format!("{} {}%", b.name, b.level.unwrap_or(0)),
                None => String::new(),
            }
        };
        let glyph = shown
            .and_then(|b| b.level.map(|l| level_symbol(l, b.charging)))
            .unwrap_or("battery-full");
        let rows = batteries
            .iter()
            .map(|b| {
                let detail = if b.charging {
                    Some("Charging".to_owned())
                } else {
                    b.time_to_empty.map(time_left)
                };
                PageRow::Choice(Choice {
                    id: b.path.clone(),
                    label: b.name.clone(),
                    lead: Lead::Glyph(b.kind.glyph().into()),
                    battery: b.level,
                    detail,
                    enabled: true,
                    ..Default::default()
                })
            })
            .collect();
        Some(Module {
            id: "batteries".into(),
            size,
            name: "Batteries".into(),
            state,
            glyph: glyph.into(),
            on: false,
            toggles: false,
            page: Some(rows),
            page_title: None,
            pane: "Battery".into(),
            ..Default::default()
        })
    }
}

impl State {
    /// A row of Energy Mode's page: the user's choice of profile, which ends any hold.
    pub fn energy_choose(&mut self, profile: String) {
        self.isle.services.errors.remove("energy-mode");
        self.isle.run_service(
            async move { isle_power::set_profile(&profile).await },
            |state, result| {
                if let Err(err) = result {
                    state.isle.services.errors.insert(
                        "energy-mode".into(),
                        format!("Couldn't change the energy mode: {err}"),
                    );
                    state.isle.refresh_island();
                }
            },
        );
    }
}
