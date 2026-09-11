import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme

// A tray item's menu as rows: separators, checks, submenu chevrons. Choosing an entry triggers it and ends.
ColumnLayout {
    id: root
    property var item: null
    readonly property bool empty: opener.children.values.length === 0
    signal done
    spacing: 1

    QsMenuOpener { id: opener; menu: root.item ? root.item.menu : null }
    Repeater {
        model: opener.children.values
        Item {
            required property var modelData
            Layout.fillWidth: true
            implicitHeight: modelData.isSeparator ? 9 : 28
            Rectangle { visible: modelData.isSeparator; anchors.centerIn: parent; width: parent.width - Theme.s3 * 2; height: 1; color: Theme.hairline }
            Rectangle {
                visible: !modelData.isSeparator
                anchors.fill: parent; radius: Theme.radiusChip
                color: entryArea.containsMouse ? Theme.raised : "transparent"
                opacity: modelData.enabled ? 1 : 0.45
                RowLayout {
                    anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
                    spacing: Theme.s2
                    Glyph { visible: modelData.checkState === Qt.Checked; name: "check"; size: 10; color: Theme.accent }
                    Label { text: modelData.text; size: Theme.sizeSmall; Layout.fillWidth: true; elide: Text.ElideRight }
                    Glyph { visible: modelData.hasChildren; name: "chevron-right"; size: 10; color: Theme.text3 }
                }
                MouseArea { id: entryArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: modelData.enabled
                    onClicked: { modelData.triggered(); root.done(); } }
            }
        }
    }
}
