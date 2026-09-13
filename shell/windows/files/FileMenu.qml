import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// What a right click on the folder offers. Items are { label, glyph, action, danger, enabled }; a
// null item is a rule between groups.
Item {
    id: root
    property var items: []
    // Whether the menu is up. It cannot be taken from the card's own visibility: a child is only
    // visible when its parent is, so a parent whose visibility is read from its child never becomes
    // visible and neither does the child.
    property bool shown: false

    anchors.fill: parent
    visible: shown

    // Where the menu was asked for. Its own height is not known until the items it was given have
    // been laid out, so it is placed from these rather than from whatever size it had last time.
    property real askedX: 0
    property real askedY: 0

    function popup(x, y) {
        askedX = x;
        askedY = y;
        root.shown = true;
        card.forceActiveFocus();
    }
    // Kept inside the window, so a menu asked for near an edge does not hang off it.
    Binding {
        target: card
        property: "x"
        value: Math.max(Theme.s2, Math.min(root.askedX, root.width - card.width - Theme.s2))
        when: root.shown
    }
    Binding {
        target: card
        property: "y"
        value: Math.max(Theme.s2, Math.min(root.askedY, root.height - card.height - Theme.s2))
        when: root.shown
    }
    function close() { root.shown = false; }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: root.close()
    }

    // The raised tone, not glass: glass is for a surface over the wallpaper, where the compositor
    // blurs behind it. In a window it is the window's own colour and the menu cannot be seen.
    Rectangle {
        id: card
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
