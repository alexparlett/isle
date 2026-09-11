import QtQuick
import QtQuick.Layouts
import Quickshell.Widgets
import qs.theme
import qs.services

// The tray's icons in a row: click activates, middle click the secondary action, right click asks for the
// menu, Ctrl-click puts the icon away (the Apps page brings it back). After them, the apps kept running:
// click shows or puts away its windows, right click asks for its menu (show or hide, quit), middle click quits.
RowLayout {
    id: root
    property int cell: 32
    property int iconSize: 18
    // The item whose menu was asked for, and where in this row it sits.
    signal menuRequested(var item, point at)
    signal activated(var item)
    spacing: Theme.s1
    visible: Tray.items.length > 0 || Windows.kept.length > 0

    Repeater {
        model: Tray.items
        Item {
            required property var modelData
            implicitWidth: root.cell; implicitHeight: root.cell
            opacity: area.containsMouse ? 1 : 0.7
            Rectangle { anchors.fill: parent; radius: Theme.radiusChip; color: area.containsMouse ? Theme.raised : "transparent" }
            IconImage { anchors.centerIn: parent; implicitSize: root.iconSize; source: modelData.icon }
            MouseArea {
                id: area
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                onClicked: mouse => {
                    if (mouse.modifiers & Qt.ControlModifier) { Tray.setHidden(modelData.id, true); return; }
                    if (mouse.button === Qt.RightButton && modelData.hasMenu) root.menuRequested(modelData, mapToItem(root, mouse.x, mouse.y));
                    else if (mouse.button === Qt.MiddleButton) Tray.secondary(modelData);
                    else { Tray.activate(modelData); root.activated(modelData); }
                }
            }
        }
    }
    Repeater {
        model: Windows.kept
        Item {
            required property var modelData
            implicitWidth: root.cell; implicitHeight: root.cell
            opacity: parkedArea.containsMouse || !modelData.hidden ? 1 : 0.6
            Rectangle { anchors.fill: parent; radius: Theme.radiusChip; color: parkedArea.containsMouse ? Theme.raised : "transparent" }
            AppIcon { anchors.centerIn: parent; size: root.iconSize; source: modelData.icon }
            MouseArea {
                id: parkedArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                onClicked: mouse => {
                    if (mouse.button === Qt.RightButton) root.menuRequested(modelData, mapToItem(root, mouse.x, mouse.y));
                    else if (mouse.button === Qt.MiddleButton) Windows.quitApp(modelData);
                    else { Windows.toggleKept(modelData); if (modelData.hidden) root.activated(modelData); }
                }
            }
        }
    }
    // The icons pack to the left however wide the row is.
    Item { Layout.fillWidth: true }
}
