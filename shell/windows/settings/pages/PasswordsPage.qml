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
                readonly property var impl: modelData.impl
                readonly property bool on: Vault.enabled(modelData)
                label: modelData.name
                description: !impl.installed ? "Not installed" : !on ? modelData.note : impl.state + (impl.error ? "  ·  " + impl.error : "")
                RowLayout {
                    spacing: Theme.s2
                    // The provider's own options and actions: a toggle, a choice, a field with its button, a button.
                    Repeater {
                        model: row.on ? row.impl.actions : []
                        RowLayout {
                            id: act
                            required property var modelData
                            property string input: ""
                            spacing: Theme.s1
                            Label { visible: modelData.on !== undefined || !!modelData.options; text: modelData.label; size: Theme.sizeCaption; color: Theme.text2 }
                            Toggle { visible: modelData.on !== undefined; checked: !!modelData.on; enabled: !row.impl.busy; onToggled: row.impl.act(modelData.id, "") }
                            Dropdown { visible: !!modelData.options; listWidth: 220; options: modelData.options || []; value: modelData.value || ""; enabled: !row.impl.busy; onPicked: v => row.impl.act(modelData.id, v) }
                            Field { visible: !!modelData.input; implicitWidth: 120; implicitHeight: 32; placeholder: modelData.input || ""; input.echoMode: TextInput.Password; onTextChanged: act.input = text; onAccepted: { row.impl.act(modelData.id, text); text = ""; } }
                            Button { visible: modelData.on === undefined && !modelData.options; text: modelData.label; variant: modelData.primary ? "accent" : "text"; implicitHeight: 32; enabled: !row.impl.busy && (!modelData.input || act.input !== ""); onClicked: { row.impl.act(modelData.id, act.input); act.input = ""; } }
                        }
                    }
                    Toggle { enabled: row.impl.installed; checked: row.on; onToggled: v => Vault.setEnabled(row.modelData.id, v) }
                }
            }
        }
        SettingsRow { visible: Vault.providers.length === 0; label: "No password manager has a provider" }
    }
}
