import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme

// A card over the window that asked for it, for the few things that need answering before they can be done:
// a title, whatever the caller puts inside, and two buttons. Escape and the backdrop both cancel.
PopupWindow {
    id: root
    required property Item over
    property string title: ""
    property string confirmLabel: "Add"
    property bool confirmEnabled: true
    default property alias content: body.data
    signal confirmed
    signal cancelled

    function open() { visible = true; }
    function close() { visible = false; }

    anchor.item: over
    anchor.rect.x: Math.round((over ? over.width : 0) / 2 - implicitWidth / 2)
    anchor.rect.y: Math.round((over ? over.height : 0) / 2 - implicitHeight / 2)
    implicitWidth: 460
    implicitHeight: Math.min(560, card.implicitHeight)
    visible: false
    color: "transparent"

    Rectangle {
        id: card
        anchors.fill: parent
        radius: Theme.radiusPanel
        color: Theme.light ? "#FFFFFF" : Theme.raised
        border.width: 1
        border.color: Theme.hairlineStrong
        implicitHeight: col.implicitHeight + Theme.s5 * 2
        focus: root.visible
        onVisibleChanged: if (visible) forceActiveFocus()
        Keys.onEscapePressed: { root.cancelled(); root.close(); }

        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s5 }
            spacing: Theme.s4
            Label { text: root.title; size: Theme.sizeHeading; weight: Font.DemiBold }
            ColumnLayout { id: body; Layout.fillWidth: true; spacing: Theme.s3 }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s2
                Item { Layout.fillWidth: true }
                Button { text: "Cancel"; variant: "text"; onClicked: { root.cancelled(); root.close(); } }
                Button { text: root.confirmLabel; variant: "accent"; enabled: root.confirmEnabled; onClicked: { root.confirmed(); root.close(); } }
            }
        }
    }
}
