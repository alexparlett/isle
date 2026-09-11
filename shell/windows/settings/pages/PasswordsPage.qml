import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Passwords"
    subtitle: "Which password managers the launcher searches and the Keychain lists."

    Component.onCompleted: Vault.rescan()

    SettingsGroup {
        heading: "Search from"
        Repeater {
            model: Vault.providers
            SettingsRow {
                id: row
                required property var modelData
                label: modelData.name
                description: modelData.impl.installed ? modelData.note : "Not installed"
                Toggle { enabled: row.modelData.impl.installed; checked: Vault.enabled(row.modelData); onToggled: v => Vault.setEnabled(row.modelData.id, v) }
            }
        }
        SettingsRow { visible: Vault.providers.length === 0; label: "No password manager has a provider" }
    }
}
