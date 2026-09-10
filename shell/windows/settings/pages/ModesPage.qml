import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Modes"
    subtitle: "Normal, game, Big Picture. Entering one applies a profile; leaving restores the last."

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
            description: "When gamemoded reports a game, or a fullscreen window belongs to Steam, Heroic or Lutris."
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
            description: "A nested compositor for Steam's gamepad UI: HDR, upscaling, a fixed frame rate."
            Toggle { checked: Prefs.p.gamescope; onToggled: v => Prefs.p.gamescope = v }
        }
    }
}
