import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui

WidgetBase {
    id: root
    title: ""

    SystemClock { id: clock; precision: root.settings.seconds ? SystemClock.Seconds : SystemClock.Minutes }
    readonly property string timeFormat: (root.settings.format === "12h" ? "h:mm" : "HH:mm") + (root.settings.seconds ? ":ss" : "") + (root.settings.format === "12h" ? " AP" : "")

    // Monday of this week, for the strip.
    readonly property var monday: { const d = new Date(clock.date); const day = (d.getDay() + 6) % 7; d.setDate(d.getDate() - day); return d; }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s3

        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                spacing: 0
                Label { text: Qt.formatTime(clock.date, root.timeFormat); size: 34; weight: Font.DemiBold; tabular: true; font.letterSpacing: -1 }
                Label { text: Qt.formatDate(clock.date, "dddd d MMMM"); color: Theme.text2; size: Theme.sizeSmall }
            }
            Item { Layout.fillWidth: true }
        }

        Item { Layout.fillHeight: true }

        RowLayout {
            Layout.fillWidth: true
            visible: root.settings.week !== false
            spacing: Theme.s1
            Repeater {
                model: 7
                Rectangle {
                    required property int index
                    readonly property var day: { const d = new Date(root.monday); d.setDate(d.getDate() + index); return d; }
                    readonly property bool today: day.toDateString() === clock.date.toDateString()
                    readonly property bool past: day < clock.date && !today
                    Layout.fillWidth: true
                    implicitHeight: 44
                    radius: Theme.radiusControl
                    color: today ? Theme.raised : "transparent"
                    border.width: 1
                    border.color: today ? Theme.hairlineStrong : "transparent"
                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 2
                        Label { text: Qt.formatDate(day, "ddd").charAt(0); size: 10; weight: Font.DemiBold; color: Theme.text3; Layout.alignment: Qt.AlignHCenter }
                        Label { text: day.getDate(); tabular: true; weight: today ? Font.DemiBold : Font.Medium; color: past ? Theme.text3 : Theme.text; Layout.alignment: Qt.AlignHCenter }
                    }
                }
            }
        }
    }
}
