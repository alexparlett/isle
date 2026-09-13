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

    ColumnLayout {
        anchors.fill: parent
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

                Field {
                    id: search
                    Layout.preferredWidth: 200
                    glyph: "search"
                    placeholder: "Search this folder"
                    onTextChanged: dir.filter = text
                    input.Keys.onEscapePressed: { text = ""; list.forceActiveFocus(); }
                }
            }
        }

        DetailList {
            id: list
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
