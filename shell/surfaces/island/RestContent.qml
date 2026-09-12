import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// Rest: workspace dots, the clock, and glyphs that only appear when they say something.
RowLayout {
    id: rest
    required property var clock
    // The screen this pill is on: its desktops, not the focused monitor's.
    property string screenName: ""
    spacing: Theme.s3 - 2

    // The workspace dots; a click opens Mission Control.
    Item {
        implicitWidth: dots.implicitWidth; implicitHeight: dots.implicitHeight
        Row {
            id: dots
            spacing: 5
            Repeater {
                model: { Compositor.workspaces.values; Compositor.monitors; return rest.screenName ? Compositor.workspaceIdsOn(rest.screenName) : Compositor.workspaceIds; }
                Rectangle {
                    required property int modelData
                    width: 6; height: 6; radius: 3
                    color: modelData === (rest.screenName ? Compositor.activeIdOn(rest.screenName) : Compositor.focusedId) ? Theme.accent : Theme.text3
                    Behavior on color { ColorAnimation { duration: Theme.quick } }
                }
            }
        }
        MouseArea { anchors.fill: parent; anchors.margins: -4; cursorShape: Qt.PointingHandCursor; onClicked: Surfaces.overview = true }
    }

    Label {
        text: Qt.formatTime(clock.date, DateTime.timeFormat)
        tabular: true
    }

    Row {
        spacing: Theme.s2
        // An event within the half hour; a click opens the dashboard, where the Calendar widget has it.
        Item {
            visible: Calendars.soon !== null
            implicitWidth: 14; implicitHeight: 14
            anchors.verticalCenter: parent.verticalCenter
            Glyph { anchors.centerIn: parent; name: "calendar"; size: 14; color: Theme.accent }
            MouseArea { anchors.fill: parent; anchors.margins: -4; cursorShape: Qt.PointingHandCursor; onClicked: Surfaces.dashboard = true }
        }
        Glyph { name: "bell-off"; size: 14; visible: Notifications.silenced && Notifications.count === 0; color: Theme.text3 }
        // Waiting notifications: the bell with a count; a click opens the centre.
        Item {
            visible: Notifications.count > 0
            implicitWidth: bellRow.implicitWidth; implicitHeight: bellRow.implicitHeight
            Row {
                id: bellRow
                spacing: 3
                Glyph { name: Notifications.silenced ? "bell-off" : "bell"; size: 14; color: Notifications.silenced ? Theme.text3 : Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                Label { text: Notifications.count; size: Theme.sizeCaption; weight: Font.DemiBold; tabular: true; color: Theme.text2; anchors.verticalCenter: parent.verticalCenter }
            }
            MouseArea { anchors.fill: parent; anchors.margins: -4; cursorShape: Qt.PointingHandCursor; onClicked: Notifications.openCentre() }
        }
        Glyph { name: "bluetooth"; size: 14; visible: Bluetooth.anyConnected }
        Glyph { name: "volume-x"; size: 14; visible: Audio.muted }
        Glyph { name: "mic-off"; size: 14; visible: Audio.sourceMuted; color: Theme.warn }
        // Privacy: something is listening or watching.
        Glyph { name: "mic"; size: 14; visible: Audio.micInUse && !Audio.sourceMuted; color: Theme.live }
        Glyph { name: "camera"; size: 14; visible: Audio.cameraInUse; color: Theme.live }
        Glyph { name: "screen-share"; size: 14; visible: Audio.screenShared; color: Theme.live }
        Glyph { name: "shield"; size: 14; visible: Vpn.active !== null; color: Theme.ok }
        Glyph { name: "download"; size: 14; visible: (Updates.count > 0 || IsleUpdate.behind > 0) && !Updates.restartNeeded; color: Theme.accent }
        // A restart is owed to an update already made: louder than updates waiting.
        Glyph { name: "rotate-cw"; size: 14; visible: Updates.restartNeeded; color: Theme.warn }
    }
}
