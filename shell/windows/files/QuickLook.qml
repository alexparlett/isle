import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui
import qs.services

// A look at what is picked without opening it: the picture itself where there is one, and what is
// known about it where there is not. Space opens and closes it, as Finder does.
Item {
    id: root
    property string path: ""
    property bool shown: false
    // Filled once a folder has been counted, and forgotten when the look moves on.
    property string folderSize: ""

    anchors.fill: parent
    visible: shown

    function look(at) {
        if (!at) return;
        path = at;
        folderSize = "";
        shown = true;
        card.forceActiveFocus();
    }
    function close() { shown = false; }
    // Opening is the window's business: it knows what to do when nothing handles the file.
    signal opened(string path)
    function toggle(at) { shown && at === path ? close() : look(at); }

    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.5) }
    MouseArea { anchors.fill: parent; onClicked: root.close() }

    Rectangle {
        id: card
        width: Math.min(760, root.width - Theme.s6 * 2)
        height: Math.min(620, root.height - Theme.s6 * 2)
        anchors.centerIn: parent
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairlineStrong
        radius: Theme.radiusPanel
        focus: true
        // Space closes it again, and so does Escape, so the key that opened it is the key that ends it.
        Keys.onEscapePressed: root.close()
        Keys.onSpacePressed: root.close()

        MouseArea { anchors.fill: parent }

        ColumnLayout {
            anchors { fill: parent; margins: Theme.s4 }
            spacing: Theme.s3

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s3
                IconImage {
                    implicitSize: 20
                    source: Quickshell.iconPath(Engine.iconNameFor(root.path), "text-x-generic")
                }
                Label {
                    Layout.fillWidth: true
                    size: Theme.sizeHeading
                    elide: Text.ElideMiddle
                    text: Engine.displayName(root.path)
                }
                Button { text: "Open"; onClicked: { root.opened(root.path); root.close(); } }
            }

            // The picture at the size it fits, or the file's particulars when it is not one.
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Image {
                    id: preview
                    anchors.fill: parent
                    visible: status === Image.Ready
                    asynchronous: true
                    cache: false
                    fillMode: Image.PreserveAspectFit
                    // Decoded no larger than it is drawn, so a very large photograph does not come
                    // into memory at its full size to be shown in a box this size.
                    sourceSize.width: Math.ceil(width)
                    sourceSize.height: Math.ceil(height)
                    source: root.shown && Thumbnails.canThumbnail(root.path)
                        ? "file://" + encodeURI(root.path) : ""
                }

                Flickable {
                    anchors.fill: parent
                    visible: !preview.visible
                    contentHeight: facts.implicitHeight
                    clip: true
                    ColumnLayout {
                        id: facts
                        width: parent.width
                        spacing: 0
                        // A folder's size is counted only when asked, since counting walks all of it.
                        RowLayout {
                            visible: root.shown && Engine.isDir(root.path)
                            Layout.fillWidth: true
                            Layout.preferredHeight: 30
                            spacing: Theme.s3
                            Label {
                                Layout.preferredWidth: 110
                                horizontalAlignment: Text.AlignRight
                                size: Theme.sizeSmall
                                color: Theme.text3
                                text: "Size"
                            }
                            Label {
                                visible: root.folderSize !== ""
                                Layout.fillWidth: true
                                size: Theme.sizeSmall
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
                            model: root.shown ? Engine.infoFor(root.path) : []
                            delegate: RowLayout {
                                id: fact
                                required property var modelData
                                Layout.fillWidth: true
                                Layout.preferredHeight: 30
                                spacing: Theme.s3
                                Label {
                                    Layout.preferredWidth: 110
                                    horizontalAlignment: Text.AlignRight
                                    size: Theme.sizeSmall
                                    color: Theme.text3
                                    text: fact.modelData.label
                                }
                                Label {
                                    Layout.fillWidth: true
                                    size: Theme.sizeSmall
                                    elide: Text.ElideMiddle
                                    text: fact.modelData.value
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
