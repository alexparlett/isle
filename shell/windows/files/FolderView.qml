import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui
import qs.services

// The folder, as a list of rows or a grid of thumbnails. Both are one selection and one set of
// keys, so the window and the chooser speak to this and not to either view.
FocusScope {
    id: root
    required property Directory directory
    // "list", "columns" or "grid".
    property string mode: "list"
    // Whether a file is opened where it lands. A chooser takes it as the choice instead.
    property bool openFiles: true
    // A folder to walk into.
    signal activated(string path)
    // A file settled on, when openFiles is false.
    signal chosen(string path)
    // A right click, with where it landed in this item's coordinates and the row under it, if any.
    signal menuAsked(real x, real y, string path)

    // The row the keys are on, and the one a shift click measures from.
    property int index: 0
    // Everything picked, by path. One click makes it the one row; ctrl adds, shift takes a run.
    property var selection: []

    // Columns pick by path; the other two pick by row. Either way it is one selection to the caller.
    readonly property bool hasSelection: mode === "columns"
        ? (columns.selected !== "" && !columns.selectedIsDir)
        : (selection.length === 1 && !Engine.isDir(selection[0]))
    readonly property string currentPath: mode === "columns"
        ? (columns.selectedIsDir ? "" : columns.selected)
        : (hasSelection ? selection[0] : "")
    // What an action acts on: everything picked, or the row the keys are on when nothing is.
    readonly property var acting: mode === "columns"
        ? (columns.selected ? [columns.selected] : [])
        : (selection.length ? selection
           : (index >= 0 && index < directory.count ? [directory.pathAt(index)] : []))

    function pick(row, modifiers) {
        if (row < 0 || row >= directory.count) return;
        const path = directory.pathAt(row);
        if (modifiers & Qt.ControlModifier) {
            const at = selection.indexOf(path);
            selection = at < 0 ? selection.concat([path]) : selection.filter(p => p !== path);
        } else if (modifiers & Qt.ShiftModifier) {
            const from = Math.min(index, row), to = Math.max(index, row);
            const run = [];
            for (let i = from; i <= to; i++) run.push(directory.pathAt(i));
            selection = run;
            return;                              // the anchor stays where the run started
        } else {
            selection = [path];
        }
        index = row;
    }

    function selectAll() {
        const all = [];
        for (let i = 0; i < directory.count; i++) all.push(directory.pathAt(i));
        selection = all;
    }

    function isPicked(path) { return selection.indexOf(path) >= 0; }

    // What a drag carries: the uri list every desktop reads, so a drop lands in other applications too.
    function uriList(paths) { return paths.map(p => "file://" + encodeURI(p)).join("\r\n"); }
    // Something dropped here: moved when it came from this machine's own folder, copied otherwise.
    signal dropped(var paths, string into)
    readonly property int nameWidth: Math.max(220, width - 320)

    function open(row) {
        if (row < 0 || row >= directory.count) return;
        const path = directory.pathAt(row);
        if (directory.isDirAt(row)) root.activated(path);
        else if (root.openFiles) Compositor.exec("xdg-open " + JSON.stringify(path));
        else root.chosen(path);
    }

    // The column already sorting turns over; another starts up, rather than inheriting a direction
    // set for something else.
    function sortBy(column) {
        if (directory.sort === column)
            directory.sortOrder = directory.sortOrder === Qt.AscendingOrder ? Qt.DescendingOrder : Qt.AscendingOrder;
        else {
            directory.sort = column;
            directory.sortOrder = Qt.AscendingOrder;
        }
    }

    // The icon theme's name for a row, read from the file itself when its name said nothing. Only
    // the rows on screen ever ask, so a folder of fifty thousand costs the forty that are visible.
    function iconFor(path, iconName) {
        return Engine.isGenericIcon(iconName) ? Engine.sniffIconName(path) : iconName;
    }

    // The folder changed under the selection: start at the top rather than on whatever is there now.
    Connections {
        target: root.directory
        function onPathChanged() {
            root.index = 0;
            root.selection = [];
            list.positionViewAtBeginning();
            grid.positionViewAtBeginning();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The header is the list's; a grid has no columns to head.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            visible: root.mode === "list"
            color: "transparent"
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 + 8 }
                spacing: Theme.s3
                Repeater {
                    model: [{ label: "Name", column: Directory.ByName, width: root.nameWidth, align: Text.AlignLeft },
                            { label: "Size", column: Directory.BySize, width: 90, align: Text.AlignRight },
                            { label: "Modified", column: Directory.ByModified, width: 130, align: Text.AlignRight }]
                    delegate: Item {
                        id: head
                        required property var modelData
                        Layout.preferredWidth: head.modelData.width
                        Layout.fillWidth: head.modelData.column === Directory.ByName
                        Layout.fillHeight: true
                        RowLayout {
                            anchors.fill: parent
                            spacing: 2
                            layoutDirection: head.modelData.align === Text.AlignRight ? Qt.RightToLeft : Qt.LeftToRight
                            Label {
                                text: head.modelData.label
                                size: Theme.sizeCaption
                                color: root.directory.sort === head.modelData.column ? Theme.text2 : Theme.text3
                            }
                            Glyph {
                                visible: root.directory.sort === head.modelData.column
                                name: root.directory.sortOrder === Qt.AscendingOrder ? "chevron-up" : "chevron-down"
                                size: 11
                                color: Theme.text3
                            }
                            Item { Layout.fillWidth: true }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.sortBy(head.modelData.column) }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            DropArea {
                anchors.fill: parent
                keys: ["text/uri-list"]
                onDropped: drop => {
                    root.dropped(String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                        .filter(u => u.startsWith("file://"))
                        .map(u => decodeURI(u.slice(7))), root.directory.path);
                    drop.acceptProposedAction();
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.RightButton
                onClicked: mouse => {
                    const at = mapToItem(root, mouse.x, mouse.y);
                    root.menuAsked(at.x, at.y, "");
                }
            }

            ListView {
                id: list
                anchors.fill: parent
                visible: root.mode === "list"
                enabled: visible
                model: root.directory
                clip: true
                focus: root.mode === "list"
                currentIndex: root.index
                onCurrentIndexChanged: if (visible && currentIndex !== root.index) root.pick(currentIndex, Qt.NoModifier)
                // A folder of fifty thousand rows only ever builds the ones on screen.
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds

                Keys.onReturnPressed: root.open(root.index)
                Keys.onEnterPressed: root.open(root.index)

                delegate: Rectangle {
                    id: row
                    required property int index
                    required property string name
                    required property string path
                    required property string iconName
                    required property var size
                    required property var modified
                    required property bool isDir
                    required property bool isSymlink

                    width: list.width
                    height: 30

                    Drag.active: rowArea.drag.active
                    Drag.dragType: Drag.Automatic
                    Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                    Drag.mimeData: ({ "text/uri-list": root.uriList(root.isPicked(row.path) ? root.selection : [row.path]) })
                    color: root.isPicked(row.path) ? Theme.pressed
                         : ListView.isCurrentItem ? Theme.raised
                         : rowArea.containsMouse ? Theme.raised : "transparent"

                    DropArea {
                        anchors.fill: parent
                        enabled: row.isDir
                        keys: ["text/uri-list"]
                        onDropped: drop => {
                            root.dropped(String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                                .filter(u => u.startsWith("file://"))
                                .map(u => decodeURI(u.slice(7))), row.path);
                            drop.acceptProposedAction();
                        }
                        Rectangle {
                            anchors.fill: parent
                            visible: parent.containsDrag
                            color: "transparent"
                            border.width: 2
                            border.color: Theme.accent
                            radius: Theme.radiusControl
                        }
                    }

                    RowLayout {
                        anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 }
                        spacing: Theme.s3

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.nameWidth
                            spacing: Theme.s2 + 2
                            IconImage {
                                implicitSize: 18
                                source: Quickshell.iconPath(root.iconFor(row.path, row.iconName), "text-x-generic")
                            }
                            Label {
                                Layout.fillWidth: true
                                text: row.name
                                color: row.isSymlink ? Theme.text2 : Theme.text
                            }
                        }
                        Label {
                            Layout.preferredWidth: 90
                            horizontalAlignment: Text.AlignRight
                            size: Theme.sizeSmall
                            color: Theme.text3
                            tabular: true
                            text: row.isDir ? "—" : Engine.formatSize(row.size)
                        }
                        Label {
                            Layout.preferredWidth: 130
                            horizontalAlignment: Text.AlignRight
                            size: Theme.sizeSmall
                            color: Theme.text3
                            tabular: true
                            text: Engine.formatModified(row.modified)
                        }
                    }

                    MouseArea {
                        id: rowArea
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        drag.target: row
                        // An automatic drag is handed to the platform, and is started rather than
                        // simply declared. The row itself must not travel, so it is put back after.
                        drag.onActiveChanged: {
                            if (!rowArea.drag.active) return;
                            if (!root.isPicked(row.path)) root.pick(row.index, Qt.NoModifier);
                            row.Drag.startDrag();
                        }
                        onClicked: mouse => {
                            root.pick(row.index, mouse.modifiers);
                            list.forceActiveFocus();
                            if (mouse.button === Qt.RightButton) {
                                const at = mapToItem(root, mouse.x, mouse.y);
                                root.menuAsked(at.x, at.y, row.path);
                            }
                        }
                        onDoubleClicked: root.open(row.index)
                    }
                }
            }

            GridView {
                id: grid
                anchors.fill: parent
                visible: root.mode === "grid"
                enabled: visible
                model: root.directory
                clip: true
                focus: root.mode === "grid"
                currentIndex: root.index
                onCurrentIndexChanged: if (visible && currentIndex !== root.index) root.pick(currentIndex, Qt.NoModifier)
                cellWidth: 116
                cellHeight: 116
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds

                Keys.onReturnPressed: root.open(root.index)
                Keys.onEnterPressed: root.open(root.index)

                delegate: Item {
                    id: cell
                    required property int index
                    required property string name
                    required property string path
                    required property string iconName
                    required property var modified
                    required property bool isDir

                    width: grid.cellWidth
                    height: grid.cellHeight

                    Rectangle {
                        anchors { fill: parent; margins: 3 }
                        radius: Theme.radiusControl
                        color: root.isPicked(cell.path) ? Theme.pressed
                             : cell.GridView.isCurrentItem ? Theme.raised
                             : cellArea.containsMouse ? Theme.raised : "transparent"

                        ColumnLayout {
                            anchors { fill: parent; margins: Theme.s2 }
                            spacing: Theme.s1

                            Item {
                                Layout.alignment: Qt.AlignHCenter
                                Layout.preferredWidth: 64
                                Layout.preferredHeight: 64

                                IconImage {
                                    anchors.centerIn: parent
                                    implicitSize: 48
                                    // Kept until a thumbnail arrives, and left in place when none can be made.
                                    visible: !thumb.visible
                                    source: Quickshell.iconPath(root.iconFor(cell.path, cell.iconName), "text-x-generic")
                                }
                                Image {
                                    id: thumb
                                    // Empty until the file is on disk; the signal says when it is.
                                    property string file: !cell.isDir && Thumbnails.canThumbnail(cell.path)
                                        ? Thumbnails.thumbnail(cell.path, cell.modified.getTime() / 1000) : ""
                                    Connections {
                                        target: Thumbnails
                                        function onReady(path, file) { if (path === cell.path) thumb.file = file; }
                                    }
                                    anchors.centerIn: parent
                                    visible: status === Image.Ready
                                    asynchronous: true
                                    // The cache file keeps its name when the picture behind it changes,
                                    // so Qt must not answer from a pixmap it kept under that name.
                                    cache: false
                                    fillMode: Image.PreserveAspectFit
                                    sourceSize.width: 128
                                    sourceSize.height: 128
                                    width: Math.min(implicitWidth, 64)
                                    height: Math.min(implicitHeight, 64)
                                    source: thumb.file ? "file://" + thumb.file : ""
                                }
                            }

                            Label {
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                                size: Theme.sizeCaption
                                maximumLineCount: 2
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                                text: cell.name
                            }
                        }

                        MouseArea {
                            id: cellArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => {
                                root.pick(cell.index, mouse.modifiers);
                                grid.forceActiveFocus();
                                if (mouse.button === Qt.RightButton) {
                                    const at = mapToItem(root, mouse.x, mouse.y);
                                    root.menuAsked(at.x, at.y, cell.path);
                                }
                            }
                            onDoubleClicked: root.open(cell.index)
                        }
                    }
                }
            }

            Scrollbar { target: root.mode === "grid" ? grid : list; visible: root.mode !== "columns" }

            FileColumns {
                id: columns
                anchors.fill: parent
                visible: root.mode === "columns"
                enabled: visible
                rootPath: root.directory.path
                showHidden: root.directory.showHidden
                onActivated: path => root.activated(path)
                onChosen: path => root.openFiles ? Compositor.exec("xdg-open " + JSON.stringify(path)) : root.chosen(path)
            }

            // Nothing to show, and why.
            Label {
                anchors.centerIn: parent
                visible: root.mode !== "columns" && root.directory.status === Directory.Ready && root.directory.count === 0
                color: Theme.text3
                text: root.directory.total === 0 ? "This folder is empty" : "Nothing here matches"
            }
            Spinner {
                anchors.centerIn: parent
                visible: root.directory.status === Directory.Loading
                size: 20
                color: Theme.text3
            }
        }
    }
}
