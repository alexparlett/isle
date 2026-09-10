import QtQuick
import qs.theme

// A thin overlay bar for a Flickable: shows while the content moves or the pointer is near, drags the content.
Item {
    id: root
    required property Flickable target
    anchors { right: parent.right; top: parent.top; bottom: parent.bottom; rightMargin: 2 }
    width: 8
    readonly property real span: Math.max(1, target.contentHeight - target.height)
    visible: target.contentHeight > target.height + 1
    readonly property real thumbH: Math.max(24, height * target.height / Math.max(1, target.contentHeight))
    opacity: target.moving || area.containsMouse || area.pressed ? 1 : 0.35
    Behavior on opacity { NumberAnimation { duration: Theme.quick } }

    Rectangle {
        id: thumb
        width: area.containsMouse || area.pressed ? 6 : 4
        anchors.right: parent.right
        radius: 3
        height: root.thumbH
        y: (root.height - height) * Math.min(1, Math.max(0, root.target.contentY / root.span))
        color: area.pressed ? Theme.text2 : Theme.text3
        Behavior on width { NumberAnimation { duration: Theme.quick } }
    }
    MouseArea {
        id: area
        anchors { fill: parent; leftMargin: -6 }
        hoverEnabled: true
        cursorShape: Qt.ArrowCursor
        property real grab: 0
        onPressed: mouse => { grab = mouse.y - (mouse.y >= thumb.y && mouse.y <= thumb.y + thumb.height ? thumb.y : thumb.height / 2); root.target.contentY = root.span * (mouse.y - grab) / Math.max(1, root.height - thumb.height); }
        onPositionChanged: mouse => { if (pressed) root.target.contentY = Math.min(root.span, Math.max(0, root.span * (mouse.y - grab) / Math.max(1, root.height - thumb.height))); }
    }
}
