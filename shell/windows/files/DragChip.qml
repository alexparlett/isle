import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import Isle.Files
import qs.theme
import qs.ui

// What the pointer carries during a drag: the thing's icon and name, or a count when it is several.
// Drawn off to the side rather than hidden, because an item that is not rendered cannot be grabbed,
// and a drag with no image is a drag with nothing under the pointer.
Item {
    id: root
    property var paths: []

    x: -4000
    y: -4000
    width: body.implicitWidth + Theme.s3 * 2
    height: 34

    // Takes a picture of itself and hands it to the item being dragged, which then starts the drag.
    // Grabbing is answered later and Qt takes the picture when the drag is marked active, so the
    // caller marks it active in `then` rather than before asking.
    function carry(item, what, then) {
        root.paths = what;
        Qt.callLater(() => {
            const asked = root.grabToImage(result => {
                item.Drag.imageSource = result.url;
                item.Drag.hotSpot = Qt.point(root.width / 2, root.height / 2);
                if (then) then();
            });
            // A grab that cannot be taken must not stop the drag; it only goes without a picture.
            if (!asked) { console.warn("drag chip: grab refused"); if (then) then(); }
        });
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusChip
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairlineStrong
    }

    RowLayout {
        id: body
        anchors.centerIn: parent
        spacing: Theme.s2

        IconImage {
            implicitSize: 18
            visible: root.paths.length === 1
            source: root.paths.length === 1
                ? Quickshell.iconPath(Engine.iconNameFor(root.paths[0]), "text-x-generic") : ""
        }
        // Several carried together are a count, the way a stack of them is counted.
        Rectangle {
            visible: root.paths.length > 1
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            radius: 10
            color: Theme.accent
            Label {
                anchors.centerIn: parent
                size: Theme.sizeCaption
                color: Theme.ink
                text: root.paths.length
            }
        }
        Label {
            size: Theme.sizeSmall
            text: root.paths.length === 1 ? Engine.displayName(root.paths[0])
                                          : root.paths.length + " items"
        }
    }
}
