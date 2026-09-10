import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Users"
    subtitle: "Accounts on this machine. Changing anything but your own name and picture asks for an administrator."

    Component.onCompleted: Users.refresh()
    property bool renaming: false
    property bool adding: false
    property var settingPasswordFor: null

    component Avatar: Rectangle {
        property var user
        property int size: 40
        implicitWidth: size; implicitHeight: size; radius: size / 2
        color: Theme.pressed
        clip: true
        Image { anchors.fill: parent; source: user && user.icon ? "file://" + user.icon : ""; fillMode: Image.PreserveAspectCrop; visible: !!(user && user.icon); asynchronous: true }
        Label { anchors.centerIn: parent; visible: !(user && user.icon); text: user ? Users.initials(user) : ""; size: size / 2.6; weight: Font.DemiBold; color: Theme.text2 }
    }

    SettingsGroup {
        heading: "You"
        SettingsRow {
            label: Users.me ? (Users.me.realName || Users.me.name) : "…"
            description: Users.me ? Users.me.name + "  ·  " + (Users.me.admin ? "Administrator" : "Standard") + "  ·  uid " + Users.me.uid : ""
            RowLayout {
                spacing: Theme.s3
                Avatar { user: Users.me; size: 44 }
                Button { text: "Picture"; variant: "text"; onClicked: Users.pickIcon(Users.me) }
                Button { text: page.renaming ? "Cancel" : "Rename"; variant: "text"; onClicked: page.renaming = !page.renaming }
                Button { text: "Password"; variant: "raised"; onClicked: Users.changeOwnPassword() }
            }
        }
        SettingsRow {
            visible: page.renaming
            label: "Full name"
            RowLayout {
                spacing: Theme.s2
                Field { id: nameField; implicitWidth: 240; implicitHeight: 32; text: Users.me ? Users.me.realName : ""; placeholder: "Full name"; onAccepted: { Users.setName(Users.me, text); page.renaming = false; } }
                Button { text: "Save"; variant: "accent"; implicitHeight: 32; onClicked: { Users.setName(Users.me, nameField.text); page.renaming = false; } }
            }
        }
    }

    SettingsGroup {
        heading: "Other users"
        Repeater {
            model: Users.others
            SettingsRow {
                id: row
                required property var modelData
                label: modelData.realName || modelData.name
                description: modelData.name + "  ·  " + (modelData.admin ? "Administrator" : "Standard") + (modelData.locked ? "  ·  locked" : "")
                RowLayout {
                    spacing: Theme.s3
                    Avatar { user: row.modelData; size: 32 }
                    Label { text: "Admin"; size: Theme.sizeCaption; color: Theme.text2 }
                    Toggle { checked: row.modelData.admin; onToggled: v => Users.setAdmin(row.modelData, v) }
                    Button { text: "Password"; variant: "text"; onClicked: page.settingPasswordFor = page.settingPasswordFor === row.modelData.name ? null : row.modelData.name }
                    Button { text: "Remove"; variant: "text"; onClicked: Users.remove(row.modelData, false) }
                }
            }
        }
        SettingsRow {
            visible: page.settingPasswordFor !== null
            label: "New password for " + (page.settingPasswordFor || "")
            RowLayout {
                spacing: Theme.s2
                Field { id: pwField; implicitWidth: 220; implicitHeight: 32; placeholder: "Password"; input.echoMode: TextInput.Password }
                Button { text: "Set"; variant: "accent"; implicitHeight: 32; enabled: pwField.text.length >= 4
                    onClicked: { Users.setPassword(Users.others.find(u => u.name === page.settingPasswordFor), pwField.text); pwField.text = ""; page.settingPasswordFor = null; } }
            }
        }
        SettingsRow { visible: Users.others.length === 0; label: "Just you"; description: "Other accounts appear here." }
        SettingsRow {
            label: page.adding ? "New user" : "Add a user"
            description: page.adding ? "A home directory is created; set a password after." : ""
            RowLayout {
                spacing: Theme.s2
                Button { visible: !page.adding; text: "Add"; glyph: "plus"; variant: "raised"; onClicked: page.adding = true }
                Field { id: newName; visible: page.adding; implicitWidth: 120; implicitHeight: 32; placeholder: "username" }
                Field { id: newReal; visible: page.adding; implicitWidth: 160; implicitHeight: 32; placeholder: "Full name" }
                Label { visible: page.adding; text: "Admin"; size: Theme.sizeCaption; color: Theme.text2 }
                Toggle { id: newAdmin; visible: page.adding }
                Button { visible: page.adding; text: "Create"; variant: "accent"; implicitHeight: 32; enabled: /^[a-z_][a-z0-9_-]{0,31}$/.test(newName.text)
                    onClicked: { Users.add(newName.text, newReal.text, newAdmin.checked); newName.text = ""; newReal.text = ""; newAdmin.checked = false; page.adding = false; } }
                Button { visible: page.adding; text: "Cancel"; variant: "text"; implicitHeight: 32; onClicked: page.adding = false }
            }
        }
        SettingsRow { visible: Users.error !== ""; label: "That did not work"; description: Users.error }
    }

    SettingsGroup {
        heading: "Login"
        SettingsRow {
            label: "Automatic login"
            description: Users.me && Users.autoLoginUser === Users.me.name ? "This account signs in at boot without a password." : Users.autoLoginUser ? Users.autoLoginUser + " signs in at boot." : "The login screen asks for a password."
            Toggle { checked: Users.me && Users.autoLoginUser === Users.me.name; onToggled: v => Users.setAutoLogin(v) }
        }
    }
}
