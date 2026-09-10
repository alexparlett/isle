import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// Rest: workspace dots, the clock, and glyphs that only appear when they say something.
RowLayout {
    required property var clock
    spacing: Theme.s3 - 2

    // The workspace dots; a click opens Mission Control.
    Item {
        implicitWidth: dots.implicitWidth; implicitHeight: dots.implicitHeight
        Row {
            id: dots
            spacing: 5
            Repeater {
                model: Compositor.workspaceIds
                Rectangle {
                    required property int modelData
                    width: 6; height: 6; radius: 3
                    color: modelData === Compositor.focusedId ? Theme.accent : Theme.text3
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
        Glyph { name: "shield"; size: 14; visible: Vpn.active !== null; color: Theme.ok }
        Glyph { name: "download"; size: 14; visible: Updates.count > 0 || IsleUpdate.behind > 0; color: Theme.accent }
    }
}
