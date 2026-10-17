//! The panes in the sidebar's order and their rows: the record that search, the index
//! Spotlight reads and the panes' own drawing all come from (`p13-settings.md` choice 16).

/// One row's identity and the words it is found by.
#[derive(Clone, Debug, PartialEq)]
pub struct RowDef {
    pub id: &'static str,
    pub label: &'static str,
    /// Words that find it without being shown: other desktops' names, Linux's.
    pub keywords: &'static [&'static str],
    /// The `settings.kdl` key it shows, when it shows one: Show changed settings marks it.
    pub key: Option<&'static str>,
    /// A sub-page or sheet the row is on, opened before it is marked.
    pub within: Option<&'static str>,
}

impl RowDef {
    pub const fn new(id: &'static str, label: &'static str) -> Self {
        Self {
            id,
            label,
            keywords: &[],
            key: None,
            within: None,
        }
    }

    pub const fn key(self, key: &'static str) -> Self {
        Self { key: Some(key), ..self }
    }

    pub const fn keywords(self, keywords: &'static [&'static str]) -> Self {
        Self { keywords, ..self }
    }

    pub const fn within(self, within: &'static str) -> Self {
        Self {
            within: Some(within),
            ..self
        }
    }
}

/// The tile's hue, by the token's name in `pane { }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Blue,
    Grey,
    Graphite,
    Purple,
    Red,
    Pink,
    Indigo,
    Teal,
    Green,
    Orange,
}

impl Hue {
    pub fn colour(self, t: &isle_ui::isle_tokens::Tokens) -> isle_ui::isle_tokens::Colour {
        match self {
            Hue::Blue => t.pane_blue,
            Hue::Grey => t.pane_grey,
            Hue::Graphite => t.pane_graphite,
            Hue::Purple => t.pane_purple,
            Hue::Red => t.pane_red,
            Hue::Pink => t.pane_pink,
            Hue::Indigo => t.pane_indigo,
            Hue::Teal => t.pane_teal,
            Hue::Green => t.pane_green,
            Hue::Orange => t.pane_orange,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub id: &'static str,
    pub name: &'static str,
    pub glyph: &'static str,
    pub hue: Hue,
    /// Panes are listed in groups, space between them.
    pub group: u8,
    pub subtitle: &'static str,
    pub keywords: &'static [&'static str],
    pub rows: &'static [RowDef],
}

const fn pane(
    id: &'static str,
    name: &'static str,
    glyph: &'static str,
    hue: Hue,
    group: u8,
    subtitle: &'static str,
) -> Pane {
    Pane {
        id,
        name,
        glyph,
        hue,
        group,
        subtitle,
        keywords: &[],
        rows: &[],
    }
}

/// Every pane P13 builds, in the sidebar's order (`p1/structure/settings.md`).
pub static PANES: &[Pane] = &[
    Pane {
        keywords: &["wireless", "wlan", "network"],
        ..pane("wi-fi", "Wi-Fi", "wifi-high", Hue::Blue, 1, "Join networks and choose which to remember.")
    },
    pane("bluetooth", "Bluetooth", "bluetooth", Hue::Blue, 1, "Connect headphones, keyboards and other devices."),
    Pane {
        keywords: &["ethernet", "wired", "ip address"],
        ..pane("network", "Network", "globe", Hue::Blue, 1, "See how this computer is connected.")
    },
    pane("vpn", "VPN", "isle-vpn", Hue::Blue, 1, "Connect to private networks."),
    Pane {
        keywords: &["power", "performance", "power profile"],
        ..pane("energy", "Energy", "lightning", Hue::Orange, 1, "Choose how the computer balances speed and power.")
    },
    Pane {
        keywords: &["about", "system", "version"],
        ..pane("general", "General", "gear", Hue::Grey, 2, "Manage this computer's overall settings.")
    },
    pane("accessibility", "Accessibility", "person-arms-spread", Hue::Blue, 2, "Adjust the display, keyboard and pointer to suit you."),
    Pane {
        keywords: &["dark mode", "light mode", "theme", "accent"],
        ..pane("appearance", "Appearance", "circle-half", Hue::Graphite, 2, "Choose how Isle looks.")
    },
    pane("apps", "Apps", "browsers", Hue::Indigo, 2, "Set how the desktop treats each app."),
    Pane {
        keywords: &["island", "status icons", "tray", "menu bar"],
        ..pane("control-centre", "Control Centre", "squares-four", Hue::Purple, 2, "Set what the island and Control Centre show.")
    },
    Pane {
        keywords: &["windows", "tiling", "hot corners"],
        ..pane("desktop", "Desktop", "app-window", Hue::Graphite, 2, "Set how windows behave on the desktop.")
    },
    Pane {
        keywords: &["monitor", "screen", "resolution", "refresh rate", "night light"],
        ..pane("displays", "Displays", "monitor", Hue::Blue, 2, "Arrange and set up the screens you use.")
    },
    Pane {
        keywords: &["search", "launcher"],
        ..pane("spotlight", "Spotlight", "magnifying-glass", Hue::Graphite, 2, "Choose what Spotlight searches and shows.")
    },
    Pane {
        keywords: &["background", "desktop picture"],
        ..pane("wallpaper", "Wallpaper", "image", Hue::Teal, 2, "Choose the picture behind your windows.")
    },
    pane("notifications", "Notifications", "bell", Hue::Red, 3, "Choose which apps notify you, and how."),
    Pane {
        keywords: &["audio", "volume", "speakers", "microphone"],
        ..pane("sound", "Sound", "speaker-high", Hue::Pink, 3, "Choose where sound plays and how it sounds.")
    },
    Pane {
        keywords: &["do not disturb", "game mode"],
        ..pane("focus", "Focus", "moon", Hue::Indigo, 3, "Choose what interrupts you, and when.")
    },
    Pane {
        keywords: &["idle", "screen saver", "sleep"],
        ..pane("lock-screen", "Lock Screen", "lock", Hue::Graphite, 4, "Set when the screen dims, locks and turns off.")
    },
    Pane {
        keywords: &["permissions", "screen recording", "screenshots"],
        ..pane("privacy-security", "Privacy & Security", "hand-palm", Hue::Blue, 4, "Review what apps have been allowed to do.")
    },
    Pane {
        keywords: &["shortcuts", "layout", "input sources", "xkb"],
        ..pane("keyboard", "Keyboard", "keyboard", Hue::Grey, 6, "Set up your keyboards, layouts and shortcuts.")
    },
    Pane {
        keywords: &["pointer", "scrolling"],
        ..pane("mouse", "Mouse", "mouse", Hue::Grey, 6, "Adjust how the pointer moves and scrolls.")
    },
];

pub fn find(id: &str) -> Option<&'static Pane> {
    PANES.iter().find(|p| p.id == id)
}

/// The pane Settings first opens on (`settings.md`, First run).
pub const FIRST: &str = "general";

/// The index Spotlight reads, one `row` per pane and per row (`isle-search`'s settings
/// source).
pub fn index() -> String {
    let quote = isle_settings::quote;
    let mut out = String::from(
        "// Written by `isle-settings --write-index` from the panes; a test keeps it current.\n",
    );
    for pane in PANES {
        let mut write = |id: &str, label: &str, also: &[&str]| {
            out.push_str(&format!(
                "row pane={} id={} label={} pane-label={}",
                quote(pane.id),
                quote(id),
                quote(label),
                quote(pane.name)
            ));
            if !also.is_empty() {
                out.push_str(&format!(" also={}", quote(&also.join(";"))));
            }
            out.push('\n');
        };
        write("", pane.name, pane.keywords);
        for row in pane.rows {
            write(row.id, row.label, row.keywords);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_the_first_pane_exists() {
        let mut ids: Vec<&str> = PANES.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PANES.len());
        assert!(find(FIRST).is_some());
        for pane in PANES {
            let mut rows: Vec<&str> = pane.rows.iter().map(|r| r.id).collect();
            rows.sort();
            rows.dedup();
            assert_eq!(rows.len(), pane.rows.len(), "{}", pane.id);
            assert!(isle_ui::glyph_svg(pane.glyph, true).is_some(), "{}'s glyph", pane.id);
        }
    }

    #[test]
    fn the_installed_index_is_the_panes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/settings/index.kdl");
        let committed = std::fs::read_to_string(path).unwrap();
        assert!(
            committed == index(),
            "data/settings/index.kdl is stale: run `isle-settings --write-index`"
        );
    }
}
