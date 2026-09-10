import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "Storage"

    Component.onCompleted: Storage.listeners++
    Component.onDestruction: Storage.listeners--

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s2
        Repeater {
            model: Storage.mounts
            ColumnLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: Theme.s1 + 1
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: Storage.label(modelData); size: Theme.sizeSmall; Layout.fillWidth: true }
                    Label { text: System.bytes(modelData.used) + " / " + System.bytes(modelData.size); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2 }
                }
                Rectangle {
                    Layout.fillWidth: true
                    height: 3; radius: 2
                    color: Theme.hairlineStrong
                    Rectangle { width: parent.width * modelData.pct; height: parent.height; radius: 2; color: modelData.pct > 0.85 ? Theme.warn : Theme.text2 }
                }
            }
        }
        Repeater {
            model: Disks.volumes
            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: Theme.s2
                Glyph { name: "hard-drive"; size: 14; color: modelData.mounted ? Theme.text : Theme.text3 }
                Label { text: modelData.label; size: Theme.sizeSmall; Layout.fillWidth: true }
                Label { text: Disks.sizeLabel(modelData.size); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2 }
                Button { text: modelData.mounted ? "Open" : "Mount"; implicitHeight: 24; onClicked: modelData.mounted ? Disks.open(modelData) : Disks.mount(modelData) }
                Button { text: "Eject"; variant: "text"; implicitHeight: 24; onClicked: Disks.eject(modelData) }
            }
        }
        Item { Layout.fillHeight: true }
    }
}
