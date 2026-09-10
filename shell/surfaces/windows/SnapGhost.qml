import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.services

// The translucent target a dragged floating window will tile to, from the isle-windows plugin's "isle_ghost" event.
Variants {
    model: Quickshell.screens

    PanelWindow {
        id: win
        required property var modelData
        screen: modelData
        readonly property var g: Windows.ghost
        readonly property bool mine: g !== null && g.x >= screen.x && g.x < screen.x + screen.width && g.y >= screen.y && g.y < screen.y + screen.height
        visible: box.opacity > 0

        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "isle-ghost"
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        mask: Region {}

        Rectangle {
            id: box
            x: win.mine ? win.g.x - win.screen.x : x
            y: win.mine ? win.g.y - win.screen.y : y
            width: win.mine ? win.g.w : width
            height: win.mine ? win.g.h : height
            radius: Theme.radiusPanel
            color: Qt.alpha(Theme.accent, 0.16)
            border.width: 1
            border.color: Qt.alpha(Theme.accent, 0.6)
            opacity: win.mine ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.quick } }
            Behavior on x { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on y { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on width { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on height { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
        }
    }
}
