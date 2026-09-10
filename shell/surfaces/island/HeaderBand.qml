import QtQuick
import qs.theme
import qs.ui
import qs.services

// The top band of the unfolded island: the clock and date, centred.
Item {
    required property var clock
    required property var panel

    Row {
        anchors.centerIn: parent
        spacing: Theme.s2
        Label { text: Qt.formatTime(clock.date, DateTime.timeFormat); tabular: true; size: Theme.sizeHeading; weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
        Label { text: "·"; color: Theme.text3; anchors.verticalCenter: parent.verticalCenter }
        Label { text: Qt.formatDate(clock.date, "ddd d MMM"); color: Theme.text2; anchors.verticalCenter: parent.verticalCenter }
    }
}
