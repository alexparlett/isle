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
    // How big a grid cell is. Finder has a slider; this is the same idea with four stops.
    property int iconSize: 64
    readonly property int cellSize: iconSize + 52
    // Whether a file is opened where it lands. A chooser takes it as the choice instead.
    property bool openFiles: true
    // A folder to walk into.
    signal activated(string path)
    // A file settled on, when openFiles is false.
    signal chosen(string path)
    // A right click, with where it landed in this item's coordinates and the row under it, if any.
    signal menuAsked(real x, real y, string path)
    // A folder asked for in a tab of its own, by the middle button.
    signal openedInTab(string path)
    // A file to open with whatever the desktop says handles it.
    signal asked(string path)

    // The row the cursor is on, and the row a run is measured from.
    property int index: 0
    property int anchor: 0
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
            const from = Math.min(anchor, row), to = Math.max(anchor, row);
            const run = [];
            for (let i = from; i <= to; i++) run.push(directory.pathAt(i));
            selection = run;
            index = row;
            return;                              // the anchor stays where the run started
        } else {
            selection = [path];
        }
        index = row;
        anchor = row;
    }

    // Moving with the keys picks as it goes; with Shift it grows the run instead.
    function step(by, modifiers) {
        const to = Math.max(0, Math.min(directory.count - 1, index + by));
        pick(to, modifiers & Qt.ShiftModifier ? Qt.ShiftModifier : Qt.NoModifier);
        if (mode === "grid") grid.positionViewAtIndex(to, GridView.Contain);
        else list.positionViewAtIndex(to, ListView.Contain);
    }

    // Home, End and the page keys, and typing the start of a name to jump to it.
    property string typed: ""
    Timer { id: typing; interval: 900; onTriggered: root.typed = "" }

    function walk(event, page) {
        if (event.key === Qt.Key_Home) { pick(0, event.modifiers); show(0); event.accepted = true; }
        else if (event.key === Qt.Key_End) { pick(directory.count - 1, event.modifiers); show(directory.count - 1); event.accepted = true; }
        else if (event.key === Qt.Key_PageUp) { step(-page, event.modifiers); event.accepted = true; }
        else if (event.key === Qt.Key_PageDown) { step(page, event.modifiers); event.accepted = true; }
        else if (event.text && event.text.length === 1 && event.text >= " " && !(event.modifiers & Qt.ControlModifier)) {
            root.typed += event.text.toLowerCase();
            typing.restart();
            const at = directory.startingWith(root.typed, 0);
            if (at >= 0) { pick(at, Qt.NoModifier); show(at); }
            event.accepted = true;
        }
    }

    function show(row) {
        if (mode === "grid") grid.positionViewAtIndex(row, GridView.Contain);
        else list.positionViewAtIndex(row, ListView.Contain);
    }

    function selectAll() {
        const all = [];
        for (let i = 0; i < directory.count; i++) all.push(directory.pathAt(i));
        selection = all;
    }

    function isPicked(path) { return selection.indexOf(path) >= 0; }

    // The row whose name is being edited where it sits, as Finder renames. Empty while none is.
    property string renaming: ""
    // A new name settled on; the window does the renaming, since it owns the jobs.
    signal renamed(string path, string name)

    function beginRename(path) { renaming = path || (acting.length === 1 ? acting[0] : ""); }

    // A folder that has only just appeared: picked, brought into view, and its name open for typing.
    function beginRenameWhenSeen(path) {
        const row = directory.rowOf(Engine.displayName(path));
        if (row < 0) { pending = path; return; }
        pending = "";
        pick(row, Qt.NoModifier);
        if (mode === "grid") grid.positionViewAtIndex(row, GridView.Contain);
        else list.positionViewAtIndex(row, ListView.Contain);
        renaming = path;
    }
    // A path waiting for the folder to be read again before it can be named.
    property string pending: ""
    function endRename(path, name) {
        renaming = "";
        const was = Engine.displayName(path);
        if (name && name !== was) root.renamed(path, name);
    }

    // What a drag carries: the uri list every desktop reads, so a drop lands in other applications too.
    function uriList(paths) { return paths.map(p => "file://" + encodeURI(p)).join("\r\n"); }
    // Something dropped here: moved when it came from this machine's own folder, copied otherwise.
    signal dropped(var paths, string into)
    readonly property int nameWidth: Math.max(220, width - 320)

    function open(row) {
        if (row < 0 || row >= directory.count) return;
        const path = directory.pathAt(row);
        if (directory.isDirAt(row)) root.activated(path);
        else if (root.openFiles) root.asked(path);
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
        function onCountChanged() {
            if (root.pending) root.beginRenameWhenSeen(root.pending);
            // A rescan rebuilds the rows; what was picked is kept by name, so a file appearing
            // elsewhere in the folder does not throw the picking away.
            const kept = root.selection.filter(p => root.directory.rowOf(Engine.displayName(p)) >= 0);
            if (kept.length !== root.selection.length) root.selection = kept;
            root.index = Math.max(0, Math.min(root.index, root.directory.count - 1));
        }
        function onPathChanged() {
            root.index = 0;
            root.anchor = 0;
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

            ListView {
                id: list
                anchors.fill: parent
                visible: root.mode === "list"
                enabled: visible
                model: root.directory
                clip: true
                focus: root.mode === "list"
                currentIndex: root.index
                // A folder of fifty thousand rows only ever builds the ones on screen.
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds


                Keys.onReturnPressed: root.open(root.index)
                Keys.onEnterPressed: root.open(root.index)
                Keys.onUpPressed: event => root.step(-1, event.modifiers)
                Keys.onDownPressed: event => root.step(1, event.modifiers)
                Keys.onPressed: event => root.walk(event, Math.max(1, Math.floor(height / 30) - 1))

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
                    required property string groupName

                    // The first row of a run carries the heading its run sits under.
                    readonly property bool opensGroup: row.groupName !== ""
                        && (row.index === 0 || root.directory.groupAt(row.index - 1) !== row.groupName)

                    width: list.width
                    readonly property int rowHeight: root.directory.paths.length > 0 ? 40 : 30
                    height: (row.opensGroup ? 26 : 0) + row.rowHeight

                    Drag.active: rowArea.dragging
                    Drag.dragType: Drag.Automatic
                    Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                    Drag.mimeData: ({ "text/uri-list": root.uriList(root.isPicked(row.path) ? root.selection : [row.path]) })
                    color: "transparent"

                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: row.rowHeight
                        color: root.isPicked(row.path) ? Theme.pressed
                             : row.ListView.isCurrentItem ? Theme.raised
                             : rowArea.containsMouse ? Theme.raised : "transparent"
                    }

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

                    Label {
                        visible: row.opensGroup
                        anchors { left: parent.left; top: parent.top; leftMargin: Theme.s4; topMargin: 5 }
                        size: Theme.sizeCaption
                        color: Theme.text3
                        text: row.groupName
                    }

                    RowLayout {
                        anchors {
                            left: parent.left
                            right: parent.right
                            bottom: parent.bottom
                            leftMargin: Theme.s4
                            rightMargin: Theme.s4
                        }
                        height: row.rowHeight
                        spacing: Theme.s3

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.nameWidth
                            spacing: Theme.s2 + 2
                            IconImage {
                                implicitSize: 18
                                source: Quickshell.iconPath(root.iconFor(row.path, row.iconName), "text-x-generic")
                            }
                            ColumnLayout {
                                visible: root.renaming !== row.path
                                Layout.fillWidth: true
                                spacing: 0
                                Label {
                                    Layout.fillWidth: true
                                    text: row.name
                                    color: row.isSymlink ? Theme.text2 : Theme.text
                                }
                                // A row that was named rather than found says which folder it is in,
                                // since a gathering has no one folder to stand for them all.
                                Label {
                                    visible: root.directory.paths.length > 0
                                    Layout.fillWidth: true
                                    size: Theme.sizeCaption
                                    color: Theme.text3
                                    elide: Text.ElideMiddle
                                    text: Engine.parentOf(row.path)
                                }
                            }
                            // Renaming happens on the row, not in a card over it.
                            Loader {
                                active: root.renaming === row.path
                                Layout.fillWidth: true
                                Layout.preferredHeight: 24
                                sourceComponent: Field {
                                    size: Theme.sizeBody
                                    text: row.name
                                    Component.onCompleted: {
                                        input.forceActiveFocus();
                                        // The ending is left out of the selection, as every file
                                        // manager does: it is rarely the part being changed.
                                        const dot = row.name.lastIndexOf(".");
                                        input.select(0, dot > 0 ? dot : row.name.length);
                                    }
                                    onAccepted: root.endRename(row.path, text.trim())
                                    input.Keys.onEscapePressed: root.renaming = ""
                                    input.onActiveFocusChanged: if (!input.activeFocus) root.endRename(row.path, text.trim())
                                }
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
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: row.rowHeight
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                        // No drag.target: giving a MouseArea one makes it hold every press back to
                        // see whether a drag follows, and a click then needs repeating. The drag is
                        // started by hand when the pointer has moved far enough to mean it.
                        property point pressedAt
                        property bool dragging: false
                        onPressed: mouse => { rowArea.pressedAt = Qt.point(mouse.x, mouse.y); rowArea.dragging = false; }
                        onReleased: rowArea.dragging = false
                        onPositionChanged: mouse => {
                            if (rowArea.dragging || !(mouse.buttons & Qt.LeftButton)) return;
                            const far = Math.abs(mouse.x - rowArea.pressedAt.x) + Math.abs(mouse.y - rowArea.pressedAt.y);
                            if (far < Qt.styleHints.startDragDistance) return;
                            rowArea.dragging = true;
                            if (!root.isPicked(row.path)) root.pick(row.index, Qt.NoModifier);
                            row.Drag.startDrag();
                        }
                        onClicked: mouse => {
                            if (mouse.button === Qt.MiddleButton) {
                                if (row.isDir) root.openedInTab(row.path);
                                return;
                            }
                            // A right click on something already picked keeps the whole picking, so
                            // the menu acts on all of it rather than on the one row under the pointer.
                            if (mouse.button !== Qt.RightButton || !root.isPicked(row.path))
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
                cellWidth: root.cellSize
                cellHeight: root.cellSize
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds


                Keys.onReturnPressed: root.open(root.index)
                Keys.onEnterPressed: root.open(root.index)
                Keys.onUpPressed: event => root.step(-1, event.modifiers)
                Keys.onDownPressed: event => root.step(1, event.modifiers)
                Keys.onPressed: event => root.walk(event, Math.max(1, Math.floor(height / 30) - 1))

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
                                Layout.preferredWidth: root.iconSize
                                Layout.preferredHeight: root.iconSize

                                IconImage {
                                    anchors.centerIn: parent
                                    implicitSize: Math.round(root.iconSize * 0.75)
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
                                    sourceSize.width: 256
                                    sourceSize.height: 256
                                    width: Math.min(implicitWidth, root.iconSize)
                                    height: Math.min(implicitHeight, root.iconSize)
                                    source: thumb.file ? "file://" + thumb.file : ""
                                }
                            }

                            Label {
                                visible: root.renaming !== cell.path
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                                size: Theme.sizeCaption
                                maximumLineCount: 2
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                                text: cell.name
                            }
                            Loader {
                                active: root.renaming === cell.path
                                Layout.fillWidth: true
                                Layout.preferredHeight: 22
                                sourceComponent: Field {
                                    size: Theme.sizeCaption
                                    text: cell.name
                                    Component.onCompleted: {
                                        input.forceActiveFocus();
                                        const dot = cell.name.lastIndexOf(".");
                                        input.select(0, dot > 0 ? dot : cell.name.length);
                                    }
                                    onAccepted: root.endRename(cell.path, text.trim())
                                    input.Keys.onEscapePressed: root.renaming = ""
                                    input.onActiveFocusChanged: if (!input.activeFocus) root.endRename(cell.path, text.trim())
                                }
                            }
                        }

                        MouseArea {
                            id: cellArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => {
                                if (mouse.button !== Qt.RightButton || !root.isPicked(cell.path))
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

            MouseArea {
                anchors.fill: parent
                visible: root.mode !== "columns"
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                propagateComposedEvents: true
                onPressed: mouse => {
                    const view = root.mode === "grid" ? grid : list;
                    const at = mapToItem(view, mouse.x, mouse.y);
                    const row = view.indexAt(at.x + view.contentX, at.y + view.contentY);
                    // A row takes its own click; this only has the part with no row in it.
                    mouse.accepted = row < 0;
                }
                onClicked: mouse => {
                    root.selection = [];
                    if (mouse.button !== Qt.RightButton) return;
                    const where = mapToItem(root, mouse.x, mouse.y);
                    root.menuAsked(where.x, where.y, "");
                }
            }

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
