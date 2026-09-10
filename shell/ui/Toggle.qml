import QtQuick
import qs.theme

// A 36×20 switch, accent when on.
Item {
    id: root
    property bool checked: false
    signal toggled(bool on)

    implicitWidth: 36
    implicitHeight: 20

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: root.checked ? Theme.accent : Theme.pressed
        border.width: root.checked ? 0 : 1
        border.color: Theme.hairline
        Behavior on color { ColorAnimation { duration: Theme.quick } }

        Rectangle {
            width: 14; height: 14; radius: 7
            y: 3
            x: root.checked ? parent.width - width - 3 : 3
            color: root.checked ? Theme.onAccent : Theme.text
            Behavior on x { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
        }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled(!root.checked)
    }
}
