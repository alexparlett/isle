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
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton) root.rightClicked(mouse.x, mouse.y);
            else if (mouse.button === Qt.MiddleButton) root.middleClicked();
            else root.clicked();
        }
        z: -1
    }
}
