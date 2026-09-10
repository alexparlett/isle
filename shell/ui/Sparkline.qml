import QtQuick
import qs.theme

// Bars for a list of 0..1 values, newest on the right.
Item {
    id: root
    property var values: []
    property color color: Theme.text2
    property int bars: 24
    implicitHeight: 28

    readonly property var shown: values.slice(-bars)

    Row {
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        spacing: 2
        layoutDirection: Qt.RightToLeft
        Repeater {
            model: root.shown.slice().reverse()
            Rectangle {
                required property real modelData
                width: Math.max(2, (root.width - (root.bars - 1) * 2) / root.bars)
                height: Math.max(2, root.height * Math.min(1, modelData))
                anchors.bottom: parent.bottom
                radius: 1
                color: root.color
                opacity: 0.8
            }
        }
    }
}
