import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.services

// The frozen frame each monitor was grabbed to, shown under the pick so what is chosen is what is cut out.
Variants {
    model: Quickshell.screens

    PanelWindow {
        id: win
        required property var modelData
        screen: modelData
        visible: Capture.frozenDir !== ""

        // Above windows, below the island and the pick, which are overlay surfaces.
        WlrLayershell.layer: WlrLayer.Top
        WlrLayershell.namespace: "isle-freeze"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "black"
        mask: Region {}

        Image {
            anchors.fill: parent
            source: Capture.frozenDir ? "file://" + Capture.frozenDir + "/" + win.screen.name + ".png" : ""
            cache: false
            fillMode: Image.Stretch
        }
    }
}
