import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Shortcuts"
    subtitle: "What the keys do. The column is the profile in use; Change records a new chord."

    // Any keyboard on the Mac profile makes this the Mac column.
    readonly property bool mac: Keyboard.macDevices.length > 0

    // Recording a shortcut: the row listens, the compositor's binds step aside until it is done.
    property string recording: ""
    property string clash: ""
    function record(id) {
        if (recording) Keyboard.setRecording(false);
        recording = id; clash = "";
        if (id) Keyboard.setRecording(true);
    }
    Component.onDestruction: if (recording) Keyboard.setRecording(false)
    Component.onCompleted: Keyboard.refreshDevices()

    SettingsGroup {
        heading: "Shortcuts"
        Item {
            Layout.fillWidth: true
            implicitHeight: 32
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                Label { text: "Action"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.preferredWidth: parent.width * 0.44 }
                Label { text: page.mac ? "Mac" : "Windows"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.fillWidth: true }
                Label { text: "Change"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.preferredWidth: 96; horizontalAlignment: Text.AlignRight }
            }
        }
        Repeater {
            model: Keyboard.actions.filter(a => page.mac ? a.mac : a.win).filter(a => !/\(Windows\)|\(Mac\)/.test(a.label))
            Item {
                id: shortcutRow
                required property var modelData
                readonly property bool overridden: !!Keyboard.overrides[modelData.id]
                readonly property bool recording: page.recording === modelData.id
                // Handlers cannot see the page id from inside the delegate; bindings can.
                readonly property var host: page
                Layout.fillWidth: true
                implicitHeight: 34
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: Theme.s3; rightMargin: Theme.s3 } height: 1; color: Theme.hairline }
                RowLayout {
                    anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                    Label { text: modelData.label; Layout.preferredWidth: parent.width * 0.44 }
                    // The chord columns, or the recorder across both while this row records.
                    Label { visible: !shortcutRow.recording; text: (page.mac ? modelData.mac : modelData.win) || "—"; mono: true; size: Theme.sizeSmall; color: shortcutRow.overridden ? Theme.accent : Theme.text2; Layout.fillWidth: true }
                    Rectangle {
                        visible: shortcutRow.recording
                        Layout.fillWidth: true
                        Layout.minimumWidth: 260
                        implicitHeight: 26
                        radius: Theme.radiusControl
                        color: Theme.raised
                        border.width: 1
                        border.color: page.clash ? Theme.warn : Theme.accent
                        focus: shortcutRow.recording
                        onVisibleChanged: if (visible) forceActiveFocus()
                        Keys.onPressed: event => {
                            event.accepted = true;
                            const host = shortcutRow.host, id = shortcutRow.modelData.id;
                            if (event.key === Qt.Key_Escape && !event.modifiers) { host.record(""); return; }
                            // The override rebuilds the list, and this delegate with it: finish here first.
                            if (event.key === Qt.Key_Backspace && !event.modifiers) { host.record(""); Keyboard.setOverride(id, ""); return; }
                            const chord = Keyboard.chordFromEvent(event);
                            if (!chord) return;
                            const used = Keyboard.usedBy(chord, id);
                            if (used) { host.clash = used.label; return; }
                            host.record("");
                            Keyboard.setOverride(id, chord);
                        }
                        Label {
                            anchors { left: parent.left; right: parent.right; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2; verticalCenter: parent.verticalCenter }
                            elide: Text.ElideRight
                            text: shortcutRow.host.clash ? "Already used by " + shortcutRow.host.clash : "Press the new shortcut  ·  Backspace restores the default  ·  Esc keeps it"
                            size: Theme.sizeCaption; color: shortcutRow.host.clash ? Theme.warn : Theme.text2
                        }
                    }
                    RowLayout {
                        Layout.preferredWidth: 96
                        Layout.alignment: Qt.AlignRight
                        spacing: Theme.s1
                        visible: !modelData.range
                        Item { Layout.fillWidth: true }
                        Label { visible: shortcutRow.overridden && !shortcutRow.recording; text: "Reset"; size: Theme.sizeCaption; color: Theme.text3
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Keyboard.setOverride(modelData.id, "") } }
                        Label { text: shortcutRow.recording ? "Cancel" : "Change"; size: Theme.sizeCaption; color: Theme.accent
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: page.record(shortcutRow.recording ? "" : modelData.id) } }
                    }
                    Item { Layout.preferredWidth: 96; visible: !!modelData.range }
                }
            }
        }
    }
}
