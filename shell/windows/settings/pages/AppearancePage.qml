import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Appearance"
    subtitle: "One material, one accent. Installed apps follow the same tokens."

    SettingsGroup {
        heading: "Theme"
        SettingsRow {
            label: "Appearance"
            readonly property string sunWindow: Qt.formatTime(new Date(2000, 0, 1, Math.floor(Theming.sunrise / 60), Theming.sunrise % 60), DateTime.timeFormat) + " to " + Qt.formatTime(new Date(2000, 0, 1, Math.floor(Theming.sunset / 60), Theming.sunset % 60), DateTime.timeFormat)
            description: Prefs.p.theme === "auto" ? "Light from sunrise to sunset, today " + sunWindow + ", from the timezone. Now " + (Theming.light ? "light." : "dark.") : "Dark is the default. Light swaps the tokens; installed apps follow. Auto follows the sun."
            RowLayout {
                spacing: Theme.s2
                Button { text: "Dark"; variant: Prefs.p.theme === "dark" ? "accent" : "raised"; onClicked: Prefs.p.theme = "dark" }
                Button { text: "Light"; variant: Prefs.p.theme === "light" ? "accent" : "raised"; onClicked: Prefs.p.theme = "light" }
                Button { text: "Auto"; variant: Prefs.p.theme === "auto" ? "accent" : "raised"; onClicked: Prefs.p.theme = "auto" }
            }
        }
    }

    SettingsGroup {
        heading: "Accent"
        SettingsRow {
            label: "Colour"
            description: "The one thing that is active, focused or live."
            RowLayout {
                spacing: Theme.s2
                Repeater {
                    model: ["#7FA6FF", "#8FD3C5", "#E8B86D", "#F28C8C", "#C8B5FF", "#F2F2F3"]
                    Rectangle {
                        required property string modelData
                        readonly property bool sel: (Prefs.p.accent || "#7FA6FF").toLowerCase() === modelData.toLowerCase()
                        width: 26; height: 26; radius: 13
                        color: modelData
                        border.width: sel ? 3 : 0
                        border.color: Theme.ink
                        Rectangle { anchors.fill: parent; anchors.margins: -3; radius: 16; color: "transparent"; border.width: 2; border.color: parent.sel ? Theme.text : "transparent" }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Prefs.p.accent = modelData }
                    }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Motion"
        SettingsRow {
            label: "Reduce motion"
            description: "No morphs, no fades. Game mode does this on its own."
            Toggle { checked: Prefs.p.reducedMotion; onToggled: v => { Prefs.p.reducedMotion = v; Theme.motion = v || Modes.game ? 0 : 1; } }
        }
    }

    SettingsGroup {
        heading: "Windows"
        SettingsRow {
            label: "Title bars"
            description: "A bar with the title and three controls over every window that does not draw its own."
            Toggle { checked: Prefs.p.titleBars; onToggled: v => Prefs.p.titleBars = v }
        }
        SettingsRow {
            label: "Draw their own"
            description: "Window classes that carry their own title bar without saying so, one per comma. Steam does."
            Field {
                implicitWidth: 260; implicitHeight: 30; size: Theme.sizeSmall
                placeholder: "steam, Chatgpt"
                text: (Prefs.p.titleBarsExcept || []).join(", ")
                function commit() { const l = text.split(",").map(s => s.trim()).filter(s => s); if (JSON.stringify(l) !== JSON.stringify(Prefs.p.titleBarsExcept || [])) Prefs.p.titleBarsExcept = l; }
                onAccepted: commit()
                onActiveFocusChanged: if (!activeFocus) commit()
            }
        }
    }
}
