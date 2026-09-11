import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Passwords"
    subtitle: "Password managers the launcher and the Keychain search and copy from: every one whose tool is on this machine."

    Component.onCompleted: Vault.refresh()

    Repeater {
        model: Vault.providers
        SettingsGroup {
            id: group
            required property var modelData
            readonly property var impl: modelData.impl
            heading: modelData.name
            SettingsRow {
                visible: !group.impl.installed
                label: "Not on this machine"
                description: group.modelData.note + " Install it the way " + group.modelData.name + " documents; it is picked up from then on."
                Button { text: "Check again"; variant: "text"; onClicked: group.impl.refresh() }
            }
            SettingsRow {
                visible: group.impl.installed
                label: "Use it"
                description: "In the launcher, with the * prefix and among the plain results, and in the Keychain under its own category."
                Toggle { checked: Vault.enabled(group.modelData); onToggled: v => Vault.setEnabled(group.modelData.id, v) }
            }
            SettingsRow {
                visible: group.impl.installed && Vault.enabled(group.modelData) && !group.impl.loggedIn
                label: "Not signed in"
                description: "Sign-in is the manager's own flow, in a terminal window."
                Button { text: "Sign in"; variant: "accent"; onClicked: group.impl.signIn() }
            }
            SettingsRow {
                visible: group.impl.loggedIn && Vault.enabled(group.modelData)
                label: group.impl.locked ? "Signed in, locked" : "Signed in" + (group.impl.email ? " as " + group.impl.email : "")
                description: group.impl.locked ? "The session's lock code opens it; the vault is unread until then." : group.impl.items.length + (group.impl.items.length === 1 ? " item" : " items") + (group.impl.hasLock ? "  ·  locks when idle" : "")
                RowLayout {
                    spacing: Theme.s2
                    Field { id: codeField; visible: group.impl.locked; implicitWidth: 140; implicitHeight: 32; placeholder: "Lock code"; input.echoMode: TextInput.Password; onAccepted: { group.impl.unlock(text); text = ""; } }
                    Button { visible: group.impl.locked; text: "Unlock"; variant: "accent"; implicitHeight: 32; enabled: codeField.text !== ""; onClicked: { group.impl.unlock(codeField.text); codeField.text = ""; } }
                    Button { visible: !group.impl.locked; text: group.impl.busy ? "Reading…" : "Refresh"; variant: "text"; enabled: !group.impl.busy; onClicked: group.impl.refresh() }
                    Button { text: "Sign out"; variant: "text"; onClicked: group.impl.signOut() }
                }
            }
            SettingsRow { visible: group.impl.error !== ""; label: "Last error"; description: group.impl.error }
        }
    }
}
