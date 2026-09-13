import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui

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
    // Where the keys are, and where a run measures from, held as paths: a row number means a
    // different file after a sort, an expand or anything appearing in the folder.
    property string cursor: ""
    property string anchorPath: ""

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
        : (selection.length ? selection : (cursor && directory.rowOfPath(cursor) >= 0 ? [cursor] : []))

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
            cursor = path;
            return;                              // the anchor stays where the run started
        } else {
            selection = [path];
        }
        index = row;
        cursor = path;
        anchor = row;
        anchorPath = path;
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

    // Everything the band has covered, asked of the view row by row across the band.
    function pickWithin(x, y, w, h) {
        const v = mode === "grid" ? grid : list;
        const step = mode === "grid" ? Math.max(8, cellSize / 3) : 12;
        const found = [];
        for (let py = y; py <= y + h; py += step) {
            for (let px = x; px <= x + w; px += step) {
                const at = v.indexAt(px + v.contentX, py + v.contentY);
                if (at >= 0) {
                    const path = directory.pathAt(at);
                    if (found.indexOf(path) < 0) found.push(path);
                }
            }
        }
        selection = found;
        // The run a later Shift measures from starts where the band ended, not wherever the last
        // click happened to be.
        if (found.length) {
            cursor = found[found.length - 1];
            anchorPath = found[0];
            index = directory.rowOfPath(cursor);
            anchor = directory.rowOfPath(anchorPath);
        }
    }

    function selectAll() {
        const all = [];
        for (let i = 0; i < directory.count; i++) all.push(directory.pathAt(i));
        selection = all;
    }

    function isPicked(path) { return selection.indexOf(path) >= 0; }

    // A different listing: nothing is picked, nothing is being renamed, and nothing is waiting to be.
    function forget() {
        index = 0;
        anchor = 0;
        cursor = "";
        anchorPath = "";
        selection = [];
        renaming = "";
        pending = "";
        list.positionViewAtBeginning();
        grid.positionViewAtBeginning();
    }

    // The row whose name is being edited where it sits, as Finder renames. Empty while none is.
    property string renaming: ""
    // A new name settled on; the window does the renaming, since it owns the jobs.
    signal renamed(string path, string name)

    function beginRename(path) { renaming = path || (acting.length === 1 ? acting[0] : ""); }

    // A folder that has only just appeared: picked, brought into view, and its name open for typing.
    function beginRenameWhenSeen(path) {
        const row = directory.rowOfPath(path);
        if (row < 0) { pending = path; return; }
        pending = "";
        pick(row, Qt.NoModifier);
        if (mode === "grid") grid.positionViewAtIndex(row, GridView.Contain);
        else list.positionViewAtIndex(row, ListView.Contain);
        renaming = path;
    }
    // A path waiting for the folder to be read again before it can be named.
    property string pending: ""
    // Settling a name and giving it up both end the edit, and tearing the field down drops focus,
    // which arrives here a second time; only the first one counts.
    function endRename(path, name) {
        if (renaming !== path) return;
        renaming = "";
        const was = Engine.displayName(path);
        if (name && name !== was) root.renamed(path, name);
    }
    function cancelRename() { renaming = ""; }

    // What a drag carries: the uri list every desktop reads, so a drop lands in other applications too.
    function uriList(paths) { return paths.map(p => "file://" + encodeURI(p)).join("\r\n"); }
    // Something dropped here: moved when it came from this machine's own folder, copied otherwise.
    signal dropped(var paths, string into)
    // What each column is given; the name takes whatever is left.
    property int sizeWidth: 90
    property int modifiedWidth: 130
    readonly property int nameWidth: Math.max(160, width - sizeWidth - modifiedWidth - 100)

    function open(row) { openPath(directory.pathAt(row)); }
    // Opening a path rather than a row: a menu was opened over a particular file, and the rows may
    // have moved between then and the item being chosen.
    function openPath(path) {
        if (!path) return;
        if (Engine.isDir(path)) root.activated(path);
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
            const kept = root.selection.filter(p => root.directory.rowOfPath(p) >= 0);
            if (kept.length !== root.selection.length) root.selection = kept;
            // The rows have been rebuilt, so the row numbers are followed back to the files they
            // were standing on rather than left pointing at whatever is there now.
            const at = root.directory.rowOfPath(root.cursor);
            root.index = at >= 0 ? at : Math.max(0, Math.min(root.index, root.directory.count - 1));
            const from = root.directory.rowOfPath(root.anchorPath);
            root.anchor = from >= 0 ? from : root.index;
        }
        function onPathChanged() { root.forget(); }
        // A gathering — Recents, or what a search found — is as much a change of listing as a
        // folder is, and says so with its own signal.
        function onPathsChanged() { root.forget(); }
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
                id: header
                anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 + 8 }
                spacing: Theme.s3
                Repeater {
                    model: [{ label: "Name", column: Directory.ByName, width: root.nameWidth, align: Text.AlignLeft },
                            { label: "Size", column: Directory.BySize, width: root.sizeWidth, align: Text.AlignRight },
                            { label: "Modified", column: Directory.ByModified, width: root.modifiedWidth, align: Text.AlignRight }]
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

                        // The line at the head's left edge is dragged to give the column either side
                        // a different width.
                        MouseArea {
                            visible: head.modelData.column !== Directory.ByName
                            anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
                            width: 7
                            cursorShape: Qt.SizeHorCursor
                            // The drag is measured against the header rather than against this
                            // handle: the handle sits on a cell whose width the drag is changing, so
                            // its own origin moves as the pointer does and every step counts twice.
                            property real from: 0
                            property real was: 0
                            onPressed: mouse => {
                                from = mapToItem(header, mouse.x, 0).x;
                                was = head.modelData.column === Directory.BySize ? root.sizeWidth : root.modifiedWidth;
                            }
                            onPositionChanged: mouse => {
                                if (!(mouse.buttons & Qt.LeftButton)) return;
                                const by = Math.round(mapToItem(header, mouse.x, 0).x - from);
                                if (head.modelData.column === Directory.BySize)
                                    root.sizeWidth = Math.max(60, Math.min(300, was - by));
                                else
                                    root.modifiedWidth = Math.max(80, Math.min(320, was - by));
                            }
                        }
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
                Keys.onRightPressed: root.directory.expand(root.index)
                Keys.onLeftPressed: root.directory.collapse(root.index)
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
                    required property int nestDepth
                    required property bool opened

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
                        // Above the row's own click area, which covers the whole row and is declared
                        // after this. Nothing in here takes a click except the triangle, so the rest
                        // of the row still reaches it.
                        z: 1

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.nameWidth
                            spacing: Theme.s2 + 2
                            // A row inside a folder opened in place sits a step further in.
                            Item { Layout.preferredWidth: row.nestDepth * 16; Layout.preferredHeight: 1 }
                            // A fixed box for the triangle, so the layout keeps room for it whether
                            // or not the row has one.
                            Item {
                                Layout.preferredWidth: 14
                                Layout.preferredHeight: 14
                                Glyph {
                                    anchors.centerIn: parent
                                    visible: row.isDir
                                    name: row.opened ? "chevron-down" : "chevron-right"
                                    size: 12
                                    color: Theme.text3
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    enabled: row.isDir
                                    onClicked: row.opened ? root.directory.collapse(row.index)
                                                          : root.directory.expand(row.index)
                                }
                            }
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
                                    input.Keys.onEscapePressed: root.cancelRename()
                                    input.onActiveFocusChanged: if (!input.activeFocus) root.endRename(row.path, text.trim())
                                }
                            }
                        }
                        Label {
                            Layout.preferredWidth: root.sizeWidth
                            horizontalAlignment: Text.AlignRight
                            size: Theme.sizeSmall
                            color: Theme.text3
                            tabular: true
                            text: row.isDir ? "—" : Engine.formatSize(row.size)
                        }
                        Label {
                            Layout.preferredWidth: root.modifiedWidth
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
                Keys.onRightPressed: root.directory.expand(root.index)
                Keys.onLeftPressed: root.directory.collapse(root.index)
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

                    Drag.active: cellArea.dragging
                    Drag.dragType: Drag.Automatic
                    Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                    Drag.mimeData: ({ "text/uri-list": root.uriList(root.isPicked(cell.path) ? root.selection : [cell.path]) })

                    // A folder in the grid takes a drop the same way a folder in the list does.
                    DropArea {
                        anchors.fill: parent
                        enabled: cell.isDir
                        keys: ["text/uri-list"]
                        onDropped: drop => {
                            root.dropped(String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                                .filter(u => u.startsWith("file://"))
                                .map(u => decodeURI(u.slice(7))), cell.path);
                            drop.acceptProposedAction();
                        }
                        Rectangle {
                            anchors { fill: parent; margins: 3 }
                            visible: parent.containsDrag
                            color: "transparent"
                            border.width: 2
                            border.color: Theme.accent
                            radius: Theme.radiusControl
                        }
                    }

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
                                    input.Keys.onEscapePressed: root.cancelRename()
                                    input.onActiveFocusChanged: if (!input.activeFocus) root.endRename(cell.path, text.trim())
                                }
                            }
                        }

                        MouseArea {
                            id: cellArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            // Started by hand once the pointer has moved far enough, for the reason
                            // the list rows give: a drag target holds every press back.
                            property point pressedAt
                            property bool dragging: false
                            onPressed: mouse => { cellArea.pressedAt = Qt.point(mouse.x, mouse.y); cellArea.dragging = false; }
                            onReleased: cellArea.dragging = false
                            onPositionChanged: mouse => {
                                if (cellArea.dragging || !(mouse.buttons & Qt.LeftButton)) return;
                                const far = Math.abs(mouse.x - cellArea.pressedAt.x) + Math.abs(mouse.y - cellArea.pressedAt.y);
                                if (far < Qt.styleHints.startDragDistance) return;
                                cellArea.dragging = true;
                                if (!root.isPicked(cell.path)) root.pick(cell.index, Qt.NoModifier);
                                cell.Drag.startDrag();
                            }
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
                id: bandArea
                anchors.fill: parent
                visible: root.mode !== "columns"
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                propagateComposedEvents: true

                property point from: Qt.point(0, 0)
                property bool banding: false
                // The release comes before the click, so what the band did has to outlive it.
                property bool banded: false

                function view() { return root.mode === "grid" ? grid : list; }

                onPressed: mouse => {
                    const v = view();
                    const at = mapToItem(v, mouse.x, mouse.y);
                    const row = v.indexAt(at.x + v.contentX, at.y + v.contentY);
                    // A row takes its own click; this only has the part with no row in it.
                    mouse.accepted = row < 0;
                    bandArea.from = Qt.point(mouse.x, mouse.y);
                    bandArea.banding = false;
                }
                onPositionChanged: mouse => {
                    if (!(mouse.buttons & Qt.LeftButton)) return;
                    if (!bandArea.banding
                        && Math.abs(mouse.x - bandArea.from.x) + Math.abs(mouse.y - bandArea.from.y)
                           < Qt.styleHints.startDragDistance) return;
                    bandArea.banding = true;
                    band.x = Math.min(bandArea.from.x, mouse.x);
                    band.y = Math.min(bandArea.from.y, mouse.y);
                    band.width = Math.abs(mouse.x - bandArea.from.x);
                    band.height = Math.abs(mouse.y - bandArea.from.y);
                    root.pickWithin(band.x, band.y, band.width, band.height);
                }
                onReleased: { bandArea.banded = bandArea.banding; bandArea.banding = false; }
                onClicked: mouse => {
                    if (bandArea.banded) { bandArea.banded = false; return; }
                    root.selection = [];
                    if (mouse.button !== Qt.RightButton) return;
                    const where = mapToItem(root, mouse.x, mouse.y);
                    root.menuAsked(where.x, where.y, "");
                }

                Rectangle {
                    id: band
                    visible: bandArea.banding
                    color: Qt.alpha(Theme.accent, 0.15)
                    border.width: 1
                    border.color: Theme.accent
                }
            }

            FileColumns {
                id: columns
                anchors.fill: parent
                visible: root.mode === "columns"
                enabled: visible
                focus: visible
                rootPath: root.directory.path
                showHidden: root.directory.showHidden
                onActivated: path => root.activated(path)
                // The same question the other views ask: what opens this, and who says so.
                onChosen: path => root.openFiles ? root.asked(path) : root.chosen(path)
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
