import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.services

// Idle dim: a click-through veil over every screen that fades in when the idle timeout passes and out on input.
Variants {
    model: Quickshell.screens

    PanelWindow {
        required property var modelData
        screen: modelData
        visible: veil.opacity > 0

        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "isle-dim"
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        // An empty input region: pointer and keys go to what is underneath, which is also what ends the idle.
        mask: Region {}

        Rectangle {
            id: veil
            anchors.fill: parent
            color: Theme.ink
            opacity: Idle.dimmed && !Lock.locked ? 0.6 : 0
            Behavior on opacity { NumberAnimation { duration: 1200; easing.type: Easing.InOutQuad } }
        }
    }
}
