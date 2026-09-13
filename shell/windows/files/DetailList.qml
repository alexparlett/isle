import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui
import qs.services

// The folder as rows of name, size and modified, with a header that sorts.
FocusScope {
    id: root
    required property Directory directory
    // Whether a file is opened where it lands. A chooser takes it as the choice instead.
    property bool openFiles: true
    // A folder to walk into.
    signal activated(string path)
    // A file settled on, when openFiles is false.
    signal chosen(string path)

    readonly property int nameWidth: Math.max(220, width - 320)
    readonly property bool hasSelection: list.currentIndex >= 0 && list.currentIndex < directory.count
        && !directory.isDirAt(list.currentIndex)
    readonly property string currentPath: hasSelection ? directory.pathAt(list.currentIndex) : ""

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

    // The folder changed under the selection: start at the top rather than on whatever is there now.
    Connections {
        target: root.directory
        function onPathChanged() { list.currentIndex = 0; list.positionViewAtBeginning(); }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
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
                        required property var modelData
                        Layout.preferredWidth: modelData.width
                        Layout.fillWidth: modelData.column === Directory.ByName
                        Layout.fillHeight: true
                        RowLayout {
                            anchors.fill: parent
                            spacing: 2
                            layoutDirection: modelData.align === Text.AlignRight ? Qt.RightToLeft : Qt.LeftToRight
                            Label {
                                text: modelData.label
                                size: Theme.sizeCaption
                                color: root.directory.sort === modelData.column ? Theme.text2 : Theme.text3
                            }
                            Glyph {
                                visible: root.directory.sort === modelData.column
                                name: root.directory.sortOrder === Qt.AscendingOrder ? "chevron-up" : "chevron-down"
                                size: 11
                                color: Theme.text3
                            }
                            Item { Layout.fillWidth: true }
                        }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.sortBy(modelData.column) }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: list
                anchors.fill: parent
                model: root.directory
                clip: true
                focus: true
                currentIndex: 0
                // A folder of fifty thousand rows only ever builds the ones on screen.
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds

                Keys.onReturnPressed: root.open(currentIndex)
                Keys.onEnterPressed: root.open(currentIndex)

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
                    color: ListView.isCurrentItem ? Theme.pressed : area.containsMouse ? Theme.raised : "transparent"

                    RowLayout {
                        anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 }
                        spacing: Theme.s3

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredWidth: root.nameWidth
                            spacing: Theme.s2 + 2
                            IconImage {
                                implicitSize: 18
                                source: Quickshell.iconPath(row.iconName, "text-x-generic")
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
                        id: area
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: { list.currentIndex = row.index; list.forceActiveFocus(); }
                        onDoubleClicked: root.open(row.index)
                    }
                }
            }

            Scrollbar { target: list }

            // Nothing to show, and why.
            Label {
                anchors.centerIn: parent
                visible: root.directory.status === Directory.Ready && root.directory.count === 0
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
