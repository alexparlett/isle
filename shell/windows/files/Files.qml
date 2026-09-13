import QtQuick
import QtQuick.Layouts
import Quickshell
import Isle.Files
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
            }
        }
    }
    }
}
