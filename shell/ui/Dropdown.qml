import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme

// A dropdown: the current option on a raised button; the list opens in a popup surface beneath it and scrolls.
// `options` are [[value, label]]; `value` selects; `picked(v)` fires on choice.
Rectangle {
    id: root
    property var options: []
    property var value
    property int listWidth: 240
    property int maxRows: 8
    signal picked(var v)

    readonly property string label: { const o = options.find(o => String(o[0]) === String(value)); return o ? o[1] : String(value); }
    property bool open: false

    implicitHeight: 32
    implicitWidth: Math.max(120, row.implicitWidth + Theme.s3 * 2)
    radius: Theme.radiusControl
    color: area.containsMouse || open ? Theme.pressed : Theme.raised
    border.width: 1
    border.color: open ? Theme.accent : Theme.hairline

    RowLayout {
        id: row
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s2 + 2 }
        spacing: Theme.s2
        Label { text: root.label; size: Theme.sizeSmall; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
        Glyph { name: root.open ? "chevron-up" : "chevron-down"; size: 14; color: Theme.text2 }
    }
    MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.open = !root.open }

    PopupWindow {
        id: popup
        anchor.item: root
        anchor.rect.y: root.height + 4
        anchor.rect.x: root.width - root.listWidth
        visible: root.open
        implicitWidth: root.listWidth
        implicitHeight: Math.min(root.maxRows, root.options.length) * 30 + Theme.s2 * 2
        color: "transparent"

        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusCard
            color: Theme.light ? "#FFFFFF" : Theme.raised
            border.width: 1
            border.color: Theme.hairlineStrong

            ListView {
                id: list
                anchors { fill: parent; margins: Theme.s2 }
                clip: true
                model: root.options
                currentIndex: root.options.findIndex(o => String(o[0]) === String(root.value))
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    readonly property bool sel: String(modelData[0]) === String(root.value)
                    width: list.width
                    height: 30
                    radius: Theme.radiusChip
                    color: sel ? Theme.pressed : hover.containsMouse ? Qt.alpha(Theme.pressed, 0.6) : "transparent"
                    RowLayout {
                        anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
                        Label { text: modelData[1]; size: Theme.sizeSmall; weight: sel ? Font.DemiBold : Font.Medium; Layout.fillWidth: true; elide: Text.ElideRight }
                        Glyph { visible: sel; name: "check"; size: 14; color: Theme.accent }
                    }
                    MouseArea { id: hover; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { root.picked(modelData[0]); root.open = false; } }
                }
                Component.onCompleted: positionViewAtIndex(Math.max(0, currentIndex), ListView.Center)
            }
        }
        // Anything outside the list closes it.
        Item { anchors.fill: parent; z: -1 }
    }
    // Close when the button loses the pointer for a while with the popup open.
    Timer { interval: 8000; running: root.open; onTriggered: root.open = false }
}
