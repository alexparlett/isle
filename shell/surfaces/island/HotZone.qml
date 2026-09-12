import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.services

// A 4px strip at the top edge of its screen, only in game mode: hovering it peeks that screen's island.
PanelWindow {
    id: root
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
