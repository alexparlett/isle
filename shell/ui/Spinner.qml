import QtQuick
import qs.theme

// A turning ring for a wait whose length is not known.
Item {
    id: root
    property int size: 16
    property color color: Theme.text2
    implicitWidth: size
    implicitHeight: size
    Glyph {
        id: glyph
        anchors.centerIn: parent
        name: "loader-circle"; size: root.size; color: root.color
        RotationAnimation on rotation { from: 0; to: 360; duration: 900; loops: Animation.Infinite; running: root.visible }
    }
}
