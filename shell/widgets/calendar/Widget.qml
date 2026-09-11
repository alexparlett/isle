import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

// A month: its name with arrows, weekday initials, six weeks of days with today marked and a dot per
// calendar that has something that day; the picked day's events listed beneath. A new day brings it back.
WidgetBase {
    id: root
    title: ""

    SystemClock { id: clock; precision: SystemClock.Minutes }
    property int shownYear: clock.date.getFullYear()
    property int shownMonth: clock.date.getMonth()
    readonly property string today: clock.date.toDateString()
    property var picked: new Date()
    onTodayChanged: { shownYear = clock.date.getFullYear(); shownMonth = clock.date.getMonth(); picked = new Date(clock.date); }
    function browse(by) { const d = new Date(shownYear, shownMonth + by, 1); shownYear = d.getFullYear(); shownMonth = d.getMonth(); }
    function toToday() { shownYear = clock.date.getFullYear(); shownMonth = clock.date.getMonth(); picked = new Date(clock.date); }
    // Six weeks from the Monday on or before the first, so the grid never changes height.
    readonly property var gridStart: { const d = new Date(shownYear, shownMonth, 1); d.setDate(d.getDate() - (d.getDay() + 6) % 7); return d; }
    readonly property var dayEvents: { Calendars.byDay; return Calendars.on(picked); }
    function colorFor(e) { const c = Calendars.colorOf(e.cal); return c || Theme.accent; }
    function timeOf(e) { return e.allDay ? "All day" : Qt.formatTime(new Date(e.start), DateTime.timeFormat); }

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
                MouseArea { anchors { fill: parent; margins: -4 } cursorShape: Qt.PointingHandCursor; onClicked: root.toToday() } }
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
                    id: cell
                    required property int index
                    readonly property var day: { const d = new Date(root.gridStart); d.setDate(d.getDate() + index); return d; }
                    readonly property bool inMonth: day.getMonth() === root.shownMonth
                    readonly property bool isToday: day.toDateString() === root.today
                    readonly property bool isPicked: day.toDateString() === root.picked.toDateString()
                    readonly property bool weekend: day.getDay() === 0 || day.getDay() === 6
                    // One dot per calendar with an event that day, three at most.
                    readonly property var marks: {
                        Calendars.byDay;
                        const seen = [];
                        for (const e of Calendars.on(day)) if (seen.indexOf(e.cal) < 0) seen.push(e.cal);
                        return seen.slice(0, 3);
                    }
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: 22
                    radius: Theme.radiusControl
                    color: isToday ? Theme.accent : isPicked ? Theme.raised : "transparent"
                    border.width: isPicked && !isToday ? 1 : 0
                    border.color: Theme.hairline
                    Label { anchors.centerIn: parent; anchors.verticalCenterOffset: cell.marks.length ? -2 : 0; text: cell.day.getDate(); tabular: true; size: Theme.sizeSmall; weight: cell.isToday ? Font.DemiBold : Font.Medium
                            color: cell.isToday ? Theme.onAccent : !cell.inMonth ? Theme.text3 : cell.weekend ? Theme.text2 : Theme.text; opacity: cell.inMonth ? 1 : 0.55 }
                    Row {
                        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 3 }
                        spacing: 2
                        Repeater {
                            model: cell.marks
                            Rectangle { required property string modelData; width: 4; height: 4; radius: 2; color: cell.isToday ? Theme.onAccent : (Calendars.colorOf(modelData) || Theme.accent) }
                        }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.picked = new Date(cell.day) }
                }
            }
        }
        // The picked day's events, when there are any: a colour bar, the time, the title.
        ColumnLayout {
            visible: root.dayEvents.length > 0
            Layout.fillWidth: true
            spacing: 2
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.hairline; Layout.bottomMargin: 2 }
            Repeater {
                model: root.dayEvents.slice(0, 4)
                RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Rectangle { width: 3; height: 14; radius: 1.5; color: root.colorFor(modelData) }
                    Label { text: root.timeOf(modelData); size: Theme.sizeCaption; tabular: true; color: Theme.text2; Layout.preferredWidth: modelData.allDay ? implicitWidth : 42 }
                    Label { text: modelData.title; size: Theme.sizeCaption; elide: Text.ElideRight; Layout.fillWidth: true }
                }
            }
            Label { visible: root.dayEvents.length > 4; text: "+" + (root.dayEvents.length - 4) + " more"; size: Theme.sizeCaption; color: Theme.text3 }
        }
    }
}
