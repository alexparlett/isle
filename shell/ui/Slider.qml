import QtQuick
import qs.theme

// A 4px track with an accent fill. `value` is 0..1; `moved(v)` fires on drag or click.
Item {
    id: root
    property real value: 0
    property bool interactive: true
    signal moved(real v)

    implicitHeight: 16
    implicitWidth: 120

    Rectangle {
        id: track
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width
        height: 4
        radius: 2
        color: Theme.hairlineStrong

        Rectangle {
            width: Math.max(4, track.width * Math.max(0, Math.min(1, root.value)))
            height: parent.height
            radius: 2
            color: Theme.accent
            Behavior on width { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.interactive
        cursorShape: Qt.PointingHandCursor
        function at(x) { root.moved(Math.max(0, Math.min(1, x / width))); }
        onPressed: mouse => at(mouse.x)
        onPositionChanged: mouse => { if (pressed) at(mouse.x); }
    }
}
