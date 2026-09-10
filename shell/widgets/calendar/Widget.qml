import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui

// A month: its name with arrows, weekday initials, six weeks of days with today marked. A new day brings it back to now.
WidgetBase {
    id: root
    title: ""

    SystemClock { id: clock; precision: SystemClock.Minutes }
    property int shownYear: clock.date.getFullYear()
    property int shownMonth: clock.date.getMonth()
    readonly property string today: clock.date.toDateString()
    onTodayChanged: { shownYear = clock.date.getFullYear(); shownMonth = clock.date.getMonth(); }
    function browse(by) { const d = new Date(shownYear, shownMonth + by, 1); shownYear = d.getFullYear(); shownMonth = d.getMonth(); }
    // Six weeks from the Monday on or before the first, so the grid never changes height.
    readonly property var gridStart: { const d = new Date(shownYear, shownMonth, 1); d.setDate(d.getDate() - (d.getDay() + 6) % 7); return d; }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s1
        // The month centred between its arrows; Today shows only while looking at another month.
        Item {
            Layout.fillWidth: true
            implicitHeight: 24
            Glyph { anchors { left: parent.left; verticalCenter: parent.verticalCenter } name: "chevron-left"; size: 15; color: Theme.text2
                MouseArea { anchors { fill: parent; margins: -8 } cursorShape: Qt.PointingHandCursor; onClicked: root.browse(-1) } }
            Label { anchors.centerIn: parent; text: Qt.formatDate(new Date(root.shownYear, root.shownMonth, 1), "MMMM yyyy"); weight: Font.DemiBold; size: Theme.sizeBody }
            Label { anchors { right: todayArrow.left; rightMargin: Theme.s3; verticalCenter: parent.verticalCenter }
                    visible: root.shownYear !== clock.date.getFullYear() || root.shownMonth !== clock.date.getMonth(); text: "Today"; size: Theme.sizeCaption; color: Theme.accent
                MouseArea { anchors { fill: parent; margins: -4 } cursorShape: Qt.PointingHandCursor; onClicked: { root.shownYear = clock.date.getFullYear(); root.shownMonth = clock.date.getMonth(); } } }
            Glyph { id: todayArrow; anchors { right: parent.right; verticalCenter: parent.verticalCenter } name: "chevron-right"; size: 15; color: Theme.text2
                MouseArea { anchors { fill: parent; margins: -8 } cursorShape: Qt.PointingHandCursor; onClicked: root.browse(1) } }
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
}
