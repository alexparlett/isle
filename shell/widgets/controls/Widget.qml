import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "Controls"

    component Pill: Rectangle {
        property string glyph
        property string label
        property bool on: false
        property bool enabled: true
        signal toggled(bool on)
        Layout.fillWidth: true
        Layout.preferredHeight: 36
        radius: Theme.radiusControl
        color: on ? Theme.accent : Theme.raised
        border.width: on ? 0 : 1
        border.color: Theme.hairline
        opacity: enabled ? 1 : 0.5
        Behavior on color { ColorAnimation { duration: Theme.quick } }
        RowLayout {
            anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
            spacing: Theme.s2
            Glyph { name: parent.parent.glyph; size: 14; weight: 1.6; color: parent.parent.on ? Theme.onAccent : Theme.text }
            Label { text: parent.parent.label; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.parent.on ? Theme.onAccent : Theme.text; Layout.fillWidth: true }
        }
        MouseArea { anchors.fill: parent; enabled: parent.enabled; cursorShape: Qt.PointingHandCursor; onClicked: parent.toggled(!parent.on) }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s2 + 2

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: Theme.s2 - 2
            rowSpacing: Theme.s2 - 2
            Pill { glyph: "wifi"; label: "Wi-Fi"; on: Network.wifiAvailable && Network.wifiEnabled; enabled: Network.wifiAvailable; onToggled: v => Network.setWifiEnabled(v) }
            Pill { glyph: "bluetooth"; label: "Bluetooth"; on: Bluetooth.enabled; enabled: Bluetooth.available; onToggled: v => Bluetooth.setEnabled(v) }
            Pill { glyph: "bell-off"; label: "Do not disturb"; on: Notifications.dnd; onToggled: v => Notifications.setDnd(v) }
            Pill { glyph: "moon"; label: "Night light"; on: NightLight.on; enabled: NightLight.available; onToggled: v => NightLight.setOn(v) }
            Pill { glyph: "gamepad-2"; label: "Game mode"; on: Modes.game; onToggled: v => Modes.toggle("game") }
        }

        RowLayout {
            spacing: Theme.s2
            Glyph { name: Audio.muted ? "volume-x" : "volume-2"; size: 14; MouseArea { anchors.fill: parent; onClicked: Audio.setMuted(!Audio.muted) } }
            Slider { Layout.fillWidth: true; value: Audio.muted ? 0 : Audio.volume; onMoved: v => { Audio.setMuted(false); Audio.setVolume(v); } }
            Label { text: Audio.muted ? "0" : Math.round(Audio.volume * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
        }
        RowLayout {
            visible: Brightness.available
            spacing: Theme.s2
            Glyph { name: "sun"; size: 14 }
            Slider { Layout.fillWidth: true; value: Brightness.value; onMoved: v => Brightness.set(v) }
            Label { text: Math.round(Brightness.value * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
        }
        Item { Layout.fillHeight: true }
    }
}
