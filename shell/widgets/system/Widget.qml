import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "System"
    meta: (System.uptime ? "up " + System.uptime : "") + (System.kernel ? " · " + System.kernel : "")

    Component.onCompleted: System.listeners++
    Component.onDestruction: System.listeners--

    component Stat: Rectangle {
        property string label
        property string value
        property var history: []
        property color tint: Theme.text2
        Layout.fillWidth: true
        Layout.fillHeight: true
        radius: Theme.radiusCard
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairline
        ColumnLayout {
            anchors { fill: parent; margins: Theme.s2 + 2 }
            spacing: Theme.s1
            RowLayout {
                Layout.fillWidth: true
                Label { text: parent.parent.parent.label; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.fillWidth: true }
                Label { text: parent.parent.parent.value; mono: true; tabular: true; size: Theme.sizeCaption }
            }
            Sparkline { Layout.fillWidth: true; Layout.fillHeight: true; values: parent.parent.history; color: parent.parent.tint }
        }
    }

    GridLayout {
        anchors.fill: parent
        columns: 2
        columnSpacing: Theme.s2 - 2
        rowSpacing: Theme.s2 - 2
        Stat { label: "CPU"; value: Math.round(System.cpu * 100) + "%"; history: System.cpuHistory }
        Stat { label: "GPU"; value: System.gpu < 0 ? "—" : Math.round(System.gpu * 100) + "%" + (System.gpuTemp ? " · " + System.gpuTemp + "°" : ""); history: System.gpuHistory; tint: Theme.accent }
        Stat { label: "Memory"; value: System.bytes(System.memUsed); history: System.memHistory }
        Stat { label: "Network"; value: "↓ " + System.rate(System.netDown); history: System.netHistory.map(v => Math.min(1, v / 1e7)) }
    }
}
