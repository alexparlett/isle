import QtQuick
import QtQuick.Layouts
import qs.theme

// A 40px list row: leading glyph, title and subtitle, trailing content. Hover raises it.
Rectangle {
    id: root
    property string glyph: ""
    property color glyphColor: Theme.text2
    property string title
    property string subtitle: ""
    property bool selected: false
    // Which buttons the row answers. A row that only opens something wants the left one alone.
    property int buttons: Qt.LeftButton
    // Whether the row can be picked up. The press is watched here because the row's own click area
    // takes it, and a handler placed over the row would never see the movement that follows.
    property bool draggable: false
    // Set by whoever answers dragStarted, once it has a picture for the drag: Qt takes the picture
    // when the drag is marked active, so the row cannot mark it itself.
    property bool carrying: false
    signal dragStarted()
    default property alias trailing: trailingSlot.data

    // Where a right click landed, in the row's own frame, for whatever puts a menu there.
    signal rightClicked(real x, real y)
    signal middleClicked()
    signal clicked

    implicitHeight: 40
    radius: Theme.radiusControl
    color: selected ? Theme.pressed : area.containsMouse ? Theme.raised : "transparent"
    Behavior on color { ColorAnimation { duration: Theme.quick } }

    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
        spacing: Theme.s3
        Glyph { visible: root.glyph !== ""; name: root.glyph; size: 14; color: root.glyphColor; Layout.preferredWidth: 18; Layout.alignment: Qt.AlignVCenter }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            Label { text: root.title; Layout.fillWidth: true }
            Label { visible: root.subtitle !== ""; text: root.subtitle; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true }
        }
        Item { id: trailingSlot; implicitWidth: childrenRect.width; implicitHeight: childrenRect.height }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: root.buttons
        property point pressedAt
        property bool dragging: false
        onPressed: mouse => { area.pressedAt = Qt.point(mouse.x, mouse.y); area.dragging = false; }
        onReleased: { area.dragging = false; root.carrying = false; }
        onPositionChanged: mouse => {
            if (!root.draggable || area.dragging || !(mouse.buttons & Qt.LeftButton)) return;
            const far = Math.abs(mouse.x - area.pressedAt.x) + Math.abs(mouse.y - area.pressedAt.y);
            if (far < Qt.styleHints.startDragDistance) return;
            area.dragging = true;
            root.dragStarted();
        }
        onClicked: mouse => {
            if (area.dragging) return;
            if (mouse.button === Qt.RightButton) root.rightClicked(mouse.x, mouse.y);
            else if (mouse.button === Qt.MiddleButton) root.middleClicked();
            else root.clicked();
        }
        z: -1
    }
}
