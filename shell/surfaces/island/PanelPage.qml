import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// A drill-down inside the control panel: back arrow, title, an optional control, then rows.
ColumnLayout {
    id: root
    property string title
    default property alias rows: list.data
    signal back
    signal shown
    signal hidden

    spacing: Theme.s2
    onVisibleChanged: visible ? shown() : hidden()

    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: Theme.s1
        spacing: Theme.s2
        Item {
            implicitWidth: 28; implicitHeight: 28
            Glyph { anchors.centerIn: parent; name: "arrow-left"; size: 14 }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.back() }
        }
        Label { text: root.title; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
    }

    ColumnLayout {
        id: list
        Layout.fillWidth: true
        spacing: 2
    }
}
