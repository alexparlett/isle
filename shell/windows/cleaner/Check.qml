import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// The box that puts a row in the basket. A protected place has a lock instead, and something already
// in the basket through a folder above it shows as ticked and cannot be taken out on its own.
Item {
    id: root
    required property var app
    required property var item
    readonly property bool real: !!item.path && !item.other && !item.skipped
    readonly property bool locked: real && app.isProtected(item.path)
    readonly property bool on: { app.basketRev; return real && app.inBasket(item.path); }
    readonly property bool own: { app.basketRev; return on && app.basket[item.path] !== undefined; }

    implicitWidth: 18
    implicitHeight: 18
    Layout.preferredWidth: 18

    Rectangle {
        anchors.fill: parent
        visible: root.real && !root.locked
        radius: 5
        color: root.on ? Theme.accent : "transparent"
        border.width: root.on ? 0 : 1.5
        border.color: boxArea.containsMouse ? Theme.text2 : Theme.hairlineStrong
        opacity: root.on && !root.own ? 0.5 : 1
        Glyph { anchors.centerIn: parent; visible: root.on; name: "check"; size: 12; weight: 2.5; color: Theme.onAccent }
        MouseArea {
            id: boxArea
            anchors { fill: parent; margins: -6 }
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            enabled: !root.on || root.own
            onClicked: root.app.toggle(root.item)
        }
    }
    Glyph {
        anchors.centerIn: parent
        visible: root.locked
        name: "lock"
        size: 12
        color: Theme.text3
    }
}
