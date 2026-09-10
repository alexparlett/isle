import QtQuick
import QtQuick.Layouts
import qs.theme

// A text input: raised, hairline, accent hairline when focused, optional leading glyph and placeholder.
Rectangle {
    id: root
    property alias text: input.text
    property alias input: input
    property string placeholder: ""
    property string glyph: ""
    property int size: Theme.sizeBody
    signal accepted
    // Tab moves to this Field; Shift+Tab comes back.
    property var next: null

    implicitHeight: 40
    radius: Theme.radiusControl
    color: Theme.raised
    border.width: 1
    border.color: input.activeFocus ? Theme.accent : Theme.hairline
    Behavior on border.color { ColorAnimation { duration: Theme.quick } }

    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
        spacing: Theme.s2 + 2
        Glyph { visible: root.glyph !== ""; name: root.glyph; size: root.size; color: input.activeFocus ? Theme.text2 : Theme.text3 }
        TextInput {
            id: input
            Layout.fillWidth: true
            color: Theme.text
            font.family: Theme.fontUi
            font.pixelSize: root.size
            font.weight: Font.Medium
            selectionColor: Theme.accent
            selectedTextColor: Theme.onAccent
            cursorVisible: activeFocus
            // Drag to select, double-click a word, triple-click the line, and keep the highlight through focus loss.
            selectByMouse: true
            persistentSelection: true
            mouseSelectionMode: TextInput.SelectCharacters
            clip: true
            renderType: Text.NativeRendering
            onAccepted: root.accepted()
            KeyNavigation.tab: root.next ? root.next.input : null
            Label { anchors.verticalCenter: parent.verticalCenter; visible: input.text === ""; text: root.placeholder; color: Theme.text3; weight: Font.Normal; size: root.size }
        }
    }
}
