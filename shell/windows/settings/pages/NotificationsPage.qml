import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Notifications"
    subtitle: "Toasts, what stays quiet, and which apps may interrupt."

    SettingsGroup {
        heading: "Silence"
        SettingsRow {
            label: "Do not disturb"
            description: "Nothing toasts, bar critical ones and the apps below."
            Toggle { checked: Notifications.dnd; onToggled: v => Notifications.setDnd(v) }
        }
        SettingsRow {
            label: "Quiet hours"
            description: Prefs.p.dndQuiet ? "Silent from " + Prefs.p.dndQuietFrom + ":00 to " + Prefs.p.dndQuietTo + ":00" + (Notifications.quietHours ? "  ·  on now" : "") : "Silent between two hours of the day."
            RowLayout {
                spacing: Theme.s2
                NumberField { visible: Prefs.p.dndQuiet; value: Prefs.p.dndQuietFrom; from: 0; to: 23; unit: "h"; onCommitted: v => Prefs.p.dndQuietFrom = v }
                Label { visible: Prefs.p.dndQuiet; text: "to"; size: Theme.sizeSmall; color: Theme.text3 }
                NumberField { visible: Prefs.p.dndQuiet; value: Prefs.p.dndQuietTo; from: 0; to: 23; unit: "h"; onCommitted: v => Prefs.p.dndQuietTo = v }
                Toggle { checked: Prefs.p.dndQuiet; onToggled: v => Prefs.p.dndQuiet = v }
            }
        }
        SettingsRow {
            label: "Fullscreen"
            description: "Silent while a fullscreen window has focus."
            Toggle { checked: Prefs.p.dndFullscreen; onToggled: v => Prefs.p.dndFullscreen = v }
        }
        SettingsRow {
            label: "Summary afterwards"
            description: "When silence lifts, a toast says how many arrived."
            Toggle { checked: Prefs.p.dndSummary; onToggled: v => Prefs.p.dndSummary = v }
        }
    }

    SettingsGroup {
        heading: "Toasts"
        SettingsRow {
            label: "Sound"
            description: ""
            Toggle { checked: Prefs.p.notifySound; onToggled: v => Prefs.p.notifySound = v }
        }
        SettingsRow {
            label: "Low urgency"
            description: "Low ones wait in the centre; critical ones always toast."
        }
        SettingsRow {
            label: "Kept"
            description: Notifications.holding
            Button { text: "Clear all"; variant: "text"; enabled: Notifications.count > 0; onClicked: Notifications.clearAll() }
        }
    }

    SettingsGroup {
        heading: "Apps"
        Repeater {
            model: Prefs.p.knownApps
            SettingsRow {
                id: appRow
                required property string modelData
                readonly property bool muted: Notifications.isMuted(modelData)
                readonly property bool allowed: Notifications.isAllowed(modelData)
                label: modelData
                description: muted ? "Muted: waits in the centre, never toasts" : allowed ? "Always toasts, through do not disturb and quiet hours" : "Toasts, unless silenced"
                RowLayout {
                    spacing: Theme.s3
                    Label { text: "Toasts"; size: Theme.sizeCaption; color: Theme.text3 }
                    Toggle { checked: !appRow.muted; onToggled: v => Notifications.setMuted(appRow.modelData, !v) }
                    Label { text: "Always"; size: Theme.sizeCaption; color: Theme.text3; Layout.leftMargin: Theme.s2 }
                    Toggle { checked: appRow.allowed; enabled: !appRow.muted; onToggled: v => Notifications.setAllowed(appRow.modelData, v) }
                }
            }
        }
        SettingsRow { visible: Prefs.p.knownApps.length === 0; label: "No apps yet"; description: "Apps appear here after their first notification." }
    }
}
