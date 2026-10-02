import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui
import "kinds.js" as Kinds

// Files that are the same inside, a card for each set of copies, and a way to tick all but one of each.
ColumnLayout {
    id: root
    required property var app
    spacing: Theme.s3

    readonly property var found: app.dupes
    property string keep: "oldest"

    Component.onCompleted: if (!DiskUsage.duplicatesReady && !DiskUsage.hashing) DiskUsage.findDuplicates()

    // The copy each set keeps under the current choice.
    function keeper(group) {
        const files = group.files.slice();
        if (keep === "newest") files.sort((a, b) => b.newest - a.newest);
        else if (keep === "shortest") files.sort((a, b) => a.path.length - b.path.length || a.newest - b.newest);
        return files[0];
    }
    function tickRest() {
        for (const g of found.groups) {
            const k = keeper(g);
            app.removeAll([k]);
            app.addAll(g.files.filter(f => f.path !== k.path));
        }
    }

    component Stat: Rectangle {
        id: stat
        property string label
        property string value
        property string note
        property color tint: Theme.text
        Layout.fillWidth: true
        Layout.preferredWidth: 1
        implicitHeight: statCol.implicitHeight + Theme.s4 * 2
        radius: Theme.radiusCard
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairline
        ColumnLayout {
            id: statCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
            spacing: 2
            Label { text: stat.label; size: Theme.sizeSmall; color: Theme.text2 }
            Label { text: stat.value; size: Theme.sizeDisplay; weight: Font.DemiBold; color: stat.tint }
            Label { text: stat.note; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; wrapMode: Text.WordWrap; maximumLineCount: 2 }
        }
    }

    // Comparing.
    ColumnLayout {
        visible: DiskUsage.hashing
        Layout.fillWidth: true
        spacing: Theme.s2
        RowLayout {
            Layout.fillWidth: true
            Label { Layout.fillWidth: true; text: DiskUsage.hashProgress > 0 ? "Reading the files that might match  ·  " + Math.round(DiskUsage.hashProgress * 100) + "%" : "Comparing files of the same size…"; color: Theme.text2 }
            Button { text: "Stop"; glyph: "x"; implicitHeight: 30; onClicked: DiskUsage.cancelDuplicates() }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 4
            radius: 2
            color: Theme.hairline
            Rectangle { height: parent.height; radius: 2; width: parent.width * DiskUsage.hashProgress; color: Theme.accent }
        }
    }

    RowLayout {
        visible: DiskUsage.duplicatesReady
        Layout.fillWidth: true
        spacing: Theme.s4
        Stat { label: "Can be freed"; value: Engine.formatSize(root.found.canFree); tint: Theme.accent; note: "Keeping one copy of each" }
        Stat { label: "Sets of copies"; value: root.app.count(root.found.groups.length); note: root.app.count(root.found.files) + " files in all" }
        Stat { label: "Already sharing space"; value: root.app.count(root.found.sharing); note: "Copies that share their blocks on the disk, so are not counted" }
    }

    RowLayout {
        visible: DiskUsage.duplicatesReady && root.found.groups.length > 0
        Layout.fillWidth: true
        spacing: Theme.s3
        Label { Layout.fillWidth: true; text: "Your own files of a megabyte or more, matched by what is inside them. Apps, games, repositories and hidden folders are left alone."; color: Theme.text2; wrapMode: Text.WordWrap }
        Label { text: "Keep"; size: Theme.sizeSmall; color: Theme.text2 }
        Dropdown {
            listWidth: 180
            options: [["oldest", "The oldest"], ["newest", "The newest"], ["shortest", "The shortest path"]]
            value: root.keep
            onPicked: v => root.keep = v
        }
        Button { text: "Tick the rest"; glyph: "check"; onClicked: root.tickRest() }
    }

    ListView {
        id: list
        visible: DiskUsage.duplicatesReady
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        spacing: Theme.s2
        boundsBehavior: Flickable.StopAtBounds
        model: root.found.groups
        delegate: Rectangle {
            id: card
            required property var modelData
            readonly property var group: modelData
            readonly property var kept: root.keeper(group)
            readonly property int ticked: { root.app.basketRev; return group.files.filter(f => root.app.inBasket(f.path)).length; }
            width: ListView.view.width - Theme.s3
            implicitHeight: body.implicitHeight + Theme.s2 * 2
            radius: Theme.radiusCard
            color: Theme.raised
            border.width: 1
            border.color: ticked === group.files.length ? Qt.alpha(Theme.warn, 0.5) : Theme.hairline

            ColumnLayout {
                id: body
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
                spacing: 2
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: Theme.s2
                    spacing: Theme.s3
                    Rectangle {
                        implicitWidth: 32
                        implicitHeight: 32
                        radius: Theme.radiusChip
                        color: Qt.alpha(Kinds.color(card.group.kind), 0.16)
                        Glyph { anchors.centerIn: parent; name: Kinds.glyph(card.group.kind); size: 16; color: Kinds.color(card.group.kind) }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Label { Layout.fillWidth: true; text: card.group.name; weight: Font.DemiBold; elide: Text.ElideMiddle }
                        Label {
                            Layout.fillWidth: true
                            text: card.ticked === card.group.files.length ? "Every copy is ticked; nothing would be left"
                                : card.group.count + " copies of " + Engine.formatSize(card.group.each) + (card.group.shared ? ", " + card.group.shared + " already sharing space" : "")
                            size: Theme.sizeCaption
                            color: card.ticked === card.group.files.length ? Theme.warn : Theme.text3
                        }
                    }
                    Label { text: Engine.formatSize(card.group.reclaim) + " can go"; mono: true; tabular: true; weight: Font.DemiBold; color: Theme.accent }
                }
                Repeater {
                    model: card.group.files
                    ItemRow {
                        required property var modelData
                        Layout.fillWidth: true
                        app: root.app
                        item: Object.assign({}, modelData, { name: root.app.place(Engine.parentOf(modelData.path)) })
                        caption: modelData.path === card.kept.path ? modelData.name + "  ·  kept" : modelData.name
                        showAge: true
                        maxSize: card.group.each
                    }
                }
            }
        }
        Scrollbar { target: list }
        Label { anchors.centerIn: parent; visible: list.count === 0; text: "No copies of your own files in " + root.app.placeName + "."; color: Theme.text3 }
    }

    Item { visible: !DiskUsage.duplicatesReady && !DiskUsage.hashing; Layout.fillHeight: true }
}
