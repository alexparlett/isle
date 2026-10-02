import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui
import "kinds.js" as Kinds

// What the rules found, as Safe to clear and Worth a look, each a list of groups that open onto their
// items; Safe to clear also carries what the system keeps, which goes through root rather than the trash.
ColumnLayout {
    id: root
    required property var app
    spacing: Theme.s3

    readonly property bool safe: app.cleanupTab === "safe"
    readonly property var groups: safe ? app.cleanup.safe : app.cleanup.worth
    readonly property var items: groups.reduce((all, g) => all.concat(g.items), [])
    property var open: ({})

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s3
        Segmented {
            options: [["safe", "Safe to clear  ·  " + Engine.formatSize(root.app.cleanup.safeSize)],
                      ["worth", "Worth a look  ·  " + Engine.formatSize(root.app.cleanup.worthSize)]]
            value: root.app.cleanupTab
            onPicked: v => root.app.cleanupTab = v
        }
        Item { Layout.fillWidth: true }
        Button { text: "Select all"; variant: "text"; implicitHeight: 30; visible: root.items.length > 0; onClicked: root.app.addAll(root.items) }
        Button { text: "Select none"; variant: "text"; implicitHeight: 30; visible: root.items.length > 0; onClicked: root.app.removeAll(root.items) }
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        color: Theme.text2
        text: root.safe ? "What apps and tools make again on their own. These start ticked; everything goes to the trash, where it can be put back."
                        : "Big things you might still want. Nothing here is ticked for you."
    }

    Flickable {
        id: flick
        Layout.fillWidth: true
        Layout.fillHeight: true
        contentHeight: list.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: list
            width: flick.width - Theme.s3
            spacing: Theme.s2

            Repeater {
                model: root.groups
                Rectangle {
                    id: card
                    required property var modelData
                    readonly property var group: modelData
                    readonly property bool expanded: !!root.open[group.id]
                    readonly property int ticked: { root.app.basketRev; return group.items.filter(i => root.app.inBasket(i.path)).length; }
                    Layout.fillWidth: true
                    implicitHeight: body.implicitHeight + Theme.s2 * 2
                    radius: Theme.radiusCard
                    color: Theme.raised
                    border.width: 1
                    border.color: Theme.hairline

                    ColumnLayout {
                        id: body
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
                        spacing: 2

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 52
                            radius: Theme.radiusControl
                            color: headArea.containsMouse ? Theme.pressed : "transparent"
                            MouseArea {
                                id: headArea
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: { const o = Object.assign({}, root.open); o[card.group.id] = !card.expanded; root.open = o; }
                            }
                            RowLayout {
                                anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s3 }
                                spacing: Theme.s3
                                // All, some or none of the group in the basket.
                                Rectangle {
                                    implicitWidth: 18
                                    implicitHeight: 18
                                    radius: 5
                                    color: card.ticked ? Theme.accent : "transparent"
                                    border.width: card.ticked ? 0 : 1.5
                                    border.color: Theme.hairlineStrong
                                    Glyph { anchors.centerIn: parent; visible: card.ticked === card.group.items.length; name: "check"; size: 12; weight: 2.5; color: Theme.onAccent }
                                    Rectangle { anchors.centerIn: parent; visible: card.ticked > 0 && card.ticked < card.group.items.length; width: 8; height: 2; radius: 1; color: Theme.onAccent }
                                    MouseArea {
                                        anchors { fill: parent; margins: -6 }
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: card.ticked === card.group.items.length ? root.app.removeAll(card.group.items) : root.app.addAll(card.group.items)
                                    }
                                }
                                Rectangle {
                                    implicitWidth: 32
                                    implicitHeight: 32
                                    radius: Theme.radiusChip
                                    color: Qt.alpha(Kinds.color(card.group.kind), 0.16)
                                    Glyph { anchors.centerIn: parent; name: card.group.glyph; size: 16; color: Kinds.color(card.group.kind) }
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1
                                    Label { text: card.group.name; weight: Font.DemiBold; Layout.fillWidth: true }
                                    Label { text: card.group.note; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; elide: Text.ElideRight }
                                }
                                Label { text: root.app.count(card.group.items.length) + (card.group.items.length === 1 ? " item" : " items"); size: Theme.sizeCaption; color: Theme.text3 }
                                Label { text: Engine.formatSize(card.group.size); mono: true; tabular: true; weight: Font.DemiBold; Layout.preferredWidth: 84; horizontalAlignment: Text.AlignRight }
                                Glyph { name: "chevron-down"; size: 14; color: Theme.text3; rotation: card.expanded ? 180 : 0 }
                            }
                        }

                        Repeater {
                            model: card.expanded ? card.group.items : []
                            ItemRow {
                                required property var modelData
                                Layout.fillWidth: true
                                Layout.leftMargin: Theme.s5
                                app: root.app
                                item: modelData
                                caption: root.app.place(modelData.path)
                                showAge: true
                                maxSize: card.group.items.length ? card.group.items[0].size : 1
                            }
                        }
                    }
                }
            }

            // The system's own, which only root can clear and which does not go to the trash.
            Rectangle {
                readonly property var due: root.app.system.filter(i => i.size > 0 || root.app.systemRunning === i.id)
                visible: root.safe && due.length > 0
                Layout.fillWidth: true
                Layout.topMargin: Theme.s3
                implicitHeight: sys.implicitHeight + Theme.s4 * 2
                radius: Theme.radiusCard
                color: Theme.raised
                border.width: 1
                border.color: Theme.hairline
                ColumnLayout {
                    id: sys
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                    spacing: Theme.s2
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s3
                        Glyph { name: "shield"; size: 16; color: Theme.text2 }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Label { text: "System"; weight: Font.DemiBold }
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Cleared by the system itself after your password, not moved to the trash, so it cannot be put back."; size: Theme.sizeCaption; color: Theme.text3 }
                        }
                    }
                    Repeater {
                        model: parent.parent.due
                        RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.leftMargin: Theme.s6
                            spacing: Theme.s3
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Label { text: modelData.name; weight: Font.DemiBold }
                                Label { Layout.fillWidth: true; elide: Text.ElideRight; text: modelData.note; size: Theme.sizeCaption; color: Theme.text3 }
                            }
                            Label { text: modelData.size > 0 ? Engine.formatSize(modelData.size) : "Nothing"; mono: true; tabular: true; weight: Font.DemiBold; color: modelData.size > 0 ? Theme.text : Theme.text3; Layout.preferredWidth: 84; horizontalAlignment: Text.AlignRight }
                            Item {
                                Layout.preferredWidth: 96
                                implicitHeight: 30
                                Spinner { anchors.centerIn: parent; visible: root.app.systemRunning === modelData.id; size: 16 }
                                Button {
                                    anchors.fill: parent
                                    visible: root.app.systemRunning !== modelData.id
                                    enabled: modelData.size > 0 && !root.app.systemRunning
                                    text: "Clear"
                                    onClicked: root.app.clearSystem(modelData.id)
                                }
                            }
                        }
                    }
                }
            }

            Label {
                visible: root.groups.length === 0
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: Theme.s6
                text: root.safe ? "Nothing to clear in " + root.app.placeName + "." : "Nothing worth a look in " + root.app.placeName + "."
                color: Theme.text3
            }
        }
        Scrollbar { target: flick; anchors { right: parent.right; top: parent.top; bottom: parent.bottom } }
    }
}
