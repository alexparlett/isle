import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "Desktops"
    meta: Windows.groups.length + ""

    RowLayout {
        anchors.fill: parent
        spacing: Theme.s2
        Repeater {
            model: Windows.groups
            Rectangle {
                required property var modelData
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.radiusCard
                color: modelData.focused ? Theme.raised : "transparent"
                border.width: 1
                border.color: modelData.focused ? Theme.hairlineStrong : Theme.hairline
                ColumnLayout {
                    anchors { fill: parent; margins: Theme.s2 + 2 }
                    spacing: Theme.s2
                    Label { text: modelData.id; size: 10; weight: Font.DemiBold; color: Theme.text3; tabular: true }
                    Flow {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: Theme.s1
                        Repeater {
                            model: modelData.apps
                            AppIcon { required property var modelData; source: modelData.icon; size: 22 }
                        }
                    }
                }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { Compositor.focusWorkspace(modelData.id); Surfaces.dashboard = false; } }
            }
        }
    }
}
