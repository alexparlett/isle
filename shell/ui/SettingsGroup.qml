import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// A group of settings rows in one raised card, with an optional heading.
ColumnLayout {
    property string heading: ""
    // What the group as a whole offers, beside its heading: adding one of whatever it lists.
    property alias action: actionRow.data
    default property alias rows: cardCol.data
    Layout.fillWidth: true
    spacing: Theme.s2
    RowLayout {
        visible: heading !== "" || actionRow.children.length > 0
        Layout.fillWidth: true
        spacing: Theme.s2
        Label { visible: heading !== ""; text: heading; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.leftMargin: Theme.s1 }
        Item { Layout.fillWidth: true }
        RowLayout { id: actionRow; Layout.rightMargin: Theme.s3; spacing: Theme.s2 }
    }
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
