import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// What a right click on the folder offers. Items are { label, glyph, action, danger, enabled }; a
// null item is a rule between groups.
Item {
    id: root
    property var items: []
    readonly property bool open: card.visible

    anchors.fill: parent
    visible: card.visible

    function popup(x, y) {
        // Kept inside the window, so a click near an edge does not put the menu off it.
        card.x = Math.max(Theme.s2, Math.min(x, root.width - card.width - Theme.s2));
        card.y = Math.max(Theme.s2, Math.min(y, root.height - card.height - Theme.s2));
        card.visible = true;
        card.forceActiveFocus();
    }
    function close() { card.visible = false; }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: root.close()
    }

    // The raised tone, not glass: glass is for a surface over the wallpaper, where the compositor
    // blurs behind it. In a window it is the window's own colour and the menu cannot be seen.
    Rectangle {
        id: card
        visible: false
        width: 210
        height: column.implicitHeight + Theme.s2 * 2
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairlineStrong
        radius: Theme.radiusCard
        focus: true
        Keys.onEscapePressed: root.close()

        ColumnLayout {
            id: column
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
            spacing: 0

            Repeater {
                model: root.items
                delegate: Item {
                    id: item
                    required property var modelData
                    // An item says when it cannot be used; one that does not say is usable.
                    readonly property bool usable: !!item.modelData && item.modelData.enabled !== false
                    Layout.fillWidth: true
                    implicitHeight: item.modelData ? 30 : Theme.s2 + 1

                    Rectangle {
                        visible: !item.modelData
                        anchors.centerIn: parent
                        width: parent.width - Theme.s2 * 2
                        height: 1
                        color: Theme.hairline
                    }

                    Rectangle {
                        visible: !!item.modelData
                        anchors.fill: parent
                        radius: Theme.radiusChip
                        color: area.containsMouse && item.usable ? Theme.pressed : "transparent"
                        opacity: item.usable ? 1 : 0.4

                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                            spacing: Theme.s3
                            Glyph {
                                name: item.modelData ? (item.modelData.glyph || "") : ""
                                size: 14
                                color: item.modelData && item.modelData.danger ? Theme.danger : Theme.text2
                            }
                            Label {
                                Layout.fillWidth: true
                                size: Theme.sizeSmall
                                text: item.modelData ? item.modelData.label : ""
                                color: item.modelData && item.modelData.danger ? Theme.danger : Theme.text
                            }
                        }

                        MouseArea {
                            id: area
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: item.usable
                            onClicked: { root.close(); item.modelData.action(); }
                        }
                    }
                }
            }
        }
    }
}
