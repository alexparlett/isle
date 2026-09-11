import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.services

// A tray item's menu as rows: separators, checks, submenu chevrons. Choosing an entry triggers it and ends.
// An app kept running (an entry with windows) gets show or hide, and quit.
ColumnLayout {
    id: root
    property var item: null
    readonly property bool kept: !!(item && item.windows)
    readonly property bool empty: !kept && opener.children.values.length === 0
    signal done
    spacing: 1

    QsMenuOpener { id: opener; menu: root.item && !root.kept ? root.item.menu : null }
    Repeater {
        model: root.kept ? [
            { text: root.item.hidden ? "Show" : "Hide", enabled: true, triggered: () => Windows.toggleKept(root.item) },
            { isSeparator: true },
            { text: "Quit", enabled: true, triggered: () => Windows.quitApp(root.item) }
        ] : opener.children.values
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
