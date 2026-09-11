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
        SettingsRow {
            label: "Steam opens in Big Picture"
            description: "From the launcher, Steam comes up in its own Big Picture: one window, mouse and keyboard welcome, none of the desktop client's X11 menus."
            visible: Games.tools.steam
            Toggle { checked: Prefs.p.steamBigPicture; onToggled: v => Prefs.p.steamBigPicture = v }
        }
        SettingsRow {
            label: "Leaving Big Picture quits Steam"
            description: "Exit Big Picture closes Steam rather than dropping to the desktop client."
            visible: Games.tools.steam
            Toggle { checked: Prefs.p.steamQuitsWithBigPicture; onToggled: v => Prefs.p.steamQuitsWithBigPicture = v }
        }
        SettingsRow {
            label: "Steam's Big Picture switches Isle's mode"
            description: "Isle enters Big Picture mode when Steam's appears and leaves when it goes. Off, Steam's Big Picture is just another window."
            visible: Games.tools.steam
            Toggle { checked: Prefs.p.steamFollowsMode; onToggled: v => Prefs.p.steamFollowsMode = v }
        }
        SettingsRow {
            label: "Controller opens Big Picture"
            description: "Hold the Guide button on the desktop. The pad is read the whole time for it."
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
