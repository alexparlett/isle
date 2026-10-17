//! System Settings opened at a pane and row, from wherever the desktop links to it
//! (`p13-settings.md` choice 17).

use crate::{launch::Launch, state::State};

pub const SETTINGS_APP: &str = "io.github.alexparlett.Isle.Settings";

/// A pane's id from the name Control Centre gives it: `Wi-Fi` is `wi-fi`, `Privacy &
/// Security` is `privacy-security`. Battery is Energy's other name; Software Update is in
/// General.
pub fn pane_id(name: &str) -> String {
    match name {
        "Battery" => "energy".into(),
        "Software Update" => "general".into(),
        _ => name
            .to_lowercase()
            .replace(" & ", "-")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-"),
    }
}

/// The link Settings takes: `isle-settings://<pane>[/<row>]`.
pub fn uri(pane: &str, row: Option<&str>) -> String {
    match row.filter(|r| !r.is_empty()) {
        Some(row) => format!("isle-settings://{pane}/{row}"),
        None => format!("isle-settings://{pane}"),
    }
}

impl State {
    /// Opens Settings, at `pane` and `row` when given: through its bus name when its entry is
    /// installed, else the `isle-settings` built beside the compositor.
    pub fn open_settings(&mut self, pane: Option<&str>, row: Option<&str>) -> bool {
        let uris: Vec<String> = pane.map(|p| uri(p, row)).into_iter().collect();
        if self.isle.apps.get(SETTINGS_APP).is_some() {
            return self.launch(Launch::App {
                app: SETTINGS_APP.into(),
                action: None,
                uris,
            });
        }
        let Some(beside) = std::env::current_exe()
            .ok()
            .map(|exe| exe.with_file_name("isle-settings"))
            .filter(|p| p.is_file())
        else {
            tracing::warn!("System Settings is not installed");
            return false;
        };
        let mut line = format!("'{}'", beside.display());
        for uri in uris {
            line.push_str(&format!(" '{uri}'"));
        }
        self.launch(Launch::Command(line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_centres_names_become_pane_ids() {
        assert_eq!(pane_id("Wi-Fi"), "wi-fi");
        assert_eq!(pane_id("Control Centre"), "control-centre");
        assert_eq!(pane_id("Privacy & Security"), "privacy-security");
        assert_eq!(pane_id("Battery"), "energy");
        assert_eq!(uri("displays", Some("")), "isle-settings://displays");
        assert_eq!(uri("displays", Some("scale")), "isle-settings://displays/scale");
    }
}
