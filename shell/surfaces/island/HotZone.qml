import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.services

// A 4px strip at the top edge, only in game mode: hovering it peeks the island.
PanelWindow {
    id: root
    screen: Compositor.shellScreen
    visible: Modes.game
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-hotzone"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; left: true; right: true }
    implicitHeight: 4
    color: "transparent"
    signal peek
    HoverHandler { onHoveredChanged: if (hovered) root.peek() }
}
