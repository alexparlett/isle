import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui
// Loaded by URL at the root (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.files

// Finder's columns: the folder on the left, what is picked in it to the right of that, and so on.
// Each column is a directory of its own, so several are alive at once and none re-reads another.
Item {
    id: root
    // The folder the leftmost column shows.
    required property string rootPath
    property bool showHidden: false

    // The path each column shows, the first being rootPath. A pick truncates and extends it.
    property var chain: [rootPath]
    onChainChanged: if (chain.length <= 1) { selected = ""; selectedIsDir = false; }
    // What is picked in the rightmost column that has a pick, file or folder.
    property string selected: ""
    property bool selectedIsDir: false

    // A folder to open in place of the whole chain.
    signal activated(string path)
    // A file settled on.
    signal chosen(string path)
    // The right button, at a point in this item's own frame, over a path or over nothing.
    signal menuAsked(real x, real y, string path)


    // Which column the keys are in, and the listing and view of each one by column number, so the
    // keys can move within a column and between columns without any of them knowing about the rest.
    property int activeColumn: 0
    readonly property var folders: ({})
    readonly property var views: ({})

    function pick(column, path, isDir) {
        const next = chain.slice(0, column + 1);
        if (isDir) next.push(path);
        chain = next;
        selected = path;
        selectedIsDir = isDir;
        activeColumn = column;
        if (isDir) Qt.callLater(() => flick.contentX = Math.max(0, flick.contentWidth - flick.width));
    }

    // What is picked in a column: the folder that opened the column to its right, or, in the last
    // column, whatever was picked there.
    function currentIn(column) {
        return column + 1 < chain.length ? chain[column + 1] : selected;
    }
    function pickRow(column, row) {
        const folder = folders[column];
        if (!folder || row < 0 || row >= folder.count) return;
        pick(column, folder.pathAt(row), folder.isDirAt(row));
        const view = views[column];
        if (view) view.positionViewAtIndex(row, ListView.Contain);
    }
    function stepBy(by) {
        const folder = folders[activeColumn];
        if (!folder) return;
        const at = folder.rowOfPath(currentIn(activeColumn));
        pickRow(activeColumn, at < 0 ? 0 : at + by);
    }

    // Only the view being shown takes the keys; the other two are still there behind it.
    focus: visible
    // Left and right walk between columns, up and down within one, as a column browser is walked.
    Keys.onUpPressed: root.stepBy(-1)
    Keys.onDownPressed: root.stepBy(1)
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

                    width: 220
                    height: row.height

                    Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }

                    Directory {
                        id: folder
                        path: column.modelData
                        showHidden: root.showHidden
                    }
                    Component.onCompleted: { root.folders[column.index] = folder; root.views[column.index] = entries; }
                    Component.onDestruction: { delete root.folders[column.index]; delete root.views[column.index]; }

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: mouse => {
                            // Letting go in a column drops it and everything to the right of it.
                            root.chain = root.chain.slice(0, column.index + 1);
                            root.selected = "";
                            root.selectedIsDir = false;
                            root.activeColumn = column.index;
                            if (mouse.button !== Qt.RightButton) return;
                            const at = mapToItem(root, mouse.x, mouse.y);
                            root.menuAsked(at.x, at.y, "");
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

                            width: entries.width
                            height: 28
                            // The folder whose contents the next column is showing stays marked, so the
                            // chain from left to right can be read off.
                            readonly property bool onChain: root.chain[column.index + 1] === entry.path
                            color: root.selected === entry.path || entry.onChain ? Theme.pressed
                                 : entryArea.containsMouse ? Theme.raised : "transparent"

                            RowLayout {
                                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s2 }
                                spacing: Theme.s2
                                IconImage {
                                    implicitSize: 16
                                    source: Quickshell.iconPath(entry.iconName, "text-x-generic")
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: entry.name
                                    size: Theme.sizeSmall
                                }
                                // The chevron says the column to the right belongs to this row.
                                Glyph {
                                    visible: entry.isDir
                                    name: "chevron-right"
                                    size: 12
                                    color: entry.onChain ? Theme.text2 : Theme.text3
                                }
                            }

                            MouseArea {
                                id: entryArea
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onClicked: mouse => {
                                    root.pick(column.index, entry.path, entry.isDir);
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
                            text: "Empty"
                        }
                    }

                    Scrollbar { target: entries }
                }
            }

            // Finder's last column: what is picked, rather than another list. A folder opens a
            // column of its own, so only a file leaves anything for this to show.
            FilePreview {
                // The last column takes whatever the columns left, so a preview is a preview and not
                // a strip with a window of nothing beside it.
                width: Math.max(280, root.width - root.chain.length * 220)
                height: row.height
                visible: root.selected !== "" && !root.selectedIsDir
                picked: visible ? [root.selected] : []
                onOpened: path => root.chosen(path)
            }
        }
    }
}
