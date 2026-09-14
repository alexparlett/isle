import QtQuick
import qs.theme

// What a folder looks like while something is held over it: filled rather than outlined, so the row
// reads as the place the thing is going rather than as a box drawn around it.
Rectangle {
    property bool on: false

    anchors.fill: parent
    radius: Theme.radiusControl
    color: Qt.alpha(Theme.accent, 0.10)
    border.width: 1
    border.color: Qt.alpha(Theme.accent, 0.28)
    opacity: on ? 1 : 0
    visible: opacity > 0
    Behavior on opacity { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuad } }
}
