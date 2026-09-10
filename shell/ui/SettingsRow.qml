import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// One settings row: label and description on the left, a control on the right. Rows after the first get a hairline.
Item {
    id: root
    property string label
    property string description: ""
    // A leading glyph, for rows that are one of a kind: a network, a device.
    property string glyph: ""
    property color glyphColor: Theme.text2
    default property alias control: slot.data
    Layout.fillWidth: true
    implicitHeight: Math.max(48, textCol.implicitHeight + Theme.s3 * 2)
    // Search lands here: the row rings for a moment.
    readonly property bool rung: SettingsIndex.highlight !== "" && SettingsIndex.highlight === label
    Component.onCompleted: SettingsIndex.register(label, root)
    Component.onDestruction: SettingsIndex.unregister(label, root)
    Rectangle {
        anchors { fill: parent; margins: 3 }
        radius: Theme.radiusControl
        color: Qt.alpha(Theme.accent, root.rung ? 0.10 : 0)
        border.width: 1
        border.color: Qt.alpha(Theme.accent, root.rung ? 1 : 0)
        Behavior on color { ColorAnimation { duration: Theme.move } }
        Behavior on border.color { ColorAnimation { duration: Theme.move } }
    }
    Rectangle {
        anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: Theme.s3; rightMargin: Theme.s3 }
        height: 1; color: Theme.hairline
        visible: root.parent && root.parent.children[0] !== root
    }
    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
        spacing: Theme.s3
        Item {
            visible: root.glyph !== ""
            Layout.preferredWidth: 24
            Layout.fillHeight: true
            Glyph { anchors.centerIn: parent; name: root.glyph; size: 20; color: root.glyphColor }
        }
        ColumnLayout {
            id: textCol
            Layout.fillWidth: true
            spacing: 1
            Label { text: root.label; Layout.fillWidth: true }
            Label { visible: root.description !== ""; text: root.description; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        }
        Item { id: slot; implicitWidth: childrenRect.width; implicitHeight: childrenRect.height }
    }
}
