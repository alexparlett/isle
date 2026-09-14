import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui

// Finder's preview: a column beside the folder showing what one picked thing is, without opening it.
Rectangle {
    id: root
    // What is picked. Nothing, or more than one, has nothing to preview.
    property var picked: []
    readonly property string path: picked.length === 1 ? picked[0] : ""
    // Filled once a folder has been counted, and forgotten when the pick moves on.
    property string folderSize: ""
    signal opened(string path)

    color: "transparent"
    Rectangle { anchors.left: parent.left; width: 1; height: parent.height; color: Theme.hairline }

    onPathChanged: folderSize = ""

    Label {
        anchors.centerIn: parent
        width: parent.width - Theme.s4 * 2
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        visible: root.path === ""
        size: Theme.sizeSmall
        color: Theme.text3
        text: root.picked.length > 1 ? root.picked.length + " items selected" : "Nothing selected"
    }

    Flickable {
        anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s3; topMargin: Theme.s4 }
        visible: root.path !== ""
        contentHeight: body.implicitHeight
        clip: true

        // The facts stay a column a person can read down even when the pane is very wide.
        ColumnLayout {
            id: body
            width: Math.min(420, parent.width)
            x: Math.max(0, (parent.width - width) / 2)
            spacing: Theme.s3

            // The picture where there is one, and the file's own icon at a size worth looking at
            // where there is not.
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(body.width, 200)

                // The file itself, decoded no larger than the pane. Anything that is not a picture
                // never reaches Ready and the icon behind it stands instead.
                Image {
                    id: picture
                    anchors.fill: parent
                    visible: status === Image.Ready
                    asynchronous: true
                    cache: false
                    fillMode: Image.PreserveAspectFit
                    sourceSize.width: Math.ceil(width)
                    sourceSize.height: Math.ceil(height)
                    source: root.path !== "" && Thumbnails.canThumbnail(root.path)
                        ? "file://" + encodeURI(root.path) : ""
                }
                IconImage {
                    anchors.centerIn: parent
                    visible: !picture.visible
                    implicitSize: 96
                    source: Quickshell.iconPath(Engine.iconNameFor(root.path), "text-x-generic")
                }
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                maximumLineCount: 3
                elide: Text.ElideMiddle
                text: Engine.displayName(root.path)
            }

            // A folder's size is counted only when asked, since counting walks all of it.
            RowLayout {
                visible: root.path !== "" && Engine.isDir(root.path)
                Layout.fillWidth: true
                spacing: Theme.s2
                Label {
                    Layout.preferredWidth: 80
                    horizontalAlignment: Text.AlignRight
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: "Size"
                }
                Label {
                    visible: root.folderSize !== ""
                    Layout.fillWidth: true
                    size: Theme.sizeCaption
                    text: root.folderSize
                }
                Button {
                    visible: root.folderSize === ""
                    variant: "text"
                    text: "Count"
                    onClicked: root.folderSize = Engine.sizeOfFolder(root.path)
                }
            }

            Repeater {
                model: root.path !== "" ? Engine.infoFor(root.path) : []
                delegate: RowLayout {
                    id: fact
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Label {
                        Layout.preferredWidth: 80
                        Layout.alignment: Qt.AlignTop
                        horizontalAlignment: Text.AlignRight
                        size: Theme.sizeCaption
                        color: Theme.text3
                        text: fact.modelData.label
                    }
                    Label {
                        Layout.fillWidth: true
                        size: Theme.sizeCaption
                        wrapMode: Text.Wrap
                        elide: Text.ElideMiddle
                        maximumLineCount: 3
                        text: fact.modelData.value
                    }
                }
            }

            Button {
                Layout.fillWidth: true
                Layout.topMargin: Theme.s2
                Layout.bottomMargin: Theme.s4
                text: "Open"
                onClicked: root.opened(root.path)
            }
        }
    }
}
