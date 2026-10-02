import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui
import "kinds.js" as Kinds

// The scan at a glance: what kinds of thing fill it, what is worth a look, and where the space is.
Flickable {
    id: root
    required property var app

    readonly property var whole: { DiskUsage.generation; return DiskUsage.node(DiskUsage.root); }
    // [[kind, bytes]] largest first, the empty ones left out.
    readonly property var byKind: (whole.kinds || []).map((b, k) => [k, b]).filter(p => p[1] > 0).sort((a, b) => b[1] - a[1])
    readonly property var hotspots: { DiskUsage.generation; return DiskUsage.hotspots(DiskUsage.root, 8); }
    readonly property var biggest: { DiskUsage.generation; return DiskUsage.largest(DiskUsage.root, 8); }
    readonly property var old: { DiskUsage.generation; return DiskUsage.untouched(DiskUsage.root, 365, 0); }

    function kindBytes(k) { return whole.kinds ? whole.kinds[k] : 0; }

    contentHeight: column.implicitHeight + Theme.s6
    clip: true
    boundsBehavior: Flickable.StopAtBounds

    component Card: Rectangle {
        radius: Theme.radiusCard
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairline
    }

    component Stat: Card {
        id: stat
        property string label
        property real bytes
        property string note
        property color tint: Theme.text
        property var action: null
        Layout.fillWidth: true
        Layout.preferredWidth: 1
        implicitHeight: 112
        color: statArea.containsMouse && action ? Theme.pressed : Theme.raised
        ColumnLayout {
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
            spacing: 2
            Label { text: stat.label; size: Theme.sizeSmall; color: Theme.text2 }
            Label { text: stat.bytes > 0 ? Engine.formatSize(stat.bytes) : "None"; size: Theme.sizeDisplay; weight: Font.DemiBold; color: stat.tint; Layout.fillWidth: true }
            Label { text: stat.note; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; wrapMode: Text.WordWrap; maximumLineCount: 2 }
        }
        MouseArea { id: statArea; anchors.fill: parent; hoverEnabled: true; cursorShape: stat.action ? Qt.PointingHandCursor : Qt.ArrowCursor; onClicked: if (stat.action) stat.action() }
    }

    ColumnLayout {
        id: column
        width: root.width
        spacing: Theme.s4

        Card {
            Layout.fillWidth: true
            implicitHeight: Math.max(168, hero.implicitHeight) + Theme.s5 * 2
            RowLayout {
                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: Theme.s5 }
                spacing: Theme.s6

                Item {
                    Layout.preferredWidth: 168
                    Layout.preferredHeight: 168
                    Donut {
                        anchors.fill: parent
                        parts: root.byKind.map(p => [p[1], Kinds.color(p[0])])
                    }
                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 0
                        Label { Layout.alignment: Qt.AlignHCenter; text: Engine.formatSize(root.whole.size || 0); size: Theme.sizeHeading; weight: Font.DemiBold; mono: true; tabular: true }
                        Label { Layout.alignment: Qt.AlignHCenter; text: root.app.placeName; size: Theme.sizeCaption; color: Theme.text3 }
                    }
                }

                ColumnLayout {
                    id: hero
                    Layout.fillWidth: true
                    spacing: Theme.s3
                    Label { text: Engine.formatSize(root.whole.size || 0); font.pixelSize: Math.round(Theme.sizeDisplay * 1.6); weight: Font.Bold }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        color: Theme.text2
                        text: "in " + root.app.placeName + (root.app.mount ? ", on a " + Engine.formatSize(root.app.mount.size) + " disk with " + Engine.formatSize(root.app.mount.size - root.app.mount.used) + " free." : ".")
                    }
                    // The same parts as the ring, laid end to end.
                    Row {
                        Layout.fillWidth: true
                        Layout.topMargin: Theme.s1
                        height: 10
                        spacing: 2
                        Repeater {
                            model: root.byKind
                            Rectangle {
                                required property var modelData
                                height: 10
                                width: Math.max(2, (parent.width - 2 * (root.byKind.length - 1)) * modelData[1] / (root.whole.size || 1))
                                radius: 3
                                color: Kinds.color(modelData[0])
                            }
                        }
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: Theme.s4
                        // A Flow's height follows its width; a layout needs it said.
                        Layout.preferredHeight: childrenRect.height
                        Repeater {
                            model: root.byKind
                            Row {
                                required property var modelData
                                spacing: Theme.s2
                                Rectangle { width: 8; height: 8; radius: 4; color: Kinds.color(modelData[0]); anchors.verticalCenter: parent.verticalCenter }
                                Label { text: DiskUsage.kindNames[modelData[0]]; size: Theme.sizeSmall; color: Theme.text2 }
                                Label { text: Engine.formatSize(modelData[1]); size: Theme.sizeSmall; weight: Font.DemiBold; mono: true; tabular: true }
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.s4
            Stat {
                label: "Untouched in a year"
                bytes: root.old.size || 0
                note: "Folders nothing has changed in"
                action: () => root.app.page = "old"
            }
            Stat {
                label: "Caches and logs"
                bytes: root.kindBytes(9)
                tint: Kinds.color(9)
                note: "Made again by the apps that made them"
                action: () => root.app.goTo(Engine.join(Engine.home, ".cache"))
            }
            Stat {
                label: "Developer"
                bytes: root.kindBytes(7)
                tint: Kinds.color(7)
                note: "Build output, dependencies and models"
                action: () => root.app.page = "space"
            }
            Stat {
                label: "In the trash"
                bytes: root.app.trashHeld
                note: root.app.trashHeld > 0 ? "Empty it to get the space back" : "Nothing waiting"
                action: root.app.trashHeld > 0 ? () => root.app.askEmpty = true : null
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.s4

            Repeater {
                model: [
                    { title: "Where the space is", rows: root.hotspots, path: true },
                    { title: "Biggest files", rows: root.biggest, path: true },
                ]
                Card {
                    id: card
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.alignment: Qt.AlignTop
                    implicitHeight: list.implicitHeight + Theme.s4 * 2
                    ColumnLayout {
                        id: list
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                        spacing: 2
                        Label { text: card.modelData.title; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.bottomMargin: Theme.s2; Layout.leftMargin: Theme.s2 }
                        Repeater {
                            model: card.modelData.rows
                            ItemRow {
                                required property var modelData
                                Layout.fillWidth: true
                                app: root.app
                                item: modelData
                                maxSize: card.modelData.rows.length ? card.modelData.rows[0].size : 1
                                showFiles: false
                            }
                        }
                    }
                }
            }
        }
    }
}
