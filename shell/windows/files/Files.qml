import QtQuick
import QtQuick.Layouts
import Quickshell
import Isle.Files
import Quickshell.Widgets
import qs.theme
import qs.ui
import qs.services
// Loaded by URL rather than declared (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.files

// The Files window: a toolbar with the trail it has walked, and the folder beneath it.
FloatingWindow {
    id: root
    title: "Files"
    visible: Surfaces.files
    implicitWidth: 980
    implicitHeight: 680
    minimumSize: Qt.size(640, 420)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: if (!visible) Surfaces.files = false

    readonly property string path: dir.path

    // Where it has been, for the back and forward buttons and the mouse's own, as Settings keeps it.
    property var history: [Surfaces.filesPath || Engine.home]
    property int at: 0
    property bool navigating: false
    readonly property bool canBack: at > 0
    readonly property bool canForward: at < history.length - 1

    function go(to) {
        if (!to || to === dir.path) return;
        dir.path = to;
        if (navigating) return;
        const h = history.slice(0, at + 1);
        h.push(to);
        history = h.slice(-50);
        at = history.length - 1;
    }
    function back() { if (!canBack) return; navigating = true; at--; dir.path = history[at]; navigating = false; }
    function forward() { if (!canForward) return; navigating = true; at++; dir.path = history[at]; navigating = false; }
    function up() { go(Engine.parentOf(dir.path)); }

    // What Copy or Cut put aside, and which of the two it was.
    property var clipboard: []
    property bool clipboardCut: false

    // Everything an action applies to: what is picked, or the row the keys are on.
    readonly property var acting: view.acting
    readonly property string actingOne: acting.length === 1 ? acting[0] : ""

    function said(n) { return n === 1 ? Engine.displayName(acting[0]) : n + " items"; }

    function copyToClipboard(cut) {
        if (!acting.length) return;
        clipboard = acting.slice();
        clipboardCut = cut;
        IslandEvents.show({ kind: "text", glyph: cut ? "scissors" : "copy",
                            text: said(acting.length) + (cut ? " cut" : " copied") });
    }
    function paste() {
        if (!clipboard.length) return;
        clipboardCut ? FileJobs.move(clipboard, dir.path) : FileJobs.copy(clipboard, dir.path);
        if (clipboardCut) { clipboard = []; clipboardCut = false; }
    }
    function askRename() {
        if (acting.length > 1) { askRenameMany(); return; }
        if (!actingOne) return;
        sheet.mode = "name"; sheet.title = "Rename"; sheet.message = ""; sheet.acceptLabel = "Rename";
        sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "rename";
        sheet.ask(Engine.displayName(actingOne));
    }

    function askRenameMany() {
        sheet.mode = "name"; sheet.title = "Rename " + acting.length + " items";
        sheet.message = "One name for all of them. # is where the number goes; each keeps its ending.";
        sheet.acceptLabel = "Rename"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "renameMany";
        sheet.ask("item #");
    }

    function askCompress() {
        if (!acting.length) return;
        sheet.mode = "name"; sheet.title = "Compress " + said(acting.length);
        sheet.message = "The ending decides the format.";
        sheet.acceptLabel = "Compress"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "compress";
        sheet.ask((acting.length === 1 ? Engine.displayName(acting[0]) : Engine.displayName(dir.path)) + ".tar.gz");
    }
    function askNewFolder() {
        sheet.mode = "name"; sheet.title = "New folder"; sheet.message = ""; sheet.acceptLabel = "Create";
        sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "newFolder";
        sheet.ask("untitled folder");
    }
    function askDelete() {
        if (!acting.length) return;
        sheet.mode = "confirm"; sheet.title = "Delete " + said(acting.length) + "?";
        sheet.message = "This does not go to the trash and cannot be undone.";
        sheet.acceptLabel = "Delete"; sheet.danger = true; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "delete";
        sheet.ask("");
    }
    function toTrash() {
        if (acting.length) FileJobs.trash(acting);
    }

    // A job that meets something already there stops and asks through the same sheet.
    Connections {
        target: FileJobs
        function onRunningChanged() {
            for (const job of FileJobs.running)
                if (job.state === FileJob.Asking && sheet.job !== job) root.askConflict(job);
        }
        function onJobFinished(job) {
            if (job.state === FileJob.Failed)
                IslandEvents.show({ kind: "text", glyph: "triangle-alert", text: job.error });
        }
    }

    function askConflict(job) {
        sheet.mode = "confirm";
        sheet.title = job.conflictName + " is already there";
        sheet.message = "Replace it, keep both, or leave it as it is.";
        sheet.acceptLabel = "Replace"; sheet.danger = true; sheet.offerAll = job.total > 1;
        sheet.alternateLabel = "Keep both";
        sheet.job = job; sheet.what = "conflict";
        sheet.ask("");
    }

    // Opening the window again at another folder walks there rather than starting a second window.
    Connections {
        target: Surfaces
        function onFilesPathChanged() { if (Surfaces.filesPath) root.go(Surfaces.filesPath); }
    }

    Directory {
        id: dir
        path: Surfaces.filesPath || Engine.home
        showHidden: false
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onClicked: mouse => mouse.button === Qt.BackButton ? root.back() : root.forward()
    }
    Shortcut { sequences: ["Alt+Left", StandardKey.Back]; onActivated: root.back() }
    Shortcut { sequences: ["Alt+Right", StandardKey.Forward]; onActivated: root.forward() }
    Shortcut { sequences: ["Alt+Up", "Backspace"]; onActivated: root.up() }
    Shortcut { sequence: "Ctrl+H"; onActivated: dir.showHidden = !dir.showHidden }
    Shortcut { sequences: [StandardKey.Refresh]; onActivated: dir.refresh() }
    Shortcut { sequence: "Ctrl+F"; onActivated: search.input.forceActiveFocus() }
    Shortcut { sequences: ["Ctrl+1"]; onActivated: view.mode = "list" }
    Shortcut { sequences: ["Ctrl+2"]; onActivated: view.mode = "columns" }
    Shortcut { sequences: ["Ctrl+3"]; onActivated: view.mode = "grid" }
    Shortcut { sequence: "F2"; onActivated: root.askRename() }
    Shortcut { sequences: [StandardKey.Copy]; onActivated: root.copyToClipboard(false) }
    Shortcut { sequences: [StandardKey.Cut]; onActivated: root.copyToClipboard(true) }
    Shortcut { sequences: [StandardKey.Paste]; onActivated: root.paste() }
    Shortcut { sequences: [StandardKey.Delete]; onActivated: root.toTrash() }
    Shortcut { sequence: "Shift+Delete"; onActivated: root.askDelete() }
    Shortcut { sequences: [StandardKey.Undo]; onActivated: FileJobs.undo() }
    Shortcut { sequence: "Ctrl+Shift+N"; onActivated: root.askNewFolder() }
    Shortcut { sequences: [StandardKey.SelectAll]; onActivated: view.selectAll() }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 190
            color: "transparent"
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }

            Flickable {
                anchors { fill: parent; margins: Theme.s2; topMargin: Theme.s3 }
                contentHeight: sideCol.implicitHeight
                clip: true
                ColumnLayout {
                    id: sideCol
                    width: parent.width
                    spacing: 1
                    Repeater {
                        model: Places.places
                        delegate: Item {
                            id: place
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            implicitHeight: heading.visible ? 30 + row.implicitHeight : row.implicitHeight

                            readonly property bool first: index === 0
                                || Places.places[index - 1].group !== place.modelData.group

                            Label {
                                id: heading
                                visible: place.first
                                x: Theme.s3
                                y: Theme.s3
                                text: place.modelData.group
                                size: Theme.sizeCaption
                                color: Theme.text3
                            }
                            ListRow {
                                id: row
                                anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                glyph: place.modelData.icon
                                title: place.modelData.name
                                selected: dir.path === place.modelData.path
                                onClicked: root.go(place.modelData.path)
                                // Ejecting is the device's own action and belongs on the row, not in a menu.
                                Glyph {
                                    visible: place.modelData.eject
                                    name: "eject"
                                    size: 14
                                    color: ejectArea.containsMouse ? Theme.text : Theme.text3
                                    MouseArea {
                                        id: ejectArea
                                        anchors { fill: parent; margins: -6 }
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: Disks.eject(place.modelData.volume)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    ColumnLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 0

        // Toolbar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: "transparent"
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }

            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                spacing: Theme.s2

                Button { variant: "text"; glyph: "chevron-left"; enabled: root.canBack; onClicked: root.back() }
                Button { variant: "text"; glyph: "chevron-right"; enabled: root.canForward; onClicked: root.forward() }
                Button { variant: "text"; glyph: "arrow-up"; enabled: dir.path !== "/"; onClicked: root.up() }

                // The trail as buttons; the last one is where you are and does nothing.
                Flickable {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    contentWidth: crumbRow.implicitWidth
                    flickableDirection: Flickable.HorizontalFlick
                    clip: true
                    // A deep folder keeps its tail in view rather than its root.
                    onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
                    RowLayout {
                        id: crumbRow
                        height: parent.height
                        spacing: 0
                        Repeater {
                            model: Engine.crumbs(dir.path)
                            delegate: RowLayout {
                                id: crumb
                                required property var modelData
                                required property int index
                                spacing: 0
                                Glyph { visible: crumb.index > 0; name: "chevron-right"; size: 12; color: Theme.text3 }
                                Button {
                                    variant: "text"
                                    text: crumb.modelData.name
                                    enabled: crumb.modelData.path !== dir.path
                                    onClicked: root.go(crumb.modelData.path)
                                }
                            }
                        }
                    }
                }

                // Bookmarking writes where GTK keeps bookmarks, so the two desktops agree about them.
                Button {
                    variant: "text"
                    glyph: "star"
                    onClicked: Places.isBookmarked(dir.path) ? Places.removeBookmark(dir.path) : Places.addBookmark(dir.path)
                }

                Segmented {
                    options: [["list", "List"], ["columns", "Columns"], ["grid", "Grid"]]
                    value: view.mode
                    onPicked: v => view.mode = v
                }

                Field {
                    id: search
                    Layout.preferredWidth: 200
                    glyph: "search"
                    placeholder: "Search this folder"
                    onTextChanged: dir.filter = text
                    input.Keys.onEscapePressed: { text = ""; view.forceActiveFocus(); }
                }
            }
        }

        FolderView {
            id: view
            Layout.fillWidth: true
            Layout.fillHeight: true
            directory: dir
            onActivated: path => root.go(path)
            // Dropped from somewhere: moved when it is already on this machine and in another
            // folder, since that is what dragging within a desktop means.
            onDropped: (paths, into) => {
                const from = paths.filter(p => Engine.parentOf(p) !== into);
                if (from.length) FileJobs.move(from, into);
            }
            onMenuAsked: (x, y, path) => {
                const on = path !== "";
                const many = root.acting.length > 1;
                menu.items = [
                    on && !many ? { label: "Open", glyph: "external-link", action: () => Compositor.exec("xdg-open " + JSON.stringify(path)) } : undefined,
                    on ? { label: many ? "Rename " + root.acting.length + " items" : "Rename", glyph: "pencil", action: root.askRename } : undefined,
                    on && !many && FileJobs.isArchive(path) ? { label: "Extract here", glyph: "package-open", action: () => FileJobs.extract(path) } : undefined,
                    on ? { label: "Compress", glyph: "archive", action: root.askCompress } : undefined,
                    on ? null : undefined,
                    on ? { label: "Copy", glyph: "copy", action: () => root.copyToClipboard(false) } : null,
                    on ? { label: "Cut", glyph: "scissors", action: () => root.copyToClipboard(true) } : null,
                    { label: "Paste", glyph: "clipboard", enabled: root.clipboard.length > 0, action: root.paste },
                    null,
                    { label: "New folder", glyph: "folder-plus", action: root.askNewFolder },
                    on ? null : undefined,
                    on ? { label: "Move to trash", glyph: "trash", action: root.toTrash } : null,
                    on ? { label: "Delete", glyph: "x", danger: true, action: root.askDelete } : null,
                ].filter(i => i !== undefined);
                const at = view.mapToItem(overlay, x, y);
                menu.popup(at.x, at.y);
            }
        }

        // Status
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            color: "transparent"
            Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.hairline }
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                Label {
                    Layout.fillWidth: true
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: dir.status === Directory.Error ? dir.error
                        : dir.status === Directory.Loading ? "Reading…"
                        : dir.filter ? dir.count + (dir.count === 1 ? " match" : " matches")
                        : dir.count + (dir.count === 1 ? " item" : " items")
                }
                Label {
                    visible: dir.showHidden
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: "Hidden files shown"
                }
                // What is being copied or moved, while it is.
                Repeater {
                    model: FileJobs.running
                    delegate: RowLayout {
                        id: job
                        required property var modelData
                        spacing: Theme.s2
                        visible: job.modelData.state === FileJob.Running
                        Label {
                            size: Theme.sizeCaption
                            color: Theme.text2
                            text: (job.modelData.kind === FileJob.Copy ? "Copying " : "Moving ") + job.modelData.current
                                + (job.modelData.total > 1 ? " (" + job.modelData.count + " of " + job.modelData.total + ")" : "")
                        }
                        Rectangle {
                            Layout.preferredWidth: 90
                            Layout.preferredHeight: 4
                            radius: 2
                            color: Theme.hairline
                            Rectangle {
                                width: parent.width * Math.max(0, Math.min(1, job.modelData.progress))
                                height: parent.height
                                radius: 2
                                color: Theme.accent
                            }
                        }
                        Button { variant: "text"; glyph: "x"; onClicked: job.modelData.cancel() }
                    }
                }
            }
        }
    }
    }

    Item {
        id: overlay
        anchors.fill: parent

        FileMenu { id: menu }

        FileSheet {
            id: sheet
            // What the answer is for, and the job waiting on it when it is a conflict.
            property string what: ""
            property var job: null

            onRejected: {
                if (what === "conflict" && job) job.cancel();
                close();
            }
            onAlternate: forAll => {
                if (job) job.answer(FileJob.Keep, forAll);
                close();
            }
            onAccepted: (value, forAll) => {
                if (what === "rename") FileJobs.rename(root.actingOne, value);
                else if (what === "renameMany") FileJobs.renameMany(root.acting, value);
                else if (what === "compress") FileJobs.compress(root.acting, value);
                else if (what === "newFolder") FileJobs.newFolder(dir.path, value);
                else if (what === "delete") FileJobs.remove(root.acting);
                else if (what === "conflict" && job) job.answer(FileJob.Replace, forAll);
                close();
            }
        }
    }
}
