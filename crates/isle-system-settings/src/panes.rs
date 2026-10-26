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
        rows: &[
            RowDef::new("wi-fi", "Wi-Fi").keywords(&["wireless"]),
            RowDef::new("join-other", "Join other network").within("Join other network…").keywords(&["hidden network"]),
            RowDef::new("auto-join", "Automatically join this network").within("Details…"),
        ],
        ..pane("wi-fi", "Wi-Fi", "wifi-high", Hue::Blue, 1, "Join wireless networks and manage the ones you know.")
    },
    Pane {
        rows: &[
            RowDef::new("bluetooth", "Bluetooth").keywords(&["pair", "discoverable"]),
        ],
        ..pane("bluetooth", "Bluetooth", "bluetooth", Hue::Blue, 1, "Connect headphones, keyboards and other wireless devices.")
    },
    Pane {
        keywords: &["ethernet", "wired", "ip address"],
        rows: &[
            RowDef::new("ethernet", "Ethernet").keywords(&["wired", "lan"]),
            RowDef::new("ip-address", "IP address").within("Ethernet").keywords(&["dhcp", "router", "dns"]),
        ],
        ..pane("network", "Network", "globe", Hue::Blue, 1, "See how this computer is connected, and each connection's details.")
    },
    pane("vpn", "VPN", "isle-vpn", Hue::Blue, 1, "Turn your VPN connections on and off."),
    Pane {
        keywords: &["power", "performance", "power profile"],
        rows: &[
            RowDef::new("energy-mode", "Energy Mode").keywords(&["low power", "high power", "performance", "power saver"]),
        ],
        ..pane("energy", "Energy", "lightning", Hue::Orange, 1, "Choose how the computer trades speed for power.")
    },
    Pane {
        keywords: &["about", "system", "version"],
        rows: &[
            RowDef::new("about", "About").keywords(&["version", "system"]),
            RowDef::new("name", "Name").within("About").keywords(&["hostname", "computer name"]),
            RowDef::new("processor", "Processor").within("About").keywords(&["cpu"]),
            RowDef::new("memory", "Memory").within("About").keywords(&["ram"]),
            RowDef::new("graphics-0", "Graphics").within("About").keywords(&["gpu"]),
            RowDef::new("kernel", "Kernel").within("About").keywords(&["linux"]),
        ],
        ..pane("general", "General", "gear", Hue::Grey, 2, "About this computer and the system it runs.")
    },
    Pane {
        rows: &[
            RowDef::new("increase-contrast", "Increase contrast").key("appearance.increase-contrast"),
            RowDef::new("reduce-motion", "Reduce motion").key("appearance.reduce-motion").keywords(&["animation"]),
            RowDef::new("reduce-transparency", "Reduce transparency").key("appearance.reduce-transparency").keywords(&["blur"]),
            RowDef::new("text-size", "Text size").key("appearance.text-scale").keywords(&["font size", "larger text"]),
            RowDef::new("sticky-keys", "Sticky Keys").key("keyboard.sticky-keys"),
            RowDef::new("slow-keys", "Slow Keys").key("keyboard.slow-keys"),
        ],
        ..pane("accessibility", "Accessibility", "person-arms-spread", Hue::Blue, 2, "Make the screen and keyboard easier to see and use.")
    },
    Pane {
        keywords: &["dark mode", "light mode", "theme", "accent"],
        rows: &[
            RowDef::new("appearance", "Appearance").key("appearance.mode").keywords(&["dark", "light", "auto"]),
            RowDef::new("colour", "Colour").key("appearance.accent").keywords(&["accent", "color", "highlight"]),
            RowDef::new("glass", "Glass").key("appearance.glass").keywords(&["transparency", "tint"]),
            RowDef::new("title-bars", "Show title bars on other apps' windows").key("windows.title-bars").keywords(&["decorations", "csd"]),
            RowDef::new("tokens-file", "Tokens file").within("Advanced…").keywords(&["tokens", "theme", "design tokens", "tokens.kdl"]),
            RowDef::new("tokens-status", "Status").within("Advanced…").keywords(&["tokens", "theme"]),
        ],
        ..pane("appearance", "Appearance", "circle-half", Hue::Graphite, 2, "Choose light or dark, the accent colour and the glass.")
    },
    Pane {
        keywords: &["default apps", "default browser", "default terminal"],
        rows: &[
            RowDef::new("default-browser", "Web browser").keywords(&["browser", "default apps", "links"]),
            RowDef::new("default-mail", "Mail").keywords(&["email", "default apps", "mailto"]),
            RowDef::new("default-editor", "Text editor").keywords(&["editor", "default apps", "text files"]),
            RowDef::new("default-terminal", "Terminal").key("shortcuts.terminal-command").keywords(&["terminal", "default apps", "console"]),
        ],
        ..pane("apps", "Apps", "browsers", Hue::Indigo, 2, "Set the default apps, and how the desktop treats each app's windows and keys.")
    },
    Pane {
        keywords: &["island", "status icons", "tray", "menu bar"],
        rows: &[
            RowDef::new("clock", "Clock").keywords(&["time", "date"]),
            RowDef::new("clock-24-hour", "Time format").key("island.clock-24-hour").within("Clock Options…").keywords(&["24-hour", "12-hour"]),
            RowDef::new("clock-seconds", "Display the time with seconds").key("island.clock-seconds").within("Clock Options…"),
            RowDef::new("clock-weekday", "Show the day of the week").key("island.clock-weekday").within("Clock Options…"),
            RowDef::new("clock-date", "Show date").key("island.clock-date").within("Clock Options…"),
        ],
        ..pane("control-centre", "Control Centre", "squares-four", Hue::Purple, 2, "Set what the island and Control Centre show.")
    },
    Pane {
        keywords: &["windows", "tiling", "hot corners", "mission control", "stage manager", "spaces"],
        rows: &[
            RowDef::new("titlebar-action", "Double-click a window's title bar to").key("windows.titlebar-action").keywords(&["zoom", "fill"]),
            RowDef::new("edge-tiling", "Drag windows to screen edges to tile").key("windows.edge-tiling").keywords(&["snap", "tiling"]),
            RowDef::new("tile-margin", "Margins around tiled windows").key("windows.tile-margin").keywords(&["gaps", "spacing"]),
            RowDef::new("rearrange-desktops", "Automatically rearrange desktops based on most recent use").key("mission-control.rearrange-desktops").keywords(&["spaces", "mission control"]),
            RowDef::new("group-by-app", "Group windows by application").key("mission-control.group-by-app").keywords(&["mission control", "expose"]),
            RowDef::new("mission-shortcuts", "Mission Control shortcuts").within("Shortcuts…").keywords(&["mouse button", "app expose", "show desktop"]),
            RowDef::new("hot-corners", "Hot Corners").key("hot-corners").within("Hot Corners…").keywords(&["corner", "screen corners"]),
            RowDef::new("recent-apps", "Show recent apps").key("stage-manager.recent-apps").keywords(&["stage manager", "strip"]),
            RowDef::new("stage-windows", "Show windows from an application").key("stage-manager.windows").keywords(&["stage manager"]),
        ],
        ..pane("desktop", "Desktop", "app-window", Hue::Graphite, 2, "Set how windows tile and fill, and what the screen corners do.")
    },
    Pane {
        keywords: &["monitor", "screen", "resolution", "refresh rate", "night light"],
        rows: &[
            RowDef::new("main-display", "Main display").key("shell.main-screen").keywords(&["primary"]),
            RowDef::new("resolution", "Resolution").keywords(&["size"]),
            RowDef::new("refresh-rate", "Refresh rate").keywords(&["hz", "144"]),
            RowDef::new("scale", "Scale").keywords(&["hidpi", "zoom"]),
            RowDef::new("rotation", "Rotation").keywords(&["portrait", "rotate"]),
            RowDef::new("brightness", "Brightness").keywords(&["ddc"]),
            RowDef::new("vrr", "Variable refresh rate").keywords(&["freesync", "g-sync", "adaptive sync"]),
            RowDef::new("tearing", "Allow tearing in games").keywords(&["tearing"]),
            RowDef::new("schedule", "Night Light schedule").key("night-light.schedule").within("Night Light…").keywords(&["night shift", "blue light"]),
            RowDef::new("warmth", "Warmth").key("night-light.warmth").within("Night Light…").keywords(&["colour temperature"]),
            RowDef::new("graphics-card", "Graphics card").key("render-device").within("Advanced…").keywords(&["gpu", "render device"]),
        ],
        ..pane("displays", "Displays", "monitor", Hue::Blue, 2, "Arrange and set up the screens you use.")
    },
    Pane {
        keywords: &["search", "launcher"],
        rows: &[
            RowDef::new("source-menu-items", "Menu items").key("spotlight.sources-off").keywords(&["results"]),
            RowDef::new("web-search", "Web search").key("spotlight.web-search").keywords(&["search engine", "duckduckgo", "google"]),
            RowDef::new("favourites", "Favourites").key("spotlight.favourites"),
            RowDef::new("clipboard", "Don't keep copies from").key("spotlight.clipboard-exclude").keywords(&["clipboard history"]),
            RowDef::new("search-privacy", "Search Privacy").key("spotlight.files-exclude").within("Search Privacy…").keywords(&["folders", "exclude"]),
        ],
        ..pane("spotlight", "Spotlight", "magnifying-glass", Hue::Graphite, 2, "Choose what Spotlight searches and what it keeps.")
    },
    Pane {
        keywords: &["background", "desktop picture"],
        rows: &[
            RowDef::new("wallpaper", "Wallpaper").key("wallpaper.source").keywords(&["picture", "background"]),
            RowDef::new("all-screens", "Show on all screens"),
            RowDef::new("dark-mode", "Use a different picture in Dark Mode").key("wallpaper.dark-source"),
            RowDef::new("fit", "Fit").key("wallpaper.fit").keywords(&["fill", "centre"]),
        ],
        ..pane("wallpaper", "Wallpaper", "image", Hue::Teal, 2, "Choose the picture behind your windows.")
    },
    Pane {
        rows: &[
            RowDef::new("lock-screen", "On the lock screen").key("notifications.lock-screen").keywords(&["previews"]),
            RowDef::new("hold-full-screen", "Hold banners while a window is full screen").key("notifications.hold-full-screen").keywords(&["games"]),
            RowDef::new("hold-sharing", "Hold banners while sharing your screen").key("notifications.hold-sharing").keywords(&["screen sharing"]),
        ],
        ..pane("notifications", "Notifications", "bell", Hue::Red, 3, "Choose which apps can notify you, and how.")
    },
    Pane {
        keywords: &["audio", "volume", "speakers", "microphone"],
        rows: &[
            RowDef::new("alert-sound", "Alert sound").key("notifications.alert-sound").keywords(&["beep"]),
            RowDef::new("sounds", "Play sound for notifications").key("notifications.sounds"),
            RowDef::new("capture-sound", "Play sound when taking a screenshot").key("capture.sound"),
            RowDef::new("output-volume", "Output volume").keywords(&["speakers", "headphones"]),
            RowDef::new("balance", "Balance").keywords(&["left", "right"]),
            RowDef::new("input-volume", "Input volume").keywords(&["microphone", "mic"]),
        ],
        ..pane("sound", "Sound", "speaker-high", Hue::Pink, 3, "Choose alert sounds, speakers and microphones.")
    },
    Pane {
        keywords: &["do not disturb", "game mode"],
        rows: &[
            RowDef::new("Do Not Disturb", "Do Not Disturb").keywords(&["dnd"]),
            RowDef::new("Game Mode", "Game Mode").keywords(&["gaming"]),
        ],
        ..pane("focus", "Focus", "moon", Hue::Indigo, 3, "Choose what interrupts you, and when.")
    },
    Pane {
        keywords: &["idle", "screen saver", "sleep"],
        rows: &[
            RowDef::new("dim-after", "Dim the display after").key("lock-screen.dim-after").keywords(&["idle", "screen saver"]),
            RowDef::new("lock-after", "Lock after").key("lock-screen.lock-after").keywords(&["password"]),
            RowDef::new("display-off-after", "Turn display off after").key("lock-screen.display-off-after").keywords(&["sleep", "blank"]),
            RowDef::new("lock-on-sleep", "Lock when the computer sleeps").key("lock-screen.lock-on-sleep").keywords(&["suspend"]),
        ],
        ..pane("lock-screen", "Lock Screen", "lock", Hue::Graphite, 4, "Set when the screen dims, turns off and locks.")
    },
    Pane {
        keywords: &["permissions", "screen recording", "screenshots"],
        rows: &[
            RowDef::new("screencast", "Screen & System Audio Recording").keywords(&["screen sharing", "permissions"]),
            RowDef::new("screenshot", "Screenshots").keywords(&["permissions"]),
        ],
        ..pane("privacy-security", "Privacy & Security", "hand-palm", Hue::Blue, 4, "See what apps may do without asking, and take it back.")
    },
    Pane {
        keywords: &["shortcuts", "layout", "input sources", "xkb"],
        rows: &[
            RowDef::new("repeat-rate", "Key repeat rate").key("keyboard.repeat-rate"),
            RowDef::new("repeat-delay", "Delay until repeat").key("keyboard.repeat-delay"),
            RowDef::new("input-sources", "Input sources").key("keyboard.layouts").keywords(&["layout", "language", "xkb"]),
            RowDef::new("numlock", "Turn on Num Lock at login").key("keyboard.numlock"),
            RowDef::new("translate", "Use Mac shortcuts in other apps").key("keyboard.translate").keywords(&["cmd", "command", "translation"]),
            RowDef::new("caps-lock", "Caps Lock key").key("keyboard.options").within("Modifier Keys…").keywords(&["escape", "control", "modifier"]),
            RowDef::new("keyboard-shortcuts", "Keyboard Shortcuts").key("shortcuts").within("Keyboard Shortcuts…").keywords(&["hotkeys", "bindings", "keybinds"]),
        ],
        ..pane("keyboard", "Keyboard", "keyboard", Hue::Grey, 6, "Set key repeat, layouts, shortcuts and each keyboard's kind.")
    },
    Pane {
        keywords: &["pointer", "scrolling"],
        rows: &[
            RowDef::new("speed", "Tracking speed").keywords(&["sensitivity", "pointer speed"]),
            RowDef::new("acceleration", "Pointer acceleration").keywords(&["accel"]),
            RowDef::new("natural-scroll", "Natural scrolling").keywords(&["scroll direction"]),
            RowDef::new("primary-button", "Primary mouse button").keywords(&["left-handed"]),
        ],
        ..pane("mouse", "Mouse", "mouse", Hue::Grey, 6, "Set pointer speed, scrolling and the primary button.")
    },
];

pub fn find(id: &str) -> Option<&'static Pane> {
    PANES.iter().find(|p| p.id == id)
}

/// The pane Settings first opens on (`settings.md`, First run).
pub const FIRST: &str = "general";

/// The index Spotlight reads, one `row` per pane and per row (`isle-search`'s settings
/// source). Written by `--write-index`, which only debug builds have.
#[cfg(any(test, debug_assertions))]
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
