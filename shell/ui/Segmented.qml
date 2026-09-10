import QtQuick
import QtQuick.Layouts
import qs.theme

// A row of options, one chosen: `options` are [[value, label]]; `picked(v)` fires on choice.
Rectangle {
    id: root
    property var options: []
    property var value
    signal picked(var v)

    implicitHeight: 28
    implicitWidth: row.implicitWidth + 4
    radius: Theme.radiusControl
    color: Theme.pressed
    border.width: 1
    border.color: Theme.hairline

    RowLayout {
        id: row
        anchors { fill: parent; margins: 2 }
        spacing: 2
        Repeater {
            model: root.options
            Rectangle {
                required property var modelData
                readonly property bool on: String(modelData[0]) === String(root.value)
                Layout.fillHeight: true
                implicitWidth: seg.implicitWidth + Theme.s3 * 2
                radius: Theme.radiusControl - 2
                color: on ? Theme.raised : (segArea.containsMouse ? Qt.alpha(Theme.text, 0.05) : "transparent")
                border.width: on ? 1 : 0
                border.color: Theme.hairlineStrong
                Behavior on color { ColorAnimation { duration: Theme.quick } }
                Label { id: seg; anchors.centerIn: parent; text: parent.modelData[1]; size: Theme.sizeCaption; weight: Font.DemiBold; color: parent.on ? Theme.text : Theme.text2 }
                MouseArea { id: segArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.picked(parent.modelData[0]) }
            }
        }
    }
}
