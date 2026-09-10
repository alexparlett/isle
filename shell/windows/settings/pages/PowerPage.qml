import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Power"

    // Idle timeouts in seconds; the current value is offered even when it is not one of the usual steps.
    component Minutes: Dropdown {
        property string key
        readonly property int current: Prefs.p[key]
        readonly property var steps: [0, 60, 120, 300, 600, 900, 1200, 1800, 2700, 3600, 7200]
        listWidth: 140
        options: (steps.indexOf(current) >= 0 ? steps : steps.concat([current]).sort((a, b) => a - b)).map(s => [s, s === 0 ? "Never" : s < 3600 ? s / 60 + " min" : s / 3600 + (s === 3600 ? " hour" : " hours")])
        value: current
        onPicked: v => Prefs.p[key] = Number(v)
    }

    SettingsGroup {
        heading: "Profile"
        SettingsRow {
            label: "Power profile"
            description: Power.profileLabel
            RowLayout {
                spacing: Theme.s2
                Repeater {
                    model: [["power-saver", "Saver"], ["balanced", "Balanced"], ["performance", "Performance"]]
                    Button {
                        required property var modelData
                        text: modelData[1]
                        variant: Power.profile === modelData[0] ? "accent" : "raised"
                        enabled: modelData[0] !== "performance" || Power.hasPerformance
                        onClicked: Power.setProfile(modelData[0])
                    }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Idle"
        SettingsRow { label: "Dim"; description: "Game mode stops the clock."; Minutes { key: "idleDim" } }
        SettingsRow { label: "Lock"; Minutes { key: "idleLock" } }
        SettingsRow { label: "Screens off"; Minutes { key: "idleScreenOff" } }
    }

    SettingsGroup {
        visible: Power.peripherals.length > 0
        heading: "Batteries"
        Repeater {
            model: Power.peripherals
            SettingsRow {
                required property var modelData
                label: modelData.model || "Device"
                description: Math.round(modelData.percentage) + "%"
                Glyph { name: Power.glyphFor(modelData); size: 14 }
            }
        }
    }
}
