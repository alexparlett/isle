import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui

// A page that is one list: the largest files, or the folders nothing has changed in.
ColumnLayout {
    id: root
    required property var app
    property var rows: []
    property string summary: ""
    property string empty: ""
    property bool showAge: false
    spacing: Theme.s3

    property string filter: ""
    readonly property var shown: {
        const q = filter.toLowerCase();
        return q ? rows.filter(r => r.path.toLowerCase().indexOf(q) >= 0) : rows;
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s3
        Label { Layout.fillWidth: true; text: root.summary; color: Theme.text2; wrapMode: Text.WordWrap }
        Button {
            visible: root.shown.length > 0
            text: "Select all"
            variant: "text"
            implicitHeight: 30
            onClicked: root.app.addAll(root.shown)
        }
        Field {
            Layout.preferredWidth: 220
            implicitHeight: 30
            size: Theme.sizeSmall
            glyph: "search"
            placeholder: "Filter by name or place"
            onTextChanged: root.filter = text
        }
    }

    ListView {
        id: list
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: root.shown
        delegate: ItemRow {
            required property var modelData
            width: ListView.view.width - Theme.s3
            app: root.app
            item: modelData
            showAge: root.showAge
            maxSize: root.shown.length ? root.shown[0].size : 1
        }
        Scrollbar { target: list; anchors { right: parent.right; top: parent.top; bottom: parent.bottom } }
        Label { anchors.centerIn: parent; visible: list.count === 0; text: root.empty; color: Theme.text3 }
    }
}
