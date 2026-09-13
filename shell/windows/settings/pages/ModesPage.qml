import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Modes"
    subtitle: "Game mode, Big Picture, and what turns them on."

    SettingsGroup {
        heading: "Now"
        SettingsRow {
            label: "Current mode"
            description: Modes.current
            RowLayout {
                spacing: Theme.s2
                Button { text: "Normal"; variant: Modes.current === "normal" ? "accent" : "raised"; onClicked: Modes.set("normal") }
                Button { text: "Game"; variant: Modes.current === "game" ? "accent" : "raised"; onClicked: Modes.set("game") }
            }
        }
    }

    SettingsGroup {
        heading: "Game mode"
        SettingsRow {
            label: "Enter automatically"
            description: "When a game is running, or Steam is fullscreen."
            Toggle { checked: Prefs.p.gameAuto; onToggled: v => Prefs.p.gameAuto = v }
        }
        SettingsRow {
            label: "Allow tearing"
            description: "For the game window only, while game mode is on."
            Toggle { checked: Prefs.p.gameTearing; onToggled: v => Prefs.p.gameTearing = v }
        }
    }

    SettingsGroup {
        heading: "Big Picture"
        SettingsRow {
            label: "Steam through gamescope"
            description: "Steam's gamepad UI in a nested compositor."
            Toggle { checked: Prefs.p.gamescope; onToggled: v => Prefs.p.gamescope = v }
        }
        SettingsRow {
            label: "Steam's Big Picture switches Isle's mode"
            description: "Off, Steam's Big Picture is just another window."
            visible: Games.tools.steam
            Toggle { checked: Prefs.p.steamFollowsMode; onToggled: v => Prefs.p.steamFollowsMode = v }
        }
        SettingsRow {
            label: "Controller opens Big Picture"
            description: "Hold the Guide button on the desktop."
            Toggle { checked: Prefs.p.padHome; onToggled: v => Prefs.p.padHome = v }
        }
        SettingsRow {
            label: "Isle inside Steam Big Picture"
            description: Games.shortcutsError ? Games.shortcutsError
                       : Games.shortcutsInstalled ? "Two tiles in Steam's library, Isle settings and Desktop, open the quick menu and the way out."
                       : Games.steamRunning ? "Adds Isle settings and Desktop to Steam's library. Close Steam first." : "Adds Isle settings and Desktop to Steam's library."
            visible: Games.tools.steam
            Button { text: Games.shortcutsInstalled ? "Remove" : "Add"; enabled: !Games.steamRunning; onClicked: Games.setShortcuts(!Games.shortcutsInstalled) }
        }
    }
}
