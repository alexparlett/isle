import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui
import "kinds.js" as Kinds

// One folder or file: a box for the basket, its kind, its name and where it is, a bar against `maxSize`,
// and its size. A click goes to it; the box puts it in the basket.
Rectangle {
    id: row
    required property var app
    required property var item
    // The size the bar is measured against.
    property real maxSize: 1
    property bool showPath: true
    property bool showFiles: true
    // The last column says how many files a folder holds, or when anything in it last changed.
    property bool showAge: false
    // What to say under the name; where it is, when nothing is given.
    property string caption: ""

    // Too narrow for the bar and the count, which give way to the name.
    readonly property bool compact: width < 480
    // The row's own area loses the pointer to the box and the button on it, so any of the three counts.
    // Mouse areas rather than a HoverHandler, which is not told when the pointer leaves the window.
    readonly property bool hovered: area.containsMouse || showArea.containsMouse || check.hovered
    readonly property bool real: !!item.path && !item.other && !item.skipped
    readonly property bool picked: { app.basketRev; return real && app.inBasket(item.path); }
    readonly property color tint: item.skipped || item.other ? Theme.text3 : Kinds.color(item.kind)

    implicitHeight: showPath ? 46 : 38
    radius: Theme.radiusControl
    color: picked ? Qt.alpha(Theme.accent, 0.1) : hovered ? Theme.raised : "transparent"

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: row.real ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: if (row.real) row.app.open(row.item)
    }

    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s3 }
        spacing: Theme.s3

        Check {
            id: check
            app: row.app
            item: row.item
            opacity: row.picked || row.hovered ? 1 : 0.5
        }
        Rectangle {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            radius: Theme.radiusChip
            color: Qt.alpha(row.tint, 0.16)
            Glyph { anchors.centerIn: parent; name: row.item.skipped ? "hard-drive" : row.item.other ? "list" : row.item.dir ? "folder" : Kinds.glyph(row.item.kind); size: 14; color: row.tint }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 100
            Layout.preferredWidth: 400
            spacing: 1
            Label {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                text: row.item.other ? row.app.count(row.item.files) + " smaller items" : row.item.name
                weight: Font.DemiBold
                color: row.item.other || row.item.skipped ? Theme.text2 : Theme.text
            }
            Label {
                visible: row.showPath || row.item.skipped
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                text: row.item.skipped ? (row.item.name === ".snapshots" ? "Snapshots share their space with what they copy, so they are not counted" : "Another disk or a system folder, not counted")
                    : row.caption || row.app.place(Engine.parentOf(row.item.path))
                size: Theme.sizeCaption
                color: Theme.text3
            }
        }
        Rectangle {
            visible: !row.item.skipped && !row.compact
            Layout.fillWidth: true
            Layout.minimumWidth: 40
            Layout.maximumWidth: 140
            Layout.preferredWidth: 100
            implicitHeight: 6
            radius: 3
            color: Theme.hairline
            Rectangle {
                height: parent.height
                radius: 3
                width: Math.max(3, parent.width * Math.min(1, row.maxSize > 0 ? row.item.size / row.maxSize : 0))
                color: row.tint
            }
        }
        Label {
            text: row.item.skipped ? "" : Engine.formatSize(row.item.size)
            mono: true
            tabular: true
            weight: Font.DemiBold
            Layout.preferredWidth: 76
            horizontalAlignment: Text.AlignRight
        }
        Label {
            visible: row.showFiles && !row.compact
            text: row.item.dir && !row.item.skipped && !row.showAge ? row.app.count(row.item.files) + (row.item.files === 1 ? " file" : " files") : row.app.ago(row.item.newest)
            size: Theme.sizeCaption
            color: Theme.text3
            Layout.preferredWidth: 88
            horizontalAlignment: Text.AlignRight
        }
        Rectangle {
            Layout.preferredWidth: 28
            Layout.preferredHeight: 28
            radius: Theme.radiusChip
            visible: row.real
            opacity: row.hovered ? 1 : 0
            color: showArea.containsMouse ? Theme.pressed : "transparent"
            Glyph { anchors.centerIn: parent; name: "external-link"; size: 14; color: Theme.text2 }
            MouseArea { id: showArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: row.app.reveal(row.item) }
        }
    }
}
