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
    QsMenuOpener { id: opener; menu: root.menuItem ? root.menuItem.menu : null }
    Glass {
        visible: root.menuItem !== null && opener.children.values.length > 0
        z: 20
        x: Math.min(parent.width - width, menuAnchor.x); y: menuAnchor.y - height - Theme.s1
        property point menuAnchor: Qt.point(0, 0)
        id: menuCard
        width: 220; height: menuCol.implicitHeight + Theme.s2 * 2; radius: Theme.radiusCard
        ColumnLayout {
            id: menuCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
            spacing: 1
            Repeater {
                model: opener.children.values
                Item {
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: modelData.isSeparator ? 9 : 28
                    Rectangle { visible: modelData.isSeparator; anchors.centerIn: parent; width: parent.width - Theme.s3 * 2; height: 1; color: Theme.hairline }
                    Rectangle {
                        visible: !modelData.isSeparator
                        anchors.fill: parent; radius: Theme.radiusChip
                        color: entryArea.containsMouse ? Theme.raised : "transparent"
                        opacity: modelData.enabled ? 1 : 0.45
                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
                            spacing: Theme.s2
                            Glyph { visible: modelData.checkState === Qt.Checked; name: "check"; size: 10; color: Theme.accent }
                            Label { text: modelData.text; size: Theme.sizeSmall; Layout.fillWidth: true; elide: Text.ElideRight }
                            Glyph { visible: modelData.hasChildren; name: "chevron-right"; size: 10; color: Theme.text3 }
                        }
                        MouseArea { id: entryArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: modelData.enabled
                            onClicked: { modelData.triggered(); root.menuItem = null; } }
                    }
                }
            }
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

        Repeater {
            model: Tray.items
            Item {
                required property var modelData
                implicitWidth: 32; implicitHeight: 32
                opacity: 0.7
                IconImage { anchors.centerIn: parent; implicitSize: 18; source: modelData.icon }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton && modelData.hasMenu) {
                            const p = mapToItem(root, mouse.x, mouse.y);
                            menuCard.menuAnchor = Qt.point(p.x, p.y);
                            root.menuItem = root.menuItem === modelData ? null : modelData;
                        } else if (mouse.button === Qt.MiddleButton) Tray.secondary(modelData);
                        else { root.menuItem = null; Tray.activate(modelData); }
                    }
                }
            }
        }
        Item { Layout.fillWidth: true }
        Action { glyph: "lock"; action: () => Session.lock() }
        Action { glyph: "moon"; action: () => Session.sleep() }
        Action { glyph: "rotate-cw"; action: () => Session.restart() }
        Action { glyph: "power"; action: () => Session.shutdown() }
    }
}
