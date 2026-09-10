import QtQuick
import QtQuick.Layouts
import qs.theme

// text / raised / accent / danger. Optional leading glyph.
Rectangle {
    id: root
    property string text
    property string glyph: ""
    property string variant: "raised"
    property bool enabled: true
    signal clicked

    readonly property bool isText: variant === "text"
    readonly property color fg: variant === "accent" || variant === "danger" ? Theme.onAccent : isText ? Theme.text2 : Theme.text

    implicitHeight: 34
    implicitWidth: row.implicitWidth + (text === "" ? Theme.s3 : Theme.s4) * 2
    radius: Theme.radiusControl
    opacity: enabled ? 1 : 0.45
    color: variant === "accent" ? Theme.accent : variant === "danger" ? Theme.danger
         : isText ? "transparent" : (area.containsMouse ? Theme.pressed : Theme.raised)
    border.width: variant === "raised" ? 1 : 0
    border.color: Theme.hairline
    Behavior on color { ColorAnimation { duration: Theme.quick } }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Theme.s2
        Glyph { visible: root.glyph !== ""; name: root.glyph; size: 14; color: root.fg }
        Label { visible: root.text !== ""; text: root.text; weight: Font.DemiBold; color: root.fg }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        enabled: root.enabled
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
