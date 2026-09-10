import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

// The Keychain window: the system keyring and the SSH keys; reveal for a moment, copy, add, lock.
FloatingWindow {
    id: root
    title: "Keychain"
    visible: Surfaces.keychain
    implicitWidth: 760
    implicitHeight: 600
    minimumSize: Qt.size(640, 480)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: { if (visible) { KeychainService.refresh(); SshKeys.refresh(); } else { Surfaces.keychain = false; KeychainService.conceal(); } }

    property string filter: ""
    property string category: "all"
    property bool adding: false
    property bool showSecret: false
    property real genLength: 20
    property bool genSymbols: true
    // Categories are the keyring's kinds; "all" spans them and the SSH keys.
    readonly property var keyringShown: KeychainService.items
        .filter(i => category === "all" || KeychainService.categoryOf(i) === category)
        .filter(i => !filter || (i.label + " " + JSON.stringify(i.attributes)).toLowerCase().indexOf(filter.toLowerCase()) >= 0)
        .map(i => Object.assign({ source: "keyring" }, i))
    // Keys in ~/.ssh sit under SSH beside the keyring's ssh entries.
    readonly property var sshShown: category !== "all" && category !== "ssh" ? [] : SshKeys.keys
        .filter(k => !filter || (k.name + " " + k.comment).toLowerCase().indexOf(filter.toLowerCase()) >= 0)
        .map(k => Object.assign({ source: "ssh" }, k))
    readonly property var shown: sshShown.concat(keyringShown)
    readonly property var categories: KeychainService.categories
    function countIn(c) {
        if (c === "all") return KeychainService.items.length + SshKeys.keys.length;
        return KeychainService.items.filter(i => KeychainService.categoryOf(i) === c).length + (c === "ssh" ? SshKeys.keys.length : 0);
    }

    ColumnLayout {
        anchors { fill: parent; margins: Theme.s5 }
        spacing: Theme.s4

        RowLayout {
            spacing: Theme.s3
            ColumnLayout {
                spacing: 2
                Label { text: "Keychain"; size: Theme.sizeTitle; weight: Font.DemiBold }
                Label { text: KeychainService.available ? KeychainService.items.length + " in the keyring" : "No secret service: " + KeychainService.error; size: Theme.sizeSmall; color: KeychainService.available ? Theme.text2 : Theme.warn }
            }
            Item { Layout.fillWidth: true }
            Field { implicitWidth: 220; glyph: "search"; placeholder: "Search"; onTextChanged: root.filter = text }
            Button { text: "Add"; glyph: "plus"; variant: "accent"; visible: !root.adding; onClicked: root.adding = true }
            Button { text: "Lock"; variant: "text"; onClicked: KeychainService.lock() }
        }

        // Categories
        RowLayout {
            spacing: Theme.s1 + 2
            Repeater {
                model: root.categories
                Rectangle {
                    required property var modelData
                    readonly property bool sel: root.category === modelData[0]
                    readonly property int n: root.countIn(modelData[0])
                    visible: n > 0 || modelData[0] === "all" || modelData[0] === "ssh"
                    implicitHeight: 26; implicitWidth: catLabel.implicitWidth + Theme.s3 * 2
                    radius: 13; color: sel ? Theme.accent : Theme.raised; border.width: sel ? 0 : 1; border.color: Theme.hairline
                    RowLayout {
                        id: catLabel
                        anchors.centerIn: parent
                        spacing: 4
                        Label { text: modelData[1] + "  " + parent.parent.n; size: Theme.sizeCaption; weight: Font.DemiBold; color: parent.parent.sel ? Theme.onAccent : Theme.text2 }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.category = modelData[0] }
                }
            }
        }

        // The editor: a form in place of the list, laid out like a password manager's.
        Card {
            id: editor
            visible: root.adding
            Layout.fillWidth: true
            Layout.fillHeight: true
            padding: Theme.s4

            component FormRow: RowLayout {
                property string label
                default property alias control: slot.data
                Layout.fillWidth: true
                spacing: Theme.s3
                Label { text: parent.label; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.text2; Layout.preferredWidth: 110; Layout.alignment: Qt.AlignTop; Layout.topMargin: 11 }
                ColumnLayout { id: slot; Layout.fillWidth: true; spacing: Theme.s2 }
            }
            readonly property bool sshKind: newKind.value === "ssh"
            readonly property bool canSave: newLabel.text !== "" && (sshKind || newSecret.text !== "")
            // A rough strength from the character classes and length.
            readonly property real strength: {
                const t = newSecret.text;
                if (!t) return 0;
                let pool = 0;
                if (/[a-z]/.test(t)) pool += 26; if (/[A-Z]/.test(t)) pool += 26; if (/[0-9]/.test(t)) pool += 10; if (/[^A-Za-z0-9]/.test(t)) pool += 20;
                return Math.min(1, t.length * Math.log2(pool || 1) / 90);
            }

            ColumnLayout {
                width: parent.width
                spacing: Theme.s3

                RowLayout {
                    Layout.fillWidth: true
                    Layout.bottomMargin: Theme.s2
                    Label { text: "New item"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
                    Button { text: "Cancel"; variant: "text"; onClicked: { root.adding = false; root.showSecret = false; } }
                    Button { text: "Save"; variant: "accent"; enabled: editor.canSave
                        onClicked: {
                            if (editor.sshKind) SshKeys.generate(newLabel.text, newKeyType.value, newComment.text, newSecret.text);
                            else KeychainService.store(newLabel.text, ["category=" + newCategory.value].concat(newUser.text ? ["user=" + newUser.text] : []).concat(newAttr.text ? [newAttr.text] : ["app=isle"]), newSecret.text);
                            newLabel.text = ""; newUser.text = ""; newAttr.text = ""; newSecret.text = ""; newComment.text = ""; root.adding = false; root.showSecret = false;
                        } }
                }

                FormRow {
                    label: "Type"
                    Dropdown { id: newKind; listWidth: 160; options: [["login", "Login"], ["ssh", "SSH key"]]; value: root.category === "ssh" ? "ssh" : "login"; onPicked: v => value = v }
                }

                FormRow {
                    label: "Save to"
                    Dropdown {
                        id: newCategory
                        listWidth: 200
                        options: editor.sshKind ? [["ssh", "This computer · ~/.ssh"]] : KeychainService.categories.filter(c => c[0] !== "all" && c[0] !== "ssh").map(c => [c[0], "Keyring · " + c[1]])
                        value: editor.sshKind ? "ssh" : "logins"
                        onPicked: v => value = v
                    }
                }
                FormRow { label: editor.sshKind ? "File name" : "Title"; Field { id: newLabel; Layout.fillWidth: true; placeholder: editor.sshKind ? "id_ed25519_work" : "GitHub"; onVisibleChanged: if (visible) input.forceActiveFocus() } }
                FormRow { label: "Username"; visible: !editor.sshKind; Field { id: newUser; Layout.fillWidth: true; placeholder: "name or email" } }
                FormRow {
                    label: "Key type"
                    visible: editor.sshKind
                    Dropdown { id: newKeyType; listWidth: 160; options: [["ed25519", "Ed25519"], ["rsa", "RSA 4096"]]; value: "ed25519"; onPicked: v => value = v }
                }
                FormRow { label: "Comment"; visible: editor.sshKind; Field { id: newComment; Layout.fillWidth: true; placeholder: "you@host, or what the key is for" } }
                FormRow {
                    label: editor.sshKind ? "Passphrase" : "Password"
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Field { id: newSecret; Layout.fillWidth: true; placeholder: editor.sshKind ? "Optional" : "Password"; input.echoMode: root.showSecret ? TextInput.Normal : TextInput.Password; input.font.family: root.showSecret ? Theme.fontMono : Theme.fontUi }
                        Button { glyph: root.showSecret ? "eye-off" : "eye"; text: ""; variant: "raised"; onClicked: root.showSecret = !root.showSecret }
                        Button { text: "Generate"; glyph: "wand-sparkles"; variant: "raised"; onClicked: KeychainService.generate(root.genLength, root.genSymbols) }
                    }
                    // Strength, then the generator's settings.
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 4; radius: 2; color: Theme.pressed
                            Rectangle { width: parent.width * editor.strength; height: parent.height; radius: 2
                                color: editor.strength < 0.4 ? Theme.warn : editor.strength < 0.7 ? Theme.accent : Theme.ok
                                Behavior on width { NumberAnimation { duration: Theme.quick } } }
                        }
                        Label { text: newSecret.text === "" ? "" : editor.strength < 0.4 ? "Weak" : editor.strength < 0.7 ? "Fair" : "Strong"; size: Theme.sizeCaption; color: Theme.text3; Layout.preferredWidth: 44 }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s3
                        Label { text: "Length"; size: Theme.sizeCaption; color: Theme.text2 }
                        Slider { implicitWidth: 160; value: (root.genLength - 8) / 56; onMoved: v => root.genLength = Math.round(8 + v * 56) }
                        Label { text: Math.round(root.genLength); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 20 }
                        Item { Layout.preferredWidth: Theme.s2 }
                        Label { text: "Symbols"; size: Theme.sizeCaption; color: Theme.text2 }
                        Toggle { checked: root.genSymbols; onToggled: v => root.genSymbols = v }
                        Item { Layout.fillWidth: true }
                    }
                }
                FormRow {
                    label: "Attributes"
                    visible: !editor.sshKind
                    Field { id: newAttr; Layout.fillWidth: true; placeholder: "key=value, what apps look the secret up by" }
                }
                Connections { target: KeychainService; function onGeneratedChanged() { if (KeychainService.generated) { newSecret.text = KeychainService.generated; root.showSecret = true; } } }
            }
        }

        // The agent, when looking at keys.
        RowLayout {
            visible: !root.adding && root.category === "ssh"
            Layout.fillWidth: true
            spacing: Theme.s2
            Rectangle { width: 8; height: 8; radius: 4; color: SshKeys.agent.alive ? Theme.ok : Theme.warn }
            Label { text: SshKeys.agent.alive ? "Agent holds " + SshKeys.agent.loaded + (SshKeys.agent.loaded === 1 ? " key" : " keys") : "No SSH agent at " + (SshKeys.agent.sock || "SSH_AUTH_SOCK"); size: Theme.sizeSmall; color: Theme.text2 }
            Label { visible: SshKeys.error !== ""; text: "  ·  " + SshKeys.error; size: Theme.sizeSmall; color: Theme.warn }
            Item { Layout.fillWidth: true }
        }

        ListView {
            visible: !root.adding
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.shown
            spacing: 2
            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width
                height: unlocking ? 96 : 56
                radius: Theme.radiusControl
                color: area.containsMouse ? Theme.raised : "transparent"
                readonly property bool ssh: modelData.source === "ssh"
                // A locked key asks for its passphrase in place before it is loaded.
                property bool unlocking: false
                readonly property bool revealed: KeychainService.revealedPath === modelData.path
                readonly property string secret: KeychainService.revealed
                RowLayout {
                    id: itemRow
                    anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                    height: 56
                    spacing: Theme.s3
                    readonly property var d: parent
                    Glyph { name: itemRow.d.ssh ? "terminal" : modelData.locked ? "lock" : ({ wifi: "wifi", browser: "globe", ssh: "terminal", logins: "user", apps: "layout-grid" })[KeychainService.categoryOf(modelData)] || "key-round"; size: 16; color: itemRow.d.ssh ? (modelData.loaded ? Theme.ok : Theme.text2) : modelData.locked ? Theme.text3 : Theme.text2 }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Label { text: (itemRow.d.ssh ? modelData.name + (modelData.comment ? "  ·  " + modelData.comment : "") : modelData.label) || "(unnamed)"; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                        Label {
                            text: itemRow.d.revealed ? itemRow.d.secret
                                : itemRow.d.ssh ? modelData.type + " " + modelData.bits + "  ·  " + modelData.fingerprint.replace("SHA256:", "") + "  ·  " + (modelData.loaded ? "In the agent" : !modelData.hasPrivate ? "Public key only" : modelData.locked ? "Passphrase" : "Not loaded")
                                : Object.keys(modelData.attributes || {}).map(k => k + "=" + modelData.attributes[k]).join("  ") + "  ·  " + modelData.collection
                            size: Theme.sizeCaption; color: itemRow.d.revealed ? Theme.accent : Theme.text3; mono: itemRow.d.revealed; Layout.fillWidth: true
                        }
                    }
                    // Keys: the public half to the clipboard; in and out of the agent.
                    Button { visible: itemRow.d.ssh; text: "Public key"; variant: "text"; onClicked: SshKeys.copyPublic(modelData) }
                    Button { visible: itemRow.d.ssh && modelData.hasPrivate && !modelData.loaded; text: itemRow.d.unlocking ? "Cancel" : "Load"; variant: itemRow.d.unlocking ? "text" : "raised"
                        onClicked: { if (modelData.locked) itemRow.d.unlocking = !itemRow.d.unlocking; else SshKeys.load(modelData, ""); } }
                    Button { visible: itemRow.d.ssh && modelData.loaded; text: "Unload"; variant: "text"; onClicked: SshKeys.unload(modelData) }
                    Button { visible: !itemRow.d.ssh; text: itemRow.d.revealed ? "Hide" : "Reveal"; variant: "text"; onClicked: itemRow.d.revealed ? KeychainService.conceal() : KeychainService.reveal(modelData) }
                    Button { visible: !itemRow.d.ssh && itemRow.d.revealed; text: "Copy"; variant: "text"; onClicked: KeychainService.copy(KeychainService.revealed) }
                    Button { visible: !itemRow.d.ssh; text: "Delete"; variant: "text"; onClicked: KeychainService.remove(modelData) }
                }
                // The passphrase, under the row.
                RowLayout {
                    visible: parent.unlocking
                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.s3 + 16 + Theme.s3; rightMargin: Theme.s3; bottomMargin: Theme.s2 }
                    spacing: Theme.s2
                    readonly property var d: parent
                    Field {
                        id: passField
                        Layout.fillWidth: true; implicitHeight: 32; placeholder: "Passphrase for " + modelData.name; input.echoMode: TextInput.Password
                        onVisibleChanged: if (visible) { text = ""; input.forceActiveFocus(); }
                        onAccepted: { SshKeys.load(modelData, text); text = ""; parent.d.unlocking = false; }
                    }
                    Button { text: "Load"; variant: "accent"; implicitHeight: 32; enabled: passField.text !== ""; onClicked: { SshKeys.load(modelData, passField.text); passField.text = ""; parent.d.unlocking = false; } }
                }
                MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; z: -1 }
            }
            Label { anchors.centerIn: parent; visible: root.shown.length === 0 && KeychainService.available; text: root.filter ? "No matches" : root.category === "ssh" ? "No keys in ~/.ssh" : "Nothing stored yet"; color: Theme.text3 }
        }
    }
}
