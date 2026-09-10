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
    // Four rows tall or more shows the month; the arrows browse, and a new day brings it back to now.
    readonly property bool month: root.rows >= 4 && root.settings.week !== false
    property int shownYear: clock.date.getFullYear()
    property int shownMonth: clock.date.getMonth()
    readonly property string today: clock.date.toDateString()
    onTodayChanged: { shownYear = clock.date.getFullYear(); shownMonth = clock.date.getMonth(); }
    function browse(by) { const d = new Date(shownYear, shownMonth + by, 1); shownYear = d.getFullYear(); shownMonth = d.getMonth(); }
    // Six weeks from the Monday on or before the first, so the grid never changes height.
    readonly property var gridStart: { const d = new Date(shownYear, shownMonth, 1); d.setDate(d.getDate() - (d.getDay() + 6) % 7); return d; }

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

        Item { Layout.fillHeight: true; visible: !root.month }

        // The month: its name with arrows, weekday initials, six weeks of days.
        ColumnLayout {
            visible: root.month
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.s1
            // The month sits at the right with its arrows, clear of the date above it.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s2
                Item { Layout.fillWidth: true }
                Label { visible: root.shownYear !== clock.date.getFullYear() || root.shownMonth !== clock.date.getMonth(); text: "Today"; size: Theme.sizeCaption; color: Theme.accent
                    MouseArea { anchors { fill: parent; margins: -4 } cursorShape: Qt.PointingHandCursor; onClicked: { root.shownYear = clock.date.getFullYear(); root.shownMonth = clock.date.getMonth(); } } }
                Glyph { name: "chevron-left"; size: 14; color: Theme.text2; MouseArea { anchors { fill: parent; margins: -6 } cursorShape: Qt.PointingHandCursor; onClicked: root.browse(-1) } }
                Label { text: Qt.formatDate(new Date(root.shownYear, root.shownMonth, 1), "MMMM yyyy"); weight: Font.DemiBold; size: Theme.sizeSmall; horizontalAlignment: Text.AlignHCenter; Layout.preferredWidth: 120 }
                Glyph { name: "chevron-right"; size: 14; color: Theme.text2; MouseArea { anchors { fill: parent; margins: -6 } cursorShape: Qt.PointingHandCursor; onClicked: root.browse(1) } }
            }
            GridLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                columns: 7
                columnSpacing: 2
                rowSpacing: 2
                Repeater {
                    model: 7
                    Label { required property int index; Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: ["M", "T", "W", "T", "F", "S", "S"][index]; size: 10; weight: Font.DemiBold; color: Theme.text3 }
                }
                Repeater {
                    model: 42
                    Rectangle {
                        required property int index
                        readonly property var day: { const d = new Date(root.gridStart); d.setDate(d.getDate() + index); return d; }
                        readonly property bool inMonth: day.getMonth() === root.shownMonth
                        readonly property bool isToday: day.toDateString() === root.today
                        readonly property bool weekend: day.getDay() === 0 || day.getDay() === 6
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: 22
                        radius: Theme.radiusControl
                        color: isToday ? Theme.accent : "transparent"
                        Label { anchors.centerIn: parent; text: day.getDate(); tabular: true; size: Theme.sizeSmall; weight: isToday ? Font.DemiBold : Font.Medium
                                color: isToday ? Theme.onAccent : !inMonth ? Theme.text3 : weekend ? Theme.text2 : Theme.text; opacity: inMonth ? 1 : 0.55 }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.settings.week !== false && !root.month
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
