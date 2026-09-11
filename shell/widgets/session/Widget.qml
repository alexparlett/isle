import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    id: root
    title: "Session"
    meta: Power.profileLabel + (Updates.count ? "  ·  " + Updates.count + " updates" : "")

    // A tray item's menu, opened by right click, listed in place.
    property var menuItem: null
    // A click anywhere else on the dashboard, or the dashboard going, closes the menu.
    Connections { target: Surfaces; function onDashboardPressed() { root.menuItem = null; } function onDashboardChanged() { if (!Surfaces.dashboard) root.menuItem = null; } }
    Glass {
        id: menuCard
        visible: root.menuItem !== null && !menu.empty
        z: 20
        x: Math.min(parent.width - width, menuAnchor.x); y: menuAnchor.y - height - Theme.s1
        property point menuAnchor: Qt.point(0, 0)
        width: 220; height: menu.implicitHeight + Theme.s2 * 2; radius: Theme.radiusCard
        TrayMenu {
            id: menu
            item: root.menuItem
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
            onDone: root.menuItem = null
        }
    }

    component Action: Item {
        property string glyph
        property var action
        implicitWidth: 36; implicitHeight: 36
        Rectangle { anchors.fill: parent; radius: Theme.radiusControl; color: area.containsMouse ? Theme.raised : "transparent" }
        Glyph { anchors.centerIn: parent; name: parent.glyph; size: 14; color: area.containsMouse ? Theme.text : Theme.text2 }
        MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.action() }
    }

    RowLayout {
        anchors.fill: parent
        spacing: Theme.s1

        TrayIcons {
            id: tray
            onActivated: root.menuItem = null
            onMenuRequested: (item, at) => {
                menuCard.menuAnchor = tray.mapToItem(root, at.x, at.y);
                root.menuItem = root.menuItem === item ? null : item;
            }
        }
        Item { Layout.fillWidth: true }
        Action { glyph: "lock"; action: () => Session.lock() }
        Action { glyph: "moon"; action: () => Session.sleep() }
        Action { glyph: "rotate-cw"; action: () => Session.restart() }
        Action { glyph: "power"; action: () => Session.shutdown() }
    }
}
