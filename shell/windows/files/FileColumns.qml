import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui

// Finder's columns: the folder on the left, what is picked in it to the right of that, and so on.
// Each column is a directory of its own, so several are alive at once and none re-reads another.
Item {
    id: root
    // The folder the leftmost column shows.
    required property string rootPath
    property bool showHidden: false

    // The path each column shows, the first being rootPath. A pick truncates and extends it.
    property var chain: [rootPath]
    // What is picked in the rightmost column that has a pick, file or folder.
    property string selected: ""
    property bool selectedIsDir: false

    // A folder to open in place of the whole chain.
    signal activated(string path)
    // A file settled on.
    signal chosen(string path)

    onRootPathChanged: { chain = [rootPath]; selected = ""; selectedIsDir = false; }

    function pick(column, path, isDir) {
        const next = chain.slice(0, column + 1);
        if (isDir) next.push(path);
        chain = next;
        selected = path;
        selectedIsDir = isDir;
        if (isDir) Qt.callLater(() => flick.contentX = Math.max(0, flick.contentWidth - flick.width));
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

                    ListView {
                        id: entries
                        anchors { fill: parent; rightMargin: 1 }
                        model: folder
                        clip: true
                        reuseItems: true
                        boundsBehavior: Flickable.StopAtBounds
                        currentIndex: -1

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
                                onClicked: root.pick(column.index, entry.path, entry.isDir)
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
        }
    }
}
