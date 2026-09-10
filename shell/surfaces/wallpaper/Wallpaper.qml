import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The background layer on every screen.
Variants {
    model: Quickshell.screens

    PanelWindow {
        required property var modelData
        screen: modelData

        WlrLayershell.layer: WlrLayer.Background
        WlrLayershell.namespace: "isle-wallpaper"
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: Theme.ink

        Backdrop { anchors.fill: parent; image: Prefs.p.wallpaper }
    }
}
