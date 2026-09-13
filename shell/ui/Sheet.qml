import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import qs.theme

// A card over the window, for the few things that need answering before they can be done: a title, whatever
// the caller puts inside, and two buttons. It reparents itself to the window's root rather than living where
// it was written, so it is not carried about by a page's scrolling and takes the keyboard as the window's own.
Item {
    id: root
    property string title: ""
    property string confirmLabel: "Add"
    property bool confirmEnabled: true
    default property alias content: body.data
    signal confirmed
    signal cancelled

    function open() { visible = true; if (!focusFirst(body)) card.forceActiveFocus(); }
    function close() { visible = false; }

    // The first thing that takes typing, wherever the caller nested it.
    function focusFirst(item) {
        for (const c of item.children) {
            if (c.input !== undefined && c.input !== null) { c.input.forceActiveFocus(); return true; }
            if (focusFirst(c)) return true;
        }
        return false;
    }

    // The window's root, so a page's scrolling does not carry the card about. The attached Window is not
    // always there when this is built, so the tree is walked when it is not.
    Component.onCompleted: {
        let top = Window.contentItem;
        if (!top) { top = root.parent; while (top && top.parent) top = top.parent; }
        if (top && top !== root.parent) root.parent = top;
        anchors.fill = root.parent;
    }
    visible: false
    z: 100

    // The dim swallows what is behind it, and a click on it is a way out.
    Rectangle {
        anchors.fill: parent
        color: Qt.alpha(Theme.ink, 0.5)
        MouseArea { anchors.fill: parent; onClicked: { root.cancelled(); root.close(); } }
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 460
        implicitHeight: col.implicitHeight + Theme.s5 * 2
        height: implicitHeight
        radius: Theme.radiusPanel
        color: Theme.light ? "#FFFFFF" : Theme.raised
        border.width: 1
        border.color: Theme.hairlineStrong
        focus: root.visible
        Keys.onEscapePressed: { root.cancelled(); root.close(); }
        // A click on the card itself is not a click on the dim behind it.
        MouseArea { anchors.fill: parent }

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
