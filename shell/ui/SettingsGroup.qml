import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// A group of settings rows in one raised card, with an optional heading.
ColumnLayout {
    property string heading: ""
    default property alias rows: cardCol.data
    Layout.fillWidth: true
    spacing: Theme.s2
    Label { visible: heading !== ""; text: heading; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.leftMargin: Theme.s1 }
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: cardCol.implicitHeight
        radius: Theme.radiusCard
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairline
        clip: true
        ColumnLayout {
            id: cardCol
            anchors { left: parent.left; right: parent.right }
            spacing: 0
        }
    }
}
