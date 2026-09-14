import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui
import qs.services
// Loaded by URL at the root (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.files

// Finder's columns: the folder on the left, what is picked in it to the right of that, and so on.
// Each column is a directory of its own, so several are alive at once and none re-reads another.
Item {
    id: root
    // The folder the leftmost column shows, or, when it is a gathering — Recents, the trash, what a
    // search found — the paths it holds. A folder picked in it opens in the next column either way.
    required property string rootPath
    property var rootPaths: []
    property bool showHidden: false
    // The ordering and narrowing the window is set to, which every column follows.
    property int sort: Directory.ByName
    property int sortOrder: Qt.AscendingOrder
    property string filter: ""

    // How wide each column is, by the folder it shows. A column is dragged on its own; the ones
    // beside it keep the width they were given. A folder never dragged takes the starting width.
    property var widths: ({})
    function widthOf(path) { return widths[path] || Prefs.p.filesColumn; }
    function setWidth(path, w) {
        const next = {};
        for (const k in widths) next[k] = widths[k];
        next[path] = Math.round(Math.max(140, Math.min(560, w)));
        widths = next;
    }
    readonly property real chainWidth: {
        let total = 0;
        for (const p of chain) total += widthOf(p);
        return total;
    }

    // A name for the first column that is not a path, so nothing mistakes it for one.
    readonly property string gathering: "gathering:"
    function firstColumn() { return rootPaths.length > 0 ? gathering : rootPath; }

    // Picking writes the chain, which is why this cannot be a binding: it would be gone after the
    // first pick and the columns would go on showing whatever was open when it was made.
    property var chain: [firstColumn()]
    onRootPathChanged: { chain = [firstColumn()]; letGo(); }
    onRootPathsChanged: { chain = [firstColumn()]; letGo(); }

    // Everything picked, all of it in one column: a column browser walks down one branch, so a
    // selection that spanned columns would have no meaning to act on.
    property var selection: []
    // What the keys are on, which is the last thing picked.
    property string selected: ""
    property bool selectedIsDir: false
    // Which column the keys are in, and which a run of Shift-picked rows is measured from.
    property int activeColumn: 0
    property string anchorPath: ""

    // The row whose name is being edited where it sits, as Finder renames.
    property string renaming: ""

    signal activated(string path)
    signal chosen(string path)
    signal openedInTab(string path)
    signal menuAsked(real x, real y, string path)
    signal renamed(string path, string name)
    signal dropped(var paths, string into)

    // The listing and the view of each column by number, so the keys can move within a column and
    // between columns without any of them knowing about the rest.
    readonly property var folders: ({})
    readonly property var views: ({})

    function isPicked(path) { return selection.indexOf(path) >= 0; }
    function letGo() {
        selection = [];
        selected = "";
        selectedIsDir = false;
        anchorPath = "";
        renaming = "";
    }

    // Picking in a column drops every column to its right, since they belonged to the old pick.
    function pick(column, path, isDir, modifiers) {
        const folder = folders[column];
        const wasHere = column === activeColumn;
        if (modifiers & Qt.ControlModifier && wasHere) {
            const at = selection.indexOf(path);
            selection = at < 0 ? selection.concat([path]) : selection.filter(p => p !== path);
        } else if (modifiers & Qt.ShiftModifier && wasHere && folder && anchorPath) {
            const from = folder.rowOfPath(anchorPath), to = folder.rowOfPath(path);
            const run = [];
            for (let i = Math.min(from, to); i <= Math.max(from, to); i++) run.push(folder.pathAt(i));
            selection = run;
        } else {
            selection = [path];
            anchorPath = path;
        }

        // One thing picked opens its column; several have no one column to open.
        const one = selection.length === 1;
        const next = chain.slice(0, column + 1);
        if (one && isDir) next.push(path);
        chain = next;
        selected = path;
        selectedIsDir = one && isDir;
        activeColumn = column;
        renaming = "";
        if (selectedIsDir) Qt.callLater(() => flick.contentX = Math.max(0, flick.contentWidth - flick.width));
    }

    function selectAll() {
        const folder = folders[activeColumn];
        if (!folder) return;
        const all = [];
        for (let i = 0; i < folder.count; i++) all.push(folder.pathAt(i));
        selection = all;
        chain = chain.slice(0, activeColumn + 1);
        selectedIsDir = false;
    }

    // What is picked in a column: the folder that opened the column to its right, or, in the last
    // column, whatever was picked there.
    function currentIn(column) {
        return column + 1 < chain.length ? chain[column + 1] : selected;
    }
    function pickRow(column, row, modifiers) {
        const folder = folders[column];
        if (!folder || row < 0 || row >= folder.count) return;
        pick(column, folder.pathAt(row), folder.isDirAt(row), modifiers || Qt.NoModifier);
        const view = views[column];
        if (view) view.positionViewAtIndex(row, ListView.Contain);
    }
    function stepBy(by, modifiers) {
        const folder = folders[activeColumn];
        if (!folder) return;
        const at = folder.rowOfPath(currentIn(activeColumn));
        pickRow(activeColumn, at < 0 ? 0 : at + by, modifiers);
    }

    // What a drag carries: the uri list every desktop reads, so a drop lands in other applications.
    function uriList(paths) { return paths.map(p => "file://" + encodeURI(p)).join("\r\n"); }

    function beginRename(path) { renaming = path; }
    // Settling a name and giving it up both end the edit, and tearing the field down drops focus,
    // which arrives here a second time; only the first one counts.
    function endRename(path, name) {
        if (renaming !== path) return;
        renaming = "";
        if (!name || name === Engine.displayName(path)) return;
        // The pick follows the name: leaving it on the old path would leave the preview showing a
        // file that is not there and every action pointed at it.
        const to = Engine.join(Engine.parentOf(path), name);
        selection = selection.map(p => p === path ? to : p);
        if (selected === path) selected = to;
        if (anchorPath === path) anchorPath = to;
        root.renamed(path, name);
    }

    // Typing the start of a name jumps to it, within the column the keys are in.
    property string typed: ""
    Timer { id: typing; interval: 900; onTriggered: root.typed = "" }

    // Only the view being shown takes the keys; the other two are still there behind it.
    focus: visible
    // Left and right walk between columns, up and down within one, as a column browser is walked.
    Keys.onUpPressed: event => root.stepBy(-1, event.modifiers)
    Keys.onDownPressed: event => root.stepBy(1, event.modifiers)
    // Back to the column on the left, standing on the folder that opened this one.
    Keys.onLeftPressed: {
        if (root.activeColumn <= 0) return;
        const back = root.activeColumn - 1;
        const folder = root.folders[back];
        if (folder) root.pickRow(back, folder.rowOfPath(root.chain[root.activeColumn]));
    }
    // Into the column a picked folder opened, standing on its first row. The column is made when the
    // folder is picked, so on the press that picked it there is nothing there yet to step into.
    Keys.onRightPressed: {
        if (!root.selectedIsDir) return;
        const into = root.activeColumn + 1;
        if (root.folders[into]) root.pickRow(into, 0);
        else Qt.callLater(() => { if (root.folders[into]) root.pickRow(into, 0); });
    }
    Keys.onReturnPressed: {
        if (!root.selected) return;
        root.selectedIsDir ? root.activated(root.selected) : root.chosen(root.selected);
    }
    Keys.onPressed: event => {
        const folder = root.folders[root.activeColumn];
        if (!folder) return;
        if (event.key === Qt.Key_Home) { root.pickRow(root.activeColumn, 0); event.accepted = true; }
        else if (event.key === Qt.Key_End) { root.pickRow(root.activeColumn, folder.count - 1); event.accepted = true; }
        else if (event.text && event.text.length === 1 && event.text >= " " && !(event.modifiers & Qt.ControlModifier)) {
            root.typed += event.text.toLowerCase();
            typing.restart();
            const at = folder.startingWith(root.typed, 0);
            if (at >= 0) root.pickRow(root.activeColumn, at);
            event.accepted = true;
        }
    }

    // Measures a name against the row's own font, so sizing a column to fit is the width it needs
    // rather than a guess from how many letters it has.
    TextMetrics { id: measure; font.pixelSize: Theme.sizeSmall }

    Flickable {
        id: flick
        anchors.fill: parent
        contentWidth: row.implicitWidth
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        Row {
            id: row
            height: flick.height
            spacing: 0

            Repeater {
                model: root.chain

                delegate: Item {
                    id: column
                    required property string modelData
                    required property int index

                    width: root.widthOf(column.modelData)
                    height: row.height

                    Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }

                    Directory {
                        id: folder
                        path: column.modelData === root.gathering ? "" : column.modelData
                        paths: column.modelData === root.gathering ? root.rootPaths : []
                        showHidden: root.showHidden
                        sort: root.sort
                        sortOrder: root.sortOrder
                        filter: root.filter
                    }
                    // Something picked can go while the column is open — deleted, moved, renamed by
                    // something else. A pick that is no longer there is not a pick.
                    Connections {
                        target: folder
                        function onCountChanged() {
                            if (column.index !== root.activeColumn) return;
                            const kept = root.selection.filter(p => folder.rowOfPath(p) >= 0);
                            if (kept.length !== root.selection.length) root.selection = kept;
                            if (root.selected && folder.rowOfPath(root.selected) < 0 && !root.selectedIsDir)
                                root.selected = "";
                        }
                    }
                    Component.onCompleted: { root.folders[column.index] = folder; root.views[column.index] = entries; }
                    Component.onDestruction: { delete root.folders[column.index]; delete root.views[column.index]; }

                    // The empty part of a column belongs to the column, as the empty part of a list
                    // does: letting go there drops the pick and every column to the right of it.
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: mouse => {
                            root.chain = root.chain.slice(0, column.index + 1);
                            root.letGo();
                            root.activeColumn = column.index;
                            if (mouse.button !== Qt.RightButton) return;
                            const at = mapToItem(root, mouse.x, mouse.y);
                            root.menuAsked(at.x, at.y, "");
                        }
                    }

                    // Dropped on the column itself: into the folder the column is showing.
                    DropArea {
                        anchors.fill: parent
                        enabled: column.modelData !== root.gathering
                        keys: ["text/uri-list"]
                        onDropped: drop => {
                            root.dropped(String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                                .filter(u => u.startsWith("file://"))
                                .map(u => decodeURI(u.slice(7))), column.modelData);
                            drop.acceptProposedAction();
                        }
                        Rectangle {
                            anchors { fill: parent; margins: 2 }
                            visible: parent.containsDrag
                            color: "transparent"
                            border.width: 2
                            border.color: Theme.accent
                        }
                    }

                    ListView {
                        id: entries
                        anchors { fill: parent; rightMargin: 1 }
                        model: folder
                        clip: true
                        reuseItems: true
                        boundsBehavior: Flickable.StopAtBounds
                        currentIndex: folder.rowOfPath(root.currentIn(column.index))

                        delegate: Rectangle {
                            id: entry
                            required property int index
                            required property string name
                            required property string path
                            required property string iconName
                            required property bool isDir
                            required property bool isSymlink

                            width: entries.width
                            height: 28
                            // The folder whose contents the next column is showing stays marked, so
                            // the chain from left to right can be read off.
                            readonly property bool onChain: root.chain[column.index + 1] === entry.path
                            color: root.isPicked(entry.path) || entry.onChain ? Theme.pressed
                                 : entryArea.containsMouse ? Theme.raised : "transparent"

                            Drag.active: entryArea.dragging
                            Drag.dragType: Drag.Automatic
                            Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                            Drag.mimeData: ({ "text/uri-list":
                                root.uriList(root.isPicked(entry.path) ? root.selection : [entry.path]) })

                            // A folder in a column takes a drop the way a folder anywhere does.
                            DropArea {
                                anchors.fill: parent
                                enabled: entry.isDir
                                keys: ["text/uri-list"]
                                onDropped: drop => {
                                    root.dropped(String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                                        .filter(u => u.startsWith("file://"))
                                        .map(u => decodeURI(u.slice(7))), entry.path);
                                    drop.acceptProposedAction();
                                }
                                Rectangle {
                                    anchors.fill: parent
                                    visible: parent.containsDrag
                                    color: "transparent"
                                    border.width: 2
                                    border.color: Theme.accent
                                }
                            }

                            RowLayout {
                                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s2 }
                                spacing: Theme.s2
                                // Above the row's own click area, which is declared after it.
                                z: 1
                                IconImage {
                                    implicitSize: 16
                                    // Read from the file itself when its name said nothing, the way
                                    // the other views do, so a script is not a blank page.
                                    source: Quickshell.iconPath(
                                        Engine.isGenericIcon(entry.iconName) ? Engine.sniffIconName(entry.path)
                                                                             : entry.iconName,
                                        "text-x-generic")
                                }
                                Label {
                                    visible: root.renaming !== entry.path
                                    Layout.fillWidth: true
                                    text: entry.name
                                    elide: Text.ElideMiddle
                                    size: Theme.sizeSmall
                                    color: entry.isSymlink ? Theme.text2 : Theme.text
                                }
                                // Renaming happens on the row, not in a card over it.
                                Loader {
                                    active: root.renaming === entry.path
                                    visible: active
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 22
                                    sourceComponent: Field {
                                        size: Theme.sizeSmall
                                        text: entry.name
                                        Component.onCompleted: {
                                            input.forceActiveFocus();
                                            const dot = entry.name.lastIndexOf(".");
                                            input.select(0, dot > 0 ? dot : entry.name.length);
                                        }
                                        onAccepted: root.endRename(entry.path, text.trim())
                                        input.Keys.onEscapePressed: root.renaming = ""
                                        input.onActiveFocusChanged: if (!input.activeFocus) root.endRename(entry.path, text.trim())
                                    }
                                }
                                // The chevron says the column to the right belongs to this row.
                                Glyph {
                                    visible: entry.isDir && root.renaming !== entry.path
                                    name: "chevron-right"
                                    size: 12
                                    color: entry.onChain ? Theme.text2 : Theme.text3
                                }
                            }

                            MouseArea {
                                id: entryArea
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                                // Started by hand once the pointer has moved far enough: a drag
                                // target holds every press back to see whether a drag follows.
                                property point pressedAt
                                property bool dragging: false
                                onPressed: mouse => { entryArea.pressedAt = Qt.point(mouse.x, mouse.y); entryArea.dragging = false; }
                                onReleased: entryArea.dragging = false
                                onPositionChanged: mouse => {
                                    if (entryArea.dragging || !(mouse.buttons & Qt.LeftButton)) return;
                                    const far = Math.abs(mouse.x - entryArea.pressedAt.x) + Math.abs(mouse.y - entryArea.pressedAt.y);
                                    if (far < Qt.styleHints.startDragDistance) return;
                                    entryArea.dragging = true;
                                    if (!root.isPicked(entry.path)) root.pick(column.index, entry.path, entry.isDir, Qt.NoModifier);
                                    entry.Drag.startDrag();
                                }
                                onClicked: mouse => {
                                    if (mouse.button === Qt.MiddleButton) {
                                        if (entry.isDir) root.openedInTab(entry.path);
                                        return;
                                    }
                                    // A right click on something already picked keeps the whole
                                    // picking, so the menu acts on all of it.
                                    if (mouse.button !== Qt.RightButton || !root.isPicked(entry.path))
                                        root.pick(column.index, entry.path, entry.isDir, mouse.modifiers);
                                    root.forceActiveFocus();
                                    if (mouse.button !== Qt.RightButton) return;
                                    const at = mapToItem(root, mouse.x, mouse.y);
                                    root.menuAsked(at.x, at.y, entry.path);
                                }
                                onDoubleClicked: entry.isDir ? root.activated(entry.path) : root.chosen(entry.path)
                            }
                        }

                        Label {
                            anchors.centerIn: parent
                            visible: folder.status === Directory.Ready && folder.count === 0
                            size: Theme.sizeSmall
                            color: Theme.text3
                            text: folder.filter ? "No matches" : "Empty"
                        }
                    }

                    Scrollbar { target: entries }

                    // The line between two columns is this column's handle. Measured against the
                    // strip, since the column it sits on is being resized by the drag being measured.
                    MouseArea {
                        anchors { right: parent.right; top: parent.top; bottom: parent.bottom }
                        width: 6
                        z: 3
                        cursorShape: Qt.SizeHorCursor
                        property real from: 0
                        property real was: 0
                        onPressed: mouse => {
                            from = mapToItem(root, mouse.x, 0).x;
                            was = root.widthOf(column.modelData);
                        }
                        onPositionChanged: mouse => {
                            if (!(mouse.buttons & Qt.LeftButton)) return;
                            root.setWidth(column.modelData, was + mapToItem(root, mouse.x, 0).x - from);
                        }
                        // Wide enough for the longest name in it, as a column browser's divider does.
                        onDoubleClicked: {
                            measure.text = folder.longestName();
                            root.setWidth(column.modelData, measure.width + Theme.s3 + Theme.s2 * 2 + 16 + 12 + 8);
                        }
                    }
                }
            }

            // Finder's last column: what is picked, rather than another list. A folder opens a
            // column of its own, so only a file leaves anything for this to show.
            FilePreview {
                // The last column takes whatever the columns left, so a preview is a preview and not
                // a strip with a window of nothing beside it.
                width: Math.max(280, root.width - root.chainWidth)
                height: row.height
                visible: root.selection.length > 0 && !root.selectedIsDir
                picked: visible ? root.selection : []
                onOpened: path => root.chosen(path)
            }
        }
    }
}
