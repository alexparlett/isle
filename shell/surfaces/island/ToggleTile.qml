import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// A control panel tile: glyph, label, state line. Click toggles; the chevron opens the drill-down.
Rectangle {
    id: root
    property string glyph
    property string label
    property string sub: ""
    property bool on: false
    property bool enabled: true
    property bool hasMore: false
    signal toggled(bool on)
    signal more

    Layout.fillWidth: true
    implicitHeight: 56
    radius: Theme.radiusCard
    color: on ? Theme.accent : area.containsMouse ? Theme.pressed : Theme.raised
    border.width: on ? 0 : 1
    border.color: Theme.hairline
    opacity: enabled ? 1 : 0.5
    Behavior on color { ColorAnimation { duration: Theme.quick } }

    readonly property color fg: on ? Theme.onAccent : Theme.text
    readonly property color fg2: on ? Qt.alpha(Theme.onAccent, 0.7) : Theme.text2

    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s2 }
        spacing: Theme.s2 + 2
        Glyph { name: root.glyph; size: 14; weight: 1.6; color: root.fg }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Label { text: root.label; weight: Font.DemiBold; color: root.fg; Layout.fillWidth: true }
            Label { text: root.sub; size: Theme.sizeCaption; color: root.fg2; Layout.fillWidth: true; visible: root.sub !== "" }
        }
        Item {
            visible: root.hasMore
            implicitWidth: 24; implicitHeight: 24
            Glyph { anchors.centerIn: parent; name: "chevron-right"; size: 14; color: root.fg2 }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.more() }
        }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        z: -1
        enabled: root.enabled
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled(!root.on)
    }
}
