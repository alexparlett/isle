import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// A settings page: title, one line under it, then the sections.
ColumnLayout {
    id: root
    property string title
    property string subtitle: ""
    default property alias content: body.data
    spacing: Theme.s5

    ColumnLayout {
        spacing: Theme.s1
        Label { text: root.title; size: Theme.sizeTitle; weight: Font.DemiBold }
        Label { visible: root.subtitle !== ""; text: root.subtitle; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
    }

    ColumnLayout {
        id: body
        Layout.fillWidth: true
        spacing: Theme.s4
    }
}
