import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui

// Where the space went, one folder at a time: its children as tiles or a list, under a trail back.
ColumnLayout {
    id: root
    required property var app
    spacing: Theme.s3

    property string filter: ""
    readonly property var here: { DiskUsage.generation; return DiskUsage.node(app.at); }
    readonly property var rows: {
        DiskUsage.generation;
        const all = DiskUsage.children(app.at, 200);
        const q = filter.toLowerCase();
        return q ? all.filter(r => !r.other && r.name.toLowerCase().indexOf(q) >= 0) : all;
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s2

        Rectangle {
            implicitWidth: 30
            implicitHeight: 30
            radius: Theme.radiusChip
            opacity: root.app.at !== DiskUsage.root ? 1 : 0.35
            color: upArea.containsMouse ? Theme.pressed : Theme.raised
            border.width: 1
            border.color: Theme.hairline
            Glyph { anchors.centerIn: parent; name: "arrow-up"; size: 14; color: Theme.text }
            MouseArea { id: upArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.app.up() }
        }
        Flickable {
            Layout.fillWidth: true
            implicitHeight: 30
            contentWidth: trail.implicitWidth
            contentX: Math.max(0, contentWidth - width)
            clip: true
            interactive: contentWidth > width
            Row {
                id: trail
                height: 30
                spacing: Theme.s1
                Repeater {
                    model: root.app.crumbs()
                    Row {
                        required property var modelData
                        required property int index
                        height: 30
                        spacing: Theme.s1
                        Glyph { visible: index > 0; name: "chevron-right"; size: 12; color: Theme.text3; anchors.verticalCenter: parent.verticalCenter }
                        Rectangle {
                            readonly property bool current: modelData.path === root.app.at
                            height: 26
                            anchors.verticalCenter: parent.verticalCenter
                            width: crumb.implicitWidth + Theme.s3 * 2
                            radius: 13
                            color: current ? Qt.alpha(Theme.accent, 0.16) : crumbArea.containsMouse ? Theme.pressed : Theme.raised
                            border.width: 1
                            border.color: current ? Qt.alpha(Theme.accent, 0.4) : Theme.hairline
                            Label { id: crumb; anchors.centerIn: parent; text: modelData.name; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.current ? Theme.accent : Theme.text2 }
                            MouseArea { id: crumbArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.app.goTo(modelData.path) }
                        }
                    }
                }
            }
        }
        Field {
            Layout.preferredWidth: 200
            implicitHeight: 30
            size: Theme.sizeSmall
            glyph: "search"
            placeholder: "Filter by name"
            onTextChanged: root.filter = text
        }
        Segmented {
            options: [["tiles", "Tiles"], ["list", "List"]]
            value: root.app.spaceView
            onPicked: v => root.app.spaceView = v
        }
    }

    Label {
        text: Engine.formatSize(root.here.size || 0) + "  ·  " + root.app.count(root.here.files || 0) + " files  ·  last changed " + root.app.ago(root.here.newest || 0)
        size: Theme.sizeSmall
        color: Theme.text3
    }

    Treemap {
        visible: root.app.spaceView === "tiles"
        Layout.fillWidth: true
        Layout.fillHeight: true
        app: root.app
        items: visible ? root.rows.slice(0, 60) : []
        total: root.here.size || 1
    }

    ListView {
        id: list
        visible: root.app.spaceView === "list"
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: visible ? root.rows : []
        delegate: ItemRow {
            required property var modelData
            width: ListView.view.width - Theme.s3
            app: root.app
            item: modelData
            maxSize: root.rows.length ? root.rows[0].size : 1
            showPath: false
        }
        Scrollbar { target: list; anchors { right: parent.right; top: parent.top; bottom: parent.bottom } }
    }
}
