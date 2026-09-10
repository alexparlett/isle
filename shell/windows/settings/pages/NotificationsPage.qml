import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Notifications"
    subtitle: "New ones morph the island for a moment and stay in the dashboard until dismissed."

    SettingsGroup {
        SettingsRow {
            label: "Do not disturb"
            description: "Nothing toasts. Critical notifications still do."
            Toggle { checked: Notifications.dnd; onToggled: v => Notifications.setDnd(v) }
        }
        SettingsRow {
            label: "Kept"
            description: Notifications.count + " in the dashboard"
            Button { text: "Clear all"; variant: "text"; enabled: Notifications.count > 0; onClicked: Notifications.clearAll() }
        }
    }

    SettingsGroup {
        heading: "Apps"
        Repeater {
            model: Prefs.p.knownApps
            SettingsRow {
                required property string modelData
                label: modelData
                description: Notifications.isMuted(modelData) ? "Muted: kept in the dashboard, never toasts" : "Toasts"
                Toggle { checked: !Notifications.isMuted(modelData); onToggled: v => Notifications.setMuted(modelData, !v) }
            }
        }
        SettingsRow { visible: Prefs.p.knownApps.length === 0; label: "No apps yet"; description: "Apps appear here after their first notification." }
    }
}
