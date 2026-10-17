//! Control Centre's modules as a catalogue, the Accessibility module, and edit mode with
//! its gallery (`p8-services.md` choice 18, `structure/control-centre.md`).

use isle_shell::island::{
    grid::TileSize,
    panel::{EditFrame, GalleryModule, Module, PanelAction},
    rows::PageRow,
};

use crate::state::{Isle, State};

use TileSize::{Large as L, Medium as M, Small as S, Wide as W};

/// A module Isle has: what it is called, the sizes it comes in, and where it belongs.
pub struct Entry {
    pub id: &'static str,
    pub name: &'static str,
    pub glyph: &'static str,
    pub group: &'static str,
    pub sizes: &'static [TileSize],
    pub pane: &'static str,
    /// Why it is missing, when its source is.
    pub missing: &'static str,
}

const NOT_YET: &str = "Not in this version of Isle.";

/// Every module, in the gallery's order.
pub const CATALOGUE: &[Entry] = &[
    Entry {
        id: "wifi",
        name: "Wi-Fi",
        glyph: "wifi-high",
        group: "Connectivity",
        sizes: &[S, M],
        pane: "Wi-Fi",
        missing: "No Wi-Fi device is connected.",
    },
    Entry {
        id: "bluetooth",
        name: "Bluetooth",
        glyph: "bluetooth",
        group: "Connectivity",
        sizes: &[S, M],
        pane: "Bluetooth",
        missing: "No Bluetooth adapter is connected.",
    },
    Entry {
        id: "vpn",
        name: "VPN",
        glyph: "isle-vpn",
        group: "Connectivity",
        sizes: &[S, M],
        pane: "Network",
        missing: "No VPN is set up.",
    },
    Entry {
        id: "now-playing",
        name: "Now Playing",
        glyph: "music-note",
        group: "Sound and display",
        sizes: &[S, M, L, W],
        pane: "Sound",
        missing: "",
    },
    Entry {
        id: "sound",
        name: "Sound",
        glyph: "speaker-high",
        group: "Sound and display",
        sizes: &[S, M, W],
        pane: "Sound",
        missing: "The sound service is not running.",
    },
    Entry {
        id: "display",
        name: "Display",
        glyph: "sun",
        group: "Sound and display",
        sizes: &[M, W],
        pane: "Displays",
        missing: "No screen's brightness can be set.",
    },
    Entry {
        id: "night-light",
        name: "Night Light",
        glyph: "sun-horizon",
        group: "Sound and display",
        sizes: &[S, M],
        pane: "Displays",
        missing: "",
    },
    Entry {
        id: "dark-mode",
        name: "Dark Mode",
        glyph: "circle-half",
        group: "Sound and display",
        sizes: &[S, M],
        pane: "Appearance",
        missing: "",
    },
    Entry {
        id: "focus",
        name: "Focus",
        glyph: "moon",
        group: "Focus",
        sizes: &[S, M],
        pane: "Focus",
        missing: "",
    },
    Entry {
        id: "energy-mode",
        name: "Energy Mode",
        glyph: "gauge",
        group: "System",
        sizes: &[S, M],
        pane: "Energy",
        missing: "Power profiles are not available.",
    },
    Entry {
        id: "updates",
        name: "Updates",
        glyph: "arrows-clockwise",
        group: "System",
        sizes: &[S, M],
        pane: "Software Update",
        missing: NOT_YET,
    },
    Entry {
        id: "batteries",
        name: "Batteries",
        glyph: "battery-full",
        group: "System",
        sizes: &[S, M, L],
        pane: "Energy",
        missing: "No battery is reported.",
    },
    Entry {
        id: "keyboard-layout",
        name: "Keyboard layout",
        glyph: "keyboard",
        group: "System",
        sizes: &[S, M],
        pane: "Keyboard",
        missing: "",
    },
    Entry {
        id: "screenshot",
        name: "Screenshot",
        glyph: "selection",
        group: "System",
        sizes: &[S],
        pane: "Keyboard",
        missing: "",
    },
    Entry {
        id: "accessibility",
        name: "Accessibility",
        glyph: "person-arms-spread",
        group: "Accessibility",
        sizes: &[S, M],
        pane: "Accessibility",
        missing: "",
    },
    Entry {
        id: "phone",
        name: "Phone",
        glyph: "device-mobile",
        group: "Providers",
        sizes: &[S, M],
        pane: "Phone",
        missing: "No phone provider is set up.",
    },
];

pub fn entry(id: &str) -> Option<&'static Entry> {
    CATALOGUE.iter().find(|e| e.id == id)
}

/// The arrangement's entries as `(id, size)`.
fn parse(modules: &[String]) -> Vec<(String, TileSize)> {
    modules
        .iter()
        .map(|entry| {
            let (id, size) = entry.split_once(':').unwrap_or((entry.as_str(), "m"));
            (
                id.to_owned(),
                TileSize::from_name(size).unwrap_or(TileSize::Medium),
            )
        })
        .collect()
}

/// A size as `control-centre.modules` writes it.
pub fn size_letter(size: TileSize) -> &'static str {
    match size {
        S => "s",
        M => "m",
        L => "l",
        W => "w",
    }
}

fn write(arrangement: &[(String, TileSize)]) -> Vec<String> {
    arrangement
        .iter()
        .map(|(id, size)| format!("{id}:{}", size_letter(*size)))
        .collect()
}

impl Isle {
    pub fn accessibility_module(&self, size: TileSize) -> Option<Module> {
        let a = &self.settings.appearance;
        let on = [a.increase_contrast, a.reduce_motion, a.reduce_transparency]
            .iter()
            .filter(|on| **on)
            .count();
        let switch = |id: &str, label: &str, on: bool| PageRow::Switch {
            id: id.into(),
            label: label.into(),
            on,
            enabled: true,
        };
        Some(Module {
            id: "accessibility".into(),
            size,
            name: "Accessibility".into(),
            state: match on {
                0 => "Off".into(),
                1 => "1 setting on".into(),
                n => format!("{n} settings on"),
            },
            glyph: "person-arms-spread".into(),
            page: Some(vec![
                switch(
                    "increase-contrast",
                    "Increase contrast",
                    a.increase_contrast,
                ),
                switch("reduce-motion", "Reduce motion", a.reduce_motion),
                switch(
                    "reduce-transparency",
                    "Reduce transparency",
                    a.reduce_transparency,
                ),
            ]),
            pane: "Accessibility".into(),
            plain_head: true,
            ..Default::default()
        })
    }

    /// Edit mode's gallery: every module not in the arrangement, those whose source is
    /// missing here with why.
    pub fn edit_frame(&self, menu: Option<String>) -> EditFrame {
        let placed = parse(&self.settings.control_centre.modules);
        let gallery = CATALOGUE
            .iter()
            .filter(|e| !placed.iter().any(|(id, _)| id == e.id))
            .map(|e| GalleryModule {
                id: e.id.into(),
                name: e.name.into(),
                glyph: e.glyph.into(),
                group: e.group.into(),
                why: self
                    .module_of(e.id, e.sizes[0])
                    .is_none()
                    .then(|| e.missing.to_owned()),
                pane: e.pane.into(),
            })
            .collect();
        EditFrame { gallery, menu }
    }
}

impl State {
    fn arrange(&mut self, change: impl FnOnce(&mut Vec<(String, TileSize)>)) {
        let mut arrangement = parse(&self.isle.settings.control_centre.modules);
        change(&mut arrangement);
        let modules = write(&arrangement);
        // In force before the write comes back.
        self.isle.settings.control_centre.modules = modules.clone();
        self.isle.write_setting(
            "control-centre.modules",
            isle_settings::Value::List(modules),
        );
    }

    pub fn edit_action(&mut self, action: PanelAction) {
        let Some(mut panel) = self.isle.island.panel.clone() else {
            return;
        };
        match action {
            PanelAction::Edit(on) => {
                panel.edit = on;
                panel.edit_menu = None;
                panel.page = None;
                if on && !panel.pinned {
                    self.isle.island.panel = Some(panel);
                    let output = self.isle.island.panel.as_ref().map(|p| p.output.clone());
                    let output = output.and_then(|o| {
                        self.isle
                            .space
                            .outputs()
                            .find(|out| out.name() == o)
                            .cloned()
                    });
                    if let Some(output) = output {
                        self.open_control_centre(&output, true, None);
                    }
                    return;
                }
            }
            PanelAction::EditMenu(id) => panel.edit_menu = id,
            PanelAction::EditRemove(id) => {
                panel.edit_menu = None;
                self.arrange(|a| a.retain(|(m, _)| *m != id));
            }
            PanelAction::EditAdd(id) => {
                if let Some(entry) = entry(&id)
                    && self.isle.module_of(&id, entry.sizes[0]).is_some()
                {
                    let size = default_size(entry);
                    self.arrange(|a| {
                        if !a.iter().any(|(m, _)| *m == id) {
                            a.push((id, size));
                        }
                    });
                }
            }
            PanelAction::EditSize { id, size } => {
                panel.edit_menu = None;
                if entry(&id).is_some_and(|e| e.sizes.contains(&size)) {
                    self.arrange(|a| {
                        if let Some(m) = a.iter_mut().find(|(m, _)| *m == id) {
                            m.1 = size;
                        }
                    });
                }
            }
            PanelAction::EditMove { id, to } => {
                // `to` is a place among the modules shown; absent ones keep their places.
                let shown: Vec<String> = self
                    .isle
                    .panel_modules()
                    .into_iter()
                    .map(|m| m.id)
                    .collect();
                let Some(target) = shown.get(to).cloned().filter(|t| *t != id) else {
                    return;
                };
                self.arrange(|a| {
                    let Some(from) = a.iter().position(|(m, _)| *m == id) else {
                        return;
                    };
                    let item = a.remove(from);
                    let at = a.iter().position(|(m, _)| *m == target).unwrap_or(a.len());
                    let at = if from <= at { at + 1 } else { at };
                    a.insert(at.min(a.len()), item);
                });
            }
            _ => return,
        }
        self.isle.island.panel = Some(panel);
    }

    pub fn accessibility_switch(&mut self, row: &str, on: bool) {
        let key = match row {
            "increase-contrast" => "appearance.increase-contrast",
            "reduce-motion" => "appearance.reduce-motion",
            "reduce-transparency" => "appearance.reduce-transparency",
            _ => return,
        };
        let a = &mut self.isle.settings.appearance;
        match row {
            "increase-contrast" => a.increase_contrast = on,
            "reduce-motion" => a.reduce_motion = on,
            _ => a.reduce_transparency = on,
        }
        self.isle.write_setting(key, isle_settings::Value::Bool(on));
    }
}

/// The size a module takes when added: the design's default for it.
fn default_size(entry: &Entry) -> TileSize {
    match entry.id {
        "now-playing" => L,
        "sound" | "display" => W,
        "night-light" | "energy-mode" | "dark-mode" | "screenshot" => S,
        _ => M,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrangement_reads_and_writes_the_same() {
        let modules: Vec<String> = ["wifi:m", "now-playing:l", "sound:w", "night-light:s"]
            .map(String::from)
            .to_vec();
        assert_eq!(write(&parse(&modules)), modules);
    }

    #[test]
    fn every_module_has_its_default_size_among_its_sizes() {
        for entry in CATALOGUE {
            assert!(entry.sizes.contains(&default_size(entry)), "{}", entry.id);
        }
    }
}
